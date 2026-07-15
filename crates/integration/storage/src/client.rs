// SPDX-License-Identifier: AGPL-3.0-or-later
//! Storage client implementation using capability-based discovery
//!
//! Self-knowledge only — discovers storage via capabilities.

//!
//! ## Philosophy
//!
//! - ✅ **Self-Knowledge**: Knows only itself, discovers storage at runtime
//! - ✅ **Capability-Based**: Discovers ANY storage service with required capability
//! - ✅ **Vendor-Agnostic**: Works with Storage, S3, `MinIO`, GCS, or any storage
//! - ✅ **Pure Rust**: Unix socket IPC for primal communication
//!
//! ## Usage
//!
//! ```ignore
//! use toadstool_integration_storage::StorageClient;
//!
//! // Discover ANY storage service with ArtifactStorage capability
//! let client = StorageClient::discover().await?;
//!
//! // Store artifact (vendor-agnostic!)
//! client.store_artifact("model.bin", data).await?;
//! ```

use tracing::debug;
#[cfg(unix)]
use tracing::info;

#[cfg(unix)]
use toadstool_common::interned_strings::capabilities;
use toadstool_common::primal_identity::{Capability, StorageCapability};
#[cfg(unix)]
use toadstool_common::service_discovery::{DiscoveryMethod, ServiceDiscovery};

use crate::config::StorageConfig;
use crate::types::{StorageError, StorageServiceResult};

/// Storage client for artifact and pipeline operations
///
/// Capability-based, vendor-agnostic storage client
///
/// ## Design Principles
///
/// - **Self-Knowledge**: Knows only storage capabilities, not specific services
/// - **Runtime Discovery**: Finds storage services via capability system
/// - **Vendor-Agnostic**: Works with ANY storage implementing `ArtifactStorage` capability
/// - **Pure Rust IPC**: Unix socket communication (no HTTP between primals!)
///
/// ## Supported Storage Services
///
/// - Storage (ecoPrimals storage)
/// - `MinIO` (S3-compatible)
/// - AWS S3 (via adapter)
/// - Google Cloud Storage (via adapter)
/// - Any service advertising `storage:artifact` capability
#[cfg(unix)]
#[derive(Debug, Clone)]
pub struct StorageClient {
    /// RPC client for JSON-RPC over unix socket (crate-visible for impl in artifacts/pipelines)
    pub(crate) rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    /// Client configuration (crate-visible for impl in utils)
    pub(crate) config: StorageConfig,
    /// Discovered service name (for diagnostics)
    _service_name: String,
}

/// Storage client stub for non-Unix targets (Windows, WASM, etc.)
///
/// Unix socket discovery is unavailable on these platforms.
#[cfg(not(unix))]
#[derive(Debug, Clone)]
pub struct StorageClient {
    /// Client configuration (crate-visible for impl in utils)
    pub(crate) config: StorageConfig,
    /// Discovered service name (for diagnostics)
    _service_name: String,
}

impl StorageClient {
    /// Discover storage service via capability-based discovery
    ///
    /// Discovers any storage service with `ArtifactStorage` capability
    ///
    /// ## Vendor-Agnostic Discovery
    ///
    /// Finds storage services advertising `storage:artifact` capability:
    /// - Storage (ecoPrimals native storage)
    /// - `MinIO` (S3-compatible object storage)
    /// - AWS S3 (via capability adapter)
    /// - Google Cloud Storage (via capability adapter)
    /// - Custom storage implementations
    ///
    /// ## Self-Knowledge Principle
    ///
    /// This client knows:
    /// - ✅ What capabilities it needs (storage:artifact)
    /// - ✅ How to communicate via unix sockets
    ///
    /// This client does NOT know:
    /// - ❌ Specific service names (Storage, `MinIO`, etc.)
    /// - ❌ Hardcoded endpoints or ports
    /// - ❌ Implementation details
    ///
    /// # Errors
    /// Returns an error if no storage service is found or connection fails
    pub async fn discover() -> StorageServiceResult<Self> {
        Self::discover_with_capability(Capability::Storage(StorageCapability::ArtifactStorage))
            .await
    }

    /// Discover storage service by specific capability
    ///
    /// Runtime discovery — no hardcoded service names.
    ///
    /// # Errors
    /// Returns an error if no service is found or connection fails
    pub async fn discover_with_capability(capability: Capability) -> StorageServiceResult<Self> {
        #[cfg(not(unix))]
        {
            let _ = capability;
            return Err(StorageError::Connection(
                "Unix socket storage discovery is unavailable on this platform".into(),
            ));
        }

        #[cfg(unix)]
        {
            let discovery = ServiceDiscovery::new(DiscoveryMethod::Auto)
                .await
                .map_err(|e| StorageError::Connection(format!("Discovery failed: {e}")))?;

            let service = discovery
                .find_service_by_capability(capability)
                .await
                .map_err(|e| StorageError::Connection(format!("No storage service found: {e}")))?;

            let service_name = service.name.clone();

            let socket_path =
                toadstool_common::primal_sockets::get_socket_path_for_capability(&service_name);

            info!(
                "✅ Discovered storage service: {} (capability-based discovery)",
                service_name
            );

            let rpc_client =
                toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

            let client = Self {
                rpc_client,
                config: StorageConfig::default(),
                _service_name: service_name,
            };

            client.health_check().await?;

            Ok(client)
        }
    }

