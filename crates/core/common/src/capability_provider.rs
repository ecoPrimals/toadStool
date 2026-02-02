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
                    .map(|s| string_to_capability(s))
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

        let client = client_lock.as_mut().unwrap();

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
    pub fn service_name(&self) -> &str {
        &self.service_name
    }

    /// Get capabilities offered by this provider
    pub fn capabilities(&self) -> &[Capability] {
        &self.capabilities
    }

    /// Check if provider offers a specific capability
    pub fn has_capability(&self, cap: &Capability) -> bool {
        self.capabilities.contains(cap)
    }
}

/// Discover multiple providers for a capability
///
/// Useful for load balancing or failover scenarios
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
                    .map(|s| string_to_capability(s))
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
    use crate::primal_identity::*;

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
}
