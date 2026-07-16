// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC over Unix socket operations against a discovered crypto service.

use std::path::Path;
use std::time::Duration;

#[cfg(unix)]
use base64::Engine;
#[cfg(unix)]
use toadstool_common::constants::timeouts;
use toadstool_common::primal_identity::{Capability, CryptoCapability, ServiceEndpoint};
use toadstool_common::service_discovery::DiscoveredService;
#[cfg(unix)]
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
#[cfg(unix)]
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};
#[cfg(not(unix))]
use toadstool_common::{ToadStoolError, ToadStoolResult};

use crate::crypto_integration::types::{
    CryptoRequest, CryptoResponse, KeyManagementRequest, KeyManagementResponse, PermissionResponse,
    RevocationRequest, SignatureRequest, SignatureResponse, ValidationResponse,
    VerificationRequest, VerificationResponse,
};

/// Crypto service client - Makes requests to discovered services
///
/// **Design**: Works with ANY crypto provider via unix sockets (pure Rust!)
pub struct CryptoServiceClient {
    #[cfg(unix)]
    rpc_client: UnixJsonRpcClient,
    /// Service endpoint information (stored for diagnostics and future use)
    _service_endpoint: ServiceEndpoint,
    /// Request timeout for RPC calls
    timeout: Duration,
}

impl CryptoServiceClient {
    #[cfg(not(unix))]
    fn unix_unavailable<T>() -> ToadStoolResult<T> {
        Err(ToadStoolError::configuration(
            "Unix socket crypto client is unavailable on this platform",
        ))
    }

