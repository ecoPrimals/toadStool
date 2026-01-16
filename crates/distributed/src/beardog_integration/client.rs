//! BearDog Unix Socket Client (Pure Rust!)
//!
//! **Design Philosophy**:
//! - **Pure Rust**: Unix sockets, no HTTP/TLS (no ring dependency!)
//! - **Async-first**: Non-blocking operations with tokio
//! - **Local IPC**: Fast, secure primal-to-primal communication
//! - **No hardcoding**: Socket paths discovered at runtime
//! - **TRUE PRIMAL**: Songbird handles external HTTP, we use local IPC
//!
//! ## Architecture
//!
//! ToadStool = Compute orchestration (internal)
//! BearDog = Security services (local)
//! Communication = JSON-RPC over unix sockets (pure Rust!)

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;

use toadstool_common::{ToadStoolError, ToadStoolResult};
use toadstool_common::primal_sockets::get_beardog_socket_path;
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

#[allow(unused_imports)] // Keep for discovery trait implementations
use std::time::Duration;

use super::types::{
    BearDogCapability, BearDogEndpoint, EncryptionRequest, EncryptionResponse,
    KeyManagementRequest, KeyManagementResponse, PermissionResponse, RevocationRequest,
    SignatureRequest, SignatureResponse, ValidationResponse, VerificationRequest,
    VerificationResponse,
};
use super::{BearDogConfig, ServiceLocation};

/// BearDog service discovery
///
/// **Design**: Runtime discovery via mDNS, Songbird registry, or config
pub struct BearDogDiscovery {
    config: BearDogConfig,
    discovered_endpoints: Arc<RwLock<Vec<BearDogEndpoint>>>,
}

impl BearDogDiscovery {
    /// Create new discovery instance
    pub fn new(config: BearDogConfig) -> Self {
        Self {
            config,
            discovered_endpoints: Arc::new(RwLock::new(Vec::new())),
        }
    }

    /// Discover BearDog services
    ///
    /// **Design**: Multi-strategy discovery (mDNS, Songbird, static config)
    pub async fn discover(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        let mut endpoints = Vec::new();

        // Strategy 1: mDNS discovery (local network)
        if matches!(
            self.config.preferred_location,
            ServiceLocation::Local | ServiceLocation::Any
        ) {
            if let Ok(local_endpoints) = self.discover_via_mdns().await {
                endpoints.extend(local_endpoints);
            }
        }

        // Strategy 2: Songbird primal registry
        if matches!(
            self.config.preferred_location,
            ServiceLocation::Network | ServiceLocation::Any
        ) {
            if let Ok(network_endpoints) = self.discover_via_songbird().await {
                endpoints.extend(network_endpoints);
            }
        }

        // Cache discovered endpoints
        let mut cache = self.discovered_endpoints.write().await;
        *cache = endpoints.clone();

        Ok(endpoints)
    }

    /// Discover via mDNS (local network)
    ///
    /// **Design**: Look for _beardog._tcp.local service
    async fn discover_via_mdns(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        // Use ToadStool's unified primal discovery system
        // This discovers services by capability, not by hardcoded name
        use std::time::Duration;
        use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

        let discovery_config = DiscoveryConfig {
            enable_mdns: true,
            cache_ttl: Duration::from_secs(300),
            ..Default::default()
        };

        match PrimalDiscovery::with_config(discovery_config).await {
            Ok(discovery) => {
                // Look for security/encryption capability (BearDog's primary role)
                match discovery.find_capability("security").await {
                    Ok(endpoint) => {
                        // Convert discovered endpoint to BearDogEndpoint
                        let beardog_endpoint = BearDogEndpoint {
                            service_id: endpoint.service_id.clone(),
                            protocol: "http".to_string(),
                            address: endpoint.url().parse().unwrap_or_else(|_| {
                                std::net::SocketAddr::from(([127, 0, 0, 1], 8081))
                            }),
                            api_version: "v1".to_string(),
                            capabilities: vec![BearDogCapability::Encryption {
                                algorithms: vec!["aes-256".to_string()],
                            }],
                            healthy: true,
                            latency_ms: Some(endpoint.latency_ms),
                        };
                        Ok(vec![beardog_endpoint])
                    }
                    Err(_) => Ok(Vec::new()), // No BearDog found via mDNS
                }
            }
            Err(_) => Ok(Vec::new()), // mDNS not available
        }
    }

