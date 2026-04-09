// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix domain socket listener and per-connection handling for Pure JSON-RPC.
//!
//! Supports two modes per `BTSP_PROTOCOL_STANDARD.md`:
//! - **Development** (no `FAMILY_ID`): NDJSON / HTTP hybrid
//! - **Production** (`FAMILY_ID` set): BTSP handshake → length-prefixed frames

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
/// When `FAMILY_ID` is set (production), incoming connections are expected to
/// perform a BTSP handshake before any JSON-RPC traffic. After handshake,
/// communication uses length-prefixed framing per `BTSP_PROTOCOL_STANDARD.md`.
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
                "Failed to create socket directory {}: {e}",
                parent.display()
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

    let env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
    let btsp_required = toadstool_common::primal_sockets::is_btsp_required(&env);

    if btsp_required {
        info!(
            "✅ BTSP mode: length-prefixed framing on: {:?}",
            socket_path
        );
    } else {
        info!(
            "✅ Pure JSON-RPC 2.0 server (NDJSON) listening on: {:?}",
            socket_path
        );
    }

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let handler = Arc::clone(&handler);
                let btsp = btsp_required;
                tokio::spawn(async move {
                    let result = if btsp {
                        handle_btsp_connection(handler, stream).await
                    } else {
                        handle_unix_connection(handler, stream).await
                    };
                    if let Err(e) = result {
                        error!("Unix connection error: {}", e);
                    }
                });
            }
            Err(e) => error!("Accept error: {}", e),
        }
    }
}

/// Handle a single Unix connection.
///
/// Supports both HTTP (single request-response) and persistent NDJSON sessions
/// per `PRIMAL_IPC_PROTOCOL.md`: multiple newline-delimited JSON-RPC requests
/// on a single connection.
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

    if first_line.starts_with("POST")
        || first_line.starts_with("GET")
        || first_line.starts_with("HTTP")
    {
        let (_headers, body) = read_http_request_continuation_unix(&mut reader).await?;
        let response_body = process_request(&handler, &body).await?;
        write_http_response_unix(&mut writer, &response_body).await?;
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

/// Handle a BTSP-authenticated connection (production mode).
///
/// 1. Perform BTSP handshake (verify family membership)
/// 2. Process length-prefixed JSON-RPC frames until the connection closes
///
/// Per `BTSP_PROTOCOL_STANDARD.md`: no JSON-RPC methods are exposed before
/// the handshake completes. If verification fails, the connection is dropped.
#[cfg(feature = "btsp")]
async fn handle_btsp_connection(
    handler: Arc<JsonRpcHandler>,
    stream: UnixStream,
) -> ServerResult<()> {
    use toadstool_common::btsp;

    // Read family seed from environment (FAMILY_SEED or .family.seed file)
    let family_seed = resolve_family_seed()?;

    let mut stream = stream;

    match btsp::BtspServer::accept_handshake(&mut stream, &family_seed).await {
        Ok(session) => {
            info!(
                "🔒 BTSP handshake complete: cipher={}, session_id={:02x?}",
                session.cipher.as_str(),
                &session.session_id[..4]
            );
        }
        Err(e) => {
            warn!("🔒 BTSP handshake rejected: {e}");
            let _ = btsp::BtspServer::send_handshake_error(&mut stream).await;
            return Err(ServerError::Network(format!("BTSP handshake failed: {e}")));
        }
    }

    // Post-handshake: length-prefixed JSON-RPC frames
    loop {
        match btsp::framing::read_frame(&mut stream).await {
            Ok(frame) => {
                let response_body = process_request(&handler, &frame).await?;
                if let Err(e) = btsp::framing::write_frame(&mut stream, &response_body).await {
                    warn!("BTSP write error: {e}");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                warn!("BTSP read error: {e}");
                break;
            }
        }
    }

    Ok(())
}

/// Production path when the `btsp` crate feature is **disabled**.
///
/// [`is_btsp_required`](toadstool_common::primal_sockets::is_btsp_required) is true (e.g. `FAMILY_ID`
/// set), so clients expect a BTSP handshake and length-prefixed frames. Without the `btsp`
/// feature we cannot perform that handshake; we log at target `btsp`, shut down the socket, and
/// return — consistent with [`crate::tarpc_server`]. For local NDJSON development, unset family ID
/// env vars so the server uses [`handle_unix_connection`] only.
#[cfg(not(feature = "btsp"))]
async fn handle_btsp_connection(
    _handler: Arc<JsonRpcHandler>,
    mut stream: UnixStream,
) -> ServerResult<()> {
    warn!(
        target: "btsp",
        "BTSP required (FAMILY_ID set) but this binary was built without the `btsp` Cargo feature — closing connection; rebuild with `btsp` enabled or unset family ID env vars for development NDJSON"
    );
    if let Err(e) = stream.shutdown().await {
        warn!(target: "btsp", "shutdown after BTSP-disabled close: {e}");
    }
    Ok(())
}

/// Resolve the family seed for BTSP handshake verification.
///
/// Reads from `FAMILY_SEED` env var, or falls back to reading
/// `.family.seed` from the biomeOS config directory.
fn resolve_family_seed() -> ServerResult<Vec<u8>> {
    if let Ok(seed) = std::env::var("FAMILY_SEED") {
        return Ok(seed.into_bytes());
    }

    // Try .family.seed file in biomeOS config
    let biomeos_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let seed_path = biomeos_dir.join(".family.seed");
    if seed_path.exists() {
        return std::fs::read(&seed_path)
            .map_err(|e| ServerError::Configuration(format!("Failed to read family seed: {e}")));
    }

    Err(ServerError::Configuration(
        "BTSP requires FAMILY_SEED env var or .family.seed file in biomeOS directory".to_string(),
    ))
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
