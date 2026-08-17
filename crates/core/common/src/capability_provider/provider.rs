// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability provider abstraction.
//!
//! Provider of a specific capability. Abstracts away which primal provides the capability.
//! Client code doesn't depend on which concrete service implements the capability.

use crate::primal_identity::Capability;
#[cfg(unix)]
use crate::unix_jsonrpc_client::UnixJsonRpcClient;
use serde_json::Value;
use std::path::PathBuf;
#[cfg(unix)]
use std::sync::Arc;
#[cfg(unix)]
use std::sync::RwLock;

#[cfg(unix)]
use super::discovery;
use super::error::{CapabilityError, Result};

/// Provider of a specific capability
///
/// This abstracts away which primal provides the capability.
/// Client code does not depend on a fixed peer product name.
#[derive(Debug, Clone)]
pub struct CapabilityProvider {
    /// Service name (for logging/debugging only, not used for logic!)
    service_name: String,

    /// Unix socket path to communicate with provider
    socket_path: PathBuf,

    /// Capabilities this provider offers
    capabilities: Vec<Capability>,

    /// Cached client connection
    #[cfg(unix)]
    client: Arc<RwLock<Option<UnixJsonRpcClient>>>,
}

impl CapabilityProvider {
    /// Create provider from discovery service info (used by discovery module)
    #[cfg_attr(not(unix), allow(dead_code))]
    pub(crate) fn from_service_info(
        service_name: String,
        socket_path: PathBuf,
        capabilities: Vec<Capability>,
    ) -> Self {
        Self {
            service_name,
            socket_path,
            capabilities,
            #[cfg(unix)]
            client: Arc::new(RwLock::new(None)),
        }
    }

    /// Discover a provider for a specific capability
    ///
    /// This queries the coordination / discovery service to find
    /// which peer currently provides this capability.
    ///
    /// # Deep Debt Principle
    ///
    /// We don't hardcode legacy route labels for crypto or storage.
    /// We ask: "Who can do X?" and use whoever answers.
    ///
    /// # Errors
    ///
    /// Returns `CapabilityError::NoProviderFound` if no service offers this capability.
    /// Returns `CapabilityError::DiscoveryUnavailable` if can't reach discovery service.
    #[cfg(unix)]
    pub async fn discover(capability: Capability) -> Result<Self> {
        let providers = discovery::query_providers(capability.clone()).await?;
        providers
            .into_iter()
            .next()
            .ok_or(CapabilityError::NoProviderFound(capability))
    }

    /// Discover a provider for a specific capability (non-Unix stub)
    #[cfg(not(unix))]
    pub async fn discover(_capability: Capability) -> Result<Self> {
        Err(CapabilityError::DiscoveryUnavailable)
    }

