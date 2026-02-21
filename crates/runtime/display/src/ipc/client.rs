//! Display client implementation
//!
//! **ISOMORPHIC IPC**: JSON-RPC client with automatic Unix/TCP discovery.
//!
//! ## Evolution Status
//!
//! ✅ **Phase 2 COMPLETE**: Isomorphic client (polymorphic discovery)
//!
//! ## Architecture
//!
//! ```text
//! 1. DISCOVER endpoint (Unix socket OR TCP)
//!     ↓
//! 2. CONNECT via discovered transport
//!     ↓
//! 3. COMMUNICATE (same JSON-RPC protocol)
//! ```
//!
//! ## Example
//!
//! ```rust,no_run
//! use toadstool_display::ipc::DisplayClient;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Zero configuration - automatic discovery!
//! let mut client = DisplayClient::discover().await?;
//!
//! // Works whether server is using Unix sockets OR TCP fallback!
//! # Ok(())
//! # }
//! ```

use super::types::*;
use crate::window::{CreateWindowRequest, WindowId, WindowInfo};
use crate::{DisplayError, Result};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::net::{TcpStream, UnixStream};

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
/// Allows DisplayClient to work with both UnixStream and TcpStream transparently.
trait AsyncStream: AsyncRead + AsyncWrite + Unpin + Send {}

// Implement for both stream types
impl AsyncStream for UnixStream {}
impl AsyncStream for TcpStream {}

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
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
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

impl DisplayClient {
    /// Discover and connect to display server (ISOMORPHIC!)
    ///
    /// **Zero Configuration**: Automatically discovers Unix socket OR TCP endpoint!
    ///
    /// ## Behavior
    ///
    /// 1. Try Unix socket paths (optimal)
    /// 2. Try TCP discovery file (fallback)
    /// 3. Connect via discovered endpoint
    ///
    /// ## Example
    ///
    /// ```rust,no_run
    /// # use toadstool_display::ipc::DisplayClient;
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut client = DisplayClient::discover().await?;
    /// // Works on Linux (Unix) AND Android (TCP)!
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover() -> Result<Self> {
        tracing::info!("🔍 Discovering display server endpoint (isomorphic mode)...");

        let endpoint = Self::discover_endpoint()?;

        match &endpoint {
            IpcEndpoint::UnixSocket(path) => {
                tracing::info!("   Found Unix socket: {}", path.display());
            }
            IpcEndpoint::TcpLocal(addr) => {
                tracing::info!("   Found TCP endpoint: {}", addr);
            }
        }

        Self::connect_endpoint(endpoint).await
    }

    /// Discover IPC endpoint (Unix OR TCP)
    ///
    /// **Capability-based**: Tries multiple discovery methods!
    fn discover_endpoint() -> Result<IpcEndpoint> {
        // 1. Try Unix socket paths (optimal)
        let socket_paths = Self::get_socket_paths();
        for path in socket_paths {
            if path.exists() {
                tracing::debug!("   Unix socket found: {}", path.display());
                return Ok(IpcEndpoint::UnixSocket(path));
            }
        }

        // 2. Try TCP discovery file (fallback mode)
        if let Ok(endpoint) = Self::discover_tcp_endpoint() {
            tracing::debug!("   TCP endpoint discovered from file");
            return Ok(endpoint);
        }

        Err(DisplayError::IpcError(
            "Could not discover display server endpoint (tried Unix sockets and TCP discovery)"
                .to_string(),
        ))
    }

    /// Get candidate Unix socket paths
    ///
    /// **XDG-compliant**: Uses PlatformPaths for consistent path resolution
    fn get_socket_paths() -> Vec<PathBuf> {
        use toadstool_common::platform_paths::{PathEnv, PlatformPaths};

        let mut paths = Vec::new();
        let env = PathEnv::from_env();
        let platform_paths = PlatformPaths::new(&env);

        // Primary: PlatformPaths socket directory (XDG_RUNTIME_DIR or fallback)
        paths.push(platform_paths.toadstool_socket_dir().join("display.sock"));

        // Secondary: HOME/.local/share
        if let Ok(home) = std::env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("toadstool");
            path.push("display.sock");
            paths.push(path);
        }

        // Tertiary: temp_dir fallback (platform-agnostic)
        paths.push(platform_paths.toadstool_temp_dir().join("display.sock"));

