// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC request/response framing over the IPC stream.

use super::DisplayClient;
use crate::DisplayError;
use crate::ipc::types::{JsonRpcRequest, JsonRpcResponse};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

impl DisplayClient {
    /// Send a request and receive response (polymorphic!)
    ///
    /// **Works with both Unix and TCP streams transparently**
    pub(super) async fn send_request(
        &mut self,
        request: JsonRpcRequest,
    ) -> crate::Result<JsonRpcResponse> {
        // Serialize request
        let request_json = serde_json::to_string(&request)
            .map_err(|e| DisplayError::IpcError(format!("Serialization error: {e}")))?;

        // Send request
        self.stream
            .write_all(request_json.as_bytes())
            .await
            .map_err(|e| DisplayError::IpcError(format!("Write error: {e}")))?;
        self.stream
            .write_all(b"\n")
            .await
            .map_err(|e| DisplayError::IpcError(format!("Write error: {e}")))?;

        // Read response (using BufReader directly on the stream)
        let mut reader = BufReader::new(&mut self.stream);
        let mut line = String::new();

        reader
            .read_line(&mut line)
            .await
            .map_err(|e| DisplayError::IpcError(format!("Read error: {e}")))?;

        // Parse response
        serde_json::from_str(&line).map_err(|e| DisplayError::IpcError(format!("Parse error: {e}")))
    }
}
