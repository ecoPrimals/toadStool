//! JSON-RPC protocol types for display operations
//!
//! Defines the protocol between petalTongue (client) and Toadstool (server).

use crate::input::InputEvent;
use crate::window::{CreateWindowRequest, WindowInfo};
use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Protocol version (always "2.0")
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
    /// Request ID (for responses)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
}

impl JsonRpcRequest {
    /// Create a new request
    pub fn new(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
            id: Some(serde_json::json!(uuid::Uuid::new_v4().to_string())),
        }
    }

    /// Create a notification (no response expected)
    pub fn notification(method: impl Into<String>, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
            id: None,
        }
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Protocol version
    pub jsonrpc: String,
    /// Result (on success)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error (on failure)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    /// Request ID
    pub id: serde_json::Value,
}

impl JsonRpcResponse {
    /// Create a success response
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    pub fn error(id: serde_json::Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// JSON-RPC error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code
    pub code: i32,
    /// Error message
    pub message: String,
    /// Additional error data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl JsonRpcError {
    /// Parse error (-32700)
    pub fn parse_error() -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        }
    }

    /// Invalid request (-32600)
    pub fn invalid_request() -> Self {
        Self {
            code: -32600,
            message: "Invalid request".to_string(),
            data: None,
        }
    }

    /// Method not found (-32601)
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    /// Invalid params (-32602)
    pub fn invalid_params(msg: &str) -> Self {
        Self {
            code: -32602,
            message: format!("Invalid params: {}", msg),
            data: None,
        }
    }

    /// Internal error (-32603)
    pub fn internal_error(msg: &str) -> Self {
        Self {
            code: -32603,
            message: format!("Internal error: {}", msg),
            data: None,
        }
    }
}

/// Display method requests
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params")]
pub enum DisplayMethod {
    /// Create a new window
    #[serde(rename = "display.createWindow")]
    CreateWindow(CreateWindowRequest),

    /// Destroy a window
    #[serde(rename = "display.destroyWindow")]
    DestroyWindow {
        /// Window ID to destroy
        window_id: String,
    },

    /// Resize a window
    #[serde(rename = "display.resizeWindow")]
    ResizeWindow {
        /// Window ID to resize
        window_id: String,
        /// New width
        width: u32,
        /// New height
        height: u32,
    },

    /// Get window information
    #[serde(rename = "display.getWindowInfo")]
    GetWindowInfo {
        /// Window ID to query
        window_id: String,
    },

    /// Subscribe to input events
    #[serde(rename = "display.subscribeInput")]
    SubscribeInput {
        /// Window ID for events
        window_id: String,
    },

    /// Poll for pending events
    #[serde(rename = "display.pollEvents")]
    PollEvents,

    /// Get display capabilities
    #[serde(rename = "display.getCapabilities")]
    GetCapabilities,

    /// Present framebuffer (future: zero-copy)
    #[serde(rename = "display.present")]
    Present {
        /// Window ID to present
        window_id: String,
        // Future: shared memory handle
    },
}

/// Display method responses
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DisplayResult {
    /// Window created
    WindowCreated {
        /// Created window ID
        window_id: String,
    },

    /// Window destroyed
    WindowDestroyed,

    /// Window resized
    WindowResized,

    /// Window information
    WindowInfo(WindowInfo),

    /// Input subscription
    InputSubscription {
        /// Subscription status
        subscribed: bool,
    },

    /// Polled events
    Events {
        /// List of input events
        events: Vec<InputEvent>,
    },

    /// Display capabilities
    Capabilities {
        /// Capability information
        capabilities: DisplayCapabilitiesInfo,
    },

    /// Present acknowledgment
    Presented,
}

/// Display capabilities information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayCapabilitiesInfo {
    /// Primal ID
    pub primal_id: String,
    /// Socket path
    pub socket_path: String,
    /// Maximum windows supported
    pub max_windows: usize,
    /// Supported pixel formats
    pub supported_formats: Vec<String>,
    /// GPU acceleration available
    pub has_gpu_acceleration: bool,
    /// VSync available
    pub vsync_available: bool,
    /// Number of displays
    pub display_count: usize,
    /// Number of input devices
    pub input_device_count: usize,
    /// Current window count (Phase 3)
    pub window_count: usize,
    /// Isomorphic IPC support (Phase 3)
    pub isomorphic: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jsonrpc_request_serialization() {
        let req = JsonRpcRequest::new("display.createWindow", None);
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"display.createWindow\""));
    }

    #[test]
    fn test_jsonrpc_notification() {
        let notif = JsonRpcRequest::notification("display.inputEvent", None);
        assert!(notif.id.is_none());
    }

    #[test]
    fn test_jsonrpc_response_success() {
        let resp =
            JsonRpcResponse::success(serde_json::json!(1), serde_json::json!({"status": "ok"}));
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_jsonrpc_error() {
        let err = JsonRpcError::method_not_found("unknown.method");
        assert_eq!(err.code, -32601);
        assert!(err.message.contains("unknown.method"));
    }
}
