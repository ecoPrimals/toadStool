// SPDX-License-Identifier: AGPL-3.0-only

use serde::Serialize;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

pub async fn make_jsonrpc_request<T: Serialize + Sync>(
    socket_path: &str,
    method: &str,
    params: &T,
) -> ToadStoolResult<serde_json::Value> {
    // Serialize before await so params is not held across await (Send requirement)
    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": Uuid::new_v4().to_string()
    });

    let request_str = serde_json::to_string(&request)
        .map_err(|e| ToadStoolError::security(format!("Failed to serialize request: {e}")))?;

    let mut stream = UnixStream::connect(socket_path).await.map_err(|e| {
        ToadStoolError::security(format!("Failed to connect to PKI security service: {e}"))
    })?;

    stream
        .write_all(request_str.as_bytes())
        .await
        .map_err(|e| ToadStoolError::security(format!("Failed to send request: {e}")))?;

    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .map_err(|e| ToadStoolError::security(format!("Failed to read response: {e}")))?;

    let response_json: serde_json::Value = serde_json::from_slice(&response)
        .map_err(|e| ToadStoolError::security(format!("Failed to parse response: {e}")))?;

    if let Some(error) = response_json.get("error") {
        return Err(ToadStoolError::security(format!(
            "PKI security error: {error}"
        )));
    }

    response_json
        .get("result")
        .cloned()
        .ok_or_else(|| ToadStoolError::security("No result in response"))
}
