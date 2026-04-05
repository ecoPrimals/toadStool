// SPDX-License-Identifier: AGPL-3.0-only
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

use toadstool_common::interned_strings::capabilities;
use toadstool_common::primal_sockets::{discover_crypto_socket, get_socket_path_for_capability};
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use toadstool_common::{ToadStoolError, ToadStoolResult};

use super::BearDogConfig;
use super::discovery::BearDogDiscovery;
use super::types::{
    BearDogEndpoint, EncryptionRequest, EncryptionResponse, KeyManagementRequest,
    KeyManagementResponse, PermissionResponse, RevocationRequest, SignatureRequest,
    SignatureResponse, ValidationResponse, VerificationRequest, VerificationResponse,
};

/// BearDog Unix Socket Client (Pure Rust!)
///
/// **Design**: JSON-RPC 2.0 over unix sockets (no HTTP, no TLS, no ring!)
/// **TRUE PRIMAL**: Local IPC for compute primal → security primal communication
pub struct BearDogClient {
    discovery: Arc<BearDogDiscovery>,
    rpc_client: UnixJsonRpcClient,
}

impl BearDogClient {
    /// Parse CryptoCapability from JSON response (pure function, testable without network)
    #[doc(hidden)]
    pub fn parse_capabilities_from_json(
        response: &serde_json::Value,
    ) -> toadstool::encryption::CryptoCapability {
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
            .unwrap_or_default();

        toadstool::encryption::CryptoCapability {
            algorithms,
            security_level,
            hardware_backed,
        }
    }

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
        let socket_path = discover_crypto_socket().await.map_err(|e| {
            ToadStoolError::configuration(format!(
                "No crypto service discovered: {e}. Ensure a security/crypto service is running.",
            ))
        })?;

