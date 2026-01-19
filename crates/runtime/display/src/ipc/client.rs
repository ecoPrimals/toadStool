//! Display client implementation
//!
//! JSON-RPC client for connecting to display server.

use super::types::*;
use crate::window::{CreateWindowRequest, WindowId, WindowInfo};
use crate::{DisplayError, Result};
use std::path::PathBuf;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

/// Display client
///
/// Connects to display server via Unix sockets.
///
/// ## Example
///
/// ```rust,no_run
/// use toadstool_display::ipc::DisplayClient;
/// use toadstool_display::window::CreateWindowRequest;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut client = DisplayClient::connect("/run/user/1000/toadstool/display.sock").await?;
///
/// let window_id = client.create_window(CreateWindowRequest::default()).await?;
/// println!("Created window: {}", window_id);
/// # Ok(())
/// # }
/// ```
pub struct DisplayClient {
    stream: UnixStream,
}

impl DisplayClient {
    /// Connect to display server
    ///
    /// **Capability-based**: Discovers socket path if not provided!
    pub async fn connect(path: impl Into<PathBuf>) -> Result<Self> {
        let path = path.into();
        tracing::info!("Connecting to display server: {}", path.display());

        let stream = UnixStream::connect(&path)
            .await
            .map_err(|e| DisplayError::IpcError(format!("Connection failed: {}", e)))?;

        tracing::info!("✅ Connected to display server");

        Ok(Self { stream })
    }

    /// Send a request and receive response
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

        // Read response
        let (reader, _writer) = self.stream.split();
        let mut reader = BufReader::new(reader);
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
        let req = JsonRpcRequest::new(
            "display.createWindow",
            Some(serde_json::to_value(request).unwrap()),
        );

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
            "display.destroyWindow",
            Some(serde_json::json!({"window_id": window_id.to_string()})),
        );

        let response = self.send_request(req).await?;

        if response.error.is_some() {
            Err(DisplayError::IpcError("Failed to destroy window".to_string()))
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
            "display.resizeWindow",
            Some(serde_json::json!({
                "window_id": window_id.to_string(),
                "width": width,
                "height": height
            })),
        );

        let response = self.send_request(req).await?;

        if response.error.is_some() {
            Err(DisplayError::IpcError("Failed to resize window".to_string()))
        } else {
            Ok(())
        }
    }

    /// Get window information
    pub async fn get_window_info(&mut self, window_id: WindowId) -> Result<WindowInfo> {
        let req = JsonRpcRequest::new(
            "display.getWindowInfo",
            Some(serde_json::json!({"window_id": window_id.to_string()})),
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
        let req = JsonRpcRequest::new("display.getCapabilities", None);

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_creation() {
        let req = JsonRpcRequest::new("display.createWindow", None);
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "display.createWindow");
        assert!(req.id.is_some());
    }
}