    /// Create client for a discovered service with unix socket transport
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    ///
    /// **EVOLVED**: Uses the discovered service's actual endpoint, not hardcoded name.
    pub fn new(service: &DiscoveredService) -> ToadStoolResult<Self> {
        #[cfg(unix)]
        {
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
                // This allows ANY crypto service to work (Security, HSM, cloud KMS)
                toadstool_common::primal_sockets::resolve_socket_path_for_service(
                    &service.name,
                    &toadstool_common::primal_sockets::SocketPathEnv::from_env(),
                    None,
                )
            };

            let rpc_client = UnixJsonRpcClient::new(socket_path);

            Ok(Self {
                rpc_client,
                _service_endpoint: endpoint.clone(),
                timeout: timeouts::DEFAULT_REQUEST_TIMEOUT,
            })
        }
        #[cfg(not(unix))]
        {
            let _ = service;
            Err(ToadStoolError::configuration(
                "Unix socket crypto client is unavailable on this platform",
            ))
        }
    }

    /// Create client with custom timeout
    pub fn with_timeout(service: &DiscoveredService, timeout: Duration) -> ToadStoolResult<Self> {
        let mut client = Self::new(service)?;
        client.timeout = timeout;
        Ok(client)
    }

    /// Connect to a local crypto capability unix socket (startup / NUCLEUS composition).
    pub fn from_local_socket(socket: &Path) -> ToadStoolResult<Self> {
        let service = DiscoveredService {
            id: "local-crypto".to_string(),
            name: "crypto".to_string(),
            version: "1.0.0".to_string(),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            endpoints: vec![ServiceEndpoint {
                protocol: "unix".to_string(),
                address: socket.display().to_string(),
                port: 0,
                path: None,
                metadata: std::collections::HashMap::new(),
            }],
            metadata: Default::default(),
            discovered_at: std::time::SystemTime::now(),
            last_seen: std::time::SystemTime::now(),
            healthy: true,
        };
        Self::new(&service)
    }

    /// Encrypt data via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn encrypt(&self, request: CryptoRequest) -> ToadStoolResult<CryptoResponse> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
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
    }

    /// Decrypt data via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn decrypt(&self, request: CryptoRequest) -> ToadStoolResult<CryptoResponse> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
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
    }

    /// Manage keys (generate, rotate, delete) via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn manage_key(
        &self,
        request: KeyManagementRequest,
    ) -> ToadStoolResult<KeyManagementResponse> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
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
    }

    /// Retrieve a purpose key from the crypto provider secrets store.
    ///
    /// Key name: `"nucleus:{family}:purpose:{purpose}"`. When `family` is `None`,
    /// reads `TOADSTOOL_FAMILY_ID` (or related env vars).
    pub async fn retrieve_purpose_key(
        &self,
        purpose: &str,
        family: Option<&str>,
    ) -> ToadStoolResult<toadstool::encryption::EncryptionKey> {
        #[cfg(not(unix))]
        {
            let _ = (purpose, family);
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let family_id = match family {
                Some(f) => f.to_string(),
                None => std::env::var(
                    toadstool_common::interned_strings::socket_env::TOADSTOOL_FAMILY_ID,
                )
                .or_else(|_| {
                    std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_FAMILY)
                })
                .or_else(|_| {
                    std::env::var(toadstool_common::interned_strings::socket_env::BIOMEOS_FAMILY_ID)
                })
                .map_err(|_| {
                    ToadStoolError::configuration(
                        "TOADSTOOL_FAMILY_ID not set — cannot derive purpose key name",
                    )
                })?,
            };

            let key_name = format!("nucleus:{family_id}:purpose:{purpose}");

            let params = serde_json::json!({ "name": key_name });
            let response: serde_json::Value = self
                .rpc_client
                .call_typed("secrets.retrieve", params)
                .await
                .map_err(|e| {
                    ToadStoolError::Network(NetworkError::IoError {
                        reason: format!("secrets.retrieve(\"{key_name}\") failed: {e}"),
                    })
                })?;

            let key_material_b64 = response["key"]
                .as_str()
                .or_else(|| response["value"].as_str())
                .or_else(|| response.as_str())
                .ok_or_else(|| {
                    ToadStoolError::runtime(format!(
                        "secrets.retrieve(\"{key_name}\") returned no key material"
                    ))
                })?;

            let key_material = base64::engine::general_purpose::STANDARD
                .decode(key_material_b64)
                .map_err(|e| {
                    ToadStoolError::runtime(format!("purpose key base64 decode failed: {e}"))
                })?;

            let algorithm = response["algorithm"]
                .as_str()
                .unwrap_or("chacha20-poly1305")
                .to_string();

            Ok(toadstool::encryption::EncryptionKey::new(
                key_name,
                key_material,
                algorithm,
                toadstool::encryption::SecurityLevel::Enhanced,
            ))
        }
    }

    /// Sign data via unix socket
    pub async fn sign(&self, data: &[u8]) -> ToadStoolResult<SignatureResponse> {
        #[cfg(not(unix))]
        {
            let _ = data;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let request = SignatureRequest {
                request_id: uuid::Uuid::new_v4(),
                data: data.to_vec(),
                key_id: None,
                algorithm: None,
            };

            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client.call_typed("crypto.sign", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto sign timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto sign failed: {e}"),
                })
            })
        }
    }

    /// Verify signature via unix socket
    pub async fn verify(
        &self,
        data: &[u8],
        signature: &[u8],
        public_key_id: &str,
    ) -> ToadStoolResult<bool> {
        #[cfg(not(unix))]
        {
            let _ = (data, signature, public_key_id);
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let request = VerificationRequest {
                request_id: uuid::Uuid::new_v4(),
                data: data.to_vec(),
                signature: signature.to_vec(),
                public_key_id: public_key_id.to_string(),
            };

            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            let result: VerificationResponse = tokio::time::timeout(
                self.timeout,
                self.rpc_client.call_typed("crypto.verify", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto verify timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Crypto verify failed: {e}"),
                })
            })?;

            Ok(result.valid)
        }
    }

    /// Create permission via unix socket
    pub async fn create_permission(
        &self,
        request: &crate::security_provider::PermissionRequest,
    ) -> ToadStoolResult<PermissionResponse> {
        #[cfg(not(unix))]
        {
            let _ = request;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call_typed("crypto.create_permission", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission creation timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission creation failed: {e}"),
                })
            })
        }
    }

    /// Validate permission via unix socket
    pub async fn validate_permission(
        &self,
        permission: &crate::security_provider::SecurityPermission,
    ) -> ToadStoolResult<bool> {
        #[cfg(not(unix))]
        {
            let _ = permission;
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let params = serde_json::to_value(permission).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            let result: ValidationResponse = tokio::time::timeout(
                self.timeout,
                self.rpc_client
                    .call_typed("crypto.validate_permission", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission validation timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission validation failed: {e}"),
                })
            })?;

            Ok(result.valid)
        }
    }

    /// Revoke permission via unix socket
    pub async fn revoke_permission(
        &self,
        permission_id: &uuid::Uuid,
        reason: &str,
    ) -> ToadStoolResult<()> {
        #[cfg(not(unix))]
        {
            let _ = (permission_id, reason);
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
            let request = RevocationRequest {
                reason: reason.to_string(),
            };

            let mut params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Failed to serialize request: {e}"),
                })
            })?;

            if let Some(obj) = params.as_object_mut() {
                obj.insert(
                    "permission_id".to_string(),
                    serde_json::json!(permission_id.to_string()),
                );
            }

            let _: serde_json::Value = tokio::time::timeout(
                self.timeout,
                self.rpc_client.call("crypto.revoke_permission", params),
            )
            .await
            .map_err(|_| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission revocation timed out after {:?}", self.timeout),
                })
            })?
            .map_err(|e| {
                ToadStoolError::Network(NetworkError::IoError {
                    reason: format!("Permission revocation failed: {e}"),
                })
            })?;

            Ok(())
        }
    }

    /// Health check via unix socket
    ///
    /// **Pure Rust**: JSON-RPC over unix socket (no HTTP, no ring!)
    pub async fn health_check(&self) -> ToadStoolResult<bool> {
        #[cfg(not(unix))]
        {
            return Self::unix_unavailable();
        }
        #[cfg(unix)]
        {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use toadstool_common::primal_identity::{Capability, CryptoCapability, ServiceEndpoint};
    use toadstool_common::service_discovery::DiscoveredService;

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
    fn test_crypto_client_new_success_with_unix_endpoint() {
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
    fn test_crypto_client_new_with_metadata_socket_path() {
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
                address: toadstool_common::constants::network::LOCALHOST_IPV4.to_string(),
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
}
