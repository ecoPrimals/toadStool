// SPDX-License-Identifier: AGPL-3.0-or-later
//! TCP listener and per-connection handling for Pure JSON-RPC.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::JsonRpcHandler;
use crate::pure_jsonrpc::handler::ConnectionTrustHints;
use toadstool_common::interned_strings::socket_env;

use super::process_request;

pub(crate) fn tcp_idle_timeout() -> Duration {
    let secs = std::env::var(socket_env::TOADSTOOL_TCP_IDLE_TIMEOUT_SECS)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(toadstool_config::defaults::network::TCP_IDLE_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// Serve JSON-RPC on a TCP listener (isomorphic fallback).
///
/// # Errors
///
/// Returns [`ServerError`] if getting local address fails.
pub async fn serve_tcp(handler: Arc<JsonRpcHandler>, listener: TcpListener) -> ServerResult<()> {
    let local_addr = listener
        .local_addr()
        .map_err(|e| ServerError::Network(e.to_string()))?;
    info!(
        "✅ Pure JSON-RPC 2.0 server listening on TCP: {}",
        local_addr
    );

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let _ = stream.set_nodelay(true);
                let handler = Arc::clone(&handler);
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp_connection(handler, stream).await {
                        debug!("TCP connection from {addr} ended: {e}");
                    }
                });
            }
            Err(e) => error!("TCP accept error: {}", e),
        }
    }
}

/// Handle a single TCP connection with persistent keep-alive.
///
/// Detects riboCipher transport signal before protocol dispatch per
/// `RIBOCIPHER_TRANSPORT_SIGNAL_STANDARD.md`. Falls back to legacy
/// peek-and-guess with WARN for unsignalled connections (Wave 111–112).
pub(crate) async fn handle_tcp_connection(
    handler: Arc<JsonRpcHandler>,
    mut stream: TcpStream,
) -> ServerResult<()> {
    use super::ribocipher;

    let idle_timeout = tcp_idle_timeout();

    // Read first byte for riboCipher detection
    let mut first = [0u8; 1];
    let n = match tokio::time::timeout(
        idle_timeout,
        tokio::io::AsyncReadExt::read(&mut stream, &mut first),
    )
    .await
    {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return Err(ServerError::Network(e.to_string())),
        Err(_) => {
            return Err(ServerError::Network(
                "TCP idle timeout on initial read".into(),
            ));
        }
    };
    if n == 0 {
        return Ok(());
    }

    // riboCipher detection
    match first[0] {
        ribocipher::CLEAR => {
            let mut pt = [0u8; 1];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut pt)
                .await
                .map_err(|e| {
                    ServerError::Network(format!("riboCipher: failed to read protocol type: {e}"))
                })?;
            info!(
                protocol_type = format_args!("0x{:02X}", pt[0]),
                "riboCipher clear signal on TCP"
            );
            return handle_ribocipher_clear_tcp(handler, stream, pt[0]).await;
        }
        ribocipher::MITO => {
            // MitoBeacon (Wave 114): read 4-byte HMAC tag, then protocol type.
            let mut hmac_tag = [0u8; 4];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut hmac_tag)
                .await
                .map_err(|e| {
                    ServerError::Network(format!("riboCipher mito: failed to read HMAC tag: {e}"))
                })?;
            let mut pt = [0u8; 1];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut pt)
                .await
                .map_err(|e| {
                    ServerError::Network(format!(
                        "riboCipher mito: failed to read protocol type: {e}"
                    ))
                })?;
            info!(
                protocol_type = format_args!("0x{:02X}", pt[0]),
                hmac = format_args!(
                    "{:02x}{:02x}{:02x}{:02x}",
                    hmac_tag[0], hmac_tag[1], hmac_tag[2], hmac_tag[3]
                ),
                "riboCipher mito-beacon signal accepted on TCP"
            );
            return handle_ribocipher_clear_tcp(handler, stream, pt[0]).await;
        }
        ribocipher::NUCLEAR => {
            warn!("riboCipher nuclear tier not yet supported on TCP — rejecting");
            let reject = serde_json::json!({
                "jsonrpc": "2.0",
                "error": {"code": -32600, "message": "riboCipher nuclear tier not yet supported"},
                "id": null
            });
            let mut buf = serde_json::to_vec(&reject).unwrap_or_default();
            buf.push(b'\n');
            let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, &buf).await;
            let _ = tokio::io::AsyncWriteExt::flush(&mut stream).await;
            return Ok(());
        }
        other => {
            debug!(
                first_byte = format_args!("0x{:02X}", other),
                "riboCipher TCP: unhandled signal byte, falling through to unsignalled rejection"
            );
        }
    }

    // Wave 113: REJECT unsignalled connections
    error!(
        first_byte = format_args!("0x{:02X}", first[0]),
        "REJECTED: unsignalled TCP connection (no riboCipher prefix). \
         Clients MUST prepend [0xEC, 0x01]."
    );
    let (_, mut writer) = stream.into_split();
    let reject = serde_json::json!({
        "jsonrpc": "2.0",
        "error": {
            "code": -32600,
            "message": "Connection rejected: missing riboCipher signal. Prepend [0xEC, 0x01]."
        },
        "id": null
    });
    let mut buf = serde_json::to_vec(&reject).unwrap_or_default();
    buf.push(b'\n');
    let _ = writer.write_all(&buf).await;
    let _ = writer.flush().await;
    Ok(())
}

