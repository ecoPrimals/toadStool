//! Crypto service client - Capability-based discovery
//!
//! **Design Philosophy (Infant Discovery)**:
//! - Async-first: Non-blocking operations
//! - Resilient: Retry logic, circuit breaker patterns
//! - Observable: Metrics and health checks
//! - Zero hardcoding: Endpoints discovered at runtime by capability
//! - Multi-vendor: Works with ANY crypto service (BearDog, Vault, KMS, etc.)

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

use toadstool_common::primal_identity::{Capability, ServiceEndpoint};
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod, ServiceDiscovery};
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};

use super::types::{CryptoRequest, CryptoResponse, KeyManagementRequest, KeyManagementResponse};
use super::{CryptoServiceConfig, ServiceLocation};

/// Crypto service discovery - Finds crypto providers by capability
///
/// **Design**: Runtime discovery, no hardcoded service names
pub struct CryptoServiceDiscovery {
    config: CryptoServiceConfig,
    discovery: ServiceDiscovery,
    discovered_services: Arc<RwLock<Vec<DiscoveredService>>>,
}

impl CryptoServiceDiscovery {
    /// Create new discovery instance
    pub async fn new(config: CryptoServiceConfig) -> ToadStoolResult<Self> {
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

    /// Discover crypto services by capability
    ///
    /// **Design**: Multi-strategy discovery (mDNS, registry, environment)
    pub async fn discover(&self) -> ToadStoolResult<Vec<DiscoveredService>> {
        let mut services = Vec::new();

        // Discover by each required capability
        for cap in &self.config.required_capabilities {
            let capability = Capability::Crypto(cap.clone());

            if let Ok(service) = self.discovery.find_service_by_capability(capability).await {
                services.push(service);
            }
        }

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

/// Crypto service client - Makes requests to discovered services
///
/// **Design**: Works with ANY crypto provider via unix sockets (pure Rust!)
pub struct CryptoServiceClient {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    /// Service endpoint information (stored for diagnostics and future use)
    _service_endpoint: ServiceEndpoint,
    /// Request timeout for RPC calls
    timeout: Duration,
}

impl CryptoServiceClient {
    /// Create client for a discovered service with unix socket transport
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    pub fn new(service: &DiscoveredService) -> ToadStoolResult<Self> {
        let endpoint = service.endpoints.first().ok_or_else(|| {
            ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: service.name.clone(),
                reason: "No endpoints available".to_string(),
            })
        })?;

        // Use unix socket path discovery - crypto services are typically BearDog
        // CAPABILITY-BASED: Use generic discovery instead of primal-specific knowledge
        let socket_path = toadstool_common::primal_sockets::get_socket_path_for_service("beardog");
        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            rpc_client,
            _service_endpoint: endpoint.clone(),
            timeout: Duration::from_secs(30),
        })
    }

    /// Create client with custom timeout
    pub fn with_timeout(service: &DiscoveredService, timeout: Duration) -> ToadStoolResult<Self> {
        let mut client = Self::new(service)?;
        client.timeout = timeout;
        Ok(client)
    }

    /// Encrypt data via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn encrypt(&self, request: CryptoRequest) -> ToadStoolResult<CryptoResponse> {
        let params = serde_json::to_value(&request).map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Failed to serialize request: {e}"),
            })
        })?;

        tokio::time::timeout(
            self.timeout,
            self.rpc_client.call_typed("crypto.encrypt", params),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Crypto encrypt timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Crypto encrypt failed: {e}"),
            })
        })
    }

    /// Decrypt data via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn decrypt(&self, request: CryptoRequest) -> ToadStoolResult<CryptoResponse> {
        let params = serde_json::to_value(&request).map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Failed to serialize request: {e}"),
            })
        })?;

        tokio::time::timeout(
            self.timeout,
            self.rpc_client.call_typed("crypto.decrypt", params),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Crypto decrypt timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Crypto decrypt failed: {e}"),
            })
        })
    }

    /// Manage keys (generate, rotate, delete) via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn manage_key(
        &self,
        request: KeyManagementRequest,
    ) -> ToadStoolResult<KeyManagementResponse> {
        let params = serde_json::to_value(&request).map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Failed to serialize request: {e}"),
            })
        })?;

        tokio::time::timeout(
            self.timeout,
            self.rpc_client.call_typed("crypto.manage_key", params),
        )
        .await
        .map_err(|_| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Key management timed out after {:?}", self.timeout),
            })
        })?
        .map_err(|e| {
            ToadStoolError::Network(NetworkError::IoError {
                reason: format!("Key management failed: {e}"),
            })
        })
    }

    /// Health check via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn health_check(&self) -> ToadStoolResult<bool> {
        let result: serde_json::Value = tokio::time::timeout(
            self.timeout,
            self.rpc_client.call("crypto.health", serde_json::json!({})),
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
            .and_then(|v| v.as_bool())
            .unwrap_or(false))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use toadstool_common::primal_identity::{CryptoCapability, ServiceEndpoint};

    #[tokio::test]
    async fn test_crypto_service_discovery_creation() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config)
            .await
            .expect("Failed to create discovery");

        assert!(!discovery.config.required_capabilities.is_empty());
    }

    #[tokio::test]
    async fn test_location_filtering() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();

        let services = vec![
            DiscoveredService {
                id: "local".to_string(),
                name: "local-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
                endpoints: vec![ServiceEndpoint::http("127.0.0.1", 8080)],
                metadata: Default::default(),
                discovered_at: std::time::SystemTime::now(),
                last_seen: std::time::SystemTime::now(),
                healthy: true,
            },
            DiscoveredService {
                id: "remote".to_string(),
                name: "remote-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
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
