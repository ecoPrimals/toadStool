// capability_provider.rs - Capability-based service discovery and invocation
//
// Deep Debt Solution: Primals discover each other by capability at runtime,
// not by hardcoded names. This enables true ecosystem agnosticism.
//
// Philosophy: "Know thyself, discover others"

use crate::primal_identity::Capability;
use crate::unix_jsonrpc_client::UnixJsonRpcClient;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Errors for capability-based discovery
#[derive(Debug, thiserror::Error)]
pub enum CapabilityError {
    #[error("No provider found for capability: {0:?}")]
    NoProviderFound(Capability),

    #[error("Provider unreachable: {0}")]
    ProviderUnreachable(String),

    #[error("RPC call failed: {0}")]
    RpcFailed(String),

    #[error("Discovery service unavailable")]
    DiscoveryUnavailable,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub type Result<T> = std::result::Result<T, CapabilityError>;

/// Provider of a specific capability
///
/// This abstracts away which primal provides the capability.
/// Client code doesn't know or care if it's beardog, songbird, etc.
#[derive(Debug, Clone)]
pub struct CapabilityProvider {
    /// Service name (for logging/debugging only, not used for logic!)
    service_name: String,

    /// Unix socket path to communicate with provider
    socket_path: PathBuf,

    /// Capabilities this provider offers
    capabilities: Vec<Capability>,

    /// Cached client connection
    client: Arc<RwLock<Option<UnixJsonRpcClient>>>,
}

impl CapabilityProvider {
    /// Discover a provider for a specific capability
    ///
    /// This queries the discovery service (typically Songbird) to find
    /// which primal currently provides this capability.
    ///
    /// # Deep Debt Principle
    ///
    /// We don't hardcode "beardog" for crypto or "nestgate" for storage.
    /// We ask: "Who can do X?" and use whoever answers.
    ///
    /// # Errors
    ///
    /// Returns `CapabilityError::NoProviderFound` if no service offers this capability.
    /// Returns `CapabilityError::DiscoveryUnavailable` if can't reach discovery service.
    pub async fn discover(capability: Capability) -> Result<Self> {
        // Query Songbird (or discovery service) for this capability
        let discovery_socket =
            std::env::var("SONGBIRD_SOCKET").unwrap_or_else(|_| "/primal/songbird".to_string());

        let client = UnixJsonRpcClient::new(&discovery_socket);

        // Call ipc.find_capability
        let params = serde_json::json!({
            "capability": capability_to_string(&capability)
        });

        let response = client
            .call("ipc.find_capability", params)
            .await
            .map_err(|_| CapabilityError::DiscoveryUnavailable)?;

        // Parse response
        let services = response["services"]
            .as_array()
            .ok_or_else(|| CapabilityError::InvalidResponse("No services array".into()))?;

        if services.is_empty() {
            return Err(CapabilityError::NoProviderFound(capability));
        }

        // Take first available provider
        let service = &services[0];
        let service_name = service["name"]
            .as_str()
            .ok_or_else(|| CapabilityError::InvalidResponse("No name field".into()))?
            .to_string();

        let endpoint = service["endpoint"]
            .as_str()
            .ok_or_else(|| CapabilityError::InvalidResponse("No endpoint field".into()))?;

        let socket_path = PathBuf::from(endpoint);

        let capabilities = service["capabilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(string_to_capability)
                    .collect()
            })
            .unwrap_or_default();

        Ok(Self {
            service_name,
            socket_path,
            capabilities,
            client: Arc::new(RwLock::new(None)),
        })
    }

    /// Call a method on this capability provider
    ///
    /// Uses JSON-RPC 2.0 over Unix sockets (wateringHole standard)
    ///
    /// # Errors
    ///
    /// Returns error if provider is unreachable or call fails
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        // Get or create client connection
        let mut client_lock = self.client.write().await;

        if client_lock.is_none() {
            let new_client = UnixJsonRpcClient::new(&self.socket_path);
            *client_lock = Some(new_client);
        }

        // SAFETY: We just ensured the client is Some above
        let Some(client) = client_lock.as_mut() else {
            return Err(CapabilityError::ProviderUnreachable(
                "Client initialization failed unexpectedly".to_string(),
            ));
        };

        // Call method
        client
            .call(method, params)
            .await
            .map_err(|e| CapabilityError::RpcFailed(e.to_string()))
    }

    /// Get service name (for logging/debugging only!)
    ///
    /// WARNING: Do NOT use this for logic decisions!
    /// Use capabilities, not names, for behavior.
    #[must_use]
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get capabilities offered by this provider
    #[must_use]
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Check if provider offers a specific capability
    #[must_use]
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
}

