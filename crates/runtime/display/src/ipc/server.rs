// SPDX-License-Identifier: AGPL-3.0-or-later
//! Display server implementation
//!
//! **ISOMORPHIC IPC**: JSON-RPC server with automatic Unix→TCP fallback.
//!
//! ## Evolution Status
//!
//! ✅ **Phase 1 COMPLETE**: Isomorphic IPC (Try→Detect→Adapt→Succeed)
//!
//! ## Architecture
//!
//! ```text
//! 1. TRY Unix socket (optimal)
//!     ↓
//! 2. DETECT platform constraints (SELinux, unsupported)
//!     ↓
//! 3. ADAPT to TCP fallback (127.0.0.1:ephemeral)
//!     ↓
//! 4. SUCCEED or fail with real error
//! ```
//!
//! ## Platform Support
//!
//! - **Linux**: Unix sockets (optimal)
//! - **Android**: TCP fallback (automatic)
//! - **Any platform**: Zero configuration!
//!
//! ## Deep Debt Compliance
//!
//! - ✅ Platform-agnostic (runtime adaptation)
//! - ✅ Zero configuration (automatic fallback)
//! - ✅ Pure Rust (no FFI)
//! - ✅ Zero unsafe
//! - ✅ Modern async (tokio)

use super::dispatch;
use super::platform;
use crate::window::WindowManager;
use crate::{DisplayError, Result};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use toadstool_common::constants::network::LOCALHOST_IPV4;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};
use tokio::sync::RwLock;

/// IPC transport type
#[derive(Debug, Clone)]
pub enum IpcTransport {
    /// Unix domain socket (optimal)
    UnixSocket,
    /// TCP fallback for platforms without Unix sockets
    TcpFallback(SocketAddr),
}

/// Display server
///
/// **ISOMORPHIC**: Serves display operations over JSON-RPC with automatic Unix→TCP fallback.
///
/// ## Example
///
/// ```rust,ignore
/// use toadstool_display::{DisplayServer, WindowManager};
///
/// # async fn example() -> Result<()> {
/// let manager = WindowManager::new().await?;
/// let server = Arc::new(DisplayServer::new(manager));
///
/// // Automatic adaptation!
/// server.start().await?;  // Tries Unix, falls back to TCP if needed
/// # Ok(())
/// # }
/// ```
pub struct DisplayServer {
    manager: Arc<RwLock<WindowManager>>,
    socket_path: PathBuf,
    transport: Arc<RwLock<Option<IpcTransport>>>,
}

impl DisplayServer {
    /// Create a new display server
    #[must_use]
    pub fn new(manager: WindowManager) -> Self {
        // Default socket path (capability-based discovery!)
        let socket_path = platform::discover_socket_path();

        Self {
            manager: Arc::new(RwLock::new(manager)),
            socket_path,
            transport: Arc::new(RwLock::new(None)),
        }
    }

    /// Start server (ISOMORPHIC - Try→Detect→Adapt→Succeed)
    ///
    /// **Zero Configuration**: Automatically adapts to platform constraints!
    ///
    /// ## Behavior
    ///
    /// 1. **TRY** Unix socket (optimal)
    /// 2. **DETECT** platform constraints (`SELinux`, unsupported)
    /// 3. **ADAPT** to TCP fallback (127.0.0.1:ephemeral)
    /// 4. **SUCCEED** or fail with real error
    ///
    /// # Errors
    ///
    /// Returns an error if both Unix socket and TCP binding fail.
    pub async fn start(self: Arc<Self>) -> Result<()> {
        tracing::info!("🔌 Starting IPC server (isomorphic mode)...");
        tracing::info!("   Trying Unix socket IPC (optimal)...");

        // 1. TRY Unix socket first
        match self.clone().try_unix_server().await {
            Ok(()) => Ok(()),

            // 2. DETECT platform constraints
            Err(e) if platform::is_platform_constraint(&e) => {
                tracing::warn!("⚠️  Unix sockets unavailable: {}", e);
                tracing::warn!("   Detected platform constraint, adapting...");

                // 3. ADAPT to TCP fallback
                self.start_tcp_fallback().await
            }

            // 4. Real error (not a platform constraint)
            Err(e) => {
                tracing::error!("❌ Real error (not platform constraint): {}", e);
                Err(e)
            }
        }
    }

