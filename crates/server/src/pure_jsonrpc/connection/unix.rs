// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix domain socket listener and per-connection handling for Pure JSON-RPC.
//!
//! Supports two modes per `BTSP_PROTOCOL_STANDARD.md`:
//! - **Development** (no `FAMILY_ID`): NDJSON / HTTP hybrid
//! - **Production** (`FAMILY_ID` set): Auto-detects per-connection — BTSP
//!   binary clients get the full handshake + length-prefixed frames; plain-text
//!   clients (e.g. primalSpring `CompositionContext`) degrade gracefully to
//!   NDJSON / HTTP. Detection is instant via first-byte inspection.

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

/// Handle a single Unix connection with persistent keep-alive.
///
/// Supports both HTTP/1.1 keep-alive and persistent NDJSON sessions per
/// `PRIMAL_IPC_PROTOCOL.md`. Multi-step dispatch sequences (submit → status →
/// result) and health checks reuse the same connection without reconnecting.
pub(super) async fn handle_unix_connection(
    handler: Arc<JsonRpcHandler>,
    stream: UnixStream,
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
        return handle_http_keepalive_unix(handler, &mut reader, &mut writer, first_line).await;
    }

    handle_ndjson_unix(handler, &mut reader, &mut writer, first_line).await
}

/// HTTP/1.1 keep-alive loop: process multiple HTTP requests on a single connection.
///
/// Defaults to keep-alive per HTTP/1.1 spec. Closes only when the client sends
/// `Connection: close` or the connection reaches EOF.
async fn handle_http_keepalive_unix(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    first_request_line: String,
) -> ServerResult<()> {
    let mut request_line = first_request_line;
    loop {
        let (headers, body) = read_http_request_continuation_unix(reader).await?;
        let response_body = process_request(&handler, &body).await?;

        let client_wants_close = headers
            .get("connection")
            .is_some_and(|v| v.eq_ignore_ascii_case("close"));

        write_http_response_unix(writer, &response_body, client_wants_close).await?;

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

/// NDJSON persistent session: one JSON-RPC request per line, responses delimited by newlines.
async fn handle_ndjson_unix(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
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

/// Returns `true` when a byte indicates a plain-text protocol
/// (JSON-RPC, HTTP, NDJSON) rather than BTSP binary framing.
///
/// BTSP length-prefixed frames start with a 4-byte BE u32 length header.
/// For typical handshake payloads (< 2 KiB), the first byte is `0x00`.
/// All text protocols start with printable ASCII or whitespace (>= 0x09).
pub(super) const fn is_plaintext_protocol_byte(byte: u8) -> bool {
    byte >= 0x09
}

/// Wraps a stream, prepending a single already-consumed byte.
///
/// Used by the BTSP auto-detect path: we read one byte to distinguish
/// binary (BTSP) from text (JSON-RPC), then wrap the stream so
/// `BtspServer::accept_handshake` sees the complete frame including
/// that first byte.
struct PrependByte<S> {
    first: Option<u8>,
    inner: S,
}

impl<S: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for PrependByte<S> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if let Some(b) = self.first.take() {
            buf.put_slice(&[b]);
            return std::task::Poll::Ready(Ok(()));
        }
        std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<S: tokio::io::AsyncWrite + Unpin> tokio::io::AsyncWrite for PrependByte<S> {
    fn poll_write(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        std::pin::Pin::new(&mut self.inner).poll_write(cx, buf)
    }
    fn poll_flush(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_flush(cx)
    }
    fn poll_shutdown(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_shutdown(cx)
    }
}

/// Handle an incoming connection on a BTSP-enabled socket (production mode).
///
/// Auto-detects the wire protocol by peeking at the first byte:
/// - **Binary** (first byte < 0x09): BTSP length-prefixed framing. Performs a
///   full handshake (verify family membership) then processes length-prefixed
///   JSON-RPC frames.
/// - **Text** (first byte >= 0x09): Plain JSON-RPC / HTTP. Gracefully degrades
///   to `handle_unix_connection` so composition peers (e.g. primalSpring's
///   `CompositionContext`) that send newline-delimited JSON-RPC can reach
///   compute capabilities without implementing BTSP client framing.
///
/// Per `BTSP_PROTOCOL_STANDARD.md`: BTSP handshake is still enforced for
/// binary-framed clients. Plain-text fallback relies on Unix socket permissions
/// (0600) for access control.
#[cfg(feature = "btsp")]
pub(super) async fn handle_btsp_connection(
    handler: Arc<JsonRpcHandler>,
    mut stream: UnixStream,
) -> ServerResult<()> {
    use toadstool_common::btsp;

    let mut first = [0u8; 1];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut first)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    if n == 0 {
        return Ok(());
    }

    if is_plaintext_protocol_byte(first[0]) {
        info!(
            target: "btsp",
            "Plain-text connection on BTSP socket (0x{:02x}), \
             falling back to JSON-RPC for composition peer",
            first[0]
        );
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut first_line = String::from(first[0] as char);
        let n2 = reader
            .read_line(&mut first_line)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        if n2 == 0 && first_line.trim().is_empty() {
            return Ok(());
        }
        if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            return handle_http_keepalive_unix(handler, &mut reader, &mut writer, first_line).await;
        }
        return handle_ndjson_unix(handler, &mut reader, &mut writer, first_line).await;
    }

    let family_seed = resolve_family_seed()?;

    let mut stream = PrependByte {
        first: Some(first[0]),
        inner: stream,
    };

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
/// Auto-detects plain-text connections and handles them as JSON-RPC, so
/// composition peers can still reach compute capabilities. Binary-framed
/// connections (actual BTSP) are rejected because we lack the handshake
/// implementation.
#[cfg(not(feature = "btsp"))]
pub(super) async fn handle_btsp_connection(
    handler: Arc<JsonRpcHandler>,
    mut stream: UnixStream,
) -> ServerResult<()> {
    let mut first = [0u8; 1];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut first)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    if n == 0 {
        return Ok(());
    }

    if is_plaintext_protocol_byte(first[0]) {
        info!(
            target: "btsp",
            "Plain-text connection on BTSP socket (0x{:02x}) — \
             btsp feature disabled, serving as JSON-RPC",
            first[0]
        );
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        let mut first_line = String::from(first[0] as char);
        let n2 = reader
            .read_line(&mut first_line)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        if n2 == 0 && first_line.trim().is_empty() {
            return Ok(());
        }
        if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            return handle_http_keepalive_unix(handler, &mut reader, &mut writer, first_line).await;
        }
        return handle_ndjson_unix(handler, &mut reader, &mut writer, first_line).await;
    }

    warn!(
        target: "btsp",
        "BTSP binary connection (0x{:02x}) but this binary was built \
         without the `btsp` Cargo feature — closing connection; rebuild with \
         `btsp` enabled or unset family ID env vars for development NDJSON",
        first[0]
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
