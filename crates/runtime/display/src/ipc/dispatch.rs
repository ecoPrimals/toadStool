// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC request dispatch for display operations
//!
//! Handles parsing, routing, and execution of display.* JSON-RPC methods.

use super::platform;
use super::types::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::window::{CreateWindowRequest, Size, WindowId, WindowManager};
use crate::{DisplayError, Result};
use std::sync::Arc;
use tokio::sync::RwLock;

/// Handle a single JSON-RPC request
///
/// Parses the request, dispatches to the appropriate handler, and returns
/// a JSON-RPC response (success or error).
pub async fn handle_request(
    request_str: &str,
    manager: &Arc<RwLock<WindowManager>>,
) -> JsonRpcResponse {
    let request: JsonRpcRequest = match serde_json::from_slice(request_str.as_bytes()) {
        Ok(req) => req,
        Err(_) => {
            return JsonRpcResponse::error(serde_json::json!(null), JsonRpcError::parse_error());
        }
    };

    let id = request.id.clone().unwrap_or(serde_json::json!(null));

    let result = dispatch_method(&request, manager).await;

    match result {
        Ok(value) => JsonRpcResponse::success(id, value),
        Err(e) => JsonRpcResponse::error(id, JsonRpcError::internal_error(&e.to_string())),
    }
}

/// Dispatch method to handler
async fn dispatch_method(
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
            let window_id = mgr.create_window(params)?;

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
            mgr.destroy_window(window_id)?;

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
            #[allow(clippy::cast_possible_truncation)] // Display dimensions fit in u32
            let width = params["width"]
                .as_u64()
                .ok_or_else(|| DisplayError::IpcError("Missing width".to_string()))?
                as u32;
            #[allow(clippy::cast_possible_truncation)] // Display dimensions fit in u32
            let height = params["height"]
                .as_u64()
                .ok_or_else(|| DisplayError::IpcError("Missing height".to_string()))?
                as u32;

            let mut mgr = manager.write().await;
            mgr.resize_window(window_id, Size { width, height })?;

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

            Ok(serde_json::json!({
                "primal_id": "toadstool-primary",
                "socket_path": platform::discover_socket_path().display().to_string(),
                "transport": "isomorphic",
                "max_windows": 16,
                "supported_formats": ["RGBA8888", "BGRA8888"],
                "has_gpu_acceleration": true,
                "vsync_available": true,
                "display_count": 1,
                "input_device_count": 0,
                "window_count": mgr.window_count(),
                "isomorphic": true,
            }))
        }
        _ => Err(DisplayError::IpcError(format!(
            "Unknown method: {}",
            request.method
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_manager() -> Option<Arc<RwLock<WindowManager>>> {
        WindowManager::new()
            .await
            .ok()
            .map(RwLock::new)
            .map(Arc::new)
    }

    #[tokio::test]
    async fn test_handle_request_parse_error() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let response = handle_request("not valid json {{{", &manager).await;
        assert!(
            response.error.is_some(),
            "parse error should return error response"
        );
        let err = response.error.unwrap();
        assert_eq!(err.code, -32700, "parse error code");
        assert!(err.message.to_lowercase().contains("parse"));
    }

    #[tokio::test]
    async fn test_handle_request_empty_string_parse_error() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let response = handle_request("", &manager).await;
        assert!(response.error.is_some());
        assert_eq!(response.error.unwrap().code, -32700);
    }

    #[tokio::test]
    async fn test_handle_request_unknown_method() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str = r#"{"jsonrpc":"2.0","method":"display.nonexistent","params":{},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(
            response.error.is_some(),
            "unknown method should return error"
        );
        let err = response.error.unwrap();
        assert_eq!(err.code, -32603, "internal error code for unknown method");
        assert!(err.message.contains("Unknown method"));
        assert!(err.message.contains("display.nonexistent"));
    }

    #[tokio::test]
    async fn test_handle_request_get_capabilities() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str =
            r#"{"jsonrpc":"2.0","method":"display.get_capabilities","params":{},"id":42}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(
            response.error.is_none(),
            "get_capabilities should succeed: {:?}",
            response.error
        );
        let result = response.result.expect("success response has result");
        assert_eq!(result["primal_id"], "toadstool-primary");
        assert_eq!(result["max_windows"], 16);
        assert_eq!(result["isomorphic"], true);
        assert!(result["socket_path"]
            .as_str()
            .unwrap()
            .contains("toadstool"));
    }

    #[tokio::test]
    async fn test_handle_request_destroy_window_missing_params() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str =
            r#"{"jsonrpc":"2.0","method":"display.destroy_window","params":{},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(response.error.is_some());
        assert!(response
            .error
            .unwrap()
            .message
            .contains("Missing window_id"));
    }

    #[tokio::test]
    async fn test_handle_request_resize_window_missing_params() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str =
            r#"{"jsonrpc":"2.0","method":"display.resize_window","params":{},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_null_id_uses_null_in_response() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str = r#"{"jsonrpc":"2.0","method":"display.get_capabilities","id":null}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(response.error.is_none());
        assert!(response.result.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_create_window_default_params() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str =
            r#"{"jsonrpc":"2.0","method":"display.create_window","params":{},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(
            response.error.is_none(),
            "create_window with empty params should succeed: {:?}",
            response.error
        );
        assert!(response.result.is_some());
        let result = response.result.unwrap();
        assert!(result.get("window_id").is_some());
    }

    #[tokio::test]
    async fn test_handle_request_resize_window_missing_width() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let window_id = crate::window::WindowId::new();
        let request_str = format!(
            r#"{{"jsonrpc":"2.0","method":"display.resize_window","params":{{"window_id":"{}","height":600}},"id":1}}"#,
            window_id.as_string()
        );
        let response = handle_request(&request_str, &manager).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_get_window_info_missing_params() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str =
            r#"{"jsonrpc":"2.0","method":"display.get_window_info","params":{},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(response.error.is_some());
        assert!(response
            .error
            .unwrap()
            .message
            .contains("Missing window_id"));
    }

    #[tokio::test]
    async fn test_handle_request_resize_window_missing_height() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let window_id = crate::window::WindowId::new();
        let request_str = format!(
            r#"{{"jsonrpc":"2.0","method":"display.resize_window","params":{{"window_id":"{}","width":800}},"id":1}}"#,
            window_id.as_string()
        );
        let response = handle_request(&request_str, &manager).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_resize_window_invalid_window_id() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str = r#"{"jsonrpc":"2.0","method":"display.resize_window","params":{"window_id":"not-a-uuid","width":800,"height":600},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_destroy_window_invalid_id() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str = r#"{"jsonrpc":"2.0","method":"display.destroy_window","params":{"window_id":"invalid-uuid"},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_get_window_info_invalid_id() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str = r#"{"jsonrpc":"2.0","method":"display.get_window_info","params":{"window_id":"bad-id"},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_resize_window_params_not_object() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str =
            r#"{"jsonrpc":"2.0","method":"display.resize_window","params":"not-an-object","id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(response.error.is_some());
    }

    #[tokio::test]
    async fn test_handle_request_create_window_with_params() {
        let manager = match test_manager().await {
            Some(m) => m,
            None => return,
        };
        let request_str = r#"{"jsonrpc":"2.0","method":"display.create_window","params":{"width":640,"height":480,"title":"Test"},"id":1}"#;
        let response = handle_request(request_str, &manager).await;
        assert!(
            response.error.is_none(),
            "create_window with params should succeed: {:?}",
            response.error
        );
        assert!(response.result.is_some());
    }
}
