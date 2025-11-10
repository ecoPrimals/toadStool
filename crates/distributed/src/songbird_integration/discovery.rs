//! Network discovery and node management

use std::sync::Arc;
use std::time::Duration;

use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

use super::types::{
    CapabilityTracker, CoordinationStrategy, DiscoveryClient, SongbirdDiscoveryConfig, DistributionPlan,
    NetworkCapacity, NetworkHealthMonitor, NetworkStatus, NodeCapabilities, NodeId,
    NodeRegistration, NodeRegistry, NodeType, RegistrationResponse, SongbirdConnection,
    SongbirdNetworkDiscovery, SubTask, SubTaskPlan,
};

impl SongbirdNetworkDiscovery {
    pub async fn new(
        config: SongbirdDiscoveryConfig,
        connection: Arc<SongbirdConnection>,
    ) -> ToadStoolResult<Self> {
        let discovery_client = DiscoveryClient::new(connection.clone()).await?;
        let node_registry = RwLock::new(NodeRegistry::new());
        let capability_tracker = CapabilityTracker::new();
        let health_monitor = NetworkHealthMonitor::new(config.node_timeout);

        let discovery = Self {
            discovery_client,
            node_registry,
            capability_tracker,
            health_monitor,
        };

        // Start periodic discovery in a background task
        let discovery_clone = discovery.clone();
        let discovery_interval = config.discovery_interval;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(discovery_interval);
            loop {
                interval.tick().await;
                if let Err(e) = discovery_clone.perform_discovery().await {
                    eprintln!("Discovery failed: {e}");
                }
            }
        });

        Ok(discovery)
    }

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
        let mut node_index = 0;

        for subtask in subtasks {
            // Find best matching node for this subtask
            let best_node = Self::find_best_node_for_subtask_static(subtask, &available_nodes)?;

            subtask_plans.push(SubTaskPlan {
                subtask_id: subtask.id,
                target_nodes: vec![best_node.node_id.clone()],
                resource_allocation: subtask.resource_requirements.clone(),
                dependencies: Vec::new(), // Simplified for now
            });

            node_index = (node_index + 1) % available_nodes.len();
        }

        debug!(
            "Created distribution plan with {} subtask assignments",
            subtask_plans.len()
        );

        Ok(DistributionPlan {
            plan_id: Uuid::new_v4(),
            job_id: Uuid::new_v4(), // Should come from parent job
            subtasks: subtask_plans,
            coordination_strategy: CoordinationStrategy::Parallel,
        })
    }

    fn find_best_node_for_subtask_static<'a>(
        subtask: &SubTask,
        available_nodes: &'a [&NodeRegistration],
    ) -> ToadStoolResult<&'a NodeRegistration> {
        // Score nodes based on capability match and current load
        let mut best_node = None;
        let mut best_score = 0.0;

        for node in available_nodes {
            let mut score = 0.0;

            // CPU capability scoring
            if node.capabilities.cpu_cores >= subtask.resource_requirements.cpu.min_cores {
                score += 10.0;
                // Bonus for having more capacity than needed (better for load balancing)
                let excess_ratio =
                    node.capabilities.cpu_cores / subtask.resource_requirements.cpu.min_cores;
                score += (excess_ratio - 1.0).min(5.0);
            }

            // Memory capability scoring
            let required_memory_gb =
                subtask.resource_requirements.memory.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            if node.capabilities.memory_gb >= required_memory_gb {
                score += 8.0;
            }

            // Storage capability scoring
            let required_storage_gb =
                subtask.resource_requirements.storage.min_bytes as f64 / (1024.0 * 1024.0 * 1024.0);
            if node.capabilities.storage_gb >= required_storage_gb {
                score += 5.0;
            }

            // Specialized hardware bonus
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

    pub async fn register_node(
        &self,
        registration: NodeRegistration,
    ) -> ToadStoolResult<RegistrationResponse> {
        let mut registry = self.node_registry.write().await;

        // Validate registration
        if registration.node_id.is_empty() {
            return Err(ToadStoolError::runtime("Node ID cannot be empty"));
        }

        if registration.endpoints.is_empty() {
            return Err(ToadStoolError::runtime(
                "At least one endpoint must be provided",
            ));
        }

        // Register the node
        registry.register_node(registration.clone())?;

        // Note: Capability tracker updates would happen here in production

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

    pub async fn get_network_status(&self) -> ToadStoolResult<NetworkStatus> {
        let registry = self.node_registry.read().await;
        let all_nodes = registry.get_all_nodes();
        let active_nodes = registry.get_active_nodes();

        // Calculate total capacity
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

        // Calculate current utilization (simplified)
        let current_utilization = if total_capacity.cpu_cores > 0.0 {
            0.65 // Placeholder for actual utilization calculation
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

    async fn perform_discovery(&self) -> ToadStoolResult<()> {
        // Discover new nodes through Songbird
        let discovered_nodes = self.discovery_client.discover_nodes().await?;

        debug!("Discovered {} nodes", discovered_nodes.len());

        let mut registry = self.node_registry.write().await;
        for node in discovered_nodes {
            registry.update_node_health(&node.node_id, true);
        }

        Ok(())
    }
}

impl Clone for SongbirdNetworkDiscovery {
    fn clone(&self) -> Self {
        Self {
            discovery_client: self.discovery_client.clone(),
            node_registry: RwLock::new(NodeRegistry::new()), // Create new empty registry for clone
            capability_tracker: self.capability_tracker.clone(),
            health_monitor: self.health_monitor.clone(),
        }
    }
}

// Move DiscoveryClient full implementation here from types.rs
impl Clone for DiscoveryClient {
    fn clone(&self) -> Self {
        Self {
            connection: self.connection.clone(),
            http_client: self.http_client.clone(),
        }
    }
}

impl DiscoveryClient {
    pub async fn new(connection: Arc<SongbirdConnection>) -> ToadStoolResult<Self> {
        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ToadStoolError::runtime(format!("Failed to create HTTP client: {e}")))?;

        Ok(Self {
            connection,
            http_client,
        })
    }

    pub async fn discover_nodes(&self) -> ToadStoolResult<Vec<NodeRegistration>> {
        // Query Songbird for active nodes
        let discovery_url = format!("{}/api/v1/nodes/active", self.connection.active_endpoint);

        let mut request = self.http_client.get(&discovery_url);

        // Add authentication if available
        if let Some(ref token) = self.connection.auth_token {
            request = request.header("Authorization", format!("Bearer {token}"));
        }

        let response = request
            .send()
            .await
            .map_err(|e| ToadStoolError::runtime(format!("Discovery request failed: {e}")))?;

        if response.status().is_success() {
            // Parse response and convert to NodeRegistration format
            let nodes_json: serde_json::Value = response.json().await.map_err(|e| {
                ToadStoolError::runtime(format!("Failed to parse discovery response: {e}"))
            })?;

            let mut discovered_nodes = Vec::new();
            if let Some(nodes_array) = nodes_json.as_array() {
                for node_data in nodes_array {
                    if let Ok(node_reg) = self.parse_node_data(node_data) {
                        discovered_nodes.push(node_reg);
                    }
                }
            }

            Ok(discovered_nodes)
        } else {
            Err(ToadStoolError::runtime(format!(
                "Discovery failed with status: {}",
                response.status()
            )))
        }
    }

    fn parse_node_data(&self, node_data: &serde_json::Value) -> ToadStoolResult<NodeRegistration> {
        use super::types::NodeMetadata;

        let node_id = node_data["node_id"]
            .as_str()
            .ok_or_else(|| ToadStoolError::runtime("Missing node_id in discovery data"))?
            .to_string();

        let node_type = match node_data["type"].as_str().unwrap_or("ToadStool") {
            "ToadStool" => NodeType::ToadStool,
            "NestGate" => NodeType::NestGate,
            "BearDog" => NodeType::BearDog,
            "Songbird" => NodeType::Songbird,
            custom => NodeType::Custom(custom.to_string()),
        };

        let capabilities = NodeCapabilities {
            cpu_cores: node_data["capabilities"]["cpu_cores"]
                .as_f64()
                .unwrap_or(0.0),
            memory_gb: node_data["capabilities"]["memory_gb"]
                .as_f64()
                .unwrap_or(0.0),
            storage_gb: node_data["capabilities"]["storage_gb"]
                .as_f64()
                .unwrap_or(0.0),
            gpu_count: node_data["capabilities"]["gpu_count"].as_u64().unwrap_or(0) as u32,
            specialized_hardware: node_data["capabilities"]["specialized_hardware"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
            software_capabilities: node_data["capabilities"]["software_capabilities"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        };

        let endpoints = node_data["endpoints"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec!["unknown".to_string()]);

        let protocols = node_data["protocols"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec!["http".to_string()]);

        Ok(NodeRegistration {
            node_id,
            node_type,
            capabilities: capabilities.clone(),
            endpoints,
            protocols,
            metadata: NodeMetadata {
                version: node_data["version"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                build_info: node_data["build_info"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                capabilities,
            },
        })
    }
}

impl NetworkHealthMonitor {
    pub fn new(_timeout: Duration) -> Self {
        use std::collections::HashMap;
        Self {
            health_checks: HashMap::new(),
        }
    }
}

impl Clone for NetworkHealthMonitor {
    fn clone(&self) -> Self {
        use std::collections::HashMap;
        Self {
            health_checks: HashMap::new(),
        }
    }
}

impl CapabilityTracker {
    pub fn new() -> Self {
        use std::collections::HashMap;
        Self {
            capabilities: HashMap::new(),
        }
    }
}

impl Clone for CapabilityTracker {
    fn clone(&self) -> Self {
        use std::collections::HashMap;
        Self {
            capabilities: HashMap::new(),
        }
    }
}

impl NodeRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_active_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }

    pub fn get_all_nodes(&self) -> Vec<&NodeRegistration> {
        self.nodes.values().collect()
    }

    pub fn get_nodes_by_types(&self, types: &[NodeType]) -> Vec<&NodeRegistration> {
        self.nodes
            .values()
            .filter(|node| {
                types.iter().any(|t| {
                    matches!(
                        (t, &node.node_type),
                        (NodeType::ToadStool, NodeType::ToadStool)
                            | (NodeType::NestGate, NodeType::NestGate)
                            | (NodeType::BearDog, NodeType::BearDog)
                            | (NodeType::Songbird, NodeType::Songbird)
                    )
                })
            })
            .collect()
    }

    pub fn register_node(&mut self, registration: NodeRegistration) -> ToadStoolResult<()> {
        self.register(registration);
        Ok(())
    }

    pub fn update_node_health(&mut self, node_id: &NodeId, _healthy: bool) {
        // Mark node as active if it exists
        if self.nodes.contains_key(node_id) {
            // Node remains in registry as active
        }
    }
}