    /// Call a method on this capability provider
    ///
    /// Uses JSON-RPC 2.0 over Unix sockets (wateringHole standard)
    ///
    /// # Errors
    ///
    /// Returns error if provider is unreachable or call fails
    #[cfg(unix)]
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let client = {
            let mut client_lock = self
                .client
                .write()
                .unwrap_or_else(std::sync::PoisonError::into_inner);

            #[expect(
                clippy::option_if_let_else,
                reason = "map_or_else would cause borrow conflict with mutation"
            )]
            if let Some(c) = client_lock.as_ref() {
                c.clone()
            } else {
                let new_client = UnixJsonRpcClient::new(&self.socket_path);
                *client_lock = Some(new_client.clone());
                new_client
            }
        };

        client
            .call(method, params)
            .await
            .map_err(|e| CapabilityError::RpcFailed(e.to_string()))
    }

    /// Call a method on this capability provider (non-Unix stub)
    #[cfg(not(unix))]
    pub async fn call(&self, _method: &str, _params: Value) -> Result<Value> {
        Err(CapabilityError::RpcFailed(
            "Unix domain sockets are not available on this platform".to_string(),
        ))
    }

    /// Get the socket path for this provider.
    ///
    /// Used by callers that need to construct their own client connection
    /// (e.g., when wrapping in a domain-specific client type).
    #[must_use]
    pub fn socket_path(&self) -> &std::path::Path {
        &self.socket_path
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::primal_identity::{
        Capability, ComputeCapability, CryptoCapability, StorageCapability,
    };

    use super::CapabilityProvider;

    #[test]
    fn from_service_info_sets_socket_path_and_service_name() {
        let path = std::path::PathBuf::from("/tmp/cap-provider-test.sock");
        let p = CapabilityProvider::from_service_info(
            "alpha-service".to_string(),
            path,
            vec![Capability::Crypto(CryptoCapability::Encryption)],
        );
        assert_eq!(p.service_name(), "alpha-service");
        assert_eq!(p.socket_path(), Path::new("/tmp/cap-provider-test.sock"));
    }

    #[test]
    fn socket_path_accessor_returns_inner_path() {
        let p = CapabilityProvider::from_service_info(
            "x".to_string(),
            std::path::PathBuf::from("/var/run/custom.sock"),
            vec![],
        );
        assert_eq!(p.socket_path().as_os_str(), "/var/run/custom.sock");
    }

    #[test]
    fn capabilities_empty_slice_when_none_registered() {
        let p = CapabilityProvider::from_service_info(
            "empty".to_string(),
            std::path::PathBuf::from("/tmp/e.sock"),
            vec![],
        );
        assert!(p.capabilities().is_empty());
        assert!(!p.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
    }

    #[test]
    fn capabilities_and_has_capability_reflect_vector_contents() {
        let caps = vec![
            Capability::Compute(ComputeCapability::NativeExecution),
            Capability::Storage(StorageCapability::ObjectStorage),
        ];
        let p = CapabilityProvider::from_service_info(
            "multi".to_string(),
            std::path::PathBuf::from("/tmp/m.sock"),
            caps.clone(),
        );
        assert_eq!(p.capabilities(), caps.as_slice());
        assert!(p.has_capability(&Capability::Compute(ComputeCapability::NativeExecution)));
        assert!(p.has_capability(&Capability::Storage(StorageCapability::ObjectStorage)));
        assert!(!p.has_capability(&Capability::Compute(ComputeCapability::GpuCompute)));
    }

    #[test]
    fn has_capability_requires_exact_variant_match() {
        let p = CapabilityProvider::from_service_info(
            "crypto-only".to_string(),
            std::path::PathBuf::from("/tmp/c.sock"),
            vec![Capability::Crypto(CryptoCapability::Encryption)],
        );
        assert!(!p.has_capability(&Capability::Crypto(CryptoCapability::KeyManagement)));
        assert!(p.has_capability(&Capability::Crypto(CryptoCapability::Encryption)));
    }

    #[test]
    fn clone_shares_same_logical_fields() {
        let a = CapabilityProvider::from_service_info(
            "svc".to_string(),
            std::path::PathBuf::from("/tmp/a.sock"),
            vec![Capability::Crypto(CryptoCapability::DigitalSignatures)],
        );
        let b = a.clone();
        assert_eq!(a.service_name(), b.service_name());
        assert_eq!(a.socket_path(), b.socket_path());
        assert_eq!(a.capabilities(), b.capabilities());
    }

    #[test]
    fn independent_instances_do_not_share_state() {
        let p1 = CapabilityProvider::from_service_info(
            "a".to_string(),
            std::path::PathBuf::from("/tmp/one.sock"),
            vec![Capability::Crypto(CryptoCapability::Encryption)],
        );
        let p2 = CapabilityProvider::from_service_info(
            "b".to_string(),
            std::path::PathBuf::from("/tmp/two.sock"),
            vec![Capability::Storage(StorageCapability::BlockStorage)],
        );
        assert_ne!(p1.service_name(), p2.service_name());
        assert_ne!(p1.socket_path(), p2.socket_path());
        assert!(!p1.has_capability(&Capability::Storage(StorageCapability::BlockStorage)));
        assert!(p2.has_capability(&Capability::Storage(StorageCapability::BlockStorage)));
    }
}
