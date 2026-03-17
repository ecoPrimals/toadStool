// SPDX-License-Identifier: AGPL-3.0-only
//! Capability provider abstraction.
//!
//! Provider of a specific capability. Abstracts away which primal provides the capability.
//! Client code doesn't know or care if it's beardog, songbird, etc.

use crate::primal_identity::Capability;
use crate::unix_jsonrpc_client::UnixJsonRpcClient;
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::discovery;
use super::error::{CapabilityError, Result};

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
    /// Create provider from discovery service info (used by discovery module)
    pub(crate) fn from_service_info(
        service_name: String,
        socket_path: PathBuf,
        capabilities: Vec<Capability>,
    ) -> Self {
        Self {
            service_name,
            socket_path,
            capabilities,
            client: Arc::new(RwLock::new(None)),
        }
    }

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
        let providers = discovery::query_providers(capability.clone()).await?;
        providers
            .into_iter()
            .next()
            .ok_or(CapabilityError::NoProviderFound(capability))
    }

    /// Call a method on this capability provider
    ///
    /// Uses JSON-RPC 2.0 over Unix sockets (wateringHole standard)
    ///
    /// # Errors
    ///
    /// Returns error if provider is unreachable or call fails
    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let client = {
            let mut client_lock = self.client.write().await;

            #[allow(clippy::option_if_let_else)]
            // map_or_else would cause borrow conflict with mutation
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
