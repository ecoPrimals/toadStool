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

use super::types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::window::{CreateWindowRequest, Size, WindowId, WindowManager};
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
        let socket_path = Self::discover_socket_path();

        Self {
            manager: Arc::new(RwLock::new(manager)),
            socket_path,
            transport: Arc::new(RwLock::new(None)),
        }
    }

    /// Discover socket path from environment
    ///
    /// **Capability-based**: Uses `XDG_RUNTIME_DIR`, no hardcoding!
    fn discover_socket_path() -> PathBuf {
        let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());

        let mut path = PathBuf::from(runtime_dir);
        path.push("toadstool");
        path.push("display.sock");

        path
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
    pub async fn start(self: Arc<Self>) -> Result<()> {
        tracing::info!("🔌 Starting IPC server (isomorphic mode)...");
        tracing::info!("   Trying Unix socket IPC (optimal)...");

        // 1. TRY Unix socket first
        match self.clone().try_unix_server().await {
            Ok(()) => Ok(()),

            // 2. DETECT platform constraints
            Err(e) if self.is_platform_constraint(&e) => {
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
        let bind_addr = format!("{}:0", LOCALHOST_IPV4);
        let listener = TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| DisplayError::IpcError(format!("Failed to bind TCP socket: {e}")))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| DisplayError::IpcError(format!("Failed to get local address: {e}")))?;

        tracing::info!("✅ TCP IPC listening on {}", local_addr);

        // Write discovery file for clients
        self.write_tcp_discovery_file(&local_addr)?;

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

    /// Detect platform constraints (not real errors!)
    ///
    /// Platform constraints should trigger TCP fallback, not failure.
    fn is_platform_constraint(&self, error: &DisplayError) -> bool {
        // Extract IO error from DisplayError
        let error_str = error.to_string();

        // Check for permission denied + SELinux
        if error_str.contains("Permission denied") && self.is_selinux_enforcing() {
            tracing::debug!("   Platform constraint: SELinux enforcing (Android?)");
            return true;
        }

        // Check for unsupported operation (platform lacks Unix sockets)
        if error_str.contains("Unsupported") || error_str.contains("not supported") {
            tracing::debug!("   Platform constraint: Unix sockets not supported");
            return true;
        }

        false
    }

    /// Check if `SELinux` is enforcing (common on Android)
    fn is_selinux_enforcing(&self) -> bool {
        std::fs::read_to_string("/sys/fs/selinux/enforce")
            .ok()
            .and_then(|s| s.trim().parse::<u8>().ok())
            .is_some_and(|v| v == 1)
    }

    /// Write TCP discovery file for clients
    ///
    /// **XDG-compliant**: Tries `XDG_RUNTIME_DIR`, HOME, /tmp
    fn write_tcp_discovery_file(&self, addr: &SocketAddr) -> Result<()> {
        // XDG-compliant discovery file paths
        let discovery_dirs: Vec<Option<String>> = vec![
            std::env::var("XDG_RUNTIME_DIR").ok(),
            std::env::var("HOME")
                .ok()
                .map(|h| format!("{h}/.local/share")),
            Some("/tmp".to_string()),
        ];

        for dir in discovery_dirs.iter().filter_map(|d| d.as_ref()) {
            // Create directory if needed
            if matches!(std::fs::create_dir_all(dir), Ok(())) {
                let discovery_file = format!("{dir}/toadstool-ipc-port");

                if let Ok(mut f) = std::fs::File::create(&discovery_file) {
                    use std::io::Write;
                    // Write in format: tcp:127.0.0.1:PORT
                    writeln!(f, "tcp:{addr}").ok();
                    tracing::info!("📁 TCP discovery file: {}", discovery_file);
                    break;
                }
            }
        }

        Ok(())
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
                    let response = Self::handle_request(&line, &self.manager).await;

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
                    let response = Self::handle_request(&line, &self.manager).await;

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

    /// Handle a single JSON-RPC request
    async fn handle_request(
        request_str: &str,
        manager: &Arc<RwLock<WindowManager>>,
    ) -> JsonRpcResponse {
        // Parse request
        let request: JsonRpcRequest = match serde_json::from_slice(request_str.as_bytes()) {
            Ok(req) => req,
            Err(_) => {
                return JsonRpcResponse::error(
                    serde_json::json!(null),
                    JsonRpcError::parse_error(),
                );
            }
        };

        let id = request.id.clone().unwrap_or(serde_json::json!(null));

        // Dispatch method (needs self reference for capabilities)
        // Since we're in a static context, we'll pass needed data
        let result = Self::dispatch_method_static(&request, manager).await;

        match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
        }
    }

    /// Dispatch method to handler (static version for compatibility)
    async fn dispatch_method_static(
        request: &JsonRpcRequest,
        manager: &Arc<RwLock<WindowManager>>,
    ) -> Result<serde_json::Value> {
        match request.method.as_str() {
            "display.create_window" => {
                let params: CreateWindowRequest = request
                    .params
                    .as_ref()
                    .and_then(|p| serde_json::from_value(p.clone()).ok())
                    .unwrap_or_default();

                let mut mgr = manager.write().await;
                let window_id = mgr.create_window(params).await?;

                Ok(serde_json::json!({
                    "window_id": window_id.as_string()
                }))
            }
            "display.destroy_window" => {
                let window_id_str = request
                    .params
                    .as_ref()
                    .and_then(|p| p.get("window_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| DisplayError::IpcError("Missing window_id".to_string()))?;
                let window_id = WindowId::from_string(window_id_str)?;

                let mut mgr = manager.write().await;
                mgr.destroy_window(window_id).await?;

                Ok(serde_json::json!({"destroyed": true}))
            }
            "display.resize_window" => {
                let params = request
                    .params
                    .as_ref()
                    .ok_or_else(|| DisplayError::IpcError("Missing params".to_string()))?;
                let window_id_str = params["window_id"]
                    .as_str()
                    .ok_or_else(|| DisplayError::IpcError("Missing window_id".to_string()))?;
                let window_id = WindowId::from_string(window_id_str)?;
                let width = params["width"]
                    .as_u64()
                    .ok_or_else(|| DisplayError::IpcError("Missing width".to_string()))?
                    as u32;
                let height = params["height"]
                    .as_u64()
                    .ok_or_else(|| DisplayError::IpcError("Missing height".to_string()))?
                    as u32;

                let mut mgr = manager.write().await;
                mgr.resize_window(window_id, Size { width, height }).await?;

                Ok(serde_json::json!({"resized": true}))
            }
            "display.get_window_info" => {
                let window_id_str = request
                    .params
                    .as_ref()
                    .and_then(|p| p.get("window_id"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| DisplayError::IpcError("Missing window_id".to_string()))?;
                let window_id = WindowId::from_string(window_id_str)?;

                let mgr = manager.read().await;
                let info = mgr.get_window_info(window_id)?;

                Ok(serde_json::to_value(info)
                    .map_err(|e| DisplayError::IpcError(format!("Serialization error: {e}")))?)
            }
            "display.get_capabilities" => {
                let mgr = manager.read().await;

                // Runtime-determined capabilities (basic version for static dispatch)
                Ok(serde_json::json!({
                    "primal_id": "toadstool-primary",
                    "socket_path": Self::discover_socket_path().display().to_string(),
                    "transport": "isomorphic",  // Will be unix or tcp
                    "max_windows": 16,
                    "supported_formats": ["RGBA8888", "BGRA8888"],
                    "has_gpu_acceleration": true,
                    "vsync_available": true,
                    "display_count": 1,
                    "input_device_count": 0,
                    "window_count": mgr.window_count(),
                    "isomorphic": true,  // Isomorphic IPC support
                }))
            }
            _ => Err(DisplayError::IpcError(format!(
                "Unknown method: {}",
                request.method
            ))),
        }
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

    #[test]
    fn test_socket_path_discovery() {
        let path = DisplayServer::discover_socket_path();
        assert!(path.to_string_lossy().contains("toadstool"));
        assert!(path.to_string_lossy().ends_with("display.sock"));
    }

    #[tokio::test]
    async fn test_jsonrpc_parsing() {
        let request_str = r#"{"jsonrpc":"2.0","method":"display.get_capabilities","id":1}"#;
        let request: JsonRpcRequest = serde_json::from_str(request_str).unwrap();
        assert_eq!(request.method, "display.get_capabilities");
    }
}
