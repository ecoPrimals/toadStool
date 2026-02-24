//! Network discovery and node management

use std::sync::Arc;
use std::time::Duration;

use sysinfo::System;
use toadstool::error::{ToadStoolError, ToadStoolResult};
use tokio::sync::RwLock;
use tracing::debug;
use uuid::Uuid;

use toadstool_common::constants::ecosystem::{node_type, well_known};

use super::types::{
    CapabilityTracker, CoordinationStrategy, DiscoveryClient, DistributionPlan, NetworkCapacity,
    NetworkHealthMonitor, NetworkStatus, NodeCapabilities, NodeId, NodeRegistration, NodeRegistry,
    NodeType, RegistrationResponse, SongbirdConnection, SongbirdDiscoveryConfig,
    SongbirdNetworkDiscovery, SubTask, SubTaskPlan,
};

impl SongbirdNetworkDiscovery {
    pub async fn new(
        config: SongbirdDiscoveryConfig,
        connection: Arc<SongbirdConnection>,
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

        // Start periodic discovery in a background task
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
        config: SongbirdDiscoveryConfig,
        connection: Arc<SongbirdConnection>,
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

        // Calculate current utilization from local system metrics
        // This represents this node's contribution to network utilization
        let current_utilization = if total_capacity.cpu_cores > 0.0 {
            // Use sysinfo to get actual CPU and memory utilization
            let mut sys = System::new();
            sys.refresh_cpu_usage();
            sys.refresh_memory();

            // CPU utilization: average across all cores (0.0 - 1.0)
            let cpu_utilization = sys.global_cpu_info().cpu_usage() / 100.0;

            // Memory utilization: used / total
            let memory_utilization = if sys.total_memory() > 0 {
                (sys.total_memory() - sys.available_memory()) as f64 / sys.total_memory() as f64
            } else {
                0.0
            };

            // Combined utilization (weighted average: 60% CPU, 40% memory)
            // This heuristic reflects that CPU is often the primary bottleneck
            (cpu_utilization as f64 * 0.6 + memory_utilization * 0.4).clamp(0.0, 1.0)
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
        // Clone uses fallback path since Clone trait is sync
        // In async contexts, use the async new() constructor instead
        let socket_path = toadstool_common::primal_sockets::get_biomeos_dir()
            .join(format!("{}.sock", well_known::SONGBIRD));

        Self {
            connection: Arc::clone(&self.connection),
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
        }
    }
}

impl DiscoveryClient {
    pub async fn new(connection: Arc<SongbirdConnection>) -> ToadStoolResult<Self> {
        // CAPABILITY-BASED: Discover ANY coordination service (not hardcoded "songbird")
        let socket_path = toadstool_common::primal_sockets::discover_coordination_socket()
            .await
            .unwrap_or_else(|_| {
                toadstool_common::primal_sockets::get_biomeos_dir()
                    .join(format!("{}.sock", well_known::SONGBIRD))
            });

        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            connection,
            rpc_client,
        })
    }

    /// Test-only constructor that bypasses async discovery (avoids block_on inside tokio runtime).
    #[cfg(test)]
    pub fn for_test(connection: Arc<SongbirdConnection>, socket_path: std::path::PathBuf) -> Self {
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
        Self {
            connection,
            rpc_client,
        }
    }

    pub async fn discover_nodes(&self) -> ToadStoolResult<Vec<NodeRegistration>> {
        // Query Songbird for active nodes via JSON-RPC over unix socket
        let mut params = serde_json::json!({});

        // Add authentication if available
        if let Some(ref token) = self.connection.auth_token {
            params["auth_token"] = serde_json::json!(token);
        }

        let nodes: Vec<NodeRegistration> = self
            .rpc_client
            .call_typed("songbird.discover_nodes", params)
            .await
            .unwrap_or_else(|e| {
                debug!("Discovery failed: {e}, returning empty list");
                // Graceful degradation - return empty list if discovery fails
                Vec::new()
            });

        Ok(nodes)
    }

    #[allow(dead_code)] // Was used by HTTP implementation, may be useful for debugging
    fn parse_node_data(&self, node_data: &serde_json::Value) -> ToadStoolResult<NodeRegistration> {
        use super::types::NodeMetadata;

        let node_id = node_data["node_id"]
            .as_str()
            .ok_or_else(|| ToadStoolError::runtime("Missing node_id in discovery data"))?
            .to_string();

        let type_str = node_data["type"].as_str().unwrap_or(node_type::TOADSTOOL);
        let parsed_node_type = match type_str {
            s if s == node_type::TOADSTOOL => NodeType::ToadStool,
            s if s == node_type::NESTGATE => NodeType::NestGate,
            s if s == node_type::BEARDOG => NodeType::BearDog,
            s if s == node_type::SONGBIRD => NodeType::Songbird,
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
            node_type: parsed_node_type,
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
    pub fn new(timeout: Duration) -> Self {
        use std::collections::HashMap;
        Self {
            health_checks: HashMap::new(),
            last_check: None,
            check_interval: timeout,
        }
    }
}

impl Clone for NetworkHealthMonitor {
    fn clone(&self) -> Self {
        use std::collections::HashMap;
        Self {
            health_checks: HashMap::new(),
            last_check: self.last_check,
            check_interval: self.check_interval,
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::songbird_integration::types::{
        ConnectionHealth, NodeCapabilities, NodeMetadata, NodeType, ProtocolConfig,
        SongbirdConnection, SongbirdDiscoveryConfig,
    };
    use crate::types::resources::{
        CpuRequirements, MemoryRequirements, NetworkRequirements, StorageRequirements,
    };
    use crate::types::ResourceRequirements;
    use std::sync::Arc;
    use toadstool_common::constants::ecosystem::node_type;

    fn make_protocol_config() -> ProtocolConfig {
        use crate::songbird_integration::types::{
            GrpcProtocolConfig, HttpProtocolConfig, MessageQueueProtocolConfig, SongbirdProtocol,
        };
        use std::collections::HashMap;

        ProtocolConfig {
            protocol: SongbirdProtocol::HTTP,
            http: HttpProtocolConfig {
                timeout_ms: 5000,
                max_retries: 3,
                headers: HashMap::new(),
            },
            grpc: GrpcProtocolConfig {
                timeout_ms: 5000,
                max_message_size: 1024 * 1024,
                compression: false,
            },
            message_queue: MessageQueueProtocolConfig {
                queue_name: "test".to_string(),
                exchange: "test".to_string(),
                routing_key: "test".to_string(),
            },
        }
    }

    fn make_songbird_connection() -> SongbirdConnection {
        SongbirdConnection {
            endpoints: vec!["unix:///tmp/test-songbird.sock".to_string()],
            active_endpoint: "unix:///tmp/test-songbird.sock".to_string(),
            auth_token: None,
            health_status: ConnectionHealth::Healthy,
            protocol_config: make_protocol_config(),
            #[cfg(feature = "channels")]
            reply_channel: None,
        }
    }

    fn make_node_registration(
        node_id: &str,
        node_type: NodeType,
        cpu: f64,
        memory_gb: f64,
        storage_gb: f64,
    ) -> NodeRegistration {
        let caps = NodeCapabilities {
            cpu_cores: cpu,
            memory_gb,
            storage_gb,
            gpu_count: 0,
            specialized_hardware: vec![],
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

    #[test]
    fn test_node_registry_new() {
        let registry = NodeRegistry::new();
        let active = registry.get_active_nodes();
        assert!(active.is_empty());
    }

    #[test]
    fn test_node_registry_register_and_get_active() {
        let mut registry = NodeRegistry::new();
        let reg = make_node_registration("node-1", NodeType::ToadStool, 4.0, 8.0, 100.0);
        registry.register_node(reg).unwrap();
        let active = registry.get_active_nodes();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].node_id, "node-1");
    }

    #[test]
    fn test_node_registry_get_all_nodes() {
        let mut registry = NodeRegistry::new();
        registry
            .register_node(make_node_registration(
                "a",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
            ))
            .unwrap();
        registry
            .register_node(make_node_registration(
                "b",
                NodeType::BearDog,
                1.0,
                2.0,
                25.0,
            ))
            .unwrap();
        let all = registry.get_all_nodes();
        assert_eq!(all.len(), 2);
    }

    #[test]
    fn test_node_registry_get_nodes_by_types() {
        let mut registry = NodeRegistry::new();
        registry
            .register_node(make_node_registration(
                "ts",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
            ))
            .unwrap();
        registry
            .register_node(make_node_registration(
                "bd",
                NodeType::BearDog,
                2.0,
                4.0,
                50.0,
            ))
            .unwrap();
        let toadstools = registry.get_nodes_by_types(&[NodeType::ToadStool]);
        assert_eq!(toadstools.len(), 1);
        assert_eq!(toadstools[0].node_id, "ts");
    }

    #[test]
    fn test_node_registry_update_node_health() {
        let mut registry = NodeRegistry::new();
        registry
            .register_node(make_node_registration(
                "n1",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
            ))
            .unwrap();
        registry.update_node_health(&"n1".to_string(), true);
        // No panic, node exists
    }

    #[test]
    fn test_network_health_monitor_new() {
        let monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
        let _ = monitor;
    }

    #[test]
    fn test_capability_tracker_new() {
        let tracker = CapabilityTracker::new();
        let _ = tracker;
    }

    fn make_discovery() -> (SongbirdNetworkDiscovery, std::path::PathBuf) {
        let config = SongbirdDiscoveryConfig {
            discovery_interval: Duration::from_secs(60),
            node_timeout: Duration::from_secs(30),
        };
        let conn = Arc::new(make_songbird_connection());
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("songbird.sock");
        let discovery = SongbirdNetworkDiscovery::for_test(config, conn, socket_path.clone());
        (discovery, socket_path)
    }

    #[test]
    fn test_songbird_discovery_for_test() {
        let (discovery, _) = make_discovery();
        let _ = discovery;
    }

    #[tokio::test]
    async fn test_songbird_discovery_register_node_success() {
        let (discovery, _) = make_discovery();

        let reg = make_node_registration("reg-node", NodeType::ToadStool, 4.0, 8.0, 100.0);
        let result = discovery.register_node(reg).await;
        assert!(result.is_ok());
        let resp = result.unwrap();
        assert_eq!(resp.node_id, "reg-node");
        assert_eq!(resp.status, "registered");
    }

    #[tokio::test]
    async fn test_songbird_discovery_register_node_empty_id() {
        let (discovery, _) = make_discovery();

        let mut reg = make_node_registration("x", NodeType::ToadStool, 4.0, 8.0, 100.0);
        reg.node_id = String::new();
        let result = discovery.register_node(reg).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("empty"));
        }
    }

    #[tokio::test]
    async fn test_songbird_discovery_register_node_empty_endpoints() {
        let (discovery, _) = make_discovery();

        let mut reg = make_node_registration("x", NodeType::ToadStool, 4.0, 8.0, 100.0);
        reg.endpoints = vec![];
        let result = discovery.register_node(reg).await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("endpoint"));
        }
    }

    #[tokio::test]
    async fn test_songbird_discovery_get_network_capacity_empty() {
        let (discovery, _) = make_discovery();

        let capacity = discovery.get_network_capacity().await.unwrap();
        assert_eq!(capacity.total_nodes, 0);
        assert_eq!(capacity.total_cpu_cores, 0.0);
        assert_eq!(capacity.total_memory_gb, 0.0);
    }

    #[tokio::test]
    async fn test_songbird_discovery_get_network_capacity_with_nodes() {
        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "n1",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
            ))
            .await
            .unwrap();
        discovery
            .register_node(make_node_registration(
                "n2",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
            ))
            .await
            .unwrap();

        let capacity = discovery.get_network_capacity().await.unwrap();
        assert_eq!(capacity.total_nodes, 2);
        assert!((capacity.total_cpu_cores - 6.0).abs() < 0.01);
        assert!((capacity.total_memory_gb - 12.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_songbird_discovery_get_optimal_distribution_no_nodes() {
        use crate::songbird_integration::types::SubTask;

        let (discovery, _) = make_discovery();

        let subtask = SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 2.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec![],
        };

        let result = discovery
            .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
            .await;
        assert!(result.is_err());
        if let Err(e) = result {
            assert!(e.to_string().contains("suitable") || e.to_string().contains("distribution"));
        }
    }

    #[tokio::test]
    async fn test_songbird_discovery_get_network_status() {
        let (discovery, _) = make_discovery();

        let status = discovery.get_network_status().await.unwrap();
        assert_eq!(status.total_nodes, 0);
        assert_eq!(status.active_nodes, 0);
    }

    #[test]
    fn test_discovery_client_parse_node_data_success() {
        let conn = Arc::new(make_songbird_connection());
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("songbird.sock");
        let client = DiscoveryClient::for_test(conn, socket_path);

        let node_json = serde_json::json!({
            "node_id": "parsed-node",
            "type": node_type::TOADSTOOL,
            "capabilities": {
                "cpu_cores": 8.0,
                "memory_gb": 16.0,
                "storage_gb": 256.0,
                "gpu_count": 1,
                "specialized_hardware": ["nvidia"],
                "software_capabilities": ["cuda"]
            },
            "endpoints": ["http://10.0.0.1:8080"],
            "protocols": ["http"],
            "version": "2.0",
            "build_info": "test-build"
        });

        let parsed = client.parse_node_data(&node_json).unwrap();
        assert_eq!(parsed.node_id, "parsed-node");
        assert!(matches!(parsed.node_type, NodeType::ToadStool));
        assert_eq!(parsed.capabilities.cpu_cores, 8.0);
        assert_eq!(parsed.capabilities.gpu_count, 1);
        assert_eq!(parsed.endpoints.len(), 1);
    }

    #[test]
    fn test_discovery_client_parse_node_data_missing_node_id() {
        let conn = Arc::new(make_songbird_connection());
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("songbird.sock");
        let client = DiscoveryClient::for_test(conn, socket_path);

        let node_json = serde_json::json!({
            "type": node_type::TOADSTOOL,
            "capabilities": {},
            "endpoints": ["http://x"]
        });

        let result = client.parse_node_data(&node_json);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("node_id"));
    }

    #[test]
    fn test_discovery_client_parse_node_data_custom_type() {
        let conn = Arc::new(make_songbird_connection());
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("songbird.sock");
        let client = DiscoveryClient::for_test(conn, socket_path);

        let node_json = serde_json::json!({
            "node_id": "custom-node",
            "type": "custom-type",
            "capabilities": {
                "cpu_cores": 1.0,
                "memory_gb": 2.0,
                "storage_gb": 10.0,
                "gpu_count": 0,
                "specialized_hardware": [],
                "software_capabilities": []
            },
            "endpoints": ["http://x"],
            "protocols": ["http"]
        });

        let parsed = client.parse_node_data(&node_json).unwrap();
        assert!(matches!(parsed.node_type, NodeType::Custom(s) if s == "custom-type"));
    }

    // ── Additional coverage: type constructors, serialization, node tracking, network status ──

    #[test]
    fn test_node_type_serialization() {
        let t = NodeType::ToadStool;
        let json = serde_json::to_string(&t).unwrap();
        let restored: NodeType = serde_json::from_str(&json).unwrap();
        assert!(matches!(restored, NodeType::ToadStool));

        let t2 = NodeType::Custom("my-type".to_string());
        let json2 = serde_json::to_string(&t2).unwrap();
        let restored2: NodeType = serde_json::from_str(&json2).unwrap();
        assert!(matches!(restored2, NodeType::Custom(s) if s == "my-type"));
    }

    #[test]
    fn test_node_registration_serialization_roundtrip() {
        let reg = make_node_registration("ser-node", NodeType::NestGate, 2.0, 4.0, 50.0);
        let json = serde_json::to_string(&reg).unwrap();
        let parsed: NodeRegistration = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.node_id, reg.node_id);
        assert!(matches!(parsed.node_type, NodeType::NestGate));
    }

    #[test]
    fn test_network_capacity_default_values() {
        let cap = NetworkCapacity {
            total_nodes: 0,
            total_cpu_cores: 0.0,
            total_memory_gb: 0.0,
            total_storage_gb: 0.0,
        };
        assert_eq!(cap.total_nodes, 0);
        assert_eq!(cap.total_storage_gb, 0.0);
    }

    #[test]
    fn test_registration_response_structure() {
        let resp = RegistrationResponse {
            node_id: "resp-node".to_string(),
            status: "registered".to_string(),
            assigned_channels: vec!["global".to_string(), "type_ToadStool".to_string()],
        };
        assert_eq!(resp.node_id, "resp-node");
        assert_eq!(resp.assigned_channels.len(), 2);
    }

    #[tokio::test]
    async fn test_songbird_discovery_get_optimal_distribution_with_nodes() {
        use crate::songbird_integration::types::SubTask;

        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "n1",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
            ))
            .await
            .unwrap();

        let subtask = SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![1, 2, 3],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 2.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec![],
        };

        let result = discovery
            .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
            .await;
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.subtasks.len(), 1);
        assert!(matches!(
            plan.coordination_strategy,
            CoordinationStrategy::Parallel
        ));
    }

    #[tokio::test]
    async fn test_songbird_discovery_get_network_status_with_nodes() {
        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "stat-node",
                NodeType::ToadStool,
                8.0,
                16.0,
                200.0,
            ))
            .await
            .unwrap();

        let status = discovery.get_network_status().await.unwrap();
        assert_eq!(status.total_nodes, 1);
        assert_eq!(status.active_nodes, 1);
        assert!((status.total_capacity.cpu_cores - 8.0).abs() < 0.01);
        assert!((status.total_capacity.memory_gb - 16.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_songbird_discovery_clone_creates_fresh_registry() {
        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "clone-test",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
            ))
            .await
            .unwrap();

        let cloned = discovery.clone();
        let capacity = cloned.get_network_capacity().await.unwrap();
        assert_eq!(capacity.total_nodes, 0, "Clone has fresh empty registry");
    }

    #[test]
    fn test_network_health_monitor_clone() {
        let monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
        let cloned = monitor.clone();
        assert_eq!(monitor.check_interval, cloned.check_interval);
    }

    #[test]
    fn test_capability_tracker_clone() {
        let tracker = CapabilityTracker::new();
        let _cloned = tracker.clone();
    }

    #[test]
    fn test_discovery_client_parse_node_data_nestgate_type() {
        let conn = Arc::new(make_songbird_connection());
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("songbird.sock");
        let client = DiscoveryClient::for_test(conn, socket_path);

        let node_json = serde_json::json!({
            "node_id": "nest-node",
            "type": node_type::NESTGATE,
            "capabilities": {
                "cpu_cores": 4.0,
                "memory_gb": 8.0,
                "storage_gb": 100.0,
                "gpu_count": 0,
                "specialized_hardware": [],
                "software_capabilities": []
            },
            "endpoints": ["http://nest:8080"],
            "protocols": ["http"]
        });

        let parsed = client.parse_node_data(&node_json).unwrap();
        assert!(matches!(parsed.node_type, NodeType::NestGate));
    }

    #[test]
    fn test_discovery_client_parse_node_data_beardog_type() {
        let conn = Arc::new(make_songbird_connection());
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("songbird.sock");
        let client = DiscoveryClient::for_test(conn, socket_path);

        let node_json = serde_json::json!({
            "node_id": "bd-node",
            "type": node_type::BEARDOG,
            "capabilities": {"cpu_cores": 1.0, "memory_gb": 2.0, "storage_gb": 10.0, "gpu_count": 0, "specialized_hardware": [], "software_capabilities": []},
            "endpoints": ["http://bd"],
            "protocols": ["http"]
        });

        let parsed = client.parse_node_data(&node_json).unwrap();
        assert!(matches!(parsed.node_type, NodeType::BearDog));
    }

    #[test]
    fn test_discovery_client_parse_node_data_songbird_type() {
        let conn = Arc::new(make_songbird_connection());
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("songbird.sock");
        let client = DiscoveryClient::for_test(conn, socket_path);

        let node_json = serde_json::json!({
            "node_id": "sb-node",
            "type": node_type::SONGBIRD,
            "capabilities": {"cpu_cores": 1.0, "memory_gb": 2.0, "storage_gb": 10.0, "gpu_count": 0, "specialized_hardware": [], "software_capabilities": []},
            "endpoints": ["http://sb"],
            "protocols": ["http"]
        });

        let parsed = client.parse_node_data(&node_json).unwrap();
        assert!(matches!(parsed.node_type, NodeType::Songbird));
    }

    #[test]
    fn test_node_registry_get_nodes_by_types_multiple() {
        let mut registry = NodeRegistry::new();
        registry
            .register_node(make_node_registration(
                "ts1",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
            ))
            .unwrap();
        registry
            .register_node(make_node_registration(
                "ts2",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
            ))
            .unwrap();
        registry
            .register_node(make_node_registration(
                "ng",
                NodeType::NestGate,
                1.0,
                2.0,
                25.0,
            ))
            .unwrap();

        let toadstools = registry.get_nodes_by_types(&[NodeType::ToadStool]);
        assert_eq!(toadstools.len(), 2);
        let nestgates = registry.get_nodes_by_types(&[NodeType::NestGate]);
        assert_eq!(nestgates.len(), 1);
    }

    #[test]
    fn test_node_registry_get_nodes_by_types_empty_filter() {
        let mut registry = NodeRegistry::new();
        registry
            .register_node(make_node_registration(
                "x",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
            ))
            .unwrap();
        let result = registry.get_nodes_by_types(&[]);
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_get_optimal_distribution_subtask_specialized_hardware_bonus() {
        use crate::songbird_integration::types::SubTask;

        let (discovery, _) = make_discovery();
        let mut reg = make_node_registration("gpu-node", NodeType::ToadStool, 8.0, 32.0, 500.0);
        reg.capabilities.specialized_hardware = vec!["nvidia".to_string()];
        discovery.register_node(reg).await.unwrap();

        let subtask = SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 2.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 1024 * 1024 * 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 1024 * 1024 * 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec!["nvidia".to_string()],
        };

        let result = discovery
            .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
            .await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_subtask_plan_structure() {
        use crate::songbird_integration::types::SubTaskPlan;
        let plan = SubTaskPlan {
            subtask_id: uuid::Uuid::new_v4(),
            target_nodes: vec!["node-1".to_string()],
            resource_allocation: ResourceRequirements::default(),
            dependencies: vec![],
        };
        assert_eq!(plan.target_nodes.len(), 1);
    }

    #[test]
    fn test_distribution_plan_structure() {
        use crate::songbird_integration::types::DistributionPlan;
        let plan = DistributionPlan {
            plan_id: uuid::Uuid::new_v4(),
            job_id: uuid::Uuid::new_v4(),
            subtasks: vec![],
            coordination_strategy: CoordinationStrategy::Parallel,
        };
        assert!(plan.subtasks.is_empty());
    }

    #[tokio::test]
    async fn test_get_optimal_distribution_multiple_subtasks() {
        use crate::songbird_integration::types::SubTask;

        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "n1",
                NodeType::ToadStool,
                8.0,
                16.0,
                200.0,
            ))
            .await
            .unwrap();
        discovery
            .register_node(make_node_registration(
                "n2",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
            ))
            .await
            .unwrap();

        let subtasks = vec![
            SubTask {
                id: uuid::Uuid::new_v4(),
                payload: vec![1],
                resource_requirements: ResourceRequirements {
                    cpu: CpuRequirements {
                        min_cores: 2.0,
                        max_cores: None,
                    },
                    memory: MemoryRequirements {
                        min_bytes: 1024,
                        max_bytes: None,
                    },
                    storage: StorageRequirements {
                        min_bytes: 1024,
                        max_bytes: None,
                    },
                    network: NetworkRequirements {
                        bandwidth_mbps: None,
                        latency_ms: None,
                    },
                    gpu: None,
                },
                priority: 0,
                constraints: vec![],
            },
            SubTask {
                id: uuid::Uuid::new_v4(),
                payload: vec![2],
                resource_requirements: ResourceRequirements {
                    cpu: CpuRequirements {
                        min_cores: 2.0,
                        max_cores: None,
                    },
                    memory: MemoryRequirements {
                        min_bytes: 1024,
                        max_bytes: None,
                    },
                    storage: StorageRequirements {
                        min_bytes: 1024,
                        max_bytes: None,
                    },
                    network: NetworkRequirements {
                        bandwidth_mbps: None,
                        latency_ms: None,
                    },
                    gpu: None,
                },
                priority: 0,
                constraints: vec![],
            },
        ];

        let result = discovery
            .get_optimal_distribution(&subtasks, &[NodeType::ToadStool])
            .await;
        assert!(result.is_ok());
        let plan = result.unwrap();
        assert_eq!(plan.subtasks.len(), 2);
    }

    #[test]
    fn test_discovery_client_parse_node_data_minimal() {
        let conn = Arc::new(make_songbird_connection());
        let temp_dir = tempfile::tempdir().expect("temp dir");
        let socket_path = temp_dir.path().join("songbird.sock");
        let client = DiscoveryClient::for_test(conn, socket_path);

        let node_json = serde_json::json!({
            "node_id": "minimal-node",
            "capabilities": {},
            "endpoints": ["http://localhost:8080"]
        });
        let result = client.parse_node_data(&node_json);
        assert!(result.is_ok());
        let parsed = result.unwrap();
        assert_eq!(parsed.node_id, "minimal-node");
    }

    #[test]
    fn test_node_registry_register_direct() {
        let mut registry = NodeRegistry::new();
        let reg = make_node_registration("direct-reg", NodeType::Songbird, 1.0, 2.0, 10.0);
        registry.register(reg);
        assert_eq!(registry.get_all_nodes().len(), 1);
    }

    #[tokio::test]
    async fn test_get_network_capacity_single_node() {
        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "single",
                NodeType::NestGate,
                2.0,
                4.0,
                50.0,
            ))
            .await
            .unwrap();
        let cap = discovery.get_network_capacity().await.unwrap();
        assert_eq!(cap.total_nodes, 1);
        assert!((cap.total_cpu_cores - 2.0).abs() < 0.01);
    }

    // ─── Priority 1: Node registration and tracking, health monitoring, capability matching, topology, recovery ───

    #[tokio::test]
    async fn test_node_registration_tracking_multiple_registrations() {
        let (discovery, _) = make_discovery();
        for i in 0..5 {
            discovery
                .register_node(make_node_registration(
                    &format!("node-{}", i),
                    NodeType::ToadStool,
                    4.0 + i as f64,
                    8.0,
                    100.0,
                ))
                .await
                .unwrap();
        }
        let capacity = discovery.get_network_capacity().await.unwrap();
        assert_eq!(capacity.total_nodes, 5);
        assert!((capacity.total_cpu_cores - (4.0 + 5.0 + 6.0 + 7.0 + 8.0)).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_node_registration_overwrites_same_node_id() {
        let mut registry = NodeRegistry::new();
        let reg1 = make_node_registration("dup-node", NodeType::ToadStool, 2.0, 4.0, 50.0);
        let reg2 = make_node_registration("dup-node", NodeType::BearDog, 8.0, 16.0, 200.0);
        registry.register_node(reg1).unwrap();
        registry.register_node(reg2).unwrap();
        let nodes = registry.get_active_nodes();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, "dup-node");
        assert!(matches!(nodes[0].node_type, NodeType::BearDog));
    }

    #[test]
    fn test_network_health_monitor_state_transition_healthy_to_degraded() {
        use crate::songbird_integration::types::ConnectionHealth;
        let mut monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
        let node_id = "n1".to_string();
        monitor.update_node_health(node_id.clone(), ConnectionHealth::Healthy);
        assert_eq!(monitor.get_node_health(&node_id), ConnectionHealth::Healthy);
        monitor.update_node_health(node_id.clone(), ConnectionHealth::Degraded);
        assert_eq!(
            monitor.get_node_health(&node_id),
            ConnectionHealth::Degraded
        );
    }

    #[test]
    fn test_network_health_monitor_state_transition_unhealthy_to_healthy() {
        use crate::songbird_integration::types::ConnectionHealth;
        let mut monitor = NetworkHealthMonitor::with_interval(Duration::from_secs(60));
        let node_id = "recovered".to_string();
        monitor.update_node_health(node_id.clone(), ConnectionHealth::Unhealthy);
        assert_eq!(
            monitor.get_node_health(&node_id),
            ConnectionHealth::Unhealthy
        );
        monitor.update_node_health(node_id.clone(), ConnectionHealth::Healthy);
        assert_eq!(monitor.get_node_health(&node_id), ConnectionHealth::Healthy);
        let healthy = monitor.healthy_nodes();
        assert_eq!(healthy.len(), 1);
        assert_eq!(healthy[0], "recovered");
    }

    #[test]
    fn test_network_health_monitor_remove_node() {
        use crate::songbird_integration::types::ConnectionHealth;
        let mut monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
        monitor.update_node_health("removed".to_string(), ConnectionHealth::Healthy);
        assert_eq!(
            monitor.get_node_health(&"removed".to_string()),
            ConnectionHealth::Healthy
        );
        monitor.remove_node(&"removed".to_string());
        assert_eq!(
            monitor.get_node_health(&"removed".to_string()),
            ConnectionHealth::Unknown
        );
    }

    #[test]
    fn test_network_health_monitor_unknown_for_unregistered_node() {
        use crate::songbird_integration::types::ConnectionHealth;
        let monitor = NetworkHealthMonitor::new(Duration::from_secs(30));
        assert_eq!(
            monitor.get_node_health(&"never-registered".to_string()),
            ConnectionHealth::Unknown
        );
    }

    #[tokio::test]
    async fn test_capability_matching_prefers_node_with_specialized_hardware() {
        use crate::songbird_integration::types::SubTask;

        let (discovery, _) = make_discovery();
        let mut gpu_node =
            make_node_registration("gpu-node", NodeType::ToadStool, 8.0, 32.0, 500.0);
        gpu_node.capabilities.specialized_hardware = vec!["nvidia".to_string()];
        discovery.register_node(gpu_node).await.unwrap();

        let mut cpu_node =
            make_node_registration("cpu-only", NodeType::ToadStool, 8.0, 32.0, 500.0);
        cpu_node.capabilities.specialized_hardware = vec![];
        discovery.register_node(cpu_node).await.unwrap();

        let subtask = SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 2.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 1024 * 1024 * 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 1024 * 1024 * 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec!["nvidia".to_string()],
        };

        let plan = discovery
            .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
            .await
            .unwrap();
        assert_eq!(plan.subtasks[0].target_nodes[0], "gpu-node");
    }

    #[tokio::test]
    async fn test_capability_matching_selects_higher_cpu_for_excess_ratio_bonus() {
        use crate::songbird_integration::types::SubTask;

        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "small",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
            ))
            .await
            .unwrap();
        discovery
            .register_node(make_node_registration(
                "large",
                NodeType::ToadStool,
                16.0,
                32.0,
                500.0,
            ))
            .await
            .unwrap();

        let subtask = SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 2.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec![],
        };

        let plan = discovery
            .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
            .await
            .unwrap();
        assert_eq!(plan.subtasks[0].target_nodes[0], "large");
    }

    #[tokio::test]
    async fn test_network_topology_capacity_aggregation() {
        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "a",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
            ))
            .await
            .unwrap();
        discovery
            .register_node(make_node_registration(
                "b",
                NodeType::NestGate,
                4.0,
                8.0,
                100.0,
            ))
            .await
            .unwrap();
        discovery
            .register_node(make_node_registration(
                "c",
                NodeType::BearDog,
                1.0,
                2.0,
                25.0,
            ))
            .await
            .unwrap();

        let capacity = discovery.get_network_capacity().await.unwrap();
        assert_eq!(capacity.total_nodes, 3);
        assert!((capacity.total_cpu_cores - 7.0).abs() < 0.01);
        assert!((capacity.total_memory_gb - 14.0).abs() < 0.01);
        assert!((capacity.total_storage_gb - 175.0).abs() < 0.01);
    }

    #[tokio::test]
    async fn test_recovery_from_node_failure_update_health_on_existing_node() {
        let mut registry = NodeRegistry::new();
        registry
            .register_node(make_node_registration(
                "failing-node",
                NodeType::ToadStool,
                4.0,
                8.0,
                100.0,
            ))
            .unwrap();
        registry.update_node_health(&"failing-node".to_string(), false);
        let nodes = registry.get_active_nodes();
        assert_eq!(nodes.len(), 1);
        registry.update_node_health(&"failing-node".to_string(), true);
        let nodes_after = registry.get_active_nodes();
        assert_eq!(nodes_after.len(), 1);
    }

    #[tokio::test]
    async fn test_recovery_update_health_on_nonexistent_node_no_panic() {
        let mut registry = NodeRegistry::new();
        registry
            .register_node(make_node_registration(
                "exists",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
            ))
            .unwrap();
        registry.update_node_health(&"nonexistent".to_string(), true);
        registry.update_node_health(&"nonexistent".to_string(), false);
        let nodes = registry.get_active_nodes();
        assert_eq!(nodes.len(), 1);
    }

    #[tokio::test]
    async fn test_get_optimal_distribution_prefers_nestgate_when_requested() {
        use crate::songbird_integration::types::SubTask;

        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "ts",
                NodeType::ToadStool,
                8.0,
                16.0,
                200.0,
            ))
            .await
            .unwrap();
        discovery
            .register_node(make_node_registration(
                "ng",
                NodeType::NestGate,
                8.0,
                16.0,
                200.0,
            ))
            .await
            .unwrap();

        let subtask = SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 2.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec![],
        };

        let plan = discovery
            .get_optimal_distribution(&[subtask], &[NodeType::NestGate])
            .await
            .unwrap();
        assert_eq!(plan.subtasks[0].target_nodes[0], "ng");
    }

    #[tokio::test]
    async fn test_registration_response_assigned_channels_include_type() {
        let (discovery, _) = make_discovery();
        let reg = make_node_registration("channels-test", NodeType::BearDog, 2.0, 4.0, 50.0);
        let resp = discovery.register_node(reg).await.unwrap();
        assert!(resp.assigned_channels.contains(&"global".to_string()));
        assert!(resp.assigned_channels.iter().any(|c| c.contains("BearDog")));
    }

    #[tokio::test]
    async fn test_find_best_node_no_node_meets_requirements_returns_error() {
        use crate::songbird_integration::types::SubTask;

        let (discovery, _) = make_discovery();
        discovery
            .register_node(make_node_registration(
                "weak",
                NodeType::ToadStool,
                1.0,
                1.0,
                10.0,
            ))
            .await
            .unwrap();

        let subtask = SubTask {
            id: uuid::Uuid::new_v4(),
            payload: vec![],
            resource_requirements: ResourceRequirements {
                cpu: CpuRequirements {
                    min_cores: 16.0,
                    max_cores: None,
                },
                memory: MemoryRequirements {
                    min_bytes: 64 * 1024 * 1024 * 1024,
                    max_bytes: None,
                },
                storage: StorageRequirements {
                    min_bytes: 100 * 1024 * 1024 * 1024,
                    max_bytes: None,
                },
                network: NetworkRequirements {
                    bandwidth_mbps: None,
                    latency_ms: None,
                },
                gpu: None,
            },
            priority: 0,
            constraints: vec![],
        };

        let result = discovery
            .get_optimal_distribution(&[subtask], &[NodeType::ToadStool])
            .await;
        assert!(result.is_err());
    }

    #[test]
    fn test_node_registry_get_node() {
        let mut registry = NodeRegistry::new();
        let reg = make_node_registration("lookup", NodeType::Songbird, 1.0, 2.0, 10.0);
        registry.register_node(reg.clone()).unwrap();
        let found = registry.get_node(&"lookup".to_string());
        assert!(found.is_some());
        assert_eq!(found.unwrap().node_id, "lookup");
    }

    #[test]
    fn test_node_registry_list_nodes() {
        let mut registry = NodeRegistry::new();
        registry
            .register_node(make_node_registration(
                "l1",
                NodeType::ToadStool,
                2.0,
                4.0,
                50.0,
            ))
            .unwrap();
        registry
            .register_node(make_node_registration(
                "l2",
                NodeType::BearDog,
                1.0,
                2.0,
                25.0,
            ))
            .unwrap();
        let list = registry.list_nodes();
        assert_eq!(list.len(), 2);
    }
}