/// Discover multiple providers for a capability
///
/// Useful for load balancing or failover scenarios
///
/// # Errors
///
/// Returns [`CapabilityError`] if:
/// - Discovery service is unavailable (socket unreachable)
/// - Response is invalid (missing services array, name, or endpoint fields)
pub async fn discover_all(capability: Capability) -> Result<Vec<CapabilityProvider>> {
    let discovery_socket =
        std::env::var("SONGBIRD_SOCKET").unwrap_or_else(|_| "/primal/songbird".to_string());

    let client = UnixJsonRpcClient::new(&discovery_socket);

    let params = serde_json::json!({
        "capability": capability_to_string(&capability)
    });

    let response = client
        .call("ipc.find_capability", params)
        .await
        .map_err(|_| CapabilityError::DiscoveryUnavailable)?;

    let services = response["services"]
        .as_array()
        .ok_or_else(|| CapabilityError::InvalidResponse("No services array".into()))?;

    let mut providers = Vec::new();

    for service in services {
        let service_name = service["name"]
            .as_str()
            .ok_or_else(|| CapabilityError::InvalidResponse("No name field".into()))?
            .to_string();

        let endpoint = service["endpoint"]
            .as_str()
            .ok_or_else(|| CapabilityError::InvalidResponse("No endpoint field".into()))?;

        let socket_path = PathBuf::from(endpoint);

        let capabilities = service["capabilities"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(string_to_capability)
                    .collect()
            })
            .unwrap_or_default();

        providers.push(CapabilityProvider {
            service_name,
            socket_path,
            capabilities,
            client: Arc::new(RwLock::new(None)),
        });
    }

    Ok(providers)
}

// Helper functions for capability serialization
// (These should match the Capability enum in service_discovery.rs)

fn capability_to_string(cap: &Capability) -> String {
    // Match actual Capability enum variants from primal_identity
    match cap {
        Capability::Compute(_) => "compute".to_string(),
        Capability::Storage(_) => "storage".to_string(),
        Capability::Crypto(_) => "crypto".to_string(),
        Capability::Authentication(_) => "authentication".to_string(),
        Capability::Coordination(_) => "coordination".to_string(),
        Capability::Discovery(_) => "discovery".to_string(),
        Capability::Custom { name, .. } => name.clone(),
    }
}

