// SPDX-License-Identifier: AGPL-3.0-only
//! Display IPC operations (windows, capabilities, endpoint metadata).

use super::DisplayClient;
use crate::DisplayError;
use crate::ipc::types::DisplayCapabilitiesInfo;
use crate::window::{CreateWindowRequest, WindowId, WindowInfo};

impl DisplayClient {
    /// Create a window
    ///
    /// # Errors
    ///
    /// Returns an error if the IPC request fails or the server returns an error.
    pub async fn create_window(&mut self, request: CreateWindowRequest) -> crate::Result<WindowId> {
        let params = serde_json::to_value(request)
            .map_err(|e| DisplayError::IpcError(format!("Failed to serialize request: {e}")))?;
        let req = crate::ipc::types::JsonRpcRequest::new("display.create_window", Some(params));

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
    ///
    /// # Errors
    ///
    /// Returns an error if the IPC request fails or the server returns an error.
    pub async fn destroy_window(&mut self, window_id: WindowId) -> crate::Result<()> {
        let req = crate::ipc::types::JsonRpcRequest::new(
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
    ///
    /// # Errors
    ///
    /// Returns an error if the IPC request fails or the server returns an error.
    pub async fn resize_window(
        &mut self,
        window_id: WindowId,
        width: u32,
        height: u32,
    ) -> crate::Result<()> {
        let req = crate::ipc::types::JsonRpcRequest::new(
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
    ///
    /// # Errors
    ///
    /// Returns an error if the IPC request fails or the server returns an error.
    pub async fn get_window_info(&mut self, window_id: WindowId) -> crate::Result<WindowInfo> {
        let req = crate::ipc::types::JsonRpcRequest::new(
            "display.get_window_info",
            Some(serde_json::json!({"window_id": window_id.as_string()})),
        );

        let response = self.send_request(req).await?;

        if let Some(result) = response.result {
            serde_json::from_value(result)
                .map_err(|e| DisplayError::IpcError(format!("Parse error: {e}")))
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
    ///
    /// # Errors
    ///
    /// Returns an error if the IPC request fails or the server returns an error.
    pub async fn get_capabilities(&mut self) -> crate::Result<DisplayCapabilitiesInfo> {
        let req = crate::ipc::types::JsonRpcRequest::new("display.get_capabilities", None);

        let response = self.send_request(req).await?;

        if let Some(result) = response.result {
            serde_json::from_value(result)
                .map_err(|e| DisplayError::IpcError(format!("Parse error: {e}")))
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
    #[must_use]
    pub fn endpoint_string(&self) -> String {
        match &self.endpoint {
            super::IpcEndpoint::UnixSocket(path) => path.display().to_string(),
            super::IpcEndpoint::TcpLocal(addr) => addr.to_string(),
        }
    }

    /// Get transport name for display purposes
    ///
    /// **Helper for health checks and monitoring**
    #[must_use]
    pub const fn transport_name(&self) -> &str {
        match &self.endpoint {
            super::IpcEndpoint::UnixSocket(_) => "unix",
            super::IpcEndpoint::TcpLocal(_) => "tcp",
        }
    }
}
