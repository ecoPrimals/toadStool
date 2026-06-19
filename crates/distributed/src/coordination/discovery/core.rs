// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination network discovery - main discovery orchestration

use std::sync::Arc;
use std::time::Duration;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

const CPU_USAGE_SAMPLE_WINDOW: Duration = Duration::from_millis(50);

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
            let cpu_utilization =
                f64::from(toadstool_sysmon::cpu_usage(CPU_USAGE_SAMPLE_WINDOW).unwrap_or(0.0))
                    / 100.0;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use std::time::Duration;

    use bytes::Bytes;
    use uuid::Uuid;

    use crate::ResourceRequirements;
    use crate::coordination::types::{
        ConnectionHealth, CoordinationConnection, CoordinationDiscoveryConfig,
        CoordinationTransport, HttpProtocolConfig, MessageQueueProtocolConfig, NodeCapabilities,
        NodeMetadata, NodeRegistration, NodeType, ProtocolConfig, SubTask,
    };
    use crate::types::resources::{
        CpuRequirements, MemoryRequirements, NetworkRequirements as NetReq, StorageRequirements,
    };

    fn make_protocol_config() -> ProtocolConfig {
        ProtocolConfig {
            protocol: CoordinationTransport::HTTP,
            http: HttpProtocolConfig {
                timeout_ms: 5000,
                max_retries: 3,
                headers: HashMap::new(),
            },
            message_queue: MessageQueueProtocolConfig {
                queue_name: "test".to_string(),
                exchange: "test".to_string(),
                routing_key: "test".to_string(),
            },
        }
    }

    fn test_connection() -> Arc<CoordinationConnection> {
        Arc::new(CoordinationConnection {
            endpoints: vec!["unix:///tmp/test-coordination.sock".to_string()],
            active_endpoint: "unix:///tmp/test-coordination.sock".to_string(),
            auth_token: None,
            health_status: ConnectionHealth::Healthy,
            protocol_config: make_protocol_config(),
            #[cfg(feature = "channels")]
            reply_channel: None,
        })
    }

    fn make_discovery() -> CoordinationNetworkDiscovery {
        let config = CoordinationDiscoveryConfig {
            discovery_interval: Duration::from_secs(3600),
            node_timeout: Duration::from_secs(30),
        };
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let socket_path = temp_dir.path().join("coordination.sock");
        CoordinationNetworkDiscovery::for_test(config, test_connection(), socket_path)
    }

    fn make_registration(
        node_id: &str,
        node_type: NodeType,
        cpu: f64,
        memory_gb: f64,
        storage_gb: f64,
        hw: Vec<String>,
    ) -> NodeRegistration {
        let caps = NodeCapabilities {
            cpu_cores: cpu,
            memory_gb,
            storage_gb,
            gpu_count: 0,
            specialized_hardware: hw,
            software_capabilities: vec![],
        };
        NodeRegistration {
            node_id: node_id.to_string(),
            node_type,
            capabilities: caps.clone(),
            endpoints: vec!["http://127.0.0.1:8080".to_string()],
            protocols: vec!["http".to_string()],
            metadata: NodeMetadata {
                version: "1.0".to_string(),
                build_info: "test".to_string(),
                capabilities: caps,
            },
        }
    }

    fn sample_subtask(
        min_cores: f64,
        memory_bytes: u64,
        storage_bytes: u64,
        constraints: Vec<String>,
    ) -> SubTask {
        SubTask {
            id: Uuid::new_v4(),
            payload: Bytes::new(),
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: memory_bytes,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: storage_bytes,
                    max_bytes: None,
                },
                network: NetReq {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 1,
            constraints,
        }
    }

    #[tokio::test]
    async fn register_node_rejects_empty_node_id() {
        let discovery = make_discovery();
        let mut reg = make_registration("", NodeType::ToadStool, 4.0, 8.0, 100.0, vec![]);
        reg.node_id = String::new();
        match discovery.register_node(reg).await {
            Err(e) => assert!(e.to_string().contains("Node ID cannot be empty"), "{e}"),
            Ok(_) => panic!("expected empty node id error"),
        }
    }

    #[tokio::test]
    async fn register_node_rejects_empty_endpoints() {
        let discovery = make_discovery();
        let mut reg = make_registration("n1", NodeType::ToadStool, 4.0, 8.0, 100.0, vec![]);
        reg.endpoints.clear();
        match discovery.register_node(reg).await {
            Err(e) => assert!(e.to_string().contains("At least one endpoint"), "{e}"),
            Ok(_) => panic!("expected empty endpoints error"),
        }
    }

    #[tokio::test]
    async fn get_network_capacity_sums_active_nodes() {
        let discovery = make_discovery();
        discovery
            .register_node(make_registration(
                "a",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
                vec![],
            ))
            .await
            .unwrap();
        discovery
            .register_node(make_registration(
                "b",
                NodeType::ToadStool,
                3.0,
                8.0,
                100.0,
                vec![],
            ))
            .await
            .unwrap();

        let cap = discovery.get_network_capacity().await.unwrap();
        assert_eq!(cap.total_nodes, 2);
        assert!((cap.total_cpu_cores - 5.0).abs() < f64::EPSILON);
        assert!((cap.total_memory_gb - 12.0).abs() < f64::EPSILON);
        assert!((cap.total_storage_gb - 150.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn get_optimal_distribution_assigns_best_scoring_node() {
        let discovery = make_discovery();
        discovery
            .register_node(make_registration(
                "weak",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
                vec![],
            ))
            .await
            .unwrap();
        discovery
            .register_node(make_registration(
                "strong",
                NodeType::ToadStool,
                16.0,
                64.0,
                500.0,
                vec!["gpu-a100".to_string()],
            ))
            .await
            .unwrap();

        let sub = sample_subtask(
            4.0,
            8 * 1024 * 1024 * 1024,
            100 * 1024 * 1024 * 1024,
            vec!["gpu-a100".to_string()],
        );
        let plan = discovery
            .get_optimal_distribution(&[sub], &[NodeType::ToadStool])
            .await
            .unwrap();
        assert_eq!(plan.subtasks.len(), 1);
        assert_eq!(plan.subtasks[0].target_nodes, vec!["strong".to_string()]);
    }

    #[tokio::test]
    async fn get_optimal_distribution_errors_when_no_nodes_for_preferred_types() {
        let discovery = make_discovery();
        discovery
            .register_node(make_registration(
                "store",
                NodeType::Storage,
                8.0,
                32.0,
                1000.0,
                vec![],
            ))
            .await
            .unwrap();

        let sub = sample_subtask(1.0, 1024, 1024, vec![]);
        let err = discovery
            .get_optimal_distribution(&[sub], &[NodeType::ToadStool])
            .await
            .unwrap_err();
        assert!(err.to_string().contains("No suitable nodes found"), "{err}");
    }

    #[tokio::test]
    async fn get_optimal_distribution_errors_when_no_node_meets_requirements() {
        let discovery = make_discovery();
        discovery
            .register_node(make_registration(
                "tiny",
                NodeType::ToadStool,
                1.0,
                0.5,
                0.5,
                vec![],
            ))
            .await
            .unwrap();

        let sub = sample_subtask(
            64.0,
            1024 * 1024 * 1024 * 1024,
            1024 * 1024 * 1024 * 1024,
            vec![],
        );
        let err = discovery
            .get_optimal_distribution(&[sub], &[NodeType::ToadStool])
            .await
            .unwrap_err();
        assert!(
            err.to_string()
                .contains("No suitable node found for subtask"),
            "{err}"
        );
    }

    #[tokio::test]
    async fn clone_creates_fresh_empty_node_registry() {
        let discovery = make_discovery();
        discovery
            .register_node(make_registration(
                "only",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
                vec![],
            ))
            .await
            .unwrap();
        let cloned = discovery.clone();
        let cap = cloned.get_network_capacity().await.unwrap();
        assert_eq!(cap.total_nodes, 0);
    }
}
