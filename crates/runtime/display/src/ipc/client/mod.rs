// SPDX-License-Identifier: AGPL-3.0-or-later
//! Display IPC client — JSON-RPC over Unix or TCP with automatic discovery.
//!
//! **ISOMORPHIC IPC**: JSON-RPC client with automatic Unix/TCP discovery.
//! Submodules separate endpoint discovery, connection setup, RPC framing, and
//! display operations.

mod async_stream_dispatch;
mod connection;
mod discovery;
mod operations;
mod rpc;
mod tcp_endpoint;
#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::path::PathBuf;

pub(super) use async_stream_dispatch::AsyncStreamDispatch;

pub use tcp_endpoint::DisplayIpcTcpSettings;

/// Loopback address used when constructing TCP fallback endpoints.
pub(super) const LOOPBACK: &str = toadstool_common::constants::network::LOCALHOST_IPV4;

/// Default TCP address for display IPC fallback.
///
/// Resolution order:
/// 1. `TOADSTOOL_DISPLAY_IPC_ADDR` (full `host:port`)
/// 2. TCP discovery files under runtime/temp paths ([`PlatformPaths`](toadstool_common::platform_paths::PlatformPaths))
/// 3. `TOADSTOOL_DISPLAY_IPC_PORT` or the cold-start fallback from `toadstool_config::defaults::ports`
#[must_use]
pub fn default_display_ipc_tcp_addr() -> String {
    tcp_endpoint::default_display_ipc_tcp_addr()
}

/// IPC endpoint (polymorphic - Unix OR TCP)
#[derive(Debug, Clone)]
pub enum IpcEndpoint {
    /// Unix domain socket
    UnixSocket(PathBuf),
    /// TCP socket (fallback mode)
    TcpLocal(SocketAddr),
}

/// Display client
///
/// **ISOMORPHIC**: Connects via Unix sockets OR TCP automatically.
///
/// ## Example (Automatic Discovery)
///
/// ```rust,no_run
/// use toadstool_display::ipc::DisplayClient;
/// use toadstool_display::window::CreateWindowRequest;
///
/// # async fn example() -> toadstool_display::Result<()> {
/// // Automatic discovery!
/// let mut client = DisplayClient::discover().await?;
///
/// let window_id = client.create_window(CreateWindowRequest::default()).await?;
/// println!("Created window: {}", window_id);
/// # Ok(())
/// # }
/// ```
pub struct DisplayClient {
    pub(super) stream: AsyncStreamDispatch,
    endpoint: IpcEndpoint,
}