    /// Try to start Unix socket server
    async fn try_unix_server(self: Arc<Self>) -> Result<()> {
        let path = self.socket_path.clone();

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| DisplayError::IpcError(format!("Failed to create socket dir: {e}")))?;
        }

        // Remove existing socket
        let _ = tokio::fs::remove_file(&path).await;

        // Bind listener (this is where platform constraints appear!)
        let listener = UnixListener::bind(&path)
            .map_err(|e| DisplayError::IpcError(format!("Failed to bind Unix socket: {e}")))?;

        tracing::info!(
            "✅ Unix socket JSON-RPC server listening: {}",
            path.display()
        );

        // Update transport
        *self.transport.write().await = Some(IpcTransport::UnixSocket);

        // Accept loop
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let handler = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handler.handle_unix_connection(stream).await {
                            tracing::error!("Unix connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Unix accept error: {}", e);
                }
            }
        }
    }

    /// Start TCP fallback server (isomorphic mode)
    async fn start_tcp_fallback(self: Arc<Self>) -> Result<()> {
        tracing::info!("🌐 Starting TCP IPC fallback (isomorphic mode)");
        tracing::info!("   Protocol: JSON-RPC 2.0 (same as Unix socket)");

        // Bind to localhost only (security: same as Unix socket)
        let bind_addr = format!("{LOCALHOST_IPV4}:0");
        let listener = TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| DisplayError::IpcError(format!("Failed to bind TCP socket: {e}")))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| DisplayError::IpcError(format!("Failed to get local address: {e}")))?;

        tracing::info!("✅ TCP IPC listening on {}", local_addr);

        // Write discovery file for clients
        platform::write_tcp_discovery_file(&local_addr);

        // Update transport
        *self.transport.write().await = Some(IpcTransport::TcpFallback(local_addr));

        tracing::info!("   Status: READY ✅ (isomorphic TCP fallback active)");

        // Accept loop (same as Unix)
        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let handler = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handler.handle_tcp_connection(stream).await {
                            tracing::error!("TCP connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("TCP accept error: {}", e);
                }
            }
        }
    }

    /// Handle Unix socket connection
    async fn handle_unix_connection(self: Arc<Self>, stream: UnixStream) -> Result<()> {
        tracing::debug!("New Unix client connected");

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();

            // Read request line
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // Connection closed
                    tracing::debug!("Unix client disconnected");
                    break;
                }
                Ok(_) => {
                    // Process request
                    let response = dispatch::handle_request(&line, &self.manager).await;

                    // Send response
                    let response_json = serde_json::to_string(&response)
                        .map_err(|e| DisplayError::IpcError(format!("Serialization error: {e}")))?;

                    writer
                        .write_all(response_json.as_bytes())
                        .await
                        .map_err(|e| DisplayError::IpcError(format!("Write error: {e}")))?;
                    writer
                        .write_all(b"\n")
                        .await
                        .map_err(|e| DisplayError::IpcError(format!("Write error: {e}")))?;
                }
                Err(e) => {
                    return Err(DisplayError::IpcError(format!("Read error: {e}")));
                }
            }
        }

        Ok(())
    }

    /// Handle TCP connection (same protocol as Unix!)
    async fn handle_tcp_connection(self: Arc<Self>, stream: TcpStream) -> Result<()> {
        tracing::debug!("New TCP client connected");

        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();

            // Read request line
            match reader.read_line(&mut line).await {
                Ok(0) => {
                    // Connection closed
                    tracing::debug!("TCP client disconnected");
                    break;
                }
                Ok(_) => {
                    // Process request (SAME as Unix!)
                    let response = dispatch::handle_request(&line, &self.manager).await;

                    // Send response
                    let response_json = serde_json::to_string(&response)
                        .map_err(|e| DisplayError::IpcError(format!("Serialization error: {e}")))?;

                    writer
                        .write_all(response_json.as_bytes())
                        .await
                        .map_err(|e| DisplayError::IpcError(format!("Write error: {e}")))?;
                    writer
                        .write_all(b"\n")
                        .await
                        .map_err(|e| DisplayError::IpcError(format!("Write error: {e}")))?;
                }
                Err(e) => {
                    return Err(DisplayError::IpcError(format!("Read error: {e}")));
                }
            }
        }

        Ok(())
    }

    /// Get socket path
    #[must_use]
    pub const fn socket_path(&self) -> &PathBuf {
        &self.socket_path
    }

    /// Get current transport
    pub async fn transport(&self) -> Option<IpcTransport> {
        self.transport.read().await.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
    use std::net::{Ipv4Addr, SocketAddr};

    #[tokio::test]
    async fn test_jsonrpc_parsing() {
        let request_str = r#"{"jsonrpc":"2.0","method":"display.get_capabilities","id":1}"#;
        let request: JsonRpcRequest = serde_json::from_str(request_str).unwrap();
        assert_eq!(request.method, "display.get_capabilities");
    }

    /// Create an owned manager for `DisplayServer::new` tests.
    async fn test_manager_owned() -> Option<WindowManager> {
        WindowManager::new().await.ok()
    }

    #[test]
    fn test_ipc_transport_debug() {
        let t = IpcTransport::UnixSocket;
        let s = format!("{t:?}");
        assert!(s.contains("Unix"));
    }

    #[test]
    fn test_ipc_transport_tcp_fallback() {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let t = IpcTransport::TcpFallback(addr);
        let s = format!("{t:?}");
        assert!(s.contains("TcpFallback") || s.contains("127"));
    }

    #[tokio::test]
    async fn test_display_server_new_socket_path() {
        let Some(manager) = test_manager_owned().await else {
            return;
        };
        let server = DisplayServer::new(manager);
        let path = server.socket_path();
        assert!(path.to_string_lossy().contains("toadstool"));
        assert!(path.to_string_lossy().ends_with("display.sock"));
    }

    #[test]
    fn test_jsonrpc_error_constructors() {
        let parse = JsonRpcError::parse_error();
        assert_eq!(parse.code, -32700);
        assert!(parse.message.to_lowercase().contains("parse"));

        let invalid = JsonRpcError::invalid_request();
        assert_eq!(invalid.code, -32600);

        let not_found = JsonRpcError::method_not_found("foo.bar");
        assert_eq!(not_found.code, -32601);
        assert!(not_found.message.contains("foo.bar"));

        let invalid_params = JsonRpcError::invalid_params("bad");
        assert_eq!(invalid_params.code, -32602);
        assert!(invalid_params.message.contains("bad"));

        let internal = JsonRpcError::internal_error("oops");
        assert_eq!(internal.code, -32603);
        assert!(internal.message.contains("oops"));
    }

    #[test]
    fn test_jsonrpc_response_success_roundtrip() {
        let resp = JsonRpcResponse::success(
            serde_json::json!(1),
            serde_json::json!({"window_id": "test-123"}),
        );
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.result.is_some());
        assert_eq!(parsed.result.unwrap()["window_id"], "test-123");
    }

    #[tokio::test]
    async fn test_transport_initially_none() {
        let Some(manager) = test_manager_owned().await else {
            return;
        };
        let server = DisplayServer::new(manager);
        let transport = server.transport().await;
        assert!(transport.is_none());
    }

    #[test]
    fn test_ipc_transport_clone() {
        let t = IpcTransport::UnixSocket;
        let t2 = t.clone();
        assert!(matches!(
            (t, t2),
            (IpcTransport::UnixSocket, IpcTransport::UnixSocket)
        ));

        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let t3 = IpcTransport::TcpFallback(addr);
        let t4 = t3.clone();
        assert!(matches!(
            (t3, t4),
            (IpcTransport::TcpFallback(_), IpcTransport::TcpFallback(_))
        ));
    }

    #[test]
    fn test_ipc_transport_tcp_fallback_addr() {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 12_345));
        let t = IpcTransport::TcpFallback(addr);
        let s = format!("{t:?}");
        assert!(s.contains("12345") || s.contains("127"));
    }

    #[test]
    fn test_jsonrpc_request_parse_valid() {
        let req = r#"{"jsonrpc":"2.0","method":"display.get_capabilities","id":1}"#;
        let parsed: JsonRpcRequest = serde_json::from_str(req).unwrap();
        assert_eq!(parsed.method, "display.get_capabilities");
        assert_eq!(parsed.id, Some(serde_json::json!(1)));
    }

    #[test]
    fn test_jsonrpc_response_error_roundtrip() {
        let err = JsonRpcError::internal_error("test");
        let resp = JsonRpcResponse::error(serde_json::json!(1), err);
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: JsonRpcResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.error.is_some());
        assert!(parsed.result.is_none());
    }

    #[test]
    fn test_jsonrpc_error_method_not_found() {
        let err = JsonRpcError::method_not_found("display.unknown");
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("display.unknown"));
    }

    #[test]
    fn test_jsonrpc_error_internal_error() {
        let err = JsonRpcError::internal_error("oops");
        assert_eq!(err.code, -32603);
        assert!(err.message.contains("oops"));
    }

    #[tokio::test]
    async fn test_display_server_socket_path_contains_toadstool() {
        let Some(manager) = test_manager_owned().await else {
            return;
        };
        let server = DisplayServer::new(manager);
        let path_str = server.socket_path().to_string_lossy();
        assert!(path_str.contains("toadstool") || path_str.contains("display"));
    }

    #[tokio::test]
    async fn test_handle_request_with_empty_line() {
        use crate::ipc::dispatch;
        use tokio::sync::RwLock;

        let Some(manager) = test_manager_owned().await else {
            return;
        };
        let manager = Arc::new(RwLock::new(manager));
        let response = dispatch::handle_request("", &manager).await;
        assert!(response.error.is_some() || response.result.is_none());
    }

    #[tokio::test]
    async fn test_handle_request_invalid_json() {
        use crate::ipc::dispatch;
        use tokio::sync::RwLock;

        let Some(manager) = test_manager_owned().await else {
            return;
        };
        let manager = Arc::new(RwLock::new(manager));
        let response = dispatch::handle_request("not valid json", &manager).await;
        assert!(response.error.is_some());
    }
}