    /// Connect to storage server by service name
    ///
    /// **Note**: Consider using `discover()` for capability-based discovery.
    /// This method requires knowing the service name (e.g., "storage", "minio").
    ///
    /// # Arguments
    /// * `service_name` - Name of the storage service to connect to
    ///
    /// # Errors
    /// Returns an error if the client configuration is invalid or connection fails
    pub async fn connect(service_name: &str) -> StorageServiceResult<Self> {
        #[cfg(not(unix))]
        {
            let _ = service_name;
            return Err(StorageError::Connection(
                "Unix socket storage connections are unavailable on this platform".into(),
            ));
        }

        #[cfg(unix)]
        {
            let socket =
                toadstool_common::primal_sockets::get_socket_path_for_capability(service_name);
            let config = StorageConfig {
                endpoint: format!("unix://{}", socket.display()),
                ..Default::default()
            };
            Self::with_config(config, Some(service_name.to_string())).await
        }
    }

    /// Create client with custom configuration
    ///
    /// Accepts optional service name from discovery — no hardcoded endpoints.
    ///
    /// # Arguments
    /// * `config` - Storage configuration
    /// * `service_name` - Optional discovered service name (defaults to "storage" for backward compat)
    ///
    /// # Errors
    /// Returns an error if the client configuration is invalid
    pub async fn with_config(
        config: StorageConfig,
        service_name: Option<String>,
    ) -> StorageServiceResult<Self> {
        #[cfg(not(unix))]
        {
            let _ = (config, service_name);
            return Err(StorageError::Connection(
                "Unix socket storage connections are unavailable on this platform".into(),
            ));
        }

        #[cfg(unix)]
        {
            let service_name = service_name.unwrap_or_else(|| capabilities::STORAGE.to_string());

            let socket_path =
                toadstool_common::primal_sockets::get_socket_path_for_capability(&service_name);
            let rpc_client =
                toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);

            let client = Self {
                rpc_client,
                config,
                _service_name: service_name,
            };

            client.health_check().await?;

            Ok(client)
        }
    }

    /// Check `Storage` server health via unix socket
    ///
    /// **PURE RUST**: JSON-RPC over unix socket (modern async pattern!)
    ///
    /// # Errors
    /// Returns an error if the health check request fails or server is unhealthy
    pub async fn health_check(&self) -> StorageServiceResult<()> {
        self.rpc_call("storage.health", serde_json::json!({}))
            .await?;
        debug!("Storage service health check passed via unix socket");
        Ok(())
    }

    /// Create client for testing without health check (skips RPC connectivity)
    ///
    /// Use for unit tests that exercise local logic (store_artifact, retrieve_artifact,
    /// checksum, content-type detection) without requiring a running Storage server.
    #[doc(hidden)]
    #[must_use]
    pub fn new_for_testing(config: StorageConfig, service_name: String) -> Self {
        #[cfg(unix)]
        {
            let socket_path =
                toadstool_common::primal_sockets::get_socket_path_for_capability(&service_name);
            let rpc_client =
                toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
            Self {
                rpc_client,
                config,
                _service_name: service_name,
            }
        }

        #[cfg(not(unix))]
        {
            Self {
                config,
                _service_name: service_name,
            }
        }
    }

    /// JSON-RPC call helper used by artifact and pipeline operations.
    pub(crate) async fn rpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, StorageError> {
        #[cfg(unix)]
        {
            self.rpc_client
                .call(method, params)
                .await
                .map_err(|e| StorageError::Network(e.to_string()))
        }

        #[cfg(not(unix))]
        {
            let _ = (method, params);
            Err(StorageError::Connection(
                "Unix socket storage RPC is unavailable on this platform".into(),
            ))
        }
    }

    /// Typed JSON-RPC call helper used by artifact and pipeline operations.
    pub(crate) async fn rpc_call_typed<T: for<'de> serde::Deserialize<'de>>(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<T, StorageError> {
        #[cfg(unix)]
        {
            self.rpc_client
                .call_typed(method, params)
                .await
                .map_err(|e| StorageError::Network(e.to_string()))
        }

        #[cfg(not(unix))]
        {
            let _ = (method, params);
            Err(StorageError::Connection(
                "Unix socket storage RPC is unavailable on this platform".into(),
            ))
        }
    }
}

#[cfg(test)]
#[path = "client_tests.rs"]
mod client_tests;
