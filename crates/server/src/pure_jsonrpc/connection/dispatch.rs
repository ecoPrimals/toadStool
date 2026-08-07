// SPDX-License-Identifier: AGPL-3.0-or-later
//! Transport-agnostic connection dispatch (G66).
//!
//! Generic functions for NDJSON, HTTP keep-alive, riboCipher routing, and
//! rejection — parameterised over `AsyncBufRead` / `AsyncWrite` so both
//! Unix and TCP connection handlers can delegate here without duplicating
//! protocol logic.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::{debug, warn};

use super::process_request;
use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::JsonRpcHandler;
use crate::pure_jsonrpc::handler::ConnectionTrustHints;

/// NDJSON persistent session: one JSON-RPC request per line, responses
/// delimited by newlines.
///
/// When `idle_timeout` is `Some(d)`, each line-read is bounded by `d`.
/// Pass `None` for Unix sockets (no idle timeout).
pub(super) async fn handle_ndjson<R, W>(
    handler: &JsonRpcHandler,
    reader: &mut R,
    writer: &mut W,
    first_line: String,
    trust: ConnectionTrustHints,
    idle_timeout: Option<Duration>,
) -> ServerResult<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut line = first_line;
    loop {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let response_body = process_request(handler, trimmed.as_bytes(), trust).await?;
            writer
                .write_all(&response_body)
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            writer
                .write_all(b"\n")
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            writer
                .flush()
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
        }

        line.clear();
        let n = read_line_with_timeout(reader, &mut line, idle_timeout).await?;
        if n == 0 {
            break;
        }
    }
    Ok(())
}

/// HTTP/1.1 keep-alive loop: process multiple HTTP requests on a single
/// connection.
///
/// Closes when the client sends `Connection: close`, EOF, or (for TCP)
/// idle timeout.
pub(super) async fn handle_http_keepalive<R, W>(
    handler: &JsonRpcHandler,
    reader: &mut R,
    writer: &mut W,
    first_request_line: String,
    trust: ConnectionTrustHints,
    idle_timeout: Option<Duration>,
) -> ServerResult<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    let mut request_line = first_request_line;
    loop {
        let (headers, body) = read_http_request_continuation(reader).await?;
        let response_body = process_request(handler, &body, trust).await?;

        let client_wants_close = headers
            .get("connection")
            .is_some_and(|v| v.eq_ignore_ascii_case("close"));

        write_http_response(writer, &response_body, client_wants_close).await?;

        if client_wants_close {
            break;
        }

        request_line.clear();
        let n = read_line_with_timeout(reader, &mut request_line, idle_timeout).await?;
        if n == 0 {
            break;
        }
        let trimmed = request_line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if !trimmed.starts_with("POST")
            && !trimmed.starts_with("GET")
            && !trimmed.starts_with("HTTP")
        {
            break;
        }
    }
    Ok(())
}

/// Dispatch a riboCipher clear-signalled connection by protocol type byte.
///
/// Handles PROBE, NDJSON, and HTTP protocol types. The caller has already
/// consumed the riboCipher prefix and protocol-type byte; the stream is
/// positioned at the start of the payload.
pub(super) async fn handle_ribocipher_clear<R, W>(
    handler: Arc<JsonRpcHandler>,
    reader: &mut R,
    writer: &mut W,
    protocol_type: u8,
    trust: ConnectionTrustHints,
    idle_timeout: Option<Duration>,
) -> ServerResult<()>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    use super::ribocipher::protocol_type as pt;

    match protocol_type {
        pt::PROBE => {
            let response =
                serde_json::json!({"jsonrpc":"2.0","result":{"status":"alive"},"id":null});
            let mut buf = serde_json::to_vec(&response).unwrap_or_default();
            buf.push(b'\n');
            let _ = writer.write_all(&buf).await;
            let _ = writer.flush().await;
            Ok(())
        }
        pt::NDJSON_JSONRPC => {
            handle_ndjson(&handler, reader, writer, String::new(), trust, idle_timeout).await
        }
        pt::HTTP => {
            let mut first_line = String::new();
            let n = reader
                .read_line(&mut first_line)
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            if n == 0 {
                return Ok(());
            }
            handle_http_keepalive(&handler, reader, writer, first_line, trust, idle_timeout).await
        }
        unknown => {
            warn!(
                protocol_type = format_args!("0x{:02X}", unknown),
                "riboCipher: unsupported protocol type — closing"
            );
            Ok(())
        }
    }
}

