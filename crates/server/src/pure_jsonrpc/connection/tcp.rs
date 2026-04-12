// SPDX-License-Identifier: AGPL-3.0-or-later
//! TCP listener and per-connection handling for Pure JSON-RPC.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tracing::{error, info};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::JsonRpcHandler;

use super::process_request;

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
            Ok((stream, _addr)) => {
                let handler = Arc::clone(&handler);
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp_connection(handler, stream).await {
                        error!("TCP connection error: {}", e);
                    }
                });
            }
            Err(e) => error!("TCP accept error: {}", e),
        }
    }
}

/// Handle a single TCP connection with persistent keep-alive.
///
/// Supports both HTTP/1.1 keep-alive and persistent NDJSON sessions per
/// `PRIMAL_IPC_PROTOCOL.md`. Multi-step dispatch sequences (submit → status →
/// result) and health checks reuse the same connection without reconnecting.
pub(crate) async fn handle_tcp_connection(
    handler: Arc<JsonRpcHandler>,
    stream: TcpStream,
) -> ServerResult<()> {
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

    if first_line.starts_with("POST")
        || first_line.starts_with("GET")
        || first_line.starts_with("HTTP")
    {
        return handle_http_keepalive_tcp(handler, &mut reader, &mut writer, first_line).await;
    }

    handle_ndjson_tcp(handler, &mut reader, &mut writer, first_line).await
}

/// HTTP/1.1 keep-alive loop for TCP connections.
async fn handle_http_keepalive_tcp(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: &mut tokio::net::tcp::OwnedWriteHalf,
    first_request_line: String,
) -> ServerResult<()> {
    let mut request_line = first_request_line;
    loop {
        let (headers, body) = read_http_request_continuation_tcp(reader).await?;
        let response_body = process_request(&handler, &body).await?;

        let client_wants_close = headers
            .get("connection")
            .is_some_and(|v| v.eq_ignore_ascii_case("close"));

        write_http_response_tcp(writer, &response_body, client_wants_close).await?;

        if client_wants_close {
            break;
        }

        request_line.clear();
        let n = reader
            .read_line(&mut request_line)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
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
    let mut line = first_line;
    loop {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let response_body = process_request(&handler, trimmed.as_bytes()).await?;
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
        let n = reader
            .read_line(&mut line)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
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
