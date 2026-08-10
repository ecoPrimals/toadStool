// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC over Unix socket operations against a discovered crypto service.

mod encryption_ops;
mod key_ops;
#[cfg(feature = "legacy-security")]
mod permission_ops;

use std::path::Path;
use std::time::Duration;

use toadstool_common::constants::timeouts;
use toadstool_common::primal_identity::{Capability, CryptoCapability, ServiceEndpoint};
use toadstool_common::service_discovery::DiscoveredService;
#[cfg(unix)]
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
#[cfg(unix)]
use toadstool_common::{NetworkError, ToadStoolError, ToadStoolResult};
#[cfg(not(unix))]
use toadstool_common::{ToadStoolError, ToadStoolResult};

/// Crypto service client - Makes requests to discovered services
///
/// **Design**: Works with ANY crypto provider via unix sockets (pure Rust!)
pub struct CryptoServiceClient {
    #[cfg(unix)]
    pub(super) rpc_client: UnixJsonRpcClient,
    /// Service endpoint information (stored for diagnostics and future use)
    _service_endpoint: ServiceEndpoint,
    /// Request timeout for RPC calls
    pub(super) timeout: Duration,
}

impl CryptoServiceClient {
    #[cfg(not(unix))]
    pub(super) fn unix_unavailable<T>() -> ToadStoolResult<T> {
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