    /// Discover via Songbird primal registry
    ///
    /// **Design**: Query Songbird for BearDog capability
    async fn discover_via_songbird(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        // Use unified primal discovery with Songbird as source
        use std::time::Duration;
        use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

        let mut discovery_config = DiscoveryConfig {
            enable_mdns: false, // Use Songbird, not mDNS
            cache_ttl: Duration::from_secs(300),
            ..Default::default()
        };

        // Try to discover Songbird first, then query it for BearDog
        if let Ok(songbird_endpoint) = std::env::var("SONGBIRD_ENDPOINT") {
            discovery_config
                .fallbacks
                .insert("orchestration".to_string(), songbird_endpoint);
        }

        match PrimalDiscovery::with_config(discovery_config).await {
            Ok(discovery) => {
                // Discover security capability (BearDog)
                match discovery.find_capability("security").await {
                    Ok(endpoint) => {
                        let beardog_endpoint = BearDogEndpoint {
                            service_id: endpoint.service_id.clone(),
                            protocol: "http".to_string(),
                            address: endpoint.url().parse().unwrap_or_else(|_| {
                                std::net::SocketAddr::from(([127, 0, 0, 1], 8081))
                            }),
                            api_version: "v1".to_string(),
                            capabilities: vec![BearDogCapability::Encryption {
                                algorithms: vec!["aes-256".to_string()],
                            }],
                            healthy: true,
                            latency_ms: Some(endpoint.latency_ms),
                        };
                        Ok(vec![beardog_endpoint])
                    }
                    Err(_) => Ok(Vec::new()),
                }
            }
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Get best endpoint based on location preference and health
    pub async fn get_best_endpoint(&self) -> ToadStoolResult<BearDogEndpoint> {
        let endpoints = self.discovered_endpoints.read().await;

        if endpoints.is_empty() {
            return Err(ToadStoolError::not_found("No BearDog endpoints discovered"));
        }

        // Filter by health
        let healthy_endpoints: Vec<_> = endpoints.iter().filter(|e| e.healthy).collect();

        if healthy_endpoints.is_empty() {
            return Err(ToadStoolError::not_found(
                "No healthy BearDog endpoints available",
            ));
        }

        // Sort by latency (prefer low latency)
        let mut sorted = healthy_endpoints;
        sorted.sort_by_key(|e| e.latency_ms.unwrap_or(u64::MAX));

        Ok(sorted[0].clone())
    }
}

/// BearDog Unix Socket Client (Pure Rust!)
///
/// **Design**: JSON-RPC 2.0 over unix sockets (no HTTP, no TLS, no ring!)
/// **TRUE PRIMAL**: Local IPC for compute primal → security primal communication
pub struct BearDogClient {
    discovery: Arc<BearDogDiscovery>,
    rpc_client: UnixJsonRpcClient,
}

impl BearDogClient {
    /// Create new BearDog client with unix socket transport
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    ///
    /// # Errors
    /// Returns error if socket path discovery fails
    pub fn new(config: BearDogConfig) -> ToadStoolResult<Self> {
        // Get unix socket path from environment-based discovery
        let socket_path = get_beardog_socket_path();
        let rpc_client = UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            discovery: Arc::new(BearDogDiscovery::new(config)),
            rpc_client,
        })
    }

    /// Discover BearDog services
    pub async fn discover(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        self.discovery.discover().await
    }

    /// Encrypt data using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn encrypt(&self, request: EncryptionRequest) -> ToadStoolResult<EncryptionResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;