fn string_to_capability(s: &str) -> Capability {
    use crate::primal_identity::{
        AuthCapability, Capability, ComputeCapability, CoordinationCapability, CryptoCapability,
        DiscoveryCapability, StorageCapability,
    };

    // Parse actual capability variants, use Custom for unknown
    match s {
        "compute" => Capability::Compute(ComputeCapability::NativeExecution),
        "storage" => Capability::Storage(StorageCapability::ObjectStorage),
        "crypto" => Capability::Crypto(CryptoCapability::Encryption),
        "authentication" | "security" => {
            Capability::Authentication(AuthCapability::TokenManagement)
        }
        "coordination" => Capability::Coordination(CoordinationCapability::ServiceDiscovery),
        "discovery" => Capability::Discovery(DiscoveryCapability::RegistryDiscovery),
        other => Capability::Custom {
            name: other.to_string(),
            version: "1.0".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primal_identity::CryptoCapability;

    fn run_async<F: std::future::Future<Output = O> + Send, O: Send>(
        f: impl FnOnce() -> F + Send,
    ) -> O {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        rt.block_on(f())
    }

    #[tokio::test]
    async fn test_capability_provider_structure() {
        // Test that provider has expected fields
        let provider = CapabilityProvider {
            service_name: "test-provider".to_string(),
            socket_path: PathBuf::from("/tmp/test.sock"),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            client: Arc::new(RwLock::new(None)),
        };

        assert_eq!(provider.service_name(), "test-provider");
        assert_eq!(provider.capabilities().len(), 1);
    }

    #[test]
    fn test_capability_serialization() {
        let cap = Capability::Crypto(CryptoCapability::Encryption);
        let s = capability_to_string(&cap);
        assert_eq!(s, "crypto");

        let cap2 = string_to_capability("crypto");
        assert_eq!(capability_to_string(&cap2), "crypto");
    }

    #[test]
    fn test_capability_serialization_all_variants() {
        use crate::primal_identity::*;

        // Test all capability type conversions
        let compute = Capability::Compute(ComputeCapability::NativeExecution);
        assert_eq!(capability_to_string(&compute), "compute");

        let storage = Capability::Storage(StorageCapability::ObjectStorage);
        assert_eq!(capability_to_string(&storage), "storage");

        let crypto = Capability::Crypto(CryptoCapability::Encryption);
        assert_eq!(capability_to_string(&crypto), "crypto");

        let auth = Capability::Authentication(AuthCapability::TokenManagement);
        assert_eq!(capability_to_string(&auth), "authentication");

        let coord = Capability::Coordination(CoordinationCapability::ServiceDiscovery);
        assert_eq!(capability_to_string(&coord), "coordination");

        let disc = Capability::Discovery(DiscoveryCapability::RegistryDiscovery);
        assert_eq!(capability_to_string(&disc), "discovery");

        let custom = Capability::Custom {
            name: "custom_cap".to_string(),
            version: "1.0".to_string(),
        };
        assert_eq!(capability_to_string(&custom), "custom_cap");
    }

    #[test]
    fn test_string_to_capability_all_variants() {
        // Test string parsing for all types
        let compute = string_to_capability("compute");
        assert_eq!(capability_to_string(&compute), "compute");

        let storage = string_to_capability("storage");
        assert_eq!(capability_to_string(&storage), "storage");

        let crypto = string_to_capability("crypto");
        assert_eq!(capability_to_string(&crypto), "crypto");

        let auth1 = string_to_capability("authentication");
        assert_eq!(capability_to_string(&auth1), "authentication");

        let auth2 = string_to_capability("security");
        assert_eq!(capability_to_string(&auth2), "authentication");

        let coord = string_to_capability("coordination");
        assert_eq!(capability_to_string(&coord), "coordination");

        let disc = string_to_capability("discovery");
        assert_eq!(capability_to_string(&disc), "discovery");

        let custom = string_to_capability("unknown_capability");
        assert_eq!(capability_to_string(&custom), "unknown_capability");
    }

    #[test]
    fn test_capability_error_variants() {
        use crate::primal_identity::CryptoCapability;

        let err1 =
            CapabilityError::NoProviderFound(Capability::Crypto(CryptoCapability::Encryption));
        assert!(err1.to_string().contains("No provider found"));

        let err2 = CapabilityError::ProviderUnreachable("test-service".to_string());
        assert!(err2.to_string().contains("test-service"));

        let err3 = CapabilityError::RpcFailed("connection timeout".to_string());
        assert!(err3.to_string().contains("connection timeout"));

        let err4 = CapabilityError::DiscoveryUnavailable;
        assert!(err4.to_string().contains("unavailable"));

        let err5 = CapabilityError::InvalidResponse("malformed json".to_string());
        assert!(err5.to_string().contains("malformed json"));
    }

    #[tokio::test]
    async fn test_has_capability() {
        use crate::primal_identity::*;

        let provider = CapabilityProvider {
            service_name: "test-provider".to_string(),
            socket_path: PathBuf::from("/tmp/test.sock"),
            capabilities: vec![
                Capability::Crypto(CryptoCapability::Encryption),
                Capability::Crypto(CryptoCapability::KeyManagement),
            ],
            client: Arc::new(RwLock::new(None)),
        };

        // Test capability checks
        assert!(provider.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
        assert!(provider.has_capability(&Capability::Crypto(CryptoCapability::KeyManagement)));
        assert!(!provider.has_capability(&Capability::Storage(StorageCapability::ObjectStorage)));
    }

    #[tokio::test]
    async fn test_capabilities_getter() {
        use crate::primal_identity::*;

        let caps = vec![
            Capability::Crypto(CryptoCapability::Encryption),
            Capability::Storage(StorageCapability::ObjectStorage),
        ];

        let provider = CapabilityProvider {
            service_name: "multi-provider".to_string(),
            socket_path: PathBuf::from("/tmp/multi.sock"),
            capabilities: caps.clone(),
            client: Arc::new(RwLock::new(None)),
        };

        let retrieved_caps = provider.capabilities();
        assert_eq!(retrieved_caps.len(), 2);
        assert_eq!(retrieved_caps, &caps[..]);
    }

    #[tokio::test]
    async fn test_service_name_getter() {
        let provider = CapabilityProvider {
            service_name: "my-service".to_string(),
            socket_path: PathBuf::from("/tmp/service.sock"),
            capabilities: vec![],
            client: Arc::new(RwLock::new(None)),
        };

        assert_eq!(provider.service_name(), "my-service");
    }

    #[test]
    fn test_custom_capability_roundtrip() {
        let custom = Capability::Custom {
            name: "my_custom_cap".to_string(),
            version: "2.0".to_string(),
        };

        let serialized = capability_to_string(&custom);
        assert_eq!(serialized, "my_custom_cap");

        let deserialized = string_to_capability(&serialized);
        match deserialized {
            Capability::Custom { name, .. } => assert_eq!(name, "my_custom_cap"),
            _ => panic!("Expected Custom capability"),
        }
    }

    #[test]
    fn test_capability_error_debug() {
        let err = CapabilityError::DiscoveryUnavailable;
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("DiscoveryUnavailable"));
    }

    #[tokio::test]
    async fn test_provider_clone() {
        use crate::primal_identity::CryptoCapability;

        let provider1 = CapabilityProvider {
            service_name: "original".to_string(),
            socket_path: PathBuf::from("/tmp/orig.sock"),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            client: Arc::new(RwLock::new(None)),
        };

        let provider2 = provider1.clone();
        assert_eq!(provider1.service_name(), provider2.service_name());
        assert_eq!(provider1.capabilities(), provider2.capabilities());
    }

    #[test]
    fn test_discover_fails_when_socket_unavailable() {
        temp_env::with_var(
            "SONGBIRD_SOCKET",
            Some("/tmp/nonexistent_toadstool_test_12345.sock"),
            || {
                let result = run_async(|| {
                    CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
                });
                assert!(result.is_err());
                assert!(matches!(
                    result.unwrap_err(),
                    CapabilityError::DiscoveryUnavailable
                ));
            },
        );
    }

    #[test]
    fn test_discover_all_fails_when_socket_unavailable() {
        temp_env::with_var(
            "SONGBIRD_SOCKET",
            Some("/tmp/nonexistent_toadstool_test_67890.sock"),
            || {
                let result =
                    run_async(|| discover_all(Capability::Crypto(CryptoCapability::Encryption)));
                assert!(result.is_err());
                assert!(matches!(
                    result.unwrap_err(),
                    CapabilityError::DiscoveryUnavailable
                ));
            },
        );
    }

    #[tokio::test]
    async fn test_call_fails_when_socket_unavailable() {
        let provider = CapabilityProvider {
            service_name: "unreachable".to_string(),
            socket_path: PathBuf::from("/tmp/nonexistent_toadstool_call_test.sock"),
            capabilities: vec![Capability::Crypto(CryptoCapability::Encryption)],
            client: Arc::new(RwLock::new(None)),
        };

        let result = provider.call("test.method", serde_json::json!({})).await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), CapabilityError::RpcFailed(_)));
    }

    /// Spawn a mock JSON-RPC server that returns the given result
    async fn spawn_mock_discovery_server(
        result: serde_json::Value,
    ) -> (PathBuf, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::UnixListener;

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system time before UNIX_EPOCH")
            .as_nanos();
        let socket_path = std::env::temp_dir().join(format!(
            "toadstool_cap_test_{}_{}.sock",
            std::process::id(),
            nanos
        ));
        let _ = std::fs::remove_file(&socket_path);

        let listener = UnixListener::bind(&socket_path).expect("bind mock socket");
        let handle = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.expect("accept");
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            let _ = reader.read_line(&mut line).await;

            let id = serde_json::from_str::<serde_json::Value>(&line)
                .ok()
                .and_then(|r| r.get("id").cloned())
                .unwrap_or(serde_json::json!(1));

            let response = serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": result
            });
            let resp_line = format!("{}\n", serde_json::to_string(&response).unwrap());
            let _ = writer.write_all(resp_line.as_bytes()).await;
            let _ = writer.flush().await;
        });

        // UnixListener::bind() creates the socket file and calls listen() before
        // returning, so clients can connect immediately — no sleep needed.
        (socket_path, handle)
    }

    #[test]
    fn test_discover_success() {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let result = serde_json::json!({
            "services": [{
                "name": "beardog",
                "endpoint": "/tmp/beardog.sock",
                "capabilities": ["crypto", "encryption"]
            }]
        });
        let (socket_path, _server) = rt.block_on(spawn_mock_discovery_server(result));
        let path_str = socket_path.to_str().unwrap().to_string();
        let provider = temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
            rt.block_on(CapabilityProvider::discover(Capability::Crypto(
                CryptoCapability::Encryption,
            )))
            .expect("discover should succeed")
        });
        std::fs::remove_file(&socket_path).ok();

        assert_eq!(provider.service_name(), "beardog");
        assert!(provider.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
    }

    #[test]
    fn test_discover_no_provider_found() {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let result = serde_json::json!({ "services": [] });
        let (socket_path, _server) = rt.block_on(spawn_mock_discovery_server(result));
        let path_str = socket_path.to_str().unwrap().to_string();
        let err = temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
            rt.block_on(CapabilityProvider::discover(Capability::Crypto(
                CryptoCapability::Encryption,
            )))
        })
        .unwrap_err();
        std::fs::remove_file(&socket_path).ok();

        assert!(matches!(err, CapabilityError::NoProviderFound(_)));
    }

    #[test]
    fn test_discover_invalid_response_no_services_array() {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let result = serde_json::json!({ "not_services": [] });
        let (socket_path, _server) = rt.block_on(spawn_mock_discovery_server(result));
        let path_str = socket_path.to_str().unwrap().to_string();
        let err = temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
            rt.block_on(CapabilityProvider::discover(Capability::Crypto(
                CryptoCapability::Encryption,
            )))
        })
        .unwrap_err();
        std::fs::remove_file(&socket_path).ok();

        assert!(matches!(err, CapabilityError::InvalidResponse(_)));
        assert!(err.to_string().contains("No services array"));
    }

    #[test]
    fn test_discover_invalid_response_no_name() {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let result = serde_json::json!({
            "services": [{ "endpoint": "/tmp/x.sock", "capabilities": [] }]
        });
        let (socket_path, _server) = rt.block_on(spawn_mock_discovery_server(result));
        let path_str = socket_path.to_str().unwrap().to_string();
        let err = temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
            rt.block_on(CapabilityProvider::discover(Capability::Crypto(
                CryptoCapability::Encryption,
            )))
        })
        .unwrap_err();
        std::fs::remove_file(&socket_path).ok();

        assert!(matches!(err, CapabilityError::InvalidResponse(_)));
        assert!(err.to_string().contains("No name field"));
    }

    #[test]
    fn test_discover_invalid_response_no_endpoint() {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let result = serde_json::json!({
            "services": [{ "name": "beardog", "capabilities": [] }]
        });
        let (socket_path, _server) = rt.block_on(spawn_mock_discovery_server(result));
        let path_str = socket_path.to_str().unwrap().to_string();
        let err = temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
            rt.block_on(CapabilityProvider::discover(Capability::Crypto(
                CryptoCapability::Encryption,
            )))
        })
        .unwrap_err();
        std::fs::remove_file(&socket_path).ok();

        assert!(matches!(err, CapabilityError::InvalidResponse(_)));
        assert!(err.to_string().contains("No endpoint field"));
    }

    #[test]
    fn test_discover_all_success() {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let result = serde_json::json!({
            "services": [
                { "name": "beardog1", "endpoint": "/tmp/b1.sock", "capabilities": ["crypto"] },
                { "name": "beardog2", "endpoint": "/tmp/b2.sock", "capabilities": ["crypto"] }
            ]
        });
        let (socket_path, _server) = rt.block_on(spawn_mock_discovery_server(result));
        let path_str = socket_path.to_str().unwrap().to_string();
        let providers = temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
            rt.block_on(discover_all(Capability::Crypto(
                CryptoCapability::Encryption,
            )))
            .expect("discover_all should succeed")
        });
        std::fs::remove_file(&socket_path).ok();

        assert_eq!(providers.len(), 2);
        assert_eq!(providers[0].service_name(), "beardog1");
        assert_eq!(providers[1].service_name(), "beardog2");
    }

    #[test]
    fn test_discover_capabilities_from_service() {
        let rt = tokio::runtime::Runtime::new().expect("create runtime");
        let result = serde_json::json!({
            "services": [{
                "name": "beardog",
                "endpoint": "/tmp/beardog.sock",
                "capabilities": ["crypto", "authentication", "custom_cap"]
            }]
        });
        let (socket_path, _server) = rt.block_on(spawn_mock_discovery_server(result));
        let path_str = socket_path.to_str().unwrap().to_string();
        let provider = temp_env::with_var("SONGBIRD_SOCKET", Some(path_str.as_str()), || {
            rt.block_on(CapabilityProvider::discover(Capability::Crypto(
                CryptoCapability::Encryption,
            )))
            .expect("discover should succeed")
        });
        std::fs::remove_file(&socket_path).ok();

        let caps = provider.capabilities();
        assert_eq!(caps.len(), 3);
        assert!(provider.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
    }

    #[test]
    fn test_discover_default_socket_path() {
        temp_env::with_var_unset("SONGBIRD_SOCKET", || {
            let result = run_async(|| {
                CapabilityProvider::discover(Capability::Crypto(CryptoCapability::Encryption))
            });
            assert!(result.is_err());
            assert!(matches!(
                result.unwrap_err(),
                CapabilityError::DiscoveryUnavailable
            ));
        });
    }
}
