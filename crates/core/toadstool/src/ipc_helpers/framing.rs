// SPDX-License-Identifier: AGPL-3.0-or-later
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
        .map_err(|e| ToadStoolError::integration(format!("Failed to serialize JSON-RPC: {e}")))?;

    // Write with newline delimiter (zero-copy: avoid format! allocation)
    let mut data = String::with_capacity(json_str.len() + 1);
    data.push_str(&json_str);
    data.push('\n');

    stream
        .write_all(data.as_bytes())
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to write to stream: {e}")))?;

    stream
        .flush()
        .await
        .map_err(|e| ToadStoolError::integration(format!("Failed to flush stream: {e}")))?;

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
        .map_err(|e| ToadStoolError::integration(format!("Failed to read from stream: {e}")))?;

    if line.is_empty() {
        return Err(ToadStoolError::integration("Connection closed by peer"));
    }

    serde_json::from_slice(line.as_bytes())
        .map_err(|e| ToadStoolError::integration(format!("Failed to parse JSON-RPC: {e}")))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use tokio::net::UnixStream;

    /// Create a connected pair of Unix sockets for testing.
    async fn socket_pair() -> (UnixStream, UnixStream) {
        // std::os::unix::net::UnixStream::pair gives us a connected pair.
        let (a, b) = std::os::unix::net::UnixStream::pair().unwrap();
        a.set_nonblocking(true).unwrap();
        b.set_nonblocking(true).unwrap();
        (
            UnixStream::from_std(a).unwrap(),
            UnixStream::from_std(b).unwrap(),
        )
    }

    #[tokio::test]
    async fn test_write_and_read_roundtrip() {
        let (mut writer, mut reader) = socket_pair().await;
        let msg = json!({"jsonrpc": "2.0", "method": "test", "id": 1});
        write_json_rpc(&mut writer, &msg).await.unwrap();

        let received = read_json_rpc(&mut reader).await.unwrap();
        assert_eq!(received["method"], "test");
        assert_eq!(received["id"], 1);
    }

    #[tokio::test]
    async fn test_write_complex_message() {
        let (mut writer, mut reader) = socket_pair().await;
        let msg = json!({
            "jsonrpc": "2.0",
            "method": "compute.execute",
            "params": {"workload": "test", "data": [1, 2, 3]},
            "id": "abc-123"
        });
        write_json_rpc(&mut writer, &msg).await.unwrap();

        let received = read_json_rpc(&mut reader).await.unwrap();
        assert_eq!(received["method"], "compute.execute");
        assert_eq!(received["params"]["workload"], "test");
    }

    #[tokio::test]
    async fn test_write_notification_no_id() {
        let (mut writer, mut reader) = socket_pair().await;
        let msg = json!({"jsonrpc": "2.0", "method": "notify", "params": {}});
        write_json_rpc(&mut writer, &msg).await.unwrap();

        let received = read_json_rpc(&mut reader).await.unwrap();
        assert_eq!(received["method"], "notify");
        assert!(received.get("id").is_none());
    }

    #[tokio::test]
    async fn test_multiple_messages_sequential() {
        let (mut writer, mut reader) = socket_pair().await;

        // Write 3 messages and then immediately read them — keep writer alive
        for i in 0u32..3 {
            let msg = json!({"jsonrpc": "2.0", "method": "seq", "id": i});
            write_json_rpc(&mut writer, &msg).await.unwrap();
            let received = read_json_rpc(&mut reader).await.unwrap();
            assert_eq!(received["id"], i, "message order mismatch at i={i}");
        }
    }
}
