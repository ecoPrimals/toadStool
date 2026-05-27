// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix domain socket listener and per-connection handling for Pure JSON-RPC.
//!
//! Supports two modes per `ecoPrimals/infra/wateringHole/BTSP_PROTOCOL_STANDARD.md`:
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
    let listener = Arc::new(prebind_unix_listener(&socket_path).await?);
    serve_unix_prebound(handler, listener).await
}

/// Bind a Unix socket listener early (Wave 49 startup optimization).
///
/// Returns the bound listener so the caller can pass it to
/// [`serve_unix_prebound`] after constructing the handler. This
/// ensures `connect()` succeeds as soon as the socket path exists,
/// even before the full handler is ready.
pub async fn prebind_unix_listener(socket_path: &std::path::Path) -> ServerResult<UnixListener> {
    info!(
        "Pre-binding JSON-RPC Unix socket: {:?}",
        socket_path
    );

    if let Some(parent) = socket_path.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| {
            ServerError::Initialization(format!(
                "Failed to create socket directory {}: {e}",
                parent.display()
            ))
        })?;
    }

    if socket_path.exists() {
        warn!("Removing old JSON-RPC socket: {:?}", socket_path);
        tokio::fs::remove_file(socket_path)
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
    }

    let listener =
        UnixListener::bind(socket_path).map_err(|e| ServerError::Network(e.to_string()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode =
            std::env::var(toadstool_common::interned_strings::socket_env::TOADSTOOL_SOCKET_MODE)
                .ok()
                .and_then(|s| {
                    u32::from_str_radix(s.trim_start_matches("0o").trim_start_matches('0'), 8).ok()
                })
                .unwrap_or(0o600);
        let mut perms = tokio::fs::metadata(socket_path)
            .await
            .map_err(|e| ServerError::Internal(e.to_string()))?
            .permissions();
        perms.set_mode(mode);
        tokio::fs::set_permissions(socket_path, perms)
            .await
            .map_err(|e| ServerError::Internal(e.to_string()))?;
        info!("Set JSON-RPC socket permissions to {mode:04o}");
    }

    info!("✅ JSON-RPC socket bound: {:?}", socket_path);
    Ok(listener)
}

/// Spawn a minimal health-only accept loop on a pre-bound listener.
///
/// Accepts connections and responds to `health.liveness` / `health.check` /
/// `health.readiness` with immediate JSON-RPC responses while the full
/// `JsonRpcHandler` is still being constructed. All other methods return
/// a `-32002` "server initializing" error.
///
/// Returns a `JoinHandle` that resolves when `stop` receives a value. The
/// caller should send to `stop` once the full handler is ready, then
/// pass the same `listener` to [`serve_unix_prebound`].
pub fn spawn_early_health_responder(
    listener: &Arc<UnixListener>,
    mut stop: tokio::sync::watch::Receiver<bool>,
) -> tokio::task::JoinHandle<()> {
    let listener = Arc::clone(listener);
    tokio::spawn(async move {
        loop {
            tokio::select! {
                biased;
                _ = stop.changed() => break,
                result = listener.accept() => {
                    match result {
                        Ok((stream, _)) => {
                            tokio::spawn(handle_early_health(stream));
                        }
                        Err(e) => {
                            warn!("Early health accept error: {e}");
                        }
                    }
                }
            }
        }
        info!("Early health responder stopped — full handler taking over");
    })
}

async fn handle_early_health(stream: UnixStream) {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = String::new();
    if reader.read_line(&mut line).await.is_err() || line.trim().is_empty() {
        return;
    }
    let trimmed = line.trim();

    let method = serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .and_then(|v| v.get("method")?.as_str().map(String::from));
    let id = serde_json::from_str::<serde_json::Value>(trimmed)
        .ok()
        .and_then(|v| v.get("id").cloned())
        .unwrap_or(serde_json::Value::Null);

    let response = match method.as_deref() {
        Some("health.liveness") => {
            serde_json::json!({"jsonrpc":"2.0","result":{"status":"alive"},"id":id})
        }
        Some("health.check" | "toadstool.health" | "compute.health") => {
            serde_json::json!({"jsonrpc":"2.0","result":{"status":"starting","uptime_secs":0},"id":id})
        }
        Some("health.readiness") => {
            serde_json::json!({"jsonrpc":"2.0","result":{"status":"starting"},"id":id})
        }
        _ => {
            serde_json::json!({"jsonrpc":"2.0","error":{"code":-32002,"message":"Server initializing"},"id":id})
        }
    };

    let mut buf = serde_json::to_vec(&response).unwrap_or_default();
    buf.push(b'\n');
    let _ = writer.write_all(&buf).await;
    let _ = writer.flush().await;
}

