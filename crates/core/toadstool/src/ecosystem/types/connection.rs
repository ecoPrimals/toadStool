// SPDX-License-Identifier: AGPL-3.0-or-later
//! Service status, communication channels, and multi-protocol clients.

#[cfg(feature = "networking")]
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// Status of a service instance
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ServiceStatus {
    /// Discovered but not connected
    Discovered,
    /// Connecting to service
    Connecting,
    /// Connected and ready
    Connected,
    /// Connection failed
    Failed(String),
    /// Disconnected
    Disconnected,
    /// Service is being removed
    Removing,
}

impl ServiceStatus {
    /// Check if service is usable
    pub const fn is_usable(&self) -> bool {
        matches!(self, Self::Connected)
    }

    /// Check if service is in error state
    pub const fn is_error(&self) -> bool {
        matches!(self, Self::Failed(_))
    }

    /// Get error message if in failed state
    pub fn error_message(&self) -> Option<&str> {
        match self {
            Self::Failed(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Communication channel with a service
#[derive(Debug, Clone)]
pub struct ServiceChannel {
    /// Service identifier
    pub service_id: String,
    /// Service name (for logging/debugging only)
    pub service_name: String,
    /// Service endpoint
    pub endpoint: String,
    /// Client type
    pub client: ServiceClient,
    /// Last successful heartbeat
    pub last_heartbeat: SystemTime,
    /// Current status
    pub status: ServiceStatus,
}

/// Client for communicating with services
///
/// This enum supports multiple protocols following the wateringHole standard:
/// - JSON-RPC 2.0 (PRIMARY): Universal language-agnostic access
/// - tarpc (OPTIONAL): High-performance binary RPC for internal paths
/// - HTTP (DEPRECATED): Use Songbird for HTTP/TLS
#[derive(Debug, Clone)]
pub enum ServiceClient {
    /// tarpc client (OPTIONAL - for performance-critical internal paths)
    #[cfg(feature = "networking")]
    Tarpc(Arc<tokio::sync::Mutex<Option<TarpcClientWrapper>>>),

    /// JSON-RPC 2.0 over unix sockets (PRIMARY - wateringHole standard!)
    #[cfg(feature = "networking")]
    UnixSocket(toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient),

    /// No-op client when networking feature is disabled.
    /// Intentional degraded-mode fallback for builds without networking.
    #[cfg(not(feature = "networking"))]
    Disabled,
}

/// Wrapper for tarpc client with JSON-RPC fallback.
///
/// Per wateringHole `UNIVERSAL_IPC_STANDARD_V3.md`: tarpc is optional for
/// high-performance internal paths. Until a binary tarpc transport is wired,
/// this wrapper gracefully degrades to JSON-RPC over the same Unix socket.
#[cfg(feature = "networking")]
#[derive(Debug, Clone)]
pub struct TarpcClientWrapper {
    fallback: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
}

#[cfg(feature = "networking")]
impl TarpcClientWrapper {
    pub const fn with_fallback(
        client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    ) -> Self {
        Self { fallback: client }
    }

    pub const fn fallback_client(
        &self,
    ) -> &toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient {
        &self.fallback
    }
}