/// Read HTTP headers and body after the request line has been consumed.
pub(super) async fn read_http_request_continuation<R>(
    reader: &mut R,
) -> ServerResult<(HashMap<String, String>, Vec<u8>)>
where
    R: AsyncBufRead + Unpin,
{
    let mut headers = HashMap::new();
    let mut line = String::new();

    loop {
        line.clear();
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        if n == 0 || line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = parse_http_header_field(&line) {
            headers.insert(name, value);
        }
    }

    let content_length: usize = headers
        .get("content-length")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let mut body = vec![0u8; content_length];
    reader
        .read_exact(&mut body)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;

    Ok((headers, body))
}

/// Write an HTTP/1.1 JSON response with keep-alive or close header.
pub(super) async fn write_http_response<W>(
    writer: &mut W,
    body: &[u8],
    closing: bool,
) -> ServerResult<()>
where
    W: AsyncWrite + Unpin,
{
    let header = format_http_response_header(body.len(), closing);
    writer
        .write_all(header.as_bytes())
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    writer
        .write_all(body)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    writer
        .flush()
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    Ok(())
}

/// Send the standard unsignalled-connection rejection JSON.
pub(super) async fn write_reject_response<W>(writer: &mut W) -> ServerResult<()>
where
    W: AsyncWrite + Unpin,
{
    let reject = unsignalled_connection_reject_json();
    let mut buf = serde_json::to_vec(&reject).unwrap_or_default();
    buf.push(b'\n');
    let _ = writer.write_all(&buf).await;
    let _ = writer.flush().await;
    Ok(())
}

// ── Transport-agnostic helpers (moved from unix.rs for cross-arch) ──

/// Standard unsignalled-connection rejection JSON.
pub(crate) fn unsignalled_connection_reject_json() -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": -32600,
            "message": "Connection rejected: missing riboCipher signal. Prepend [0xEC, 0x01]."
        },
        "id": null
    })
}

/// Parse a single `Name: value` HTTP header line into normalized key/value pair.
pub(crate) fn parse_http_header_field(line: &str) -> Option<(String, String)> {
    let (name, value) = line.split_once(':')?;
    Some((name.trim().to_lowercase(), value.trim().to_string()))
}

/// Format an HTTP/1.1 JSON response header block.
pub(crate) fn format_http_response_header(body_len: usize, closing: bool) -> String {
    let conn_header = if closing {
        "Connection: close"
    } else {
        "Connection: keep-alive"
    };
    format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {body_len}\r\n\
         {conn_header}\r\n\
         \r\n"
    )
}

/// Read a line with an optional idle timeout.
///
/// Returns `Ok(0)` on EOF or timeout (caller should close the connection).
async fn read_line_with_timeout<R>(
    reader: &mut R,
    buf: &mut String,
    idle_timeout: Option<Duration>,
) -> ServerResult<usize>
where
    R: AsyncBufRead + Unpin,
{
    match idle_timeout {
        Some(timeout) => match tokio::time::timeout(timeout, reader.read_line(buf)).await {
            Ok(Ok(n)) => Ok(n),
            Ok(Err(e)) => Err(ServerError::Network(e.to_string())),
            Err(_) => {
                debug!("idle timeout — closing connection");
                Ok(0)
            }
        },
        None => reader
            .read_line(buf)
            .await
            .map_err(|e| ServerError::Network(e.to_string())),
    }
}
