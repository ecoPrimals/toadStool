// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix domain socket listener and per-connection handling for Pure JSON-RPC.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::JsonRpcHandler;

use super::process_request;

/// Serve JSON-RPC on a Unix socket.
///
/// Accepts connections, parses JSON-RPC requests (raw JSON or HTTP/JSON hybrid),
/// dispatches to the handler, and writes responses.
///
/// # Errors
///
/// Returns [`ServerError`] if directory creation, socket bind, or permission setting fails.
pub async fn serve_unix(handler: Arc<JsonRpcHandler>, socket_path: PathBuf) -> ServerResult<()> {
    info!(
        "Starting pure JSON-RPC 2.0 server on Unix socket: {:?}",
        socket_path
    );

    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            ServerError::Initialization(format!(
                "Failed to create socket directory {parent:?}: {e}"
            ))
        })?;
        info!("Ensured JSON-RPC socket directory exists: {:?}", parent);
    }

    if socket_path.exists() {
        warn!("Removing old JSON-RPC socket: {:?}", socket_path);
        tokio::fs::remove_file(&socket_path)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
    }

    let listener =
        UnixListener::bind(&socket_path).map_err(|e| ServerError::Network(e.to_string()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = tokio::fs::metadata(&socket_path)
            .await
            .map_err(|e| ServerError::Internal(e.to_string()))?
            .permissions();
        perms.set_mode(0o600);
        tokio::fs::set_permissions(&socket_path, perms)
            .await
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        info!("Set JSON-RPC socket permissions to 0600");
    }

    info!(
        "✅ Pure JSON-RPC 2.0 server listening on: {:?}",
        socket_path
    );

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let handler = Arc::clone(&handler);
                tokio::spawn(async move {
                    if let Err(e) = handle_unix_connection(handler, stream).await {
                        error!("Unix connection error: {}", e);
                    }
                });
            }
            Err(e) => error!("Accept error: {}", e),
        }
    }
}

pub(super) async fn handle_unix_connection(
    handler: Arc<JsonRpcHandler>,
    stream: UnixStream,
) -> ServerResult<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);

    let mut first_line = String::new();
    reader
        .read_line(&mut first_line)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;

    let (body, is_http): (Cow<'_, [u8]>, bool) = if first_line.starts_with("POST")
        || first_line.starts_with("GET")
        || first_line.starts_with("HTTP")
    {
        let (_headers, body) = read_http_request_continuation_unix(&mut reader).await?;
        (Cow::Owned(body), true)
    } else {
        (Cow::Borrowed(first_line.trim().as_bytes()), false)
    };

    let response_body = process_request(&handler, &body).await?;

    if is_http {
        write_http_response_unix(&mut writer, &response_body).await?;
    } else {
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

    Ok(())
}

async fn read_http_request_continuation_unix(
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
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

async fn write_http_response_unix(
    writer: &mut tokio::net::unix::OwnedWriteHalf,
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
