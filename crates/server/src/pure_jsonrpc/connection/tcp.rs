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

/// Handle a single TCP connection.
///
/// Supports both HTTP (single request-response) and persistent NDJSON sessions
/// per `PRIMAL_IPC_PROTOCOL.md`.
pub(crate) async fn handle_tcp_connection(
    handler: Arc<JsonRpcHandler>,
    stream: TcpStream,
) -> ServerResult<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;

    if first_line.starts_with("POST")
        || first_line.starts_with("GET")
        || first_line.starts_with("HTTP")
    {
        let (_headers, body) = read_http_request_continuation_tcp(&mut reader).await?;
        let response_body = process_request(&handler, &body).await?;
        write_http_response_tcp(&mut writer, &response_body).await?;
        return Ok(());
    }

    // NDJSON session: process first line, then loop for subsequent lines
    let mut line = first_line;
    loop {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }

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
) -> ServerResult<()> {
    let header = format!(
        "HTTP/1.1 200 OK\r\n\
         Content-Type: application/json\r\n\
         Content-Length: {}\r\n\
         Connection: close\r\n\
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
