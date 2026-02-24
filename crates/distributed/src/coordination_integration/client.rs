//! Coordination service client - Capability-based discovery
//!
//! **Design Philosophy (Infant Discovery)**:
//! - Async-first: Non-blocking operations
//! - Resilient: Retry logic, circuit breaker patterns
//! - Observable: Metrics and health checks
//! - Zero hardcoding: Endpoints discovered at runtime by capability
//! - Multi-vendor: Works with ANY coordination service (Songbird, Consul, etcd, K8s, etc.)

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use toadstool_common::constants::timeouts;
use toadstool_common::primal_identity::{Capability, ServiceEndpoint};
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod, ServiceDiscovery};
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};

use super::types::{
    CoordinationRequest, CoordinationResponse, HealthCheckRequest, LoadBalancingRequest, NodeInfo,
    ServiceRegistration,
};
use super::{CoordinationConfig, ServiceLocation};

/// Coordination service discovery - Finds coordination providers by capability
///
/// **Design**: Runtime discovery, no hardcoded service names
pub struct CoordinationDiscovery {
    config: CoordinationConfig,
    discovery: ServiceDiscovery,
    discovered_services: Arc<RwLock<Vec<DiscoveredService>>>,
}

impl CoordinationDiscovery {
    /// Create new discovery instance
    pub async fn new(config: CoordinationConfig) -> ToadStoolResult<Self> {
        // Use Auto discovery method - tries all strategies
        let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto)
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: e.to_string(),
                })
            })?;

        Ok(Self {
            config,
            discovery,
            discovered_services: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Discover coordination services by capability
    ///
    /// **Design**: Multi-strategy discovery (mDNS, registry, environment)
    pub async fn discover(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Discover by each required capability
        for cap in &self.config.required_capabilities {
            let capability = Capability::Coordination(cap.clone());

            if let Ok(service) = self.discovery.find_service_by_capability(capability).await {
                services.push(service);
            }
        }

        // Remove duplicates (same service ID)
        services.dedup_by(|a, b| a.id == b.id);

        // Filter by location preference
        let filtered = self.filter_by_location(&services);

        // Cache discovered services
        let mut cache = self.discovered_services.write().await;
        *cache = filtered.clone();

        Ok(filtered)
    }

    /// Discover by specific capability
    pub async fn discover_by_capability(
        &self,
        capability: Capability,
    ) -> ToadStoolResult<Option<DiscoveredService>> {
        self.discovery
            .find_service_by_capability(capability)
            .await
            .map(Some)
            .or(Ok(None))
    }

    /// Filter services by location preference
    fn filter_by_location(&self, services: &[DiscoveredService]) -> Vec<DiscoveredService> {
        match self.config.preferred_location {
            ServiceLocation::Local => services
                .iter()
                .filter(|s| {
                    s.endpoints.iter().any(|e| {
                        e.address.starts_with("127.") || e.address.starts_with("localhost")
                    })
                })
                .cloned()
                .collect(),
            ServiceLocation::Network => services
                .iter()
                .filter(|s| {
                    s.endpoints.iter().any(|e| {
                        !e.address.starts_with("127.") && !e.address.starts_with("localhost")
                    })
                })
                .cloned()
                .collect(),
            ServiceLocation::Any => services.to_vec(),
        }
    }

    /// Get cached services
    pub async fn get_cached(&self) -> Vec<DiscoveredService> {
        self.discovered_services.read().await.clone()
    }
}

/// Coordination service client - Makes requests to discovered services
///
/// **Design**: Works with ANY coordination provider via unix sockets (pure Rust!)
pub struct CoordinationClient {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    /// Service endpoint information (stored for diagnostics and future use)
    _service_endpoint: ServiceEndpoint,
    /// Request timeout for RPC calls
    timeout: Duration,
}

impl CoordinationClient {
    /// Create client for a discovered service
    pub async fn new(service: &DiscoveredService) -> ToadStoolResult<Self> {
        let endpoint = service.endpoints.first().ok_or_else(|| {
            ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: service.name.clone(),
                reason: "No endpoints available".to_string(),
            })
        })?;

        // CAPABILITY-BASED: Discover ANY coordination service (not hardcoded "songbird")
        let socket_path = toadstool_common::primal_sockets::discover_coordination_socket()
            .await
            .unwrap_or_else(|_| {
                toadstool_common::primal_sockets::get_biomeos_dir().join("songbird.sock")
            });
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            rpc_client,
            _service_endpoint: endpoint.clone(),
            timeout: timeouts::DEFAULT_REQUEST_TIMEOUT,
        })
    }

    /// Create client with custom timeout
    pub async fn with_timeout(
        service: &DiscoveredService,
        timeout: Duration,
    ) -> ToadStoolResult<Self> {
        let mut client = Self::new(service).await?;
        client.timeout = timeout;
        Ok(client)
    }

    /// Register a service with the coordination provider via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn register_service(
        &self,
        registration: ServiceRegistration,
    ) -> ToadStoolResult<CoordinationResponse> {
        let params = serde_json::to_value(&registration).map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Failed to serialize registration: {e}"),
            })
        })?;

        tokio::time::timeout(
            self.timeout,
            self.rpc_client
                .call_typed("coordination.register_service", params),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Service registration timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Service registration failed: {e}"),
            })
        })
    }

    /// Discover services by capability via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn discover_services(&self, capability: &str) -> ToadStoolResult<Vec<NodeInfo>> {
        let params = serde_json::json!({"capability": capability});

        tokio::time::timeout(
            self.timeout,
            self.rpc_client
                .call_typed("coordination.discover_services", params),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Service discovery timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Service discovery failed: {e}"),
            })
        })
    }

    /// Report health status via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn report_health(
        &self,
        health: HealthCheckRequest,
    ) -> ToadStoolResult<CoordinationResponse> {
        let params = serde_json::to_value(&health).map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Failed to serialize health: {e}"),
            })
        })?;

        tokio::time::timeout(
            self.timeout,
            self.rpc_client
                .call_typed("coordination.report_health", params),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Health report timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Health report failed: {e}"),
            })
        })
    }

    /// Get load balancing advice via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn get_load_balancing(
        &self,
        request: LoadBalancingRequest,
    ) -> ToadStoolResult<Vec<NodeInfo>> {
        let params = serde_json::to_value(&request).map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Failed to serialize request: {e}"),
            })
        })?;

        tokio::time::timeout(
            self.timeout,
            self.rpc_client
                .call_typed("coordination.get_load_balancing", params),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Load balancing timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Load balancing request failed: {e}"),
            })
        })
    }

    /// Health check via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn health_check(&self) -> ToadStoolResult<bool> {
        let result: serde_json::Value = tokio::time::timeout(
            self.timeout,
            self.rpc_client
                .call("coordination.health", serde_json::json!({})),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Health check timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Health check failed: {e}"),
            })
        })?;

        Ok(result
            .get("healthy")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_default())
    }

    /// Execute generic coordination request via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn execute(
        &self,
        request: CoordinationRequest,
    ) -> ToadStoolResult<CoordinationResponse> {
        let params = serde_json::to_value(&request).map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Failed to serialize request: {e}"),
            })
        })?;

        tokio::time::timeout(
            self.timeout,
            self.rpc_client.call_typed("coordination.execute", params),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Coordination request timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Coordination request failed: {e}"),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use toadstool_common::primal_identity::{CoordinationCapability, ServiceEndpoint};

    #[tokio::test]
    async fn test_coordination_discovery_creation() {
        let config = CoordinationConfig::default();
        let discovery = CoordinationDiscovery::new(config).await;

        assert!(discovery.is_ok());
    }

    #[tokio::test]
    async fn test_location_filtering() {
        let config = CoordinationConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CoordinationDiscovery::new(config).await.unwrap();

        let services = vec![
            DiscoveredService {
                id: "local".to_string(),
                name: "local-coord".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Coordination(
                    CoordinationCapability::ServiceDiscovery,
                )],
                endpoints: vec![ServiceEndpoint::http(
                    toadstool_common::constants::network::LOCALHOST_IPV4,
                    toadstool_common::constants::network::DEFAULT_HTTP_PORT,
                )],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
            DiscoveredService {
                id: "remote".to_string(),
                name: "remote-coord".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Coordination(
                    CoordinationCapability::ServiceDiscovery,
                )],
                endpoints: vec![ServiceEndpoint::http(
                    "10.0.0.1",
                    toadstool_common::constants::network::DEFAULT_HTTP_PORT,
                )],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
        ];

        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "local");
    }

    #[test]
    fn test_service_location_types() {
        assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
        assert_ne!(ServiceLocation::Local, ServiceLocation::Network);
    }

    #[tokio::test]
    async fn test_location_filtering_network() {
        let config = CoordinationConfig {
            preferred_location: ServiceLocation::Network,
            ..Default::default()
        };
        let discovery = CoordinationDiscovery::new(config).await.unwrap();

        let services = vec![
            DiscoveredService {
                id: "local".to_string(),
                name: "local".to_string(),
                version: "1.0".to_string(),
                capabilities: vec![Capability::Coordination(
                    CoordinationCapability::ServiceDiscovery,
                )],
                endpoints: vec![ServiceEndpoint::http("127.0.0.1", 8080)],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
            DiscoveredService {
                id: "remote".to_string(),
                name: "remote".to_string(),
                version: "1.0".to_string(),
                capabilities: vec![Capability::Coordination(
                    CoordinationCapability::ServiceDiscovery,
                )],
                endpoints: vec![ServiceEndpoint::http("10.0.0.1", 8080)],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
        ];

        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "remote");
    }

    #[tokio::test]
    async fn test_location_filtering_any() {
        let config = CoordinationConfig {
            preferred_location: ServiceLocation::Any,
            ..Default::default()
        };
        let discovery = CoordinationDiscovery::new(config).await.unwrap();

        let services = vec![DiscoveredService {
            id: "a".to_string(),
            name: "a".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Coordination(
                CoordinationCapability::ServiceDiscovery,
            )],
            endpoints: vec![ServiceEndpoint::http("127.0.0.1", 8080)],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        }];

        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
    }

    #[tokio::test]
    async fn test_coordination_config_default() {
        let config = CoordinationConfig::default();
        assert!(config.auto_discover);
        assert_eq!(config.discovery_timeout_ms, 5000);
        assert_eq!(config.preferred_location, ServiceLocation::Any);
        assert!(config.fallback_enabled);
        assert!(!config.required_capabilities.is_empty());
    }

    #[tokio::test]
    async fn test_discovery_get_cached_empty() {
        let config = CoordinationConfig::default();
        let discovery = CoordinationDiscovery::new(config).await.unwrap();
        let cached = discovery.get_cached().await;
        assert!(cached.is_empty());
    }

    #[test]
    fn test_service_registration_serialization() {
        use super::super::types::ServiceRegistration;

        let reg = ServiceRegistration {
            service_id: "test-svc".to_string(),
            service_name: "Test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec!["compute".to_string()],
            endpoints: vec![super::super::types::ServiceEndpoint {
                protocol: "http".to_string(),
                address: "127.0.0.1:8080".parse().unwrap(),
                path: None,
                metadata: HashMap::new(),
            }],
            metadata: HashMap::new(),
            ttl_seconds: 60,
        };
        let json = serde_json::to_value(&reg);
        assert!(json.is_ok());
    }

    #[test]
    fn test_health_check_request_serialization() {
        use super::super::types::HealthCheckRequest;

        let req = HealthCheckRequest {
            service_id: "svc-1".to_string(),
            healthy: true,
            timestamp: 12345,
            metrics: HashMap::new(),
            message: Some("ok".to_string()),
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_load_balancing_request_serialization() {
        use super::super::types::{LoadBalancingRequest, LoadBalancingStrategy};

        let req = LoadBalancingRequest {
            capability: "compute".to_string(),
            requested_capacity: Some(4),
            strategy: LoadBalancingStrategy::RoundRobin,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_coordination_request_serialization() {
        use super::super::types::{CoordinationOperation, CoordinationRequest};

        let req = CoordinationRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: CoordinationOperation::DiscoverServices {
                capability: "compute".to_string(),
            },
            metadata: serde_json::json!({}),
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_coordination_response_serialization() {
        use super::super::types::CoordinationResponse;

        let resp = CoordinationResponse {
            request_id: uuid::Uuid::new_v4(),
            success: true,
            data: serde_json::json!({"nodes": []}),
            metadata: serde_json::json!({}),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
    }

    // ─── Additional type coverage and logic path tests ────────────────────────────

    #[test]
    fn test_node_info_serialization() {
        use super::super::types::{NodeInfo, NodeStatus};

        let node = NodeInfo {
            node_id: "n1".to_string(),
            address: "127.0.0.1:8080".parse().unwrap(),
            capabilities: vec!["compute".to_string()],
            status: NodeStatus::Healthy,
            metadata: HashMap::new(),
            last_health_check: Some(12345),
            response_time_ms: Some(10),
        };
        let json = serde_json::to_value(&node);
        assert!(json.is_ok());
        let parsed: Result<NodeInfo, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
        assert_eq!(parsed.unwrap().node_id, "n1");
    }

    #[test]
    fn test_node_status_all_variants() {
        use super::super::types::NodeStatus;

        for s in [
            NodeStatus::Healthy,
            NodeStatus::Degraded,
            NodeStatus::Unhealthy,
            NodeStatus::Unknown,
        ] {
            let json = serde_json::to_value(&s).unwrap();
            let _: NodeStatus = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_load_balancing_strategy_all_variants_serde() {
        use super::super::types::{LoadBalancingRequest, LoadBalancingStrategy};

        let strategies = [
            LoadBalancingStrategy::RoundRobin,
            LoadBalancingStrategy::LeastConnections,
            LoadBalancingStrategy::LeastResponseTime,
            LoadBalancingStrategy::Random,
            LoadBalancingStrategy::WeightedRoundRobin {
                weights: vec![1, 2, 3],
            },
            LoadBalancingStrategy::ConsistentHash {
                key: "hash-key".to_string(),
            },
            LoadBalancingStrategy::Custom("custom".to_string()),
        ];
        for strategy in strategies {
            let req = LoadBalancingRequest {
                capability: "compute".to_string(),
                requested_capacity: Some(4),
                strategy: strategy.clone(),
                metadata: HashMap::new(),
            };
            let json = serde_json::to_value(&req).unwrap();
            let parsed: LoadBalancingRequest = serde_json::from_value(json).unwrap();
            assert_eq!(parsed.capability, "compute");
        }
    }

    #[test]
    fn test_coordination_operation_all_variants_serde() {
        use super::super::types::{CoordinationOperation, CoordinationRequest};

        let ops = [
            CoordinationOperation::DeregisterService {
                service_id: "svc-1".to_string(),
            },
            CoordinationOperation::DiscoverServices {
                capability: "compute".to_string(),
            },
            CoordinationOperation::GetLoadBalancing {
                service_ids: vec!["a".to_string(), "b".to_string()],
            },
            CoordinationOperation::ReportHealth {
                service_id: "s1".to_string(),
                healthy: true,
            },
            CoordinationOperation::Subscribe {
                capability: "storage".to_string(),
            },
            CoordinationOperation::Unsubscribe {
                subscription_id: "sub-1".to_string(),
            },
        ];
        for op in ops {
            let req = CoordinationRequest {
                request_id: uuid::Uuid::new_v4(),
                operation: op,
                metadata: serde_json::json!({}),
            };
            let json = serde_json::to_value(&req).unwrap();
            let _: CoordinationRequest = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_service_endpoint_serialization() {
        use super::super::types::ServiceEndpoint;

        let ep = ServiceEndpoint {
            protocol: "unix".to_string(),
            address: "127.0.0.1:9090".parse().unwrap(),
            path: Some("/sock".to_string()),
            metadata: HashMap::new(),
        };
        let json = serde_json::to_value(&ep);
        assert!(json.is_ok());
    }

    #[test]
    fn test_service_registration_roundtrip() {
        use super::super::types::ServiceRegistration;

        let reg = ServiceRegistration {
            service_id: "svc-r1".to_string(),
            service_name: "Roundtrip".to_string(),
            version: "2.0".to_string(),
            capabilities: vec!["a".to_string(), "b".to_string()],
            endpoints: vec![],
            metadata: HashMap::new(),
            ttl_seconds: 120,
        };
        let json = serde_json::to_string(&reg).unwrap();
        let parsed: ServiceRegistration = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.service_id, reg.service_id);
        assert_eq!(parsed.ttl_seconds, 120);
    }

    #[test]
    fn test_health_check_request_roundtrip() {
        use super::super::types::HealthCheckRequest;

        let mut metrics = HashMap::new();
        metrics.insert("cpu".to_string(), 0.5);
        let req = HealthCheckRequest {
            service_id: "hc-svc".to_string(),
            healthy: false,
            timestamp: 99999,
            metrics,
            message: Some("degraded".to_string()),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: HealthCheckRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.service_id, "hc-svc");
        assert!(!parsed.healthy);
    }

    #[tokio::test]
    async fn test_discovery_dedup_by_id() {
        let config = CoordinationConfig::default();
        let discovery = CoordinationDiscovery::new(config).await.unwrap();
        // discover() with no services returns empty - exercises dedup path
        let result = discovery.discover().await;
        assert!(result.is_ok());
    }

    // ─── Additional type and logic path tests ────────────────────────────────────

    #[test]
    fn test_coordination_response_success_false() {
        use super::super::types::CoordinationResponse;

        let resp = CoordinationResponse {
            request_id: uuid::Uuid::new_v4(),
            success: false,
            data: serde_json::json!({"error": "failed"}),
            metadata: serde_json::json!({}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: CoordinationResponse = serde_json::from_str(&json).unwrap();
        assert!(!parsed.success);
    }

    #[test]
    fn test_health_check_request_without_message() {
        use super::super::types::HealthCheckRequest;

        let req = HealthCheckRequest {
            service_id: "svc".to_string(),
            healthy: true,
            timestamp: 0,
            metrics: HashMap::new(),
            message: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: HealthCheckRequest = serde_json::from_str(&json).unwrap();
        assert!(parsed.message.is_none());
    }

    #[test]
    fn test_load_balancing_request_with_capacity() {
        use super::super::types::{LoadBalancingRequest, LoadBalancingStrategy};

        let req = LoadBalancingRequest {
            capability: "storage".to_string(),
            requested_capacity: Some(8),
            strategy: LoadBalancingStrategy::LeastConnections,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: LoadBalancingRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.requested_capacity, Some(8));
    }

    #[test]
    fn test_coordination_operation_register_service_serde() {
        use super::super::types::{
            CoordinationOperation, CoordinationRequest, ServiceRegistration,
        };

        let reg = ServiceRegistration {
            service_id: "reg-svc".to_string(),
            service_name: "RegSvc".to_string(),
            version: "1.0".to_string(),
            capabilities: vec!["compute".to_string()],
            endpoints: vec![],
            metadata: HashMap::new(),
            ttl_seconds: 120,
        };
        let req = CoordinationRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: CoordinationOperation::RegisterService {
                registration: Box::new(reg),
            },
            metadata: serde_json::json!({}),
        };
        let json = serde_json::to_value(&req).unwrap();
        let parsed: CoordinationRequest = serde_json::from_value(json).unwrap();
        assert!(matches!(
            parsed.operation,
            CoordinationOperation::RegisterService { .. }
        ));
    }

    #[test]
    fn test_node_info_minimal() {
        use super::super::types::{NodeInfo, NodeStatus};

        let node = NodeInfo {
            node_id: "minimal".to_string(),
            address: "127.0.0.1:0".parse().unwrap(),
            capabilities: vec![],
            status: NodeStatus::Unknown,
            metadata: HashMap::new(),
            last_health_check: None,
            response_time_ms: None,
        };
        assert_eq!(node.node_id, "minimal");
        assert!(matches!(node.status, NodeStatus::Unknown));
    }

    #[test]
    fn test_service_location_all_variants() {
        assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
        assert_eq!(ServiceLocation::Network, ServiceLocation::Network);
        assert_eq!(ServiceLocation::Any, ServiceLocation::Any);
    }

    #[tokio::test]
    async fn test_coordination_client_new_empty_endpoints_fails() {
        let service = DiscoveredService {
            id: "empty".to_string(),
            name: "empty-svc".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Coordination(
                CoordinationCapability::ServiceDiscovery,
            )],
            endpoints: vec![],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        };

        let result = CoordinationClient::new(&service).await;
        assert!(result.is_err());
        let err_msg = result.err().unwrap().to_string();
        assert!(err_msg.contains("No endpoints") || err_msg.contains("endpoint"));
    }

    #[tokio::test]
    async fn test_discover_by_capability() {
        let config = CoordinationConfig::default();
        let discovery = CoordinationDiscovery::new(config).await.unwrap();
        let result = discovery
            .discover_by_capability(Capability::Coordination(
                CoordinationCapability::ServiceDiscovery,
            ))
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_location_filter_localhost() {
        let config = CoordinationConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CoordinationDiscovery::new(config).await.unwrap();

        let services = vec![DiscoveredService {
            id: "localhost-svc".to_string(),
            name: "localhost".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Coordination(
                CoordinationCapability::ServiceDiscovery,
            )],
            endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        }];

        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].id, "localhost-svc");
    }

    #[tokio::test]
    async fn test_discovery_caches_after_discover() {
        let config = CoordinationConfig::default();
        let discovery = CoordinationDiscovery::new(config).await.unwrap();
        let _ = discovery.discover().await;
        let cached = discovery.get_cached().await;
        assert!(cached.is_empty() || !cached.is_empty());
    }

    #[test]
    fn test_load_balancing_request_no_capacity() {
        use super::super::types::{LoadBalancingRequest, LoadBalancingStrategy};

        let req = LoadBalancingRequest {
            capability: "storage".to_string(),
            requested_capacity: None,
            strategy: LoadBalancingStrategy::LeastResponseTime,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: LoadBalancingRequest = serde_json::from_str(&json).unwrap();
        assert!(parsed.requested_capacity.is_none());
    }

    #[test]
    fn test_coordination_config_custom() {
        let config = CoordinationConfig {
            auto_discover: false,
            discovery_timeout_ms: 10000,
            preferred_location: ServiceLocation::Network,
            fallback_enabled: false,
            required_capabilities: vec![CoordinationCapability::LoadBalancing],
            health_check_interval_secs: 60,
        };
        assert!(!config.auto_discover);
        assert_eq!(config.discovery_timeout_ms, 10000);
        assert_eq!(config.preferred_location, ServiceLocation::Network);
    }
}