/// Handle a riboCipher clear-signalled TCP connection, routed by protocol type.
async fn handle_ribocipher_clear_tcp(
    handler: Arc<JsonRpcHandler>,
    stream: TcpStream,
    protocol_type: u8,
) -> ServerResult<()> {
    use super::ribocipher::protocol_type as pt;

    match protocol_type {
        pt::PROBE => {
            let (_, mut writer) = stream.into_split();
            let response =
                serde_json::json!({"jsonrpc":"2.0","result":{"status":"alive"},"id":null});
            let mut buf = serde_json::to_vec(&response).unwrap_or_default();
            buf.push(b'\n');
            let _ = writer.write_all(&buf).await;
            let _ = writer.flush().await;
            Ok(())
        }
        pt::NDJSON_JSONRPC => {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            handle_ndjson_tcp(handler, &mut reader, &mut writer, String::new()).await
        }
        pt::HTTP => {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            let mut first_line = String::new();
            let n = reader
                .read_line(&mut first_line)
                .await
                .map_err(|e| ServerError::Network(e.to_string()))?;
            if n == 0 {
                return Ok(());
            }
            handle_http_keepalive_tcp(handler, &mut reader, &mut writer, first_line).await
        }
        unknown => {
            info!(
                protocol_type = format_args!("0x{:02X}", unknown),
                "riboCipher: unsupported protocol type on TCP — closing"
            );
            Ok(())
        }
    }
}

/// HTTP/1.1 keep-alive loop for TCP connections.
async fn handle_http_keepalive_tcp(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    first_request_line: String,
) -> ServerResult<()> {
    let idle_timeout = tcp_idle_timeout();
    let mut request_line = first_request_line;
    loop {
        let (headers, body) = read_http_request_continuation_tcp(reader).await?;
        let response_body = process_request(&handler, &body, ConnectionTrustHints::TCP).await?;

        let client_wants_close = headers
            .get("connection")
            .is_some_and(|v| v.eq_ignore_ascii_case("close"));

        write_http_response_tcp(writer, &response_body, client_wants_close).await?;

        if client_wants_close {
            break;
        }

        request_line.clear();
        let n = match tokio::time::timeout(idle_timeout, reader.read_line(&mut request_line)).await
        {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(ServerError::Network(e.to_string())),
            Err(_) => {
                debug!("HTTP keep-alive idle timeout — closing connection");
                break;
            }
        };
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

/// NDJSON persistent session for TCP connections.
async fn handle_ndjson_tcp(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    first_line: String,
) -> ServerResult<()> {
    let idle_timeout = tcp_idle_timeout();
    let mut line = first_line;
    loop {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let response_body =
                process_request(&handler, trimmed.as_bytes(), ConnectionTrustHints::TCP).await?;
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
        let n = match tokio::time::timeout(idle_timeout, reader.read_line(&mut line)).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(ServerError::Network(e.to_string())),
            Err(_) => {
                debug!("NDJSON idle timeout — closing connection");
                break;
            }
        };
        if n == 0 {
            break;
        }
    }
    Ok(())
}

async fn read_http_request_continuation_tcp(
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
) -> ServerResult<(HashMap<String, String>, Vec<u8>)> {
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
        if let Some((name, value)) = line.split_once(':') {
            headers.insert(name.trim().to_lowercase(), value.trim().to_string());
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

async fn write_http_response_tcp(
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    body: &[u8],
    closing: bool,
) -> ServerResult<()> {
    let conn_header = if closing {
        "Connection: close"
    } else {
        "Connection: keep-alive"
    };
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         {conn_header}\r\n\
         \r\n",
        body.len()
    );
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