        let rpc_client = UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            discovery: Arc::new(BearDogDiscovery::new(config)),
            rpc_client,
        })
    }

    /// Create new BearDog client with unix socket transport
    ///
    /// **DEPRECATED**: Use `new_async()` for capability-based discovery.
    ///
    /// # Errors
    /// Returns error if socket path discovery fails
    #[deprecated(
        since = "0.3.0",
        note = "Use new_async() for capability-based discovery"
    )]
    #[allow(deprecated)] // new() uses deprecated BearDogConfig; migration in progress
    pub fn new(config: BearDogConfig) -> ToadStoolResult<Self> {
        let socket_path = get_socket_path_for_capability("crypto");
        let rpc_client = UnixJsonRpcClient::new(socket_path);

        Ok(Self {
            discovery: Arc::new(BearDogDiscovery::new(config)),
            rpc_client,
        })
    }

    /// Create client with custom socket path (for testing error paths when service unavailable)
    #[cfg(test)]
    pub fn new_with_socket_path(
        config: BearDogConfig,
        socket_path: std::path::PathBuf,
    ) -> ToadStoolResult<Self> {
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
    pub async fn query_capabilities_async(
        &self,
    ) -> ToadStoolResult<toadstool::encryption::CryptoCapability> {
        let response: serde_json::Value = self
            .rpc_client
            .call_typed("crypto.capabilities", serde_json::json!({}))
            .await
            .map_err(|e| {
                tracing::warn!("security/crypto service capabilities query failed: {}", e);
                ToadStoolError::network(format!(
                    "security/crypto service capabilities query failed: {e}"
                ))
            })?;

        Ok(Self::parse_capabilities_from_json(&response))
    }

    /// Encrypt data using BearDog via unix socket
    pub async fn encrypt(&self, request: EncryptionRequest) -> ToadStoolResult<EncryptionResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        self.rpc_client.call_typed("crypto.encrypt", params).await
    }

    /// Decrypt data using BearDog via unix socket
    pub async fn decrypt(&self, request: EncryptionRequest) -> ToadStoolResult<EncryptionResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        self.rpc_client.call_typed("crypto.decrypt", params).await
    }

    /// Manage keys using BearDog via unix socket
    pub async fn key_management(
        &self,
        request: KeyManagementRequest,
    ) -> ToadStoolResult<KeyManagementResponse> {
        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        self.rpc_client
            .call_typed("crypto.key_management", params)
            .await
    }

    /// Sign data using BearDog via unix socket
    pub async fn sign(&self, data: &[u8]) -> ToadStoolResult<SignatureResponse> {
        let request = SignatureRequest {
            request_id: uuid::Uuid::new_v4(),
            data: data.to_vec(),
            key_id: None,
            algorithm: None,
        };

        let params = serde_json::to_value(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        self.rpc_client.call_typed("crypto.sign", params).await
    }

    /// Verify signature using BearDog via unix socket
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
            self.rpc_client.call_typed("crypto.verify", params).await?;

        Ok(result.valid)
    }

    /// Create permission using BearDog via unix socket
    pub async fn create_permission(
        &self,
        request: &crate::security_provider::PermissionRequest,
    ) -> ToadStoolResult<PermissionResponse> {
        let params = serde_json::to_value(request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        self.rpc_client
            .call_typed("crypto.create_permission", params)
            .await
    }

    /// Validate permission using BearDog via unix socket
    pub async fn validate_permission(
        &self,
        permission: &crate::security_provider::SecurityPermission,
    ) -> ToadStoolResult<bool> {
        let params = serde_json::to_value(permission)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        let result: ValidationResponse = self
            .rpc_client
            .call_typed("crypto.validate_permission", params)
            .await?;

        Ok(result.valid)
    }

    /// Revoke permission using BearDog via unix socket
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

        if let Some(obj) = params.as_object_mut() {
            obj.insert(
                "permission_id".to_string(),
                serde_json::json!(permission_id.to_string()),
            );
        }

        let _: serde_json::Value = self
            .rpc_client
            .call("crypto.revoke_permission", params)
            .await?;

        Ok(())
    }

    /// Check health of BearDog services
    pub async fn health_check(&self) -> ToadStoolResult<Vec<BearDogEndpoint>> {
        let endpoints = self.discovery.discover().await?;

        let mut checked_endpoints = Vec::with_capacity(endpoints.len());

        for mut endpoint in endpoints {
            let start = std::time::Instant::now();

            let health_result: Result<serde_json::Value, _> = self
                .rpc_client
                .call_typed("crypto.health", serde_json::json!({}))
                .await;

            let latency_ms = start.elapsed().as_millis() as u64;

            match health_result {
                Ok(_response) => {
                    endpoint.healthy = true;
                    endpoint.latency_ms = Some(latency_ms);
                }
                Err(_) => {
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
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
impl toadstool::encryption::CryptoProvider for BearDogClient {
    fn provider_id(&self) -> &str {
        capabilities::CRYPTO
    }

    fn capabilities(&self) -> &toadstool::encryption::CryptoCapability {
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
            nonce: Vec::new(),
            aad: None,
            kdf_info: None,
            encrypted_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
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
                    Vec::new(),
                    algorithm,
                    security_level,
                ))
            }
            super::types::KeyOperationResult::Error { message } => Err(ToadStoolError::runtime(
                format!("security/crypto service key generation failed: {message}"),
            )),
            _ => Err(ToadStoolError::runtime(
                "Unexpected response from security/crypto service",
            )),
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
                toadstool::encryption::SecurityLevel::Standard,
            )),
            super::types::KeyOperationResult::Error { message } => Err(ToadStoolError::not_found(
                format!("security/crypto service key not found: {message}"),
            )),
            _ => Err(ToadStoolError::runtime(
                "Unexpected response from security/crypto service",
            )),
        }
    }

    async fn health_check(
        &self,
    ) -> ToadStoolResult<toadstool::encryption::provider::ProviderHealth> {
        let endpoints = self.health_check().await?;

        if endpoints.is_empty() {
            return Ok(toadstool::encryption::provider::ProviderHealth::unhealthy(
                "No security/crypto endpoints available",
            ));
        }

        let healthy_count = endpoints.iter().filter(|e| e.healthy).count();
        if healthy_count == 0 {
            return Ok(toadstool::encryption::provider::ProviderHealth::unhealthy(
                "All security/crypto service endpoints unhealthy",
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
mod tests;
