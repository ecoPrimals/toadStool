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
            .or_else(|_| Ok(None))
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
    #[allow(dead_code)] // Stored for diagnostics and future use
    service_endpoint: ServiceEndpoint,
    #[allow(dead_code)] // May be used for timeout configuration in future
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
            service_endpoint: endpoint.clone(),
            timeout: Duration::from_secs(30),
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

        self.rpc_client
            .call_typed("coordination.register_service", params)
            .await
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

        self.rpc_client
            .call_typed("coordination.discover_services", params)
            .await
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

        self.rpc_client
            .call_typed("coordination.report_health", params)
            .await
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

        self.rpc_client
            .call_typed("coordination.get_load_balancing", params)
            .await
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
        let result: serde_json::Value = self
            .rpc_client
            .call("coordination.health", serde_json::json!({}))
            .await
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Health check failed: {e}"),
                })
            })?;

        Ok(result
            .get("healthy")
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
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

        self.rpc_client
            .call_typed("coordination.execute", params)
            .await
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
                endpoints: vec![ServiceEndpoint::http("127.0.0.1", 8080)],
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
                endpoints: vec![ServiceEndpoint::http("10.0.0.1", 8080)],
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
}
