// SPDX-License-Identifier: AGPL-3.0-or-later
//! Socket client construction (capability discovery and deprecated ctor).

use crate::{ToadStoolError, ToadStoolResult};

/// Production storage backend using Unix Socket API (Pure Rust!)
///
/// **TRUE PRIMAL**: Uses unix sockets for local IPC (no HTTP, no TLS, no ring!)
pub struct SocketStorageBackend {
    pub(crate) rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    pub(crate) _storage_tier: String,
    pub(crate) replication_enabled: bool,
    pub(crate) replication_factor: u32,
}

impl SocketStorageBackend {
    /// Create storage backend with capability-based discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers storage service by capability, not name.
    /// Works with ANY service providing storage.object capability.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    ///
    /// # Errors
    ///
    /// Returns an error if capability discovery fails or no storage service can be found.
    pub async fn new_async(
        storage_tier: impl Into<String>,
        replication_enabled: bool,
        replication_factor: u32,
    ) -> ToadStoolResult<Self> {
        // CAPABILITY-BASED: Discover ANY storage service (not hardcoded "nestgate")
        let socket_path = toadstool_common::primal_sockets::discover_storage_socket()
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "No storage service discovered: {e}. Ensure a storage provider is running."
                ))
            })?;

        Ok(Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            _storage_tier: storage_tier.into(),
            replication_enabled,
            replication_factor,
        })
    }

    /// Create a new storage backend with unix socket transport
    ///
    /// **DEPRECATED**: Use `new_async()` for capability-based discovery.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    #[must_use]
    #[deprecated(
        since = "0.3.0",
        note = "Use new_async() for capability-based discovery"
    )]
    pub fn new(
        _endpoint: impl Into<String>,
        storage_tier: impl Into<String>,
        replication_enabled: bool,
        replication_factor: u32,
    ) -> Self {
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability("storage");
        Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            _storage_tier: storage_tier.into(),
            replication_enabled,
            replication_factor,
        }
    }
}
