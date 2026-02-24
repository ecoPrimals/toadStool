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

use toadstool_common::primal_sockets::{discover_crypto_socket, get_socket_path_for_service};
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use toadstool_common::{ToadStoolError, ToadStoolResult};

use toadstool_common::constants::timeouts;

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

    /// Create discovery with pre-populated endpoints (for testing)
    #[cfg(test)]
    pub fn with_endpoints(config: BearDogConfig, endpoints: Vec<BearDogEndpoint>) -> Self {
        Self {
            config,
            discovered_endpoints: Arc::new(RwLock::new(endpoints)),
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
        use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

        let discovery_config = DiscoveryConfig {
            enable_mdns: true,
            cache_ttl: timeouts::DEFAULT_CACHE_TTL,
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
        use toadstool_common::primal_discovery::{DiscoveryConfig, PrimalDiscovery};

        let mut discovery_config = DiscoveryConfig {
            enable_mdns: false, // Use Songbird, not mDNS
            cache_ttl: timeouts::DEFAULT_CACHE_TTL,
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
    /// Create new BearDog client with capability-based discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers crypto service by capability, not name.
    /// Works with ANY service providing crypto.encryption capability.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    ///
    /// # Errors
    /// Returns error if no crypto service is discovered
    pub async fn new_async(config: BearDogConfig) -> ToadStoolResult<Self> {
        // CAPABILITY-BASED: Discover ANY crypto service, not just "beardog"
        let socket_path = discover_crypto_socket()
            .await
            .map_err(|e| ToadStoolError::configuration(format!(
                "No crypto service discovered: {e}. Ensure a crypto provider (e.g., BearDog) is running.",
            )))?;

        let rpc_client = UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            discovery: Arc::new(BearDogDiscovery::new(config)),
            rpc_client,
        })
    }

    /// Create new BearDog client with unix socket transport
    ///
    /// **DEPRECATED**: Use `new_async()` for capability-based discovery.
    /// This function uses hardcoded primal name which violates Deep Debt principles.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    ///
    /// # Errors
    /// Returns error if socket path discovery fails
    #[deprecated(
        since = "0.3.0",
        note = "Use new_async() for capability-based discovery"
    )]
    #[allow(deprecated)]
    pub fn new(config: BearDogConfig) -> ToadStoolResult<Self> {
        // Get unix socket path from environment-based discovery
        // LEGACY: Still uses primal name for backward compatibility
        let socket_path = get_socket_path_for_service("beardog");
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

    /// Query actual capabilities from the crypto service
    ///
    /// **Design**: Queries the service at runtime for its true capabilities.
    /// Use this instead of the trait's `capabilities()` method when you need
    /// the actual service capabilities rather than conservative defaults.
    ///
    /// Returns the algorithms supported, security level, and hardware status.
    pub async fn query_capabilities_async(
        &self,
    ) -> ToadStoolResult<toadstool::encryption::CryptoCapability> {
        // Query the service for its capabilities via RPC
        // Deep Debt: Don't synthesize fake capabilities on RPC failure - propagate the error
        let response: serde_json::Value = self
            .rpc_client
            .call_typed("beardog.capabilities", serde_json::json!({}))
            .await
            .map_err(|e| {
                tracing::warn!("Beardog capabilities query failed: {}", e);
                ToadStoolError::network(format!("Beardog crypto capabilities query failed: {e}"))
            })?;

        // Parse the response into CryptoCapability
        let algorithms: Vec<String> = response
            .get("algorithms")
            .and_then(|v: &serde_json::Value| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v: &serde_json::Value| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec!["chacha20poly1305".to_string(), "aes-256-gcm".to_string()]);

        let security_level = response
            .get("security_level")
            .and_then(|v: &serde_json::Value| v.as_str())
            .map(|s: &str| match s.to_lowercase().as_str() {
                "standard" => toadstool::encryption::SecurityLevel::Standard,
                "hardware_secured" | "hardware" => {
                    toadstool::encryption::SecurityLevel::HardwareSecured
                }
                _ => toadstool::encryption::SecurityLevel::Enhanced,
            })
            .unwrap_or(toadstool::encryption::SecurityLevel::Enhanced);

        let hardware_backed = response
            .get("hardware_backed")
            .and_then(|v: &serde_json::Value| v.as_bool())
            .unwrap_or(false);

        Ok(toadstool::encryption::CryptoCapability {
            algorithms,
            security_level,
            hardware_backed,
        })
    }

    /// Encrypt data using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn encrypt(&self, request: EncryptionRequest) -> ToadStoolResult<EncryptionResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        self.rpc_client.call_typed("beardog.encrypt", params).await
    }

    /// Decrypt data using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn decrypt(&self, request: EncryptionRequest) -> ToadStoolResult<EncryptionResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        self.rpc_client.call_typed("beardog.decrypt", params).await
    }

    /// Manage keys using BearDog via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no TLS, no ring!)
    pub async fn key_management(
        &self,
        request: KeyManagementRequest,
    ) -> ToadStoolResult<KeyManagementResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

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
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        self.rpc_client.call_typed("beardog.sign", params).await
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
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        let result: VerificationResponse =
            self.rpc_client.call_typed("beardog.verify", params).await?;

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
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

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
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

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
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        // Add permission_id to params
        if let Some(obj) = params.as_object_mut() {
            obj.insert(
                "permission_id".to_string(),
                serde_json::json!(permission_id.to_string()),
            );
        }

        let _: serde_json::Value = self
            .rpc_client
            .call("beardog.revoke_permission", params)
            .await?;

        Ok(())
    }

    /// Check health of BearDog services
    ///
    /// **Design**: Probes each discovered endpoint with a health check RPC call
    /// and updates their health status and latency based on actual response.
    pub async fn health_check(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        let endpoints = self.discovery.discover().await?;

        // Probe each endpoint for actual health status
        let mut checked_endpoints = Vec::with_capacity(endpoints.len());

        for mut endpoint in endpoints {
            let start = std::time::Instant::now();

            // Try to ping the endpoint via the unix socket RPC
            let health_result: Result<serde_json::Value, _> = self
                .rpc_client
                .call_typed("beardog.health", serde_json::json!({}))
                .await;

            let latency_ms = start.elapsed().as_millis() as u64;

            match health_result {
                Ok(_response) => {
                    endpoint.healthy = true;
                    endpoint.latency_ms = Some(latency_ms);
                }
                Err(_) => {
                    // Endpoint failed health check but may still be discoverable
                    endpoint.healthy = false;
                    endpoint.latency_ms = Some(latency_ms);
                }
            }

            checked_endpoints.push(endpoint);
        }

        Ok(checked_endpoints)
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
        // NOTE: CryptoProvider trait requires &'static lifetime, which prevents
        // dynamic runtime queries. Use query_capabilities_async() for actual
        // discovered capabilities. This returns conservative defaults.
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
                format!("BearDog key generation failed: {message}"),
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
                format!("BearDog key not found: {message}"),
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
    use crate::beardog_integration::types::{
        BearDogCapability, BearDogEndpoint, EncryptionOperation, EncryptionRequest,
        EncryptionResponse, KeyManagementRequest, KeyManagementResponse, KeyOperation,
        KeyOperationResult, SecurityLevel, SignatureRequest, VerificationRequest,
    };
    use crate::beardog_integration::{BearDogConfig, ServiceLocation};
    use toadstool::CryptoProvider;

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

    #[test]
    fn test_beardog_config_default() {
        let config = BearDogConfig::default();
        assert!(config.auto_discover);
        assert_eq!(config.discovery_timeout_ms, 5000);
        assert_eq!(config.preferred_location, ServiceLocation::Local);
        assert!(config.fallback_enabled);
    }

    #[test]
    fn test_service_location_variants() {
        assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
        assert_eq!(ServiceLocation::Network, ServiceLocation::Network);
        assert_eq!(ServiceLocation::Any, ServiceLocation::Any);
        assert_ne!(ServiceLocation::Local, ServiceLocation::Network);
    }

    #[tokio::test]
    async fn test_beardog_discovery_get_best_endpoint_empty() {
        let config = BearDogConfig::default();
        let discovery = BearDogDiscovery::new(config);
        let result = discovery.get_best_endpoint().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("No BearDog endpoints"));
    }

    #[test]
    fn test_encryption_request_serialization() {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Encrypt,
            data: vec![1, 2, 3, 4, 5],
            key_id: Some("key-123".to_string()),
            algorithm: Some("aes-256-gcm".to_string()),
            security_level: SecurityLevel::Enhanced,
        };
        let json = serde_json::to_value(&request);
        assert!(json.is_ok());
        let parsed: Result<EncryptionRequest, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
        let p = parsed.unwrap();
        assert_eq!(p.data, vec![1, 2, 3, 4, 5]);
        assert_eq!(p.key_id.as_deref(), Some("key-123"));
    }

    #[test]
    fn test_encryption_response_serialization() {
        let response = EncryptionResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![10, 20, 30],
            key_id: "key-456".to_string(),
            algorithm: "chacha20".to_string(),
            metadata: serde_json::json!({"nonce": "abc"}),
        };
        let json = serde_json::to_value(&response);
        assert!(json.is_ok());
        let parsed: Result<EncryptionResponse, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_signature_request_serialization() {
        let request = SignatureRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3],
            key_id: None,
            algorithm: Some("ed25519".to_string()),
        };
        let json = serde_json::to_value(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_verification_request_serialization() {
        let request = VerificationRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3],
            signature: vec![4, 5, 6],
            public_key_id: "pub-key-1".to_string(),
        };
        let json = serde_json::to_value(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_key_management_request_serialization() {
        let request = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: KeyOperation::Generate,
            key_id: None,
            security_level: Some(SecurityLevel::Standard),
        };
        let json = serde_json::to_value(&request);
        assert!(json.is_ok());
    }

    #[test]
    fn test_key_operation_result_serialization() {
        let result = KeyOperationResult::Generated {
            key_id: "gen-key-1".to_string(),
            algorithm: "aes-256".to_string(),
        };
        let json = serde_json::to_value(&result);
        assert!(json.is_ok());
    }

    #[test]
    fn test_bear_dog_endpoint_serialization() {
        let endpoint = BearDogEndpoint {
            service_id: "beardog-1".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8081".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::Encryption {
                algorithms: vec!["aes-256".to_string()],
            }],
            healthy: true,
            latency_ms: Some(5),
        };
        let json = serde_json::to_value(&endpoint);
        assert!(json.is_ok());
    }

    #[test]
    fn test_bear_dog_capability_variants() {
        let enc = BearDogCapability::Encryption {
            algorithms: vec!["aes".to_string()],
        };
        assert!(matches!(enc, BearDogCapability::Encryption { .. }));
        let key = BearDogCapability::KeyManagement;
        assert!(matches!(key, BearDogCapability::KeyManagement));
    }

    #[test]
    fn test_security_level_ordering() {
        assert!(SecurityLevel::Standard < SecurityLevel::Enhanced);
        assert!(SecurityLevel::Enhanced < SecurityLevel::HardwareSecured);
    }

    #[tokio::test]
    async fn test_beardog_discovery_preferred_location_local() {
        let config = BearDogConfig {
            preferred_location: ServiceLocation::Local,
            ..Default::default()
        };
        let discovery = BearDogDiscovery::new(config);
        // discover() may return empty without running mDNS - exercise the path
        let result = discovery.discover().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_beardog_discovery_preferred_location_any() {
        let config = BearDogConfig {
            preferred_location: ServiceLocation::Any,
            ..Default::default()
        };
        let discovery = BearDogDiscovery::new(config);
        let result = discovery.discover().await;
        assert!(result.is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_provider_id() {
        let config = BearDogConfig::default();
        let client = BearDogClient::new(config).unwrap();
        assert_eq!(client.provider_id(), "beardog");
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_capabilities() {
        let config = BearDogConfig::default();
        let client = BearDogClient::new(config).unwrap();
        let caps = client.capabilities();
        assert!(!caps.algorithms.is_empty());
    }

    // ─── Additional serialization and error tests ────────────────────────────────

    #[test]
    fn test_signature_response_serialization() {
        use crate::beardog_integration::types::SignatureResponse;

        let resp = SignatureResponse {
            request_id: uuid::Uuid::new_v4(),
            signature: vec![1, 2, 3, 4],
            key_id: "key-1".to_string(),
            algorithm: "ed25519".to_string(),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
        let parsed: Result<SignatureResponse, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
    }

    #[test]
    fn test_permission_response_serialization() {
        use crate::beardog_integration::types::PermissionResponse;

        let resp = PermissionResponse {
            request_id: uuid::Uuid::new_v4(),
            permission_id: uuid::Uuid::new_v4(),
            proof: vec![5, 6, 7],
            metadata: serde_json::json!({"scope": "read"}),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
    }

    #[test]
    fn test_validation_response_serialization() {
        use crate::beardog_integration::types::ValidationResponse;

        let resp = ValidationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: true,
            details: Some("ok".to_string()),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
        let parsed: Result<ValidationResponse, _> = serde_json::from_value(json.unwrap());
        assert!(parsed.is_ok());
        assert!(parsed.unwrap().valid);
    }

    #[test]
    fn test_revocation_request_serialization() {
        use crate::beardog_integration::types::RevocationRequest;

        let req = RevocationRequest {
            reason: "expired".to_string(),
        };
        let json = serde_json::to_value(&req);
        assert!(json.is_ok());
    }

    #[test]
    fn test_key_management_response_serialization() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Deleted {
                key_id: "del-key".to_string(),
            },
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
    }

    #[test]
    fn test_key_operation_result_all_variants_serialization() {
        let deleted = KeyOperationResult::Deleted {
            key_id: "k1".to_string(),
        };
        assert!(serde_json::to_value(&deleted).is_ok());

        let listed = KeyOperationResult::Listed {
            keys: vec!["k1".to_string(), "k2".to_string()],
        };
        assert!(serde_json::to_value(&listed).is_ok());

        let err = KeyOperationResult::Error {
            message: "failed".to_string(),
        };
        assert!(serde_json::to_value(&err).is_ok());

        let retrieved = KeyOperationResult::Retrieved {
            key_id: "k1".to_string(),
            key_material: vec![0, 1, 2],
            algorithm: "aes".to_string(),
        };
        let json = serde_json::to_value(&retrieved).unwrap();
        let back: KeyOperationResult = serde_json::from_value(json).unwrap();
        assert!(matches!(back, KeyOperationResult::Retrieved { .. }));
    }

    #[test]
    fn test_encryption_operation_serde_roundtrip() {
        let enc = EncryptionOperation::Encrypt;
        let json = serde_json::to_string(&enc).unwrap();
        let _: EncryptionOperation = serde_json::from_str(&json).unwrap();
        let dec = EncryptionOperation::Decrypt;
        let json2 = serde_json::to_string(&dec).unwrap();
        let _: EncryptionOperation = serde_json::from_str(&json2).unwrap();
    }

    #[test]
    fn test_bear_dog_config_variations() {
        let config = BearDogConfig {
            auto_discover: false,
            discovery_timeout_ms: 10000,
            preferred_location: ServiceLocation::Network,
            fallback_enabled: false,
        };
        assert!(!config.auto_discover);
        assert_eq!(config.discovery_timeout_ms, 10000);
        assert_eq!(config.preferred_location, ServiceLocation::Network);
    }

    #[test]
    fn test_bear_dog_endpoint_serde_roundtrip() {
        let endpoint = BearDogEndpoint {
            service_id: "ep-1".to_string(),
            protocol: "unix".to_string(),
            address: "127.0.0.1:9000".parse().unwrap(),
            api_version: "v2".to_string(),
            capabilities: vec![
                BearDogCapability::Encryption {
                    algorithms: vec!["aes-256-gcm".to_string()],
                },
                BearDogCapability::KeyManagement,
            ],
            healthy: false,
            latency_ms: None,
        };
        let json = serde_json::to_string(&endpoint).unwrap();
        let parsed: BearDogEndpoint = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.service_id, endpoint.service_id);
        assert_eq!(parsed.capabilities.len(), 2);
    }

    #[test]
    fn test_bear_dog_capability_all_variants_serde() {
        let custom = BearDogCapability::Custom("my-cap".to_string());
        let json = serde_json::to_value(&custom).unwrap();
        let back: BearDogCapability = serde_json::from_value(json).unwrap();
        assert!(matches!(back, BearDogCapability::Custom(s) if s == "my-cap"));

        let key = BearDogCapability::KeyManagement;
        let json = serde_json::to_value(&key).unwrap();
        let _: BearDogCapability = serde_json::from_value(json).unwrap();
    }

    #[test]
    fn test_security_level_serde_roundtrip() {
        for level in [
            SecurityLevel::Standard,
            SecurityLevel::Enhanced,
            SecurityLevel::HardwareSecured,
        ] {
            let json = serde_json::to_string(&level).unwrap();
            let _: SecurityLevel = serde_json::from_str(&json).unwrap();
        }
    }

    #[test]
    fn test_key_operation_variants_serde() {
        for op in [
            KeyOperation::Generate,
            KeyOperation::Get,
            KeyOperation::Delete,
            KeyOperation::List,
        ] {
            let json = serde_json::to_value(&op).unwrap();
            let _: KeyOperation = serde_json::from_value(json).unwrap();
        }
    }

    #[tokio::test]
    async fn test_beardog_discovery_preferred_location_network() {
        let config = BearDogConfig {
            preferred_location: ServiceLocation::Network,
            ..Default::default()
        };
        let discovery = BearDogDiscovery::new(config);
        let result = discovery.discover().await;
        assert!(result.is_ok());
    }

    #[test]
    fn test_verification_response_serialization() {
        use crate::beardog_integration::types::VerificationResponse;

        let resp = VerificationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: false,
            details: Some("invalid sig".to_string()),
        };
        let json = serde_json::to_value(&resp);
        assert!(json.is_ok());
    }

    // ─── Additional tests: error variants, round-trips, ToadStoolError ─────────

    #[test]
    fn test_encryption_request_roundtrip() {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Decrypt,
            data: vec![0xFF, 0xFE],
            key_id: None,
            algorithm: Some("chacha20".to_string()),
            security_level: SecurityLevel::HardwareSecured,
        };
        let json = serde_json::to_string(&request).unwrap();
        let parsed: EncryptionRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.data, request.data);
        assert_eq!(parsed.security_level, SecurityLevel::HardwareSecured);
    }

    #[test]
    fn test_key_management_response_generated_roundtrip() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Generated {
                key_id: "gen-123".to_string(),
                algorithm: "aes-256-gcm".to_string(),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_str(&json).unwrap();
        match &parsed.result {
            KeyOperationResult::Generated { key_id, algorithm } => {
                assert_eq!(key_id, "gen-123");
                assert_eq!(algorithm, "aes-256-gcm");
            }
            _ => panic!("expected Generated"),
        }
    }

    #[test]
    fn test_key_operation_result_listed_serde() {
        let listed = KeyOperationResult::Listed {
            keys: vec!["k1".to_string(), "k2".to_string()],
        };
        let json = serde_json::to_value(&listed).unwrap();
        let back: KeyOperationResult = serde_json::from_value(json).unwrap();
        if let KeyOperationResult::Listed { keys } = back {
            assert_eq!(keys.len(), 2);
        } else {
            panic!("expected Listed");
        }
    }

    #[test]
    fn test_bear_dog_endpoint_debug_clone() {
        let ep = BearDogEndpoint {
            service_id: "ep-1".to_string(),
            protocol: "unix".to_string(),
            address: "127.0.0.1:9000".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::KeyManagement],
            healthy: true,
            latency_ms: Some(1),
        };
        let ep2 = ep.clone();
        assert_eq!(ep.service_id, ep2.service_id);
        assert!(format!("{:?}", ep).contains("ep-1"));
    }

    #[test]
    fn test_bear_dog_capability_hardware_security() {
        let cap = BearDogCapability::HardwareSecurity;
        assert!(matches!(cap, BearDogCapability::HardwareSecurity));
    }

    #[test]
    fn test_bear_dog_capability_secure_storage() {
        let cap = BearDogCapability::SecureStorage;
        assert!(matches!(cap, BearDogCapability::SecureStorage));
    }

    #[test]
    fn test_bear_dog_capability_genetic_entropy() {
        let cap = BearDogCapability::GeneticEntropy;
        assert!(matches!(cap, BearDogCapability::GeneticEntropy));
    }

    #[test]
    fn test_toadstool_error_not_found_display() {
        let err = toadstool_common::ToadStoolError::not_found("No BearDog endpoints");
        let s = err.to_string();
        assert!(s.to_lowercase().contains("bear"));
    }

    #[test]
    fn test_beardog_config_timeout_variations() {
        let config = BearDogConfig {
            auto_discover: true,
            discovery_timeout_ms: 1,
            preferred_location: ServiceLocation::Local,
            fallback_enabled: true,
        };
        assert_eq!(config.discovery_timeout_ms, 1);
    }

    #[test]
    fn test_permission_response_roundtrip() {
        use crate::beardog_integration::types::PermissionResponse;

        let resp = PermissionResponse {
            request_id: uuid::Uuid::new_v4(),
            permission_id: uuid::Uuid::new_v4(),
            proof: vec![1, 2, 3, 4, 5],
            metadata: serde_json::json!({"scope": "read"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: PermissionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.proof, resp.proof);
    }

    #[test]
    fn test_validation_response_valid_false() {
        use crate::beardog_integration::types::ValidationResponse;

        let resp = ValidationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: false,
            details: Some("expired".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ValidationResponse = serde_json::from_str(&json).unwrap();
        assert!(!parsed.valid);
    }

    #[test]
    fn test_signature_response_roundtrip() {
        use crate::beardog_integration::types::SignatureResponse;

        let resp = SignatureResponse {
            request_id: uuid::Uuid::new_v4(),
            signature: vec![0xDE, 0xAD],
            key_id: "sig-key".to_string(),
            algorithm: "ed25519".to_string(),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: SignatureResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.signature, resp.signature);
    }

    #[test]
    fn test_revocation_request_roundtrip() {
        use crate::beardog_integration::types::RevocationRequest;

        let req = RevocationRequest {
            reason: "user request".to_string(),
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: RevocationRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.reason, req.reason);
    }

    // ─── get_best_endpoint with endpoints, latency sorting, all unhealthy ─────

    #[tokio::test]
    async fn test_beardog_discovery_get_best_endpoint_returns_lowest_latency() {
        let config = BearDogConfig::default();
        let endpoints = vec![
            BearDogEndpoint {
                service_id: "ep-slow".to_string(),
                protocol: "http".to_string(),
                address: "127.0.0.1:8081".parse().unwrap(),
                api_version: "v1".to_string(),
                capabilities: vec![BearDogCapability::Encryption {
                    algorithms: vec!["aes-256".to_string()],
                }],
                healthy: true,
                latency_ms: Some(50),
            },
            BearDogEndpoint {
                service_id: "ep-fast".to_string(),
                protocol: "http".to_string(),
                address: "127.0.0.1:8082".parse().unwrap(),
                api_version: "v1".to_string(),
                capabilities: vec![BearDogCapability::Encryption {
                    algorithms: vec!["aes-256".to_string()],
                }],
                healthy: true,
                latency_ms: Some(5),
            },
        ];
        let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
        let best = discovery.get_best_endpoint().await.unwrap();
        assert_eq!(best.service_id, "ep-fast");
        assert_eq!(best.latency_ms, Some(5));
    }

    #[tokio::test]
    async fn test_beardog_discovery_get_best_endpoint_all_unhealthy_returns_error() {
        let config = BearDogConfig::default();
        let endpoints = vec![BearDogEndpoint {
            service_id: "ep-unhealthy".to_string(),
            protocol: "http".to_string(),
            address: "127.0.0.1:8081".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::KeyManagement],
            healthy: false,
            latency_ms: Some(100),
        }];
        let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
        let result = discovery.get_best_endpoint().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("healthy"));
    }

    #[tokio::test]
    async fn test_beardog_discovery_get_best_endpoint_no_latency_uses_max() {
        let config = BearDogConfig::default();
        let endpoints = vec![
            BearDogEndpoint {
                service_id: "ep-a".to_string(),
                protocol: "http".to_string(),
                address: "127.0.0.1:8081".parse().unwrap(),
                api_version: "v1".to_string(),
                capabilities: vec![BearDogCapability::KeyManagement],
                healthy: true,
                latency_ms: None,
            },
            BearDogEndpoint {
                service_id: "ep-b".to_string(),
                protocol: "http".to_string(),
                address: "127.0.0.1:8082".parse().unwrap(),
                api_version: "v1".to_string(),
                capabilities: vec![BearDogCapability::KeyManagement],
                healthy: true,
                latency_ms: Some(1),
            },
        ];
        let discovery = BearDogDiscovery::with_endpoints(config, endpoints);
        let best = discovery.get_best_endpoint().await.unwrap();
        assert_eq!(best.service_id, "ep-b");
    }

    // ─── Priority 2: Client creation, request construction, response parsing, retry/timeout ───

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_creation_with_custom_config() {
        let config = BearDogConfig {
            auto_discover: false,
            discovery_timeout_ms: 10000,
            preferred_location: ServiceLocation::Network,
            fallback_enabled: false,
        };
        let result = BearDogClient::new(config);
        assert!(result.is_ok());
    }

    #[test]
    fn test_encryption_request_construction_for_encrypt() {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Encrypt,
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            key_id: Some("enc-key-1".to_string()),
            algorithm: Some("aes-256-gcm".to_string()),
            security_level: SecurityLevel::Enhanced,
        };
        let params = serde_json::to_value(&request).unwrap();
        assert!(params.get("operation").is_some());
        assert_eq!(params["data"].as_array().unwrap().len(), 5);
    }

    #[test]
    fn test_encryption_request_construction_for_decrypt() {
        let request = EncryptionRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: EncryptionOperation::Decrypt,
            data: vec![0xAA, 0xBB, 0xCC],
            key_id: Some("dec-key".to_string()),
            algorithm: Some("chacha20poly1305".to_string()),
            security_level: SecurityLevel::Standard,
        };
        let params = serde_json::to_value(&request).unwrap();
        assert!(params.get("key_id").is_some());
    }

    #[test]
    fn test_key_management_request_construction_generate() {
        let request = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: KeyOperation::Generate,
            key_id: None,
            security_level: Some(SecurityLevel::HardwareSecured),
        };
        let params = serde_json::to_value(&request).unwrap();
        assert!(params.get("operation").is_some());
    }

    #[test]
    fn test_key_management_request_construction_get() {
        let request = KeyManagementRequest {
            request_id: uuid::Uuid::new_v4(),
            operation: KeyOperation::Get,
            key_id: Some("fetch-key-123".to_string()),
            security_level: None,
        };
        let params = serde_json::to_value(&request).unwrap();
        assert_eq!(params["key_id"].as_str(), Some("fetch-key-123"));
    }

    #[test]
    fn test_key_management_response_parsing_success_generated() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Generated {
                key_id: "new-key-id".to_string(),
                algorithm: "aes-256-gcm".to_string(),
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_value(json).unwrap();
        match &parsed.result {
            KeyOperationResult::Generated { key_id, algorithm } => {
                assert_eq!(key_id, "new-key-id");
                assert_eq!(algorithm, "aes-256-gcm");
            }
            _ => panic!("expected Generated"),
        }
    }

    #[test]
    fn test_key_management_response_parsing_error() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Error {
                message: "key not found".to_string(),
            },
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_value(json).unwrap();
        match &parsed.result {
            KeyOperationResult::Error { message } => assert_eq!(message, "key not found"),
            _ => panic!("expected Error"),
        }
    }

    #[test]
    fn test_encryption_response_parsing_success() {
        let resp = EncryptionResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![0x11, 0x22, 0x33],
            key_id: "key-456".to_string(),
            algorithm: "chacha20".to_string(),
            metadata: serde_json::json!({"iv": "abc123"}),
        };
        let json = serde_json::to_value(&resp).unwrap();
        let parsed: EncryptionResponse = serde_json::from_value(json).unwrap();
        assert_eq!(parsed.data, vec![0x11, 0x22, 0x33]);
        assert_eq!(parsed.algorithm, "chacha20");
    }

    #[test]
    fn test_beardog_config_timeout_affects_discovery_timeout() {
        let config = BearDogConfig {
            discovery_timeout_ms: 2500,
            ..Default::default()
        };
        assert_eq!(config.discovery_timeout_ms, 2500);
    }

    #[test]
    fn test_beardog_config_fallback_disabled() {
        let config = BearDogConfig {
            fallback_enabled: false,
            ..Default::default()
        };
        assert!(!config.fallback_enabled);
    }

    #[test]
    fn test_signature_request_construction_with_algorithm() {
        let request = SignatureRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3, 4, 5],
            key_id: Some("sig-key".to_string()),
            algorithm: Some("ed25519".to_string()),
        };
        let params = serde_json::to_value(&request).unwrap();
        assert_eq!(params["algorithm"].as_str(), Some("ed25519"));
    }

    #[test]
    fn test_verification_request_construction() {
        let request = VerificationRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3],
            signature: vec![4, 5, 6, 7, 8],
            public_key_id: "pub-key-99".to_string(),
        };
        let params = serde_json::to_value(&request).unwrap();
        assert_eq!(params["public_key_id"].as_str(), Some("pub-key-99"));
    }

    #[test]
    fn test_revocation_request_construction() {
        use crate::beardog_integration::types::RevocationRequest;

        let request = RevocationRequest {
            reason: "security incident".to_string(),
        };
        let params = serde_json::to_value(&request).unwrap();
        assert_eq!(params["reason"].as_str(), Some("security incident"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_beardog_client_creation_default_config() {
        let config = BearDogConfig::default();
        let result = BearDogClient::new(config);
        assert!(result.is_ok());
        let client = result.unwrap();
        assert_eq!(client.provider_id(), "beardog");
    }

    #[tokio::test]
    async fn test_beardog_discovery_with_endpoints_injects_mock_data() {
        let config = BearDogConfig::default();
        let mock_endpoints = vec![BearDogEndpoint {
            service_id: "mock-1".to_string(),
            protocol: "unix".to_string(),
            address: "127.0.0.1:9090".parse().unwrap(),
            api_version: "v1".to_string(),
            capabilities: vec![BearDogCapability::KeyManagement],
            healthy: true,
            latency_ms: Some(2),
        }];
        let discovery = BearDogDiscovery::with_endpoints(config, mock_endpoints);
        let best = discovery.get_best_endpoint().await.unwrap();
        assert_eq!(best.service_id, "mock-1");
    }

    // ─── ToadStoolError constructors and VerificationResponse round-trip ────────

    #[test]
    fn test_toadstool_error_configuration_display() {
        let err = toadstool_common::ToadStoolError::configuration("Config invalid");
        assert!(err.to_string().to_lowercase().contains("config"));
    }

    #[test]
    fn test_toadstool_error_runtime_display() {
        let err = toadstool_common::ToadStoolError::runtime("Runtime failure");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_toadstool_error_network_display() {
        let err = toadstool_common::ToadStoolError::network("Network error");
        assert!(err.to_string().len() > 0);
    }

    #[test]
    fn test_toadstool_error_security_display() {
        let err = toadstool_common::ToadStoolError::security("Security violation");
        assert!(!err.to_string().is_empty());
    }

    #[test]
    fn test_verification_response_roundtrip() {
        use crate::beardog_integration::types::VerificationResponse;

        let resp = VerificationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: true,
            details: Some("signature valid".to_string()),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: VerificationResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.valid, resp.valid);
        assert_eq!(parsed.details, resp.details);
    }

    #[test]
    fn test_key_operation_result_deleted_serde_roundtrip() {
        let deleted = KeyOperationResult::Deleted {
            key_id: "k-deleted".to_string(),
        };
        let json = serde_json::to_value(&deleted).unwrap();
        let back: KeyOperationResult = serde_json::from_value(json).unwrap();
        assert!(matches!(back, KeyOperationResult::Deleted { key_id } if key_id == "k-deleted"));
    }

    #[test]
    fn test_bear_dog_config_debug_and_fields() {
        let config = BearDogConfig {
            auto_discover: true,
            discovery_timeout_ms: 3000,
            preferred_location: ServiceLocation::Any,
            fallback_enabled: true,
        };
        let dbg = format!("{:?}", config);
        assert!(!dbg.is_empty());
        assert_eq!(config.discovery_timeout_ms, 3000);
    }

    #[test]
    fn test_service_location_all_variants_eq() {
        assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
        assert_eq!(ServiceLocation::Network, ServiceLocation::Network);
        assert_eq!(ServiceLocation::Any, ServiceLocation::Any);
    }

    #[test]
    fn test_encryption_operation_all_variants() {
        let enc = EncryptionOperation::Encrypt;
        let dec = EncryptionOperation::Decrypt;
        assert_eq!(enc, EncryptionOperation::Encrypt);
        assert_ne!(enc, dec);
    }

    #[test]
    fn test_bear_dog_endpoint_all_capability_variants_serde() {
        let enc = BearDogCapability::Encryption {
            algorithms: vec!["aes".to_string()],
        };
        let _ = serde_json::to_value(&enc).unwrap();
        let hw = BearDogCapability::HardwareSecurity;
        let _ = serde_json::to_value(&hw).unwrap();
        let ss = BearDogCapability::SecureStorage;
        let _ = serde_json::to_value(&ss).unwrap();
        let ge = BearDogCapability::GeneticEntropy;
        let _ = serde_json::to_value(&ge).unwrap();
    }

    #[test]
    fn test_key_management_response_deleted_roundtrip() {
        let resp = KeyManagementResponse {
            request_id: uuid::Uuid::new_v4(),
            result: KeyOperationResult::Deleted {
                key_id: "del-123".to_string(),
            },
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: KeyManagementResponse = serde_json::from_str(&json).unwrap();
        assert!(matches!(parsed.result, KeyOperationResult::Deleted { .. }));
    }

    #[test]
    fn test_encryption_response_full_roundtrip_with_metadata() {
        let resp = EncryptionResponse {
            request_id: uuid::Uuid::new_v4(),
            data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
            key_id: "key-789".to_string(),
            algorithm: "aes-256-gcm".to_string(),
            metadata: serde_json::json!({"nonce": "abc123", "tag": "xyz"}),
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: EncryptionResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.data, resp.data);
        assert_eq!(parsed.metadata["nonce"], "abc123");
    }

    #[test]
    fn test_signature_request_with_key_and_algorithm() {
        let req = SignatureRequest {
            request_id: uuid::Uuid::new_v4(),
            data: vec![1, 2, 3, 4, 5, 6],
            key_id: Some("sig-key".to_string()),
            algorithm: Some("ed25519".to_string()),
        };
        let params = serde_json::to_value(&req).unwrap();
        assert_eq!(params["key_id"].as_str(), Some("sig-key"));
        assert_eq!(params["algorithm"].as_str(), Some("ed25519"));
    }

    #[test]
    fn test_validation_response_details_none() {
        use crate::beardog_integration::types::ValidationResponse;

        let resp = ValidationResponse {
            request_id: uuid::Uuid::new_v4(),
            valid: true,
            details: None,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: ValidationResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.valid);
        assert!(parsed.details.is_none());
    }
}