        self.rpc_client
            .call_typed("beardog.encrypt", params)
            .await
    }

    /// Decrypt data using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn decrypt(&self, request: EncryptionRequest) -> ToadStoolResult<EncryptionResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;

        self.rpc_client
            .call_typed("beardog.decrypt", params)
            .await
    }

    /// Manage keys using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn key_management(
        &self,
        request: KeyManagementRequest,
    ) -> ToadStoolResult<KeyManagementResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;

        self.rpc_client
            .call_typed("beardog.key_management", params)
            .await
    }

    /// Sign data using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn sign(&self, data: &[u8]) -> ToadStoolResult<SignatureResponse> {
        let request = SignatureRequest {
            request_id: uuid::Uuid::new_v4(),
            data: data.to_vec(),
            key_id: None,
            algorithm: None,
        };

        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;

        self.rpc_client
            .call_typed("beardog.sign", params)
            .await
    }

    /// Verify signature using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key_id: &str,
    ) -> ToadStoolResult<bool> {
        let request = VerificationRequest {
            request_id: uuid::Uuid::new_v4(),
            data: data.to_vec(),
            signature: signature.to_vec(),
            public_key_id: public_key_id.to_string(),
        };

        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;

        let result: VerificationResponse = self
            .rpc_client
            .call_typed("beardog.verify", params)
            .await?;

        Ok(result.valid)
    }

    /// Create permission using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn create_permission(
        &self,
        request: &crate::security_provider::PermissionRequest,
    ) -> ToadStoolResult<PermissionResponse> {
        let params = serde_json::to_value(request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;

        self.rpc_client
            .call_typed("beardog.create_permission", params)
            .await
    }

    /// Validate permission using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn validate_permission(
        &self,
        permission: &crate::security_provider::SecurityPermission,
    ) -> ToadStoolResult<bool> {
        let params = serde_json::to_value(permission)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;

        let result: ValidationResponse = self
            .rpc_client
            .call_typed("beardog.validate_permission", params)
            .await?;

        Ok(result.valid)
    }

    /// Revoke permission using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn revoke_permission(
        &self,
        permission_id: &uuid::Uuid,
        reason: &str,
    ) -> ToadStoolResult<()> {
        let request = RevocationRequest {
            reason: reason.to_string(),
        };

        let mut params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;
        
        // Add permission_id to params
        if let Some(obj) = params.as_object_mut() {
            obj.insert("permission_id".to_string(), serde_json::json!(permission_id.to_string()));
        }

        let _: serde_json::Value = self
            .rpc_client
            .call("beardog.revoke_permission", params)
            .await?;

        Ok(())
    }

    /// Check health of BearDog services
    pub async fn health_check(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        let endpoints = self.discovery.discover().await?;

        // TODO: Implement actual health checks for each endpoint

        Ok(endpoints)
    }
}

/// Implement CryptoProvider trait for BearDog client
///
/// **Design**: BearDog is just another crypto provider
#[async_trait]
impl toadstool::encryption::CryptoProvider for BearDogClient {
    fn provider_id(&self) -> &str {
        "beardog"
    }