        paths
    }

    /// Discover TCP endpoint from discovery file
    ///
    /// **Fallback mode**: Reads TCP port from server's discovery file
    fn discover_tcp_endpoint() -> Result<IpcEndpoint> {
        let discovery_files = Self::get_tcp_discovery_file_candidates();

        for file in discovery_files {
            if let Ok(contents) = std::fs::read_to_string(&file) {
                // Parse format: tcp:127.0.0.1:PORT
                if let Some(addr_str) = contents.trim().strip_prefix("tcp:") {
                    if let Ok(addr) = addr_str.parse::<SocketAddr>() {
                        tracing::debug!("   TCP discovery file: {}", file.display());
                        return Ok(IpcEndpoint::TcpLocal(addr));
                    }
                }
            }
        }

        Err(DisplayError::IpcError(
            "No TCP discovery file found".to_string(),
        ))
    }

    /// Get candidate TCP discovery file paths
    ///
    /// **XDG-compliant**: Uses PlatformPaths for consistent path resolution
    fn get_tcp_discovery_file_candidates() -> Vec<PathBuf> {
        use toadstool_common::platform_paths::{PathEnv, PlatformPaths};

        let mut paths = Vec::new();
        let env = PathEnv::from_env();
        let platform_paths = PlatformPaths::new(&env);

        // Primary: XDG_RUNTIME_DIR via PlatformPaths
        paths.push(platform_paths.runtime_dir().join("toadstool-ipc-port"));

        // Secondary: HOME/.local/share
        if let Ok(home) = std::env::var("HOME") {
            let mut path = PathBuf::from(home);
            path.push(".local");
            path.push("share");
            path.push("toadstool-ipc-port");
            paths.push(path);
        }

        // Tertiary: temp_dir fallback (platform-agnostic)
        paths.push(std::env::temp_dir().join("toadstool-ipc-port"));

        paths
    }

    /// Connect to endpoint (polymorphic!)
    ///
    /// **Works with both Unix and TCP transparently**
    async fn connect_endpoint(endpoint: IpcEndpoint) -> Result<Self> {
        match &endpoint {
            IpcEndpoint::UnixSocket(path) => {
                tracing::info!("🔌 Connecting via Unix socket...");

                let stream = UnixStream::connect(path).await.map_err(|e| {
                    DisplayError::IpcError(format!("Unix connection failed: {}", e))
                })?;

                tracing::info!("✅ Connected to display server (Unix socket)");

                Ok(Self {
                    stream: Box::new(stream),
                    endpoint,
                })
            }
            IpcEndpoint::TcpLocal(addr) => {
                tracing::info!("🌐 Connecting via TCP fallback...");

                let stream = TcpStream::connect(addr)
                    .await
                    .map_err(|e| DisplayError::IpcError(format!("TCP connection failed: {}", e)))?;

                tracing::info!("✅ Connected to display server (TCP fallback)");

                Ok(Self {
                    stream: Box::new(stream),
                    endpoint,
                })
            }
        }
    }

    /// Connect to specific path (backward compatibility)
    ///
    /// **Legacy method**: Use `discover()` for automatic discovery!
    pub async fn connect(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        tracing::info!("Connecting to display server: {}", path.display());

        let stream = UnixStream::connect(&path)
            .await
            .map_err(|e| DisplayError::IpcError(format!("Connection failed: {}", e)))?;

        tracing::info!("✅ Connected to display server");

        Ok(Self {
            stream: Box::new(stream),
            endpoint: IpcEndpoint::UnixSocket(path),
        })
    }

    /// Get connected endpoint
    pub fn endpoint(&self) -> &IpcEndpoint {
        &self.endpoint
    }

    /// Send a request and receive response (polymorphic!)
    ///
    /// **Works with both Unix and TCP streams transparently**
    async fn send_request(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse> {
        // Serialize request
        let request_json = serde_json::to_string(&request)
            .map_err(|e| DisplayError::IpcError(format!("Serialization error: {}", e)))?;

        // Send request
        self.stream
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| DisplayError::IpcError(format!("Write error: {}", e)))?;
        self.stream
            .write_all(b"\n")
            .await
            .map_err(|e| DisplayError::IpcError(format!("Write error: {}", e)))?;

        // Read response (using BufReader directly on the stream)
        let mut reader = BufReader::new(&mut self.stream);
        let mut line = String::new();

        reader
            .read_line(&mut line)
            .await
            .map_err(|e| DisplayError::IpcError(format!("Read error: {}", e)))?;

        // Parse response
        serde_json::from_str(&line)
            .map_err(|e| DisplayError::IpcError(format!("Parse error: {}", e)))
    }

    /// Create a window
    pub async fn create_window(&mut self, request: CreateWindowRequest) -> Result<WindowId> {
        let params = serde_json::to_value(request)
            .map_err(|e| DisplayError::IpcError(format!("Failed to serialize request: {e}")))?;
        let req = JsonRpcRequest::new("display.create_window", Some(params));

        let response = self.send_request(req).await?;

        if let Some(result) = response.result {
            let window_id_str = result["window_id"]
                .as_str()
                .ok_or_else(|| DisplayError::IpcError("Invalid response".to_string()))?;
            WindowId::from_string(window_id_str)
        } else if let Some(error) = response.error {
            Err(DisplayError::IpcError(format!(
                "Server error: {}",
                error.message
            )))
        } else {
            Err(DisplayError::IpcError("Invalid response".to_string()))
        }
    }

    /// Destroy a window
    pub async fn destroy_window(&mut self, window_id: WindowId) -> Result<()> {
        let req = JsonRpcRequest::new(
            "display.destroy_window",
            Some(serde_json::json!({"window_id": window_id.as_string()})),
        );

        let response = self.send_request(req).await?;

        if response.error.is_some() {
            Err(DisplayError::IpcError(
                "Failed to destroy window".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Resize a window
    pub async fn resize_window(
        &mut self,
        window_id: WindowId,
        width: u32,
        height: u32,
    ) -> Result<()> {
        let req = JsonRpcRequest::new(
            "display.resize_window",
            Some(serde_json::json!({
                "window_id": window_id.as_string(),
                "width": width,
                "height": height
            })),
        );

        let response = self.send_request(req).await?;

        if response.error.is_some() {
            Err(DisplayError::IpcError(
                "Failed to resize window".to_string(),
            ))
        } else {
            Ok(())
        }
    }

    /// Get window information
    pub async fn get_window_info(&mut self, window_id: WindowId) -> Result<WindowInfo> {
        let req = JsonRpcRequest::new(
            "display.get_window_info",
            Some(serde_json::json!({"window_id": window_id.as_string()})),
        );

        let response = self.send_request(req).await?;

        if let Some(result) = response.result {
            serde_json::from_value(result)
                .map_err(|e| DisplayError::IpcError(format!("Parse error: {}", e)))
        } else if let Some(error) = response.error {
            Err(DisplayError::IpcError(format!(
                "Server error: {}",
                error.message
            )))
        } else {
            Err(DisplayError::IpcError("Invalid response".to_string()))
        }
    }

    /// Get display capabilities
    pub async fn get_capabilities(&mut self) -> Result<DisplayCapabilitiesInfo> {
        let req = JsonRpcRequest::new("display.get_capabilities", None);

        let response = self.send_request(req).await?;

        if let Some(result) = response.result {
            serde_json::from_value(result)
                .map_err(|e| DisplayError::IpcError(format!("Parse error: {}", e)))
        } else if let Some(error) = response.error {
            Err(DisplayError::IpcError(format!(
                "Server error: {}",
                error.message
            )))
        } else {
            Err(DisplayError::IpcError("Invalid response".to_string()))
        }
    }

    /// Get endpoint string for display purposes
    ///
    /// **Helper for health checks and monitoring**
    pub fn endpoint_string(&self) -> String {
        match &self.endpoint {
            IpcEndpoint::UnixSocket(path) => path.display().to_string(),
            IpcEndpoint::TcpLocal(addr) => addr.to_string(),
        }
    }

    /// Get transport name for display purposes
    ///
    /// **Helper for health checks and monitoring**
    pub fn transport_name(&self) -> &str {
        match &self.endpoint {
            IpcEndpoint::UnixSocket(_) => "unix",
            IpcEndpoint::TcpLocal(_) => "tcp",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_creation() {
        let req = JsonRpcRequest::new("display.create_window", None);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "display.create_window");
        assert!(req.id.is_some());
    }
}
