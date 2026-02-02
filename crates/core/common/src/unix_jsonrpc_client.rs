//! # Unix Socket JSON-RPC 2.0 Client
//!
//! Pure Rust JSON-RPC client over unix sockets for primal-to-primal communication.
//!
//! ## TRUE PRIMAL Architecture
//!
//! - **No HTTP**: All primal communication via unix sockets (pure Rust!)
//! - **JSON-RPC 2.0**: Standard protocol, language-agnostic
//! - **Async**: Fully concurrent with tokio
//! - **Type-Safe**: serde for serialization/deserialization
//!
//! ## Design
//!
//! Complements `ManualJsonRpcServer` from toadstool-server.
//! Same protocol, different transport (unix socket vs HTTP).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::{ToadStoolError, ToadStoolResult};

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: u64,
    method: String,
    params: Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<Value>,
}

/// Unix socket JSON-RPC 2.0 client
///
/// **Pure Rust**: No HTTP, no TLS, no ring dependency!
///
/// ## Example
///
/// ```rust,ignore
/// use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
/// use toadstool_common::primal_sockets::get_beardog_socket_path;
///
/// let socket_path = get_beardog_socket_path();
/// let client = UnixJsonRpcClient::new(socket_path);
///
/// let params = serde_json::json!({
///     "data": "hello",
///     "algorithm": "aes-256-gcm"
/// });
///
/// let result = client.call("beardog.encrypt", params).await?;
/// ```
#[derive(Debug, Clone)]
pub struct UnixJsonRpcClient {
    socket_path: PathBuf,
    next_id: std::sync::Arc<std::sync::atomic::AtomicU64>,
}

