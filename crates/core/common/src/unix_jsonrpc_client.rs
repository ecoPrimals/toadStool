// SPDX-License-Identifier: AGPL-3.0-or-later
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
//! Complements `pure_jsonrpc::JsonRpcHandler` from toadstool-server.
//! Same protocol, different transport (unix socket vs HTTP).

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::borrow::Cow;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use crate::{ToadStoolError, ToadStoolResult};

/// JSON-RPC 2.0 request (zero-copy: both `jsonrpc` and `method` borrow when possible)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcRequest<'a> {
    jsonrpc: Cow<'static, str>,
    id: u64,
    method: Cow<'a, str>,
    params: Value,
}

/// JSON-RPC 2.0 response (deserialized from network bytes via `from_slice`)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcResponse<'a> {
    #[serde(borrow)]
    jsonrpc: Cow<'a, str>,
    id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError<'a>>,
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
struct JsonRpcError<'a> {
    code: i32,
    #[serde(borrow)]
    message: Cow<'a, str>,
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
/// use toadstool_common::primal_sockets::get_socket_path_for_capability;
///
/// let socket_path = get_socket_path_for_capability("crypto");
/// let client = UnixJsonRpcClient::new(socket_path);
///
/// let params = serde_json::json!({
///     "data": "hello",
///     "algorithm": "aes-256-gcm"
/// });
///
/// let result = client.call("crypto.encrypt", params).await?;
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
                "Failed to connect to {}: {e}",
                self.socket_path.display()
            ))
        })?;

        // Generate request ID
        let id = self
            .next_id
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        // Build JSON-RPC 2.0 request (zero-copy: borrows both version and method)
        let request = JsonRpcRequest {
            jsonrpc: Cow::Borrowed(crate::constants::jsonrpc::VERSION),
            id,
            method: Cow::Borrowed(method),
            params,
        };

        // Serialize request
        let request_json = serde_json::to_string(&request)
            .map_err(|e| ToadStoolError::network(format!("Failed to serialize request: {e}")))?;

        // Split stream for concurrent read/write
        let (reader, mut writer) = stream.into_split();

        // Send request (newline-delimited JSON)
        writer
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send request: {e}")))?;
        writer
            .write_all(b"\n")
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to send newline: {e}")))?;
        writer
            .flush()
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to flush: {e}")))?;

        // Read response (newline-delimited JSON)
        let mut reader = BufReader::new(reader);
        let mut response_line = String::new();
        reader
            .read_line(&mut response_line)
            .await
            .map_err(|e| ToadStoolError::network(format!("Failed to read response: {e}")))?;

        // Parse JSON-RPC response
        let response: JsonRpcResponse = serde_json::from_slice(response_line.as_bytes())
            .map_err(|e| ToadStoolError::network(format!("Invalid JSON-RPC response: {e}")))?;

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
    ///     .call_typed("crypto.encrypt", params)
    ///     .await?;
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`ToadStoolError`] if the JSON-RPC call fails or response deserialization fails.
    pub async fn call_typed<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Value,
    ) -> ToadStoolResult<T> {
        let result = self.call(method, params).await?;
        serde_json::from_value(result)
            .map_err(|e| ToadStoolError::network(format!("Failed to deserialize response: {e}")))
    }

    /// Check if socket exists and is accessible
    ///
    /// **Use Case**: Graceful degradation - check before calling
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.socket_path.exists()
    }

    /// Get socket path (for diagnostics)
    #[must_use]
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }
}

#[cfg(test)]
#[path = "unix_jsonrpc_client_tests.rs"]
mod tests;
