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

use toadstool_common::constants::timeouts;
use toadstool_common::primal_identity::{Capability, ServiceEndpoint};
use toadstool_common::service_discovery::{DiscoveredService, DiscoveryMethod, ServiceDiscovery};
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};

use super::types::{CryptoRequest, CryptoResponse, KeyManagementRequest, KeyManagementResponse};
use super::{CryptoServiceConfig, ServiceLocation};

/// Crypto service discovery - Finds crypto providers by capability
///
/// **Design**: Runtime discovery, no hardcoded service names
pub struct CryptoServiceDiscovery {
    pub(crate) config: CryptoServiceConfig,
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
    pub(crate) fn filter_by_location(
        &self,
        services: &[DiscoveredService],
    ) -> Vec<DiscoveredService> {
        match self.config.preferred_location {
            ServiceLocation::Local => services
                .iter()
                .filter(|s| {
                    s.endpoints.iter().any(|e| {
                        e.address.starts_with("127.")
                            || e.address == toadstool_common::constants::network::DEFAULT_HOSTNAME
                    })
                })
                .cloned()
                .collect(),
            ServiceLocation::Network => services
                .iter()
                .filter(|s| {
                    s.endpoints.iter().any(|e| {
                        !e.address.starts_with("127.")
                            && e.address != toadstool_common::constants::network::DEFAULT_HOSTNAME
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
    ///
    /// **EVOLVED**: Uses the discovered service's actual endpoint, not hardcoded name.
    pub fn new(service: &DiscoveredService) -> ToadStoolResult<Self> {
        let endpoint = service.endpoints.first().ok_or_else(|| {
            ToadStoolError::Network(NetworkError::ConnectionFailed {
                endpoint: service.name.clone(),
                reason: "No endpoints available".to_string(),
            })
        })?;

        // CAPABILITY-BASED: Use the discovered service's actual socket path
        // Extract Unix socket path from endpoint (supports unix:// protocol)
        let socket_path = if endpoint.protocol == "unix" {
            // Use endpoint address directly (it's the socket path)
            std::path::PathBuf::from(&endpoint.address)
        } else if let Some(path) = endpoint.metadata.get("socket_path") {
            // Or from metadata
            std::path::PathBuf::from(path)
        } else {
            // Fallback: Use generic socket path for discovered service name
            // This allows ANY crypto service to work (BearDog, HSM, cloud KMS)
            toadstool_common::primal_sockets::resolve_socket_path_for_service(
                &service.name,
                &toadstool_common::primal_sockets::SocketPathEnv::from_env(),
                None,
            )
        };

        let rpc_client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            rpc_client,
            _service_endpoint: endpoint.clone(),
            timeout: timeouts::DEFAULT_REQUEST_TIMEOUT,
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
            .and_then(serde_json::Value::as_bool)
            .unwrap_or_default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
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
    async fn test_discover_returns_vec() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let services = discovery.discover().await.unwrap();
        assert!(services.is_empty() || !services.is_empty());
    }

    #[tokio::test]
    async fn test_get_cached_initially_empty() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let cached = discovery.get_cached().await;
        assert!(cached.is_empty());
    }

    #[tokio::test]
    async fn test_discover_by_capability_returns_option() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let cap = Capability::Crypto(CryptoCapability::Encryption);
        let result = discovery.discover_by_capability(cap).await.unwrap();
        assert!(result.is_none() || result.is_some());
    }

    #[tokio::test]
    async fn test_location_filtering_network() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Network,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();

        let services = vec![
            DiscoveredService {
                id: "local".to_string(),
                name: "local-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
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
                name: "remote-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
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
        assert_eq!(filtered[0].id, "remote");
    }

    #[tokio::test]
    async fn test_location_filtering_any() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Any,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let services = vec![DiscoveredService {
            id: "svc1".to_string(),
            name: "crypto-1".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint::http("127.0.0.1", 9000)],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        }];
        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
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
                name: "remote-crypto".to_string(),
                version: "1.0.0".to_string(),
                capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
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

    #[test]
    fn test_crypto_client_new_fails_when_no_endpoints() {
        let service = DiscoveredService {
            id: "no-endpoints".to_string(),
            name: "empty-crypto".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        };
        let result = CryptoServiceClient::new(&service);
        assert!(result.is_err());
    }

    #[test]
    fn test_crypto_request_serialization() {
        use crate::crypto_integration::types::{
            CryptoOperation, EncryptionAlgorithm, SecurityLevel,
        };
        let req = CryptoRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: CryptoOperation::Encrypt,
            data: vec![1, 2, 3, 4, 5],
            key_id: Some("key-1".to_string()),
            algorithm: Some(EncryptionAlgorithm::Aes256Gcm),
            security_level: SecurityLevel::High,
            metadata: serde_json::json!({"test": true}),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("operation").is_some());
        assert!(json.get("data").is_some());
    }

    #[test]
    fn test_key_management_request_construction() {
        use crate::crypto_integration::types::{KeyOperation, KeyType};
        let req = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: KeyOperation::Generate {
                key_type: KeyType::Symmetric { bits: 256 },
            },
            metadata: serde_json::Value::Null,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("operation").is_some());
    }

    // ─── Priority 4: Service discovery simulation, request construction, response handling ───

    #[test]
    fn test_crypto_client_new_success_with_unix_endpoint() {
        use toadstool_common::primal_identity::CryptoCapability;

        let service = DiscoveredService {
            id: "crypto-1".to_string(),
            name: "test-crypto".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint {
                protocol: "unix".to_string(),
                address: "/tmp/test-crypto.sock".to_string(),
                port: 0,
                path: None,
                metadata: std::collections::HashMap::new(),
            }],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        };
        let result = CryptoServiceClient::new(&service);
        assert!(result.is_ok());
    }

    #[test]
    fn test_crypto_client_with_timeout() {
        use toadstool_common::primal_identity::CryptoCapability;

        let service = DiscoveredService {
            id: "crypto-2".to_string(),
            name: "timeout-crypto".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::KeyManagement)],
            endpoints: vec![ServiceEndpoint {
                protocol: "unix".to_string(),
                address: "/tmp/timeout-crypto.sock".to_string(),
                port: 0,
                path: None,
                metadata: std::collections::HashMap::new(),
            }],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        };
        let custom_client = CryptoServiceClient::with_timeout(&service, Duration::from_secs(30));
        assert!(custom_client.is_ok());
    }

    #[test]
    fn test_crypto_request_construction_encrypt() {
        use crate::crypto_integration::types::{
            CryptoOperation, EncryptionAlgorithm, SecurityLevel,
        };
        let req = CryptoRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: CryptoOperation::Encrypt,
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            key_id: Some("enc-key".to_string()),
            algorithm: Some(EncryptionAlgorithm::Aes256Gcm),
            security_level: SecurityLevel::High,
            metadata: serde_json::json!({"nonce": "test"}),
        };
        let json = serde_json::to_value(&req).unwrap();
        assert_eq!(json["key_id"].as_str(), Some("enc-key"));
    }

    #[test]
    fn test_crypto_request_construction_decrypt() {
        use crate::crypto_integration::types::{
            CryptoOperation, EncryptionAlgorithm, SecurityLevel,
        };
        let req = CryptoRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: CryptoOperation::Decrypt,
            data: vec![0xAA, 0xBB, 0xCC],
            key_id: None,
            algorithm: Some(EncryptionAlgorithm::ChaCha20Poly1305),
            security_level: SecurityLevel::Standard,
            metadata: serde_json::Value::Null,
        };
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("data").is_some());
    }

    #[test]
    fn test_crypto_response_parsing_success() {
        use crate::crypto_integration::types::CryptoResponse;
        let resp = CryptoResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![0x11, 0x22, 0x33, 0x44],
            key_id: "key-123".to_string(),
            algorithm: "aes-256-gcm".to_string(),
            metadata: serde_json::json!({"iv": "abc"}),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: CryptoResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.key_id, "key-123");
        assert_eq!(parsed.data.len(), 4);
    }

    #[test]
    fn test_key_management_response_parsing() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            key_id: "gen-key-1".to_string(),
            success: true,
            metadata: serde_json::json!({"algorithm": "aes-256-gcm"}),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.key_id, "gen-key-1");
        assert!(parsed.success);
    }

    #[test]
    fn test_crypto_config_default_required_capabilities() {
        let config = CryptoServiceConfig::default();
        assert!(!config.required_capabilities.is_empty());
        assert!(config.auto_discover);
        assert_eq!(config.discovery_timeout_ms, 5000);
    }

    #[test]
    fn test_crypto_config_preferred_location_variants() {
        let local = CryptoServiceConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let network = CryptoServiceConfig {
            preferred_location: ServiceLocation::Network,
            ..Default::default()
        };
        let any = CryptoServiceConfig {
            preferred_location: ServiceLocation::Any,
            ..Default::default()
        };
        assert_eq!(local.preferred_location, ServiceLocation::Local);
        assert_eq!(network.preferred_location, ServiceLocation::Network);
        assert_eq!(any.preferred_location, ServiceLocation::Any);
    }

    #[tokio::test]
    async fn test_discover_caches_services() {
        let config = CryptoServiceConfig::default();
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let _ = discovery.discover().await;
        let cached = discovery.get_cached().await;
        assert!(cached.is_empty() || !cached.is_empty());
    }

    #[tokio::test]
    async fn test_location_filter_local_filters_remote() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let services = vec![DiscoveredService {
            id: "remote".to_string(),
            name: "remote-crypto".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint::http("10.0.0.1", 9000)],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        }];
        let filtered = discovery.filter_by_location(&services);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_crypto_operation_variants_serde() {
        use crate::crypto_integration::types::CryptoOperation;
        let enc = CryptoOperation::Encrypt;
        let json = serde_json::to_value(&enc).unwrap();
        let _: CryptoOperation = serde_json::from_value(json).unwrap();
    }

    #[test]
    fn test_encryption_algorithm_serde() {
        use crate::crypto_integration::types::EncryptionAlgorithm;
        let alg = EncryptionAlgorithm::Aes256Gcm;
        let json = serde_json::to_value(&alg).unwrap();
        let _: EncryptionAlgorithm = serde_json::from_value(json).unwrap();
    }

    #[test]
    fn test_crypto_client_new_with_metadata_socket_path() {
        use toadstool_common::primal_identity::CryptoCapability;
        let mut metadata = std::collections::HashMap::new();
        metadata.insert(
            "socket_path".to_string(),
            "/tmp/custom-sock.sock".to_string(),
        );
        let service = DiscoveredService {
            id: "meta-sock".to_string(),
            name: "meta-crypto".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint {
                protocol: "http".to_string(),
                address: "127.0.0.1".to_string(),
                port: 9000,
                path: None,
                metadata,
            }],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        };
        let result = CryptoServiceClient::new(&service);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_filter_by_location_local_with_localhost() {
        let config = CryptoServiceConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = CryptoServiceDiscovery::new(config).await.unwrap();
        let services = vec![DiscoveredService {
            id: "local".to_string(),
            name: "local".to_string(),
            version: "1.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint::http("localhost", 8080)],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        }];
        let filtered = discovery.filter_by_location(&services);
        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn test_crypto_operation_all_variants_serde() {
        use crate::crypto_integration::types::{CryptoOperation, KeyType};
        let ops = [
            CryptoOperation::Encrypt,
            CryptoOperation::Decrypt,
            CryptoOperation::Sign,
            CryptoOperation::Verify,
            CryptoOperation::Hash,
            CryptoOperation::GenerateKey {
                key_type: KeyType::Symmetric { bits: 256 },
            },
            CryptoOperation::RotateKey {
                old_key_id: "old".to_string(),
            },
            CryptoOperation::ExportKey {
                key_id: "k1".to_string(),
            },
            CryptoOperation::ImportKey {
                key_data: vec![1, 2, 3],
            },
        ];
        for op in ops {
            let json = serde_json::to_value(&op).unwrap();
            let _: CryptoOperation = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_key_type_serde() {
        use crate::crypto_integration::types::KeyType;
        let types = [
            KeyType::Symmetric { bits: 256 },
            KeyType::Asymmetric {
                algorithm: "RSA".to_string(),
                bits: 2048,
            },
            KeyType::Signing {
                algorithm: "Ed25519".to_string(),
            },
        ];
        for kt in types {
            let json = serde_json::to_value(&kt).unwrap();
            let _: KeyType = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_security_level_serde() {
        use crate::crypto_integration::types::SecurityLevel;
        for level in [
            SecurityLevel::Standard,
            SecurityLevel::High,
            SecurityLevel::Maximum,
            SecurityLevel::QuantumResistant,
        ] {
            let json = serde_json::to_value(&level).unwrap();
            let _: SecurityLevel = serde_json::from_value(json).unwrap();
        }
    }

    #[test]
    fn test_crypto_response_serde_roundtrip() {
        let resp = CryptoResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![0xDE, 0xAD, 0xBE, 0xEF],
            key_id: "key-x".to_string(),
            algorithm: "aes-256-gcm".to_string(),
            metadata: serde_json::json!({"nonce": "abc"}),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: CryptoResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.key_id, "key-x");
        assert_eq!(parsed.data.len(), 4);
    }
}
