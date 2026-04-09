// SPDX-License-Identifier: AGPL-3.0-or-later

use std::path::Path;

use super::types::RemoteDispatchError;

/// Dispatches a compute job to a remote toadStool gate via Unix socket or TCP.
///
/// Remote gates register their endpoint (socket path or host:port) via
/// `gate.update`. When the router selects a remote gate, the dispatcher
/// forwards the JSON-RPC `compute.submit` request.
pub struct RemoteDispatcher;

impl RemoteDispatcher {
    /// Forward a compute job to a remote gate.
    ///
    /// Attempts Unix socket first (if endpoint looks like a path), then TCP.
    ///
    /// # Errors
    ///
    /// Returns [`RemoteDispatchError`] if the remote gate is unreachable,
    /// the JSON-RPC call fails, or the response cannot be parsed.
    pub async fn forward(
        endpoint: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RemoteDispatchError> {
        let path = Path::new(endpoint);
        if path.exists()
            && (endpoint.contains('/')
                || path
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("sock")))
        {
            return Self::forward_unix(path, method, params).await;
        }
        Self::forward_tcp(endpoint, method, params).await
    }

    async fn forward_unix(
        socket_path: &Path,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RemoteDispatchError> {
        let client = toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path);
        client
            .call(method, params)
            .await
            .map_err(|e| RemoteDispatchError::Transport(e.to_string()))
    }

    async fn forward_tcp(
        endpoint: &str,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, RemoteDispatchError> {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        // Construct JSON-RPC request
        let request = serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        });
        let body = serde_json::to_vec(&request)
            .map_err(|e| RemoteDispatchError::Serialize(e.to_string()))?;

        // TCP connection + send + receive
        let mut stream = tokio::net::TcpStream::connect(endpoint)
            .await
            .map_err(|e| RemoteDispatchError::Transport(format!("TCP connect: {e}")))?;

        stream
            .write_all(&body)
            .await
            .map_err(|e| RemoteDispatchError::Transport(format!("TCP write: {e}")))?;
        stream
            .write_all(b"\n")
            .await
            .map_err(|_| RemoteDispatchError::Transport("newline".into()))?;
        stream.shutdown().await.ok();

        let mut response_buf = Vec::new();
        stream
            .read_to_end(&mut response_buf)
            .await
            .map_err(|e| RemoteDispatchError::Transport(format!("TCP read: {e}")))?;

        let response: serde_json::Value = serde_json::from_slice(&response_buf)
            .map_err(|e| RemoteDispatchError::Serialize(format!("response parse: {e}")))?;

        if let Some(result) = response.get("result") {
            Ok(result.clone())
        } else if let Some(error) = response.get("error") {
            Err(RemoteDispatchError::Remote(error.to_string()))
        } else {
            Err(RemoteDispatchError::Remote("unexpected response".into()))
        }
    }
}