impl UnixJsonRpcClient {
    /// Create new JSON-RPC client for unix socket
    ///
    /// **TRUE PRIMAL**: Socket path from discovery, not hardcoded!
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
            next_id: std::sync::Arc::new(std::sync::atomic::AtomicU64::new(1)),
        }
    }

    /// Call JSON-RPC method over unix socket
    ///
    /// ## Errors
    ///
    /// Returns error if:
    /// - Socket connection fails
    /// - Request serialization fails
    /// - Response parsing fails
    /// - Server returns JSON-RPC error
    ///
    /// ## Modern Async
    ///
    /// Fully async with tokio - no blocking!
    pub async fn call(&self, method: &str, params: Value) -> ToadStoolResult<Value> {
        // Connect to unix socket
        let stream = UnixStream::connect(&self.socket_path).await.map_err(|e| {
            ToadStoolError::network(format!(
                "Failed to connect to {:?}: {}",
                self.socket_path, e
            ))
        })?;

        // Generate request ID
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Build JSON-RPC 2.0 request
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.to_string(),
            params,
        };

        // Serialize request
        let request_json = serde_json::to_string(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {}", e)))?;

        // Split stream for concurrent read/write
        let (reader, mut writer) = stream.into_split();

        // Send request (newline-delimited JSON)
        writer
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send request: {}", e)))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send newline: {}", e)))?;
        writer
            .flush()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to flush: {}", e)))?;

        // Read response (newline-delimited JSON)
        let mut reader = BufReader::new(reader);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to read response: {}", e)))?;

        // Parse JSON-RPC response
        let response: JsonRpcResponse = serde_json::from_str(&response_line)
            .map_err(|e| ToadStoolError::network(format!("Invalid JSON-RPC response: {}", e)))?;

        // Check for JSON-RPC error
        if let Some(error) = response.error {
            return Err(ToadStoolError::execution(format!(
                "JSON-RPC error (code {}): {}",
                error.code, error.message
            )));
        }

        // Return result
        response
            .result
            .ok_or_else(|| ToadStoolError::network("JSON-RPC response missing result"))
    }

    /// Call JSON-RPC method and deserialize response
    ///
    /// ## Type Safety
    ///
    /// Automatically deserializes response into expected type.
    ///
    /// ## Example
    ///
    /// ```rust,ignore
    /// #[derive(Deserialize)]
    /// struct EncryptResponse {
    ///     ciphertext: Vec<u8>,
    ///     nonce: Vec<u8>,
    /// }
    ///
    /// let response: EncryptResponse = client
    ///     .call_typed("beardog.encrypt", params)
    ///     .await?;
    /// ```
    pub async fn call_typed<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Value,
    ) -> ToadStoolResult<T> {
        let result = self.call(method, params).await?;
        serde_json::from_value(result)
            .map_err(|e| ToadStoolError::network(format!("Failed to deserialize response: {}", e)))
    }

    /// Check if socket exists and is accessible
    ///
    /// **Use Case**: Graceful degradation - check before calling
    pub async fn is_available(&self) -> bool {
        self.socket_path.exists()
    }

    /// Get socket path (for diagnostics)
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = UnixJsonRpcClient::new("/tmp/test.sock");
        assert_eq!(client.socket_path(), Path::new("/tmp/test.sock"));
    }

    #[test]
    fn test_request_serialization() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test.method".to_string(),
            params: serde_json::json!({"key": "value"}),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
        assert!(json.contains("\"method\":\"test.method\""));
        assert!(json.contains("\"id\":1"));
    }

    #[test]
    fn test_response_deserialization() {
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{"status":"ok"}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 1);
        assert!(response.result.is_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn test_error_response_deserialization() {
        let json =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32600,"message":"Invalid Request"}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();

        assert!(response.result.is_none());
        assert!(response.error.is_some());

        let error = response.error.unwrap();
        assert_eq!(error.code, -32600);
        assert_eq!(error.message, "Invalid Request");
    }

    #[test]
    fn test_request_with_empty_params() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 42,
            method: "simple.method".to_string(),
            params: serde_json::json!(null),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("\"id\":42"));
        assert!(json.contains("\"method\":\"simple.method\""));
    }

    #[test]
    fn test_request_with_array_params() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test.array".to_string(),
            params: serde_json::json!([1, 2, 3]),
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("[1,2,3]"));
    }

    #[test]
    fn test_response_with_empty_object_result() {
        // Test with empty object instead of null (more realistic)
        let json = r#"{"jsonrpc":"2.0","id":1,"result":{}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();

        assert!(response.result.is_some());
        assert!(response.result.unwrap().is_object());
    }

    #[test]
    fn test_error_with_data() {
        let json = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32603,"message":"Internal error","data":{"details":"stack trace here"}}}"#;
        let response: JsonRpcResponse = serde_json::from_str(json).unwrap();

        let error = response.error.unwrap();
        assert_eq!(error.code, -32603);
        assert!(error.data.is_some());
        assert!(error.data.unwrap()["details"]
            .as_str()
            .unwrap()
            .contains("stack trace"));
    }

    #[test]
    fn test_client_path_conversion() {
        // Test with &str
        let client1 = UnixJsonRpcClient::new("/tmp/test1.sock");
        assert_eq!(client1.socket_path(), Path::new("/tmp/test1.sock"));

        // Test with String
        let client2 = UnixJsonRpcClient::new("/tmp/test2.sock".to_string());
        assert_eq!(client2.socket_path(), Path::new("/tmp/test2.sock"));

        // Test with PathBuf
        let path = PathBuf::from("/tmp/test3.sock");
        let client3 = UnixJsonRpcClient::new(path);
        assert_eq!(client3.socket_path(), Path::new("/tmp/test3.sock"));
    }

    #[test]
    fn test_client_clone() {
        let client1 = UnixJsonRpcClient::new("/tmp/original.sock");
        let client2 = client1.clone();

        assert_eq!(client1.socket_path(), client2.socket_path());
    }

    #[test]
    fn test_request_id_increment() {
        let client = UnixJsonRpcClient::new("/tmp/test.sock");

        // Access the atomic counter
        let id1 = client
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let id2 = client
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let id3 = client
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // IDs should increment
        assert_eq!(id1, 1);
        assert_eq!(id2, 2);
        assert_eq!(id3, 3);
    }

    #[test]
    fn test_jsonrpc_request_debug() {
        let request = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: 1,
            method: "test".to_string(),
            params: serde_json::json!({}),
        };

        let debug_str = format!("{:?}", request);
        assert!(debug_str.contains("JsonRpcRequest"));
        assert!(debug_str.contains("test"));
    }

    #[test]
    fn test_jsonrpc_response_debug() {
        let response = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: Some(serde_json::json!({"ok": true})),
            error: None,
        };

        let debug_str = format!("{:?}", response);
        assert!(debug_str.contains("JsonRpcResponse"));
    }

    #[test]
    fn test_jsonrpc_error_debug() {
        let error = JsonRpcError {
            code: -32700,
            message: "Parse error".to_string(),
            data: None,
        };

        let debug_str = format!("{:?}", error);
        assert!(debug_str.contains("JsonRpcError"));
        assert!(debug_str.contains("-32700"));
    }

    #[test]
    fn test_response_serialization_skips_none() {
        // Response with only result (no error)
        let response1 = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: Some(serde_json::json!({"data": "value"})),
            error: None,
        };

        let json1 = serde_json::to_string(&response1).unwrap();
        assert!(!json1.contains("\"error\":"));

        // Response with only error (no result)
        let response2 = JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 2,
            result: None,
            error: Some(JsonRpcError {
                code: -32600,
                message: "Bad request".to_string(),
                data: None,
            }),
        };

        let json2 = serde_json::to_string(&response2).unwrap();
        assert!(!json2.contains("\"result\":"));
    }

    #[test]
    fn test_error_without_data() {
        let error = JsonRpcError {
            code: -32601,
            message: "Method not found".to_string(),
            data: None,
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(!json.contains("\"data\":"));
        assert!(json.contains("\"code\":-32601"));
    }

    #[test]
    fn test_client_debug() {
        let client = UnixJsonRpcClient::new("/tmp/debug.sock");
        let debug_str = format!("{:?}", client);
        assert!(debug_str.contains("UnixJsonRpcClient"));
    }
}
