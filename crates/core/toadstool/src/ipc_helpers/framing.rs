//! JSON-RPC message framing/deframing over Unix streams
//!
//! Uses newline-delimited JSON (NDJSON) for simple framing.

use serde_json::Value;
use tokio::net::UnixStream;

use crate::{ToadStoolError, ToadStoolResult};

/// Write JSON-RPC message to stream
pub(crate) async fn write_json_rpc(
    stream: &mut UnixStream,
    message: &Value,
) -> ToadStoolResult<()> {
    use tokio::io::AsyncWriteExt;

    let json_str = serde_json::to_string(message)
        .map_err(|e| ToadStoolError::integration(format!("Failed to serialize JSON-RPC: {}", e)))?;

    // Write with newline delimiter (zero-copy: avoid format! allocation)
    let mut data = String::with_capacity(json_str.len() + 1);
    data.push_str(&json_str);
    data.push('\n');

    stream
        .write_all(data.as_bytes())
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to write to stream: {}", e)))?;

    stream
        .flush()
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to flush stream: {}", e)))?;

    Ok(())
}

/// Read JSON-RPC message from stream
pub(crate) async fn read_json_rpc(stream: &mut UnixStream) -> ToadStoolResult<Value> {
    use tokio::io::{AsyncBufReadExt, BufReader};

    let mut reader = BufReader::new(stream);
    let mut line = String::new();

    reader
        .read_line(&mut line)
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to read from stream: {}", e)))?;

    if line.is_empty() {
        return Err(ToadStoolError::integration("Connection closed by peer"));
    }

    serde_json::from_slice(line.as_bytes())
        .map_err(|e| ToadStoolError::integration(format!("Failed to parse JSON-RPC: {}", e)))
}