    fn capabilities(&self) -> &toadstool::encryption::CryptoCapability {
        // TODO: Return actual discovered capabilities
        // For now, return default capabilities
        static CAPABILITIES: std::sync::OnceLock<toadstool::encryption::CryptoCapability> =
            std::sync::OnceLock::new();

        CAPABILITIES.get_or_init(|| toadstool::encryption::CryptoCapability {
            algorithms: vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()],
            security_level: toadstool::encryption::SecurityLevel::Enhanced,
            hardware_backed: false,
        })
    }

    async fn encrypt(
        &self,
        data: &[u8],
        key: &toadstool::encryption::EncryptionKey,
    ) -> ToadStoolResult<(
        toadstool::encryption::EncryptedPayload,
        toadstool::encryption::EncryptionMetadata,
    )> {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: super::types::EncryptionOperation::Encrypt,
            data: data.to_vec(),
            key_id: Some(key.id.clone()),
            algorithm: Some(key.algorithm.clone()),
            security_level: match key.security_level {
                toadstool::encryption::SecurityLevel::Standard => {
                    super::types::SecurityLevel::Standard
                }
                toadstool::encryption::SecurityLevel::Enhanced => {
                    super::types::SecurityLevel::Enhanced
                }
                toadstool::encryption::SecurityLevel::HardwareSecured => {
                    super::types::SecurityLevel::HardwareSecured
                }
            },
        };

        let response = self.encrypt(request).await?;

        let payload = toadstool::encryption::EncryptedPayload::new(response.data);
        let metadata = toadstool::encryption::EncryptionMetadata {
            algorithm: response.algorithm,
            nonce: Vec::new(), // Extract from response.metadata
            aad: None,
            kdf_info: None,
            encrypted_at: chrono::Utc::now().timestamp(),
        };

        Ok((payload, metadata))
    }

    async fn decrypt(
        &self,
        encrypted: &toadstool::encryption::EncryptedPayload,
        key: &toadstool::encryption::EncryptionKey,
        _metadata: &toadstool::encryption::EncryptionMetadata,
    ) -> ToadStoolResult<Vec<u8>> {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: super::types::EncryptionOperation::Decrypt,
            data: encrypted.ciphertext.clone(),
            key_id: Some(key.id.clone()),
            algorithm: Some(key.algorithm.clone()),
            security_level: match key.security_level {
                toadstool::encryption::SecurityLevel::Standard => {
                    super::types::SecurityLevel::Standard
                }
                toadstool::encryption::SecurityLevel::Enhanced => {
                    super::types::SecurityLevel::Enhanced
                }
                toadstool::encryption::SecurityLevel::HardwareSecured => {
                    super::types::SecurityLevel::HardwareSecured
                }
            },
        };

        let response = self.decrypt(request).await?;
        Ok(response.data)
    }

    async fn generate_key(
        &self,
        security_level: toadstool::encryption::SecurityLevel,
    ) -> ToadStoolResult<toadstool::encryption::EncryptionKey> {
        let request = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: super::types::KeyOperation::Generate,
            key_id: None,
            security_level: Some(match security_level {
                toadstool::encryption::SecurityLevel::Standard => {
                    super::types::SecurityLevel::Standard
                }
                toadstool::encryption::SecurityLevel::Enhanced => {
                    super::types::SecurityLevel::Enhanced
                }
                toadstool::encryption::SecurityLevel::HardwareSecured => {
                    super::types::SecurityLevel::HardwareSecured
                }
            }),
        };

        let response = self.key_management(request).await?;

        match response.result {
            super::types::KeyOperationResult::Generated { key_id, algorithm } => {
                Ok(toadstool::encryption::EncryptionKey::new(
                    key_id,
                    Vec::new(), // Key material stays in BearDog
                    algorithm,
                    security_level,
                ))
            }
            super::types::KeyOperationResult::Error { message } => Err(ToadStoolError::runtime(
                format!("BearDog key generation failed: {}", message),
            )),
            _ => Err(ToadStoolError::runtime("Unexpected response from BearDog")),
        }
    }

    async fn get_key(&self, key_id: &str) -> ToadStoolResult<toadstool::encryption::EncryptionKey> {
        let request = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: super::types::KeyOperation::Get,
            key_id: Some(key_id.to_string()),
            security_level: None,
        };

        let response = self.key_management(request).await?;

        match response.result {
            super::types::KeyOperationResult::Retrieved {
                key_id,
                key_material,
                algorithm,
            } => Ok(toadstool::encryption::EncryptionKey::new(
                key_id,
                key_material,
                algorithm,
                toadstool::encryption::SecurityLevel::Standard, // Default, should be in response
            )),
            super::types::KeyOperationResult::Error { message } => Err(ToadStoolError::not_found(
                format!("BearDog key not found: {}", message),
            )),
            _ => Err(ToadStoolError::runtime("Unexpected response from BearDog")),
        }
    }

    async fn health_check(
        &self,
    ) -> ToadStoolResult<toadstool::encryption::provider::ProviderHealth> {
        let endpoints = self.health_check().await?;

        if endpoints.is_empty() {
            return Ok(toadstool::encryption::provider::ProviderHealth::unhealthy(
                "No BearDog endpoints available",
            ));
        }

        let healthy_count = endpoints.iter().filter(|e| e.healthy).count();
        if healthy_count == 0 {
            return Ok(toadstool::encryption::provider::ProviderHealth::unhealthy(
                "All BearDog endpoints unhealthy",
            ));
        }

        let avg_latency =
            endpoints.iter().filter_map(|e| e.latency_ms).sum::<u64>() / healthy_count as u64;

        Ok(toadstool::encryption::provider::ProviderHealth::healthy(
            avg_latency,
        ))
    }
}

#[cfg(test)]
#[allow(deprecated)] // This module is deprecated, allow its tests
mod tests {
    use super::*;

    #[test]
    #[allow(deprecated)]
    fn test_beardog_discovery_new() {
        let config = BearDogConfig::default();
        let discovery = BearDogDiscovery::new(config);
        assert!(discovery.config.auto_discover);
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_new() {
        let config = BearDogConfig::default();
        let _client = BearDogClient::new(config);
        // Client created successfully
    }
}
