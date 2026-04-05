// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination network discovery - main discovery orchestration

use std::sync::Arc;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

use crate::coordination::types::{
    CapabilityTracker, CoordinationConnection, CoordinationDiscoveryConfig,
    CoordinationNetworkDiscovery, CoordinationStrategy, DiscoveryClient, DistributionPlan,
    NetworkCapacity, NetworkHealthMonitor, NetworkStatus, NodeCapabilities, NodeRegistration,
    NodeRegistry, NodeType, RegistrationResponse, SubTask, SubTaskPlan,
};

impl CoordinationNetworkDiscovery {
    /// Start discovery with periodic background refresh against the coordination service socket.
    pub async fn new(
        config: CoordinationDiscoveryConfig,
        connection: Arc<CoordinationConnection>,
    ) -> ToadStoolResult<Self> {
        let discovery_client = DiscoveryClient::new(Arc::clone(&connection)).await?;
        let node_registry = RwLock::new(NodeRegistry::new());
        let capability_tracker = CapabilityTracker::new();
        let health_monitor = NetworkHealthMonitor::new(config.node_timeout);

        let discovery = Self {
            discovery_client,
            node_registry,
            capability_tracker,
            health_monitor,
        };

        let discovery_clone = discovery.clone();
        let discovery_interval = config.discovery_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(discovery_interval);
            loop {
                interval.tick().await;
                if let Err(e) = discovery_clone.perform_discovery().await {
                    tracing::error!("Discovery failed: {e}");
                }
            }
        });

        Ok(discovery)
    }

    /// Test-only constructor that bypasses async discovery (avoids block_on inside tokio runtime).
    #[cfg(test)]
    pub fn for_test(
        config: CoordinationDiscoveryConfig,
        connection: Arc<CoordinationConnection>,
        socket_path: std::path::PathBuf,
    ) -> Self {
        let discovery_client = DiscoveryClient::for_test(connection, socket_path);
        Self {
            discovery_client,
            node_registry: RwLock::new(NodeRegistry::new()),
            capability_tracker: CapabilityTracker::new(),
            health_monitor: NetworkHealthMonitor::new(config.node_timeout),
        }
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // nodes are refs from registry
    /// Aggregate CPU, memory, and storage across active registered nodes.
    pub async fn get_network_capacity(&self) -> ToadStoolResult<NetworkCapacity> {
        let registry = self.node_registry.read().await;
        let nodes = registry.get_active_nodes();

        let mut total_cpu_cores = 0.0;
        let mut total_memory_gb = 0.0;
        let mut total_storage_gb = 0.0;

        for node in &nodes {
            total_cpu_cores += node.capabilities.cpu_cores;
            total_memory_gb += node.capabilities.memory_gb;
            total_storage_gb += node.capabilities.storage_gb;
        }

        debug!(
            "Network capacity: {} nodes, {} CPU cores, {}GB memory",
            nodes.len(),
            total_cpu_cores,
            total_memory_gb
        );

        Ok(NetworkCapacity {
            total_nodes: nodes.len(),
            total_cpu_cores,
            total_memory_gb,
            total_storage_gb,
        })
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // available_nodes are refs from registry
    /// Assign each subtask to a best-matching node and return a distribution plan.
    pub async fn get_optimal_distribution(
        &self,
        subtasks: &[SubTask],
        preferred_types: &[NodeType],
    ) -> ToadStoolResult<DistributionPlan> {
        let registry = self.node_registry.read().await;
        let available_nodes = registry.get_nodes_by_types(preferred_types);

        if available_nodes.is_empty() {
            return Err(ToadStoolError::runtime(
                "No suitable nodes found for distribution",
            ));
        }

        let mut subtask_plans = Vec::new();

        for subtask in subtasks {
            let best_node = Self::find_best_node_for_subtask_static(subtask, &available_nodes)?;

            subtask_plans.push(SubTaskPlan {
                subtask_id: subtask.id,
                target_nodes: vec![best_node.node_id.clone()],
                resource_allocation: subtask.resource_requirements.clone(),
                dependencies: Vec::new(),
            });
        }

        debug!(
            "Created distribution plan with {} subtask assignments",
            subtask_plans.len()
        );

        Ok(DistributionPlan {
            plan_id: Uuid::new_v4(),
            job_id: Uuid::new_v4(),
            subtasks: subtask_plans,
            coordination_strategy: CoordinationStrategy::Parallel,
        })
    }

    fn find_best_node_for_subtask_static<'a>(
        subtask: &SubTask,
        available_nodes: &'a [&NodeRegistration],
    ) -> ToadStoolResult<&'a NodeRegistration> {
        let mut best_node = None;
        let mut best_score = 0.0;

        for node in available_nodes {
            let mut score = 0.0;

            if node.capabilities.cpu_cores >= subtask.resource_requirements.cpu.min_cores {
                score += 10.0;
                let excess_ratio =
                    node.capabilities.cpu_cores / subtask.resource_requirements.cpu.min_cores;
                score += (excess_ratio - 1.0).min(5.0);
            }

            let required_memory_gb =
                subtask.resource_requirements.memory.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            if node.capabilities.memory_gb >= required_memory_gb {
                score += 8.0;
            }

            let required_storage_gb =
                subtask.resource_requirements.storage.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            if node.capabilities.storage_gb >= required_storage_gb {
                score += 5.0;
            }

            for constraint in &subtask.constraints {
                if node
                    .capabilities
                    .specialized_hardware
                    .iter()
                    .any(|hw| hw.contains(constraint))
                {
                    score += 15.0;
                }
            }

            if score > best_score {
                best_score = score;
                best_node = Some(*node);
            }
        }

        best_node.ok_or_else(|| ToadStoolError::runtime("No suitable node found for subtask"))
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // Multiple operations on registry
    /// Register a node in the local registry and return channel assignments.
    pub async fn register_node(
        &self,
        registration: NodeRegistration,
    ) -> ToadStoolResult<RegistrationResponse> {
        let mut registry = self.node_registry.write().await;

        if registration.node_id.is_empty() {
            return Err(ToadStoolError::runtime("Node ID cannot be empty"));
        }

        if registration.endpoints.is_empty() {
            return Err(ToadStoolError::runtime(
                "At least one endpoint must be provided",
            ));
        }

        registry.register_node(registration.clone())?;

        debug!("Registered node: {}", registration.node_id);

        Ok(RegistrationResponse {
            node_id: registration.node_id,
            status: "registered".to_string(),
            assigned_channels: vec![
                "global".to_string(),
                format!("type_{:?}", registration.node_type),
            ],
        })
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // all_nodes/active_nodes are refs from registry
    /// Summarize node counts, pooled capacity, and estimated local utilization.
    pub async fn get_network_status(&self) -> ToadStoolResult<NetworkStatus> {
        let registry = self.node_registry.read().await;
        let all_nodes = registry.get_all_nodes();
        let active_nodes = registry.get_active_nodes();

        let mut total_capacity = NodeCapabilities {
            cpu_cores: 0.0,
            memory_gb: 0.0,
            storage_gb: 0.0,
            gpu_count: 0,
            specialized_hardware: Vec::new(),
            software_capabilities: Vec::new(),
        };

        for node in &active_nodes {
            total_capacity.cpu_cores += node.capabilities.cpu_cores;
            total_capacity.memory_gb += node.capabilities.memory_gb;
            total_capacity.storage_gb += node.capabilities.storage_gb;
            total_capacity.gpu_count += node.capabilities.gpu_count;
        }

        let current_utilization = if total_capacity.cpu_cores > 0.0 {
            let cpu_utilization = f64::from(
                toadstool_sysmon::cpu_usage(std::time::Duration::from_millis(50)).unwrap_or(0.0),
            ) / 100.0;

            let memory_utilization = toadstool_sysmon::memory_info()
                .map(|m| {
                    if m.total > 0 {
                        #[expect(
                            clippy::cast_precision_loss,
                            reason = "precision loss acceptable for this conversion"
                        )]
                        let pct = m.used as f64 / m.total as f64;
                        pct
                    } else {
                        0.0
                    }
                })
                .unwrap_or(0.0);

            (cpu_utilization * 0.6 + memory_utilization * 0.4).clamp(0.0, 1.0)
        } else {
            0.0
        };

        debug!(
            "Network status: {} total nodes, {} active nodes, {:.1}% utilization",
            all_nodes.len(),
            active_nodes.len(),
            current_utilization * 100.0
        );

        Ok(NetworkStatus {
            total_nodes: all_nodes.len(),
            active_nodes: active_nodes.len(),
            total_capacity,
            current_utilization,
        })
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "drop order is intentional"
    )] // Must hold lock during loop
    async fn perform_discovery(&self) -> ToadStoolResult<()> {
        let discovered_nodes = self.discovery_client.discover_nodes().await?;

        debug!("Discovered {} nodes", discovered_nodes.len());

        let mut registry = self.node_registry.write().await;
        for node in discovered_nodes {
            registry.update_node_health(&node.node_id, true);
        }

        Ok(())
    }
}

impl Clone for CoordinationNetworkDiscovery {
    fn clone(&self) -> Self {
        Self {
            discovery_client: self.discovery_client.clone(),
            node_registry: RwLock::new(NodeRegistry::new()),
            capability_tracker: self.capability_tracker.clone(),
            health_monitor: self.health_monitor.clone(),
        }
    }
}
