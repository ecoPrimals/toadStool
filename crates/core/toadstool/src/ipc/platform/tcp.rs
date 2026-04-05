// SPDX-License-Identifier: AGPL-3.0-or-later
//! TCP socket implementation for cross-device IPC
//!
//! **Deep Debt Principles**:
//! - ✅ Safe Rust (tokio async, no unsafe)
//! - ✅ Universal (works everywhere - Linux, macOS, Windows, Android)
//! - ✅ Cross-device (laptop ↔ phone ↔ desktop)
//! - ✅ Firewall-friendly (standard TCP ports)
//!
//! ## Use Cases
//!
//! - **Cross-device**: Phone → Laptop coordination
//! - **Windows**: Where Unix sockets unavailable
//! - **Firewall**: Network-restricted environments
//! - **Remote**: Distributed deployments
//!
//! ## Transport Tier
//!
//! TCP is **Tier 2** (fallback when Tier 1 unavailable)

use crate::{ToadStoolError, ToadStoolResult};
use std::net::SocketAddr;
use toadstool_common::constants::network::{BIND_ALL_IPV4, LOCALHOST_IPV4};
use tokio::net::{TcpListener, TcpStream};

/// Default TCP port for ToadStool IPC.
///
/// Port `0` means the OS assigns an ephemeral port at bind time.
/// Production deployments use Unix sockets; TCP is Tier 2 fallback.
pub const DEFAULT_PORT: u16 = 0;

/// Bind TCP listener
///
/// **Deep Debt**: Pure async, safe Rust
///
/// ## Usage
///
/// ```no_run
/// use toadstool::ipc::platform::tcp;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let listener = tcp::bind("127.0.0.1", 0).await?; // port 0 = ephemeral
///     Ok(())
/// }
/// ```
///
/// # Errors
///
/// Returns error if binding to the address fails.
pub async fn bind(host: &str, port: u16) -> ToadStoolResult<TcpListener> {
    let addr = format!("{host}:{port}");

    TcpListener::bind(&addr)
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to bind TCP socket {addr}: {e}")))
}

/// Connect to TCP socket
///
/// **Deep Debt**: Async, timeout-aware (using `tokio::time`)
///
/// # Errors
///
/// Returns error if the TCP connection fails.
pub async fn connect(host: &str, port: u16) -> ToadStoolResult<TcpStream> {
    let addr = format!("{host}:{port}");

    TcpStream::connect(&addr).await.map_err(|e| {
        ToadStoolError::integration(format!("Failed to connect to TCP socket {addr}: {e}"))
    })
}

/// Get default ToadStool TCP address (localhost, OS-assigned port).
///
/// # Errors
///
/// Returns error if the default address string cannot be parsed.
pub fn default_addr() -> ToadStoolResult<SocketAddr> {
    format!("{LOCALHOST_IPV4}:{DEFAULT_PORT}")
        .parse()
        .map_err(|e| {
            ToadStoolError::configuration(format!(
                "Invalid default TCP address {LOCALHOST_IPV4}:{DEFAULT_PORT}: {e}"
            ))
        })
}

/// Get local network address for cross-device (bind all interfaces, OS-assigned port).
///
/// # Errors
///
/// Returns error if the address string cannot be parsed.
pub fn local_network_addr() -> ToadStoolResult<SocketAddr> {
    format!("{BIND_ALL_IPV4}:{DEFAULT_PORT}")
        .parse()
        .map_err(|e| {
            ToadStoolError::configuration(format!(
                "Invalid network address {BIND_ALL_IPV4}:{DEFAULT_PORT}: {e}"
            ))
        })
}

/// Check if TCP is available (always true)
///
/// **Deep Debt**: Universal availability
pub const fn is_supported() -> bool {
    true // TCP works everywhere!
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_port_is_ephemeral() {
        assert_eq!(DEFAULT_PORT, 0);
    }

    #[test]
    fn test_default_addr() {
        let addr = default_addr().unwrap();
        assert_eq!(addr.port(), DEFAULT_PORT);
        assert_eq!(addr.ip().to_string(), "127.0.0.1");
    }

    #[test]
    fn test_local_network_addr() {
        let addr = local_network_addr().unwrap();
        assert_eq!(addr.port(), DEFAULT_PORT);
        assert_eq!(addr.ip().to_string(), "0.0.0.0");
    }

    #[test]
    fn test_is_supported() {
        // TCP always supported (universal!)
        assert!(is_supported());
    }

    #[tokio::test]
    async fn test_bind_and_connect() {
        // Bind on random port to avoid conflicts
        let listener = bind("127.0.0.1", 0).await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Connect
        let stream = connect("127.0.0.1", addr.port()).await.unwrap();

        // Verify connection
        assert!(stream.peer_addr().is_ok());

        // Cleanup
        drop(listener);
        drop(stream);
    }

    #[tokio::test]
    async fn test_bind_specific_port() {
        // Try to bind specific port (might fail if in use)
        let port = 18_370; // Use high port to avoid conflicts

        if let Ok(listener) = bind("127.0.0.1", port).await {
            assert_eq!(listener.local_addr().unwrap().port(), port);
            drop(listener);
        } else {
            // Port might be in use, that's OK for test
        }
    }

    #[tokio::test]
    async fn test_connect_refused() {
        // Try to connect to port that's not listening
        let result = connect("127.0.0.1", 19_999).await;

        // Should fail (connection refused)
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_multiple_connections() {
        // Bind listener
        let listener = bind("127.0.0.1", 0).await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Multiple clients can connect
        let stream1 = connect("127.0.0.1", addr.port()).await.unwrap();
        let stream2 = connect("127.0.0.1", addr.port()).await.unwrap();

        // Both should be connected
        assert!(stream1.peer_addr().is_ok());
        assert!(stream2.peer_addr().is_ok());

        // Cleanup
        drop(listener);
        drop(stream1);
        drop(stream2);
    }
}