/// Serve JSON-RPC on a pre-bound Unix socket listener.
///
/// Used with [`prebind_unix_listener`] to start accepting connections
/// on a listener that was bound before the full handler was constructed.
pub async fn serve_unix_prebound(
    handler: Arc<JsonRpcHandler>,
    listener: Arc<UnixListener>,
) -> ServerResult<()> {
    let env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
    let btsp_required = toadstool_common::primal_sockets::is_btsp_required(&env);

    if btsp_required {
        info!("✅ BTSP mode active on pre-bound socket");
    } else {
        info!("✅ Pure JSON-RPC 2.0 server (NDJSON) accepting on pre-bound socket");
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
pub const fn is_plaintext_protocol_byte(byte: u8) -> bool {
    byte >= 0x09
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
/// Per `ecoPrimals/infra/wateringHole/BTSP_PROTOCOL_STANDARD.md`: BTSP handshake is still enforced for
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

    let mut stream = if is_plaintext_protocol_byte(first[0]) {
        info!(
            target: "btsp",
            "Plain-text connection on BTSP socket (0x{:02x}), \
             probing JSON-line BTSP or JSON-RPC",
            first[0]
        );
        let first_line = btsp::read_full_line_after_first_byte(&mut stream, first[0])
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
        if first_line.trim().is_empty() {
            return Ok(());
        }
        if first_line.starts_with("POST")
            || first_line.starts_with("GET")
            || first_line.starts_with("HTTP")
        {
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            return handle_http_keepalive_unix(handler, &mut reader, &mut writer, first_line).await;
        }
        if btsp::line_looks_like_btsp_client_hello(&first_line) {
            let family_seed = btsp::family_seed::load_family_seed_for_btsp()
                .map_err(|e| ServerError::Configuration(e.to_string()))?;
            let sec = btsp::json_line::resolve_security_socket_path()
                .map_err(|e| ServerError::Configuration(e.to_string()))?;
            let sec_s = sec.to_string_lossy().into_owned();
            let info = btsp::relay_json_line_handshake(
                &mut stream,
                first_line.trim_end(),
                &family_seed,
                &sec_s,
            )
            .await
            .map_err(|e| ServerError::Network(e.to_string()))?;
            info!(
                target: "btsp",
                "🔒 BTSP JSON-line handshake complete: cipher={}, session_id={}",
                info.cipher.as_str(),
                info.session_id
            );
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            return handle_post_handshake_session(handler, &mut reader, &mut writer, &family_seed)
                .await;
        }
        let (reader, mut writer) = stream.into_split();
        let mut reader = BufReader::new(reader);
        return handle_ndjson_unix(handler, &mut reader, &mut writer, first_line).await;
    } else {
        let family_seed = resolve_family_seed()?;
        let mut wrapped = btsp::framing::PrependByte::new(first[0], stream);
        match btsp::BtspServer::accept_handshake(&mut wrapped, &family_seed).await {
            Ok(session) => {
                info!(
                    "🔒 BTSP handshake complete: cipher={}, session_id={:02x?}",
                    session.cipher.as_str(),
                    &session.session_id[..4]
                );
            }
            Err(e) => {
                warn!("🔒 BTSP handshake rejected: {e}");
                let _ = btsp::BtspServer::send_handshake_error(&mut wrapped).await;
                return Err(ServerError::Network(format!("BTSP handshake failed: {e}")));
            }
        }
        wrapped.into_inner()
    };

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

/// After a JSON-line BTSP handshake, read the first NDJSON line and check for
/// `btsp.negotiate` (Phase 3 cipher upgrade). If the client negotiates ChaCha20-Poly1305,
/// switch to encrypted length-prefixed framing. Otherwise continue with NDJSON.
#[cfg(feature = "btsp")]
async fn handle_post_handshake_session(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    family_seed: &str,
) -> ServerResult<()> {
    use toadstool_common::btsp;

    let mut first_line = String::new();
    let n = reader
        .read_line(&mut first_line)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    if n == 0 {
        return Ok(());
    }

    match btsp::try_handle_negotiate(&first_line, writer, family_seed)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?
    {
        btsp::NegotiateOutcome::Negotiated(keys) => {
            handle_encrypted_session(handler, reader, writer, keys).await
        }
        btsp::NegotiateOutcome::NullCipher => {
            handle_ndjson_unix(handler, reader, writer, String::new()).await
        }
        btsp::NegotiateOutcome::NotNegotiate => {
            handle_ndjson_unix(handler, reader, writer, first_line).await
        }
    }
}

/// Serve JSON-RPC over BTSP Phase 3 encrypted framing.
///
/// Each request/response pair uses length-prefixed encrypted frames:
/// `[4B len BE u32][12B nonce][ciphertext + Poly1305 tag]`
#[cfg(feature = "btsp")]
async fn handle_encrypted_session(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    keys: toadstool_common::btsp::Phase3SessionKeys,
) -> ServerResult<()> {
    use toadstool_common::btsp::framing;

    info!(target: "btsp", "BTSP Phase 3: entering encrypted session loop");

    loop {
        match framing::read_encrypted_frame(reader, &keys).await {
            Ok(plaintext) => {
                let response_body = process_request(&handler, &plaintext).await?;
                if let Err(e) = framing::write_encrypted_frame(writer, &keys, &response_body).await
                {
                    warn!(target: "btsp", "Phase 3 encrypted write error: {e}");
                    break;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                warn!(target: "btsp", "Phase 3 encrypted read error: {e}");
                break;
            }
        }
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
