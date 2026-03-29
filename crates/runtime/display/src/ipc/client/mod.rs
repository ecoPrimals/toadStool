// SPDX-License-Identifier: AGPL-3.0-or-later
//! Display IPC client — JSON-RPC over Unix or TCP with automatic discovery.
//!
//! **ISOMORPHIC IPC**: JSON-RPC client with automatic Unix/TCP discovery.
//! Submodules separate endpoint discovery, connection setup, RPC framing, and
//! display operations.

mod connection;
mod discovery;
mod operations;
mod rpc;
#[cfg(test)]
mod tests;

use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpStream, UnixStream};

/// Default TCP address for display IPC fallback (override with `TOADSTOOL_DISPLAY_IPC_ADDR`).
#[must_use]
pub fn default_display_ipc_tcp_addr() -> String {
    std::env::var("TOADSTOOL_DISPLAY_IPC_ADDR").unwrap_or_else(|_| "127.0.0.1:12345".to_string())
}

/// IPC endpoint (polymorphic - Unix OR TCP)
#[derive(Debug, Clone)]
pub enum IpcEndpoint {
    /// Unix domain socket
    UnixSocket(PathBuf),
    /// TCP socket (fallback mode)
    TcpLocal(SocketAddr),
}

/// Polymorphic async stream trait
///
/// Allows `DisplayClient` to work with both `UnixStream` and `TcpStream` transparently.
pub(super) trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}

// Implement for both stream types
impl AsyncStream for UnixStream {}
impl AsyncStream for TcpStream {}

#[cfg(test)]
impl AsyncStream for tokio::io::DuplexStream {}

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
    stream: Box<dyn AsyncStream>,
    endpoint: IpcEndpoint,
}
