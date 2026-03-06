// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coordination service client - RPC operations over unix socket
//!
//! **Design**: Works with ANY coordination provider via unix sockets (pure Rust!).
//! JSON-RPC over unix socket (no HTTP, no ring!).

use std::time::Duration;

use toadstool_common::constants::timeouts;
use toadstool_common::primal_identity::ServiceEndpoint;
use toadstool_common::service_discovery::DiscoveredService;
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};

use crate::coordination_integration::types::{
    CoordinationRequest, CoordinationResponse, HealthCheckRequest, LoadBalancingRequest, NodeInfo,
    ServiceRegistration,
};

/// Coordination service client - Makes requests to discovered services
///
/// **Design**: Works with ANY coordination provider via unix sockets (pure Rust!)
#[derive(Debug)]
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

    /// Create client with custom socket path (for testing error paths when service unavailable)
    #[cfg(test)]
    pub fn new_for_testing(socket_path: std::path::PathBuf, timeout: Duration) -> Self {
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
        let endpoint = toadstool_common::primal_identity::ServiceEndpoint {
            protocol: "unix".to_string(),
            address: "/tmp/test-coordination.sock".to_string(),
            port: 0,
            path: None,
            metadata: std::collections::HashMap::new(),
        };
        Self {
            rpc_client,
            _service_endpoint: endpoint,
            timeout,
        }
    }

    /// Register a service with the coordination provider via unix socket
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
    use crate::coordination_integration::types::{
        CoordinationResponse, HealthCheckRequest, LoadBalancingRequest, LoadBalancingStrategy,
        NodeInfo, ServiceRegistration,
    };
    use std::collections::HashMap;
    use toadstool_common::service_discovery::DiscoveredService;

    #[test]
    fn test_service_registration_serialization_for_rpc() {
        let reg = ServiceRegistration {
            service_id: "rpc-svc".to_string(),
            service_name: "RPC Test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec!["compute".to_string()],
            endpoints: vec![],
            metadata: HashMap::new(),
            ttl_seconds: 30,
        };
        let json = serde_json::to_value(&reg);
        assert!(json.is_ok());
    }

    #[test]
    fn test_health_check_request_serialization_for_rpc() {
        let mut metrics = HashMap::new();
        metrics.insert("cpu".to_string(), 0.75);
        let req = HealthCheckRequest {
            service_id: "hc-svc".to_string(),
            healthy: true,
            timestamp: 12345,
            metrics,
            message: Some("ok".to_string()),
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_load_balancing_request_serialization_for_rpc() {
        let req = LoadBalancingRequest {
            capability: "storage".to_string(),
            requested_capacity: Some(4),
            strategy: LoadBalancingStrategy::RoundRobin,
            metadata: HashMap::new(),
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_coordination_request_serialization_for_rpc() {
        use crate::coordination_integration::types::{CoordinationOperation, CoordinationRequest};
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
    fn test_discover_services_params_structure() {
        let params = serde_json::json!({"capability": "storage"});
        assert_eq!(params["capability"], "storage");
    }

    #[test]
    fn test_node_info_deserialization_for_discover_services() {
        let json = serde_json::json!({
            "node_id": "n1",
            "address": "127.0.0.1:8080",
            "capabilities": ["compute"],
            "status": "Healthy",
            "metadata": {},
            "last_health_check": 12345,
            "response_time_ms": 10
        });
        let parsed: Result<Vec<NodeInfo>, _> = serde_json::from_value(serde_json::json!([json]));
        assert!(parsed.is_ok());
        let nodes = parsed.unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node_id, "n1");
    }

    #[test]
    fn test_coordination_response_deserialization() {
        let json = serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000001",
            "success": true,
            "data": {},
            "metadata": {}
        });
        let parsed: Result<CoordinationResponse, _> = serde_json::from_value(json);
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_load_balancing_strategy_variants_serialization() {
        let strategies = [
            LoadBalancingStrategy::LeastConnections,
            LoadBalancingStrategy::LeastResponseTime,
            LoadBalancingStrategy::Random,
            LoadBalancingStrategy::WeightedRoundRobin {
                weights: vec![1, 2, 3],
            },
            LoadBalancingStrategy::ConsistentHash {
                key: "hash-key".to_string(),
            },
            LoadBalancingStrategy::Custom("custom-strategy".to_string()),
        ];
        for strategy in strategies {
            let json = serde_json::to_value(&strategy);
            assert!(json.is_ok());
        }
    }

    #[test]
    fn test_node_info_full_serialization() {
        use crate::coordination_integration::types::NodeStatus;
        use std::net::SocketAddr;

        let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let node = NodeInfo {
            node_id: "node-1".to_string(),
            address: addr,
            capabilities: vec!["compute".to_string(), "storage".to_string()],
            status: NodeStatus::Healthy,
            metadata: HashMap::new(),
            last_health_check: Some(12345),
            response_time_ms: Some(10),
        };
        let json = serde_json::to_value(&node);
        assert!(json.is_ok());
        let parsed: Result<NodeInfo, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
        let p = parsed.unwrap();
        assert_eq!(p.node_id, "node-1");
        assert_eq!(p.response_time_ms, Some(10));
    }

    #[test]
    fn test_coordination_response_success_false() {
        let json = serde_json::json!({
            "request_id": "00000000-0000-0000-0000-000000000001",
            "success": false,
            "data": {"error": "something failed"},
            "metadata": {}
        });
        let parsed: Result<CoordinationResponse, _> = serde_json::from_value(json);
        assert!(parsed.is_ok());
        let resp = parsed.unwrap();
        assert!(!resp.success);
    }

    #[test]
    fn test_health_check_request_with_message() {
        let mut metrics = HashMap::new();
        metrics.insert("memory".to_string(), 0.5);
        let req = HealthCheckRequest {
            service_id: "svc".to_string(),
            healthy: false,
            timestamp: 99999,
            metrics,
            message: Some("degraded".to_string()),
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_coordination_operation_all_variants() {
        use crate::coordination_integration::types::{CoordinationOperation, CoordinationRequest};

        let ops = [
            CoordinationOperation::DeregisterService {
                service_id: "svc-1".to_string(),
            },
            CoordinationOperation::GetLoadBalancing {
                service_ids: vec!["a".to_string(), "b".to_string()],
            },
            CoordinationOperation::ReportHealth {
                service_id: "svc".to_string(),
                healthy: true,
            },
            CoordinationOperation::Subscribe {
                capability: "compute".to_string(),
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
            let json = serde_json::to_value(&req);
            assert!(json.is_ok());
        }
    }

    #[test]
    fn test_node_status_all_variants() {
        use crate::coordination_integration::types::NodeStatus;

        let _ = NodeStatus::Degraded;
        let _ = NodeStatus::Unhealthy;
        let _ = NodeStatus::Unknown;
    }

    #[tokio::test]
    async fn test_coordination_client_new_empty_endpoints_fails() {
        let service = DiscoveredService {
            id: "empty".to_string(),
            name: "empty-svc".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![],
            endpoints: vec![],
            metadata: HashMap::new(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        };
        let result = super::CoordinationClient::new(&service).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No endpoints") || err.to_string().contains("empty-svc"));
    }

    #[test]
    fn test_service_registration_serialization_roundtrip() {
        let reg = ServiceRegistration {
            service_id: "svc-1".to_string(),
            service_name: "Test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec!["compute".to_string()],
            endpoints: vec![],
            metadata: HashMap::new(),
            ttl_seconds: 60,
        };
        let json = serde_json::to_value(&reg).unwrap();
        let parsed: ServiceRegistration = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.service_id, reg.service_id);
        assert_eq!(parsed.ttl_seconds, 60);
    }

    #[test]
    fn test_health_check_request_roundtrip() {
        let mut metrics = HashMap::new();
        metrics.insert("cpu".to_string(), 0.5);
        let req = HealthCheckRequest {
            service_id: "svc".to_string(),
            healthy: true,
            timestamp: 12345,
            metrics,
            message: Some("ok".to_string()),
        };
        let json = serde_json::to_value(&req).unwrap();
        let parsed: HealthCheckRequest = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.service_id, "svc");
        assert!(parsed.healthy);
    }

    #[tokio::test]
    async fn test_register_service_unavailable() {
        let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-12345.sock");
        let client = super::CoordinationClient::new_for_testing(
            nonexistent,
            std::time::Duration::from_millis(100),
        );
        let reg = ServiceRegistration {
            service_id: "test-svc".to_string(),
            service_name: "Test".to_string(),
            version: "1.0".to_string(),
            capabilities: vec!["compute".to_string()],
            endpoints: vec![],
            metadata: HashMap::new(),
            ttl_seconds: 30,
        };
        let result = client.register_service(reg).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_discover_services_unavailable() {
        let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-discover.sock");
        let client = super::CoordinationClient::new_for_testing(
            nonexistent,
            std::time::Duration::from_millis(100),
        );
        let result = client.discover_services("compute").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_health_check_unavailable() {
        let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-health.sock");
        let client = super::CoordinationClient::new_for_testing(
            nonexistent,
            std::time::Duration::from_millis(100),
        );
        let result = client.health_check().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_load_balancing_unavailable() {
        let nonexistent = std::path::PathBuf::from("/tmp/nonexistent-coordination-lb.sock");
        let client = super::CoordinationClient::new_for_testing(
            nonexistent,
            std::time::Duration::from_millis(100),
        );
        let req = LoadBalancingRequest {
            capability: "storage".to_string(),
            requested_capacity: Some(4),
            strategy: LoadBalancingStrategy::RoundRobin,
            metadata: HashMap::new(),
        };
        let result = client.get_load_balancing(req).await;
        assert!(result.is_err());
    }
}
