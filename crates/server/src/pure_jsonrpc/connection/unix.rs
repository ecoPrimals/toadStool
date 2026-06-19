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
use crate::pure_jsonrpc::handler::ConnectionTrustHints;

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
    info!("Pre-binding JSON-RPC Unix socket: {:?}", socket_path);

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

async fn handle_early_health(mut stream: UnixStream) {
    // Strip riboCipher prefix if present (Wave 113: early-health must accept signalled clients)
    let mut first = [0u8; 1];
    if tokio::io::AsyncReadExt::read(&mut stream, &mut first)
        .await
        .is_err()
    {
        return;
    }
    if first[0] == 0xEC {
        // riboCipher clear prefix — consume protocol-type byte and proceed
        let mut pt = [0u8; 1];
        if tokio::io::AsyncReadExt::read_exact(&mut stream, &mut pt)
            .await
            .is_err()
        {
            return;
        }
        if pt[0] == 0x00 {
            // PROBE: immediate liveness response
            let response =
                serde_json::json!({"jsonrpc":"2.0","result":{"status":"alive"},"id":null});
            let mut buf = serde_json::to_vec(&response).unwrap_or_default();
            buf.push(b'\n');
            let (_, mut writer) = stream.into_split();
            let _ = writer.write_all(&buf).await;
            let _ = writer.flush().await;
            return;
        }
        // 0x01 (NDJSON) or 0x04 (HTTP) — fall through to JSON line read
    } else if first[0] == 0xED {
        // MitoBeacon (Wave 114): consume 4-byte HMAC tag + protocol-type byte
        let mut hmac_tag = [0u8; 4];
        if tokio::io::AsyncReadExt::read_exact(&mut stream, &mut hmac_tag)
            .await
            .is_err()
        {
            return;
        }
        let mut pt = [0u8; 1];
        if tokio::io::AsyncReadExt::read_exact(&mut stream, &mut pt)
            .await
            .is_err()
        {
            return;
        }
        if pt[0] == 0x00 {
            let response =
                serde_json::json!({"jsonrpc":"2.0","result":{"status":"alive"},"id":null});
            let mut buf = serde_json::to_vec(&response).unwrap_or_default();
            buf.push(b'\n');
            let (_, mut writer) = stream.into_split();
            let _ = writer.write_all(&buf).await;
            let _ = writer.flush().await;
            return;
        }
        // 0x01 (NDJSON) or 0x04 (HTTP) — fall through to JSON line read
    } else if first[0] == 0xEE {
        // Nuclear tier — not supported during early health
        return;
    } else {
        // Non-riboCipher byte — push back into the line buffer below
    }

    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    let mut line = if first[0] == 0xEC || first[0] == 0xED {
        String::new()
    } else {
        String::from(first[0] as char)
    };
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
        Some("health") => {
            serde_json::json!({"jsonrpc":"2.0","result":{"status":"starting","primal":toadstool_common::constants::primal_identity::PRIMAL_NAME,"version":env!("CARGO_PKG_VERSION")},"id":id})
        }
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
                        super::btsp_unix::handle_btsp_connection(handler, stream).await
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
/// Detects riboCipher transport signal before protocol dispatch per
/// `RIBOCIPHER_TRANSPORT_SIGNAL_STANDARD.md`. Falls back to legacy
/// peek-and-guess with WARN for unsignalled connections (Wave 111–112).
pub(super) async fn handle_unix_connection(
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

    let stream = match try_ribocipher_dispatch(&handler, stream, first[0]).await {
        Ok(Some(result)) => return result,
        Ok(None) => {
            return Err(ServerError::Internal(
                "riboCipher dispatch returned Ok(None) — invariant violation".into(),
            ));
        }
        Err(stream) => stream,
    };

    // Wave 113: REJECT unsignalled connections (upgraded from ERROR in Wave 112)
    error!(
        first_byte = format_args!("0x{:02X}", first[0]),
        "REJECTED: unsignalled connection (no riboCipher prefix). \
         Clients MUST prepend [0xEC, 0x01] per RIBOCIPHER_TRANSPORT_SIGNAL_STANDARD."
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

/// Attempt riboCipher dispatch on a Unix stream. If the first byte is a
/// riboCipher prefix, the stream is consumed and `Ok(Some(result))` is returned.
/// If not riboCipher, the stream is returned intact for legacy fallback.
pub(super) async fn try_ribocipher_dispatch(
    handler: &Arc<JsonRpcHandler>,
    mut stream: UnixStream,
    first_byte: u8,
) -> Result<Option<ServerResult<()>>, UnixStream> {
    use super::ribocipher;

    match first_byte {
        ribocipher::CLEAR => {
            let mut pt = [0u8; 1];
            if let Err(e) = tokio::io::AsyncReadExt::read_exact(&mut stream, &mut pt).await {
                return Ok(Some(Err(ServerError::Network(format!(
                    "riboCipher: failed to read protocol type: {e}"
                )))));
            }
            info!(
                protocol_type = format_args!("0x{:02X}", pt[0]),
                "riboCipher clear signal"
            );
            Ok(Some(
                handle_ribocipher_clear_unix(handler.clone(), stream, pt[0]).await,
            ))
        }
        ribocipher::MITO => {
            // MitoBeacon (Wave 114): read 4-byte HMAC tag, then protocol type.
            // HMAC validation deferred to Wave 115 (HKDF from FAMILY_SEED).
            let mut hmac_tag = [0u8; 4];
            if let Err(e) = tokio::io::AsyncReadExt::read_exact(&mut stream, &mut hmac_tag).await {
                return Ok(Some(Err(ServerError::Network(format!(
                    "riboCipher mito: failed to read HMAC tag: {e}"
                )))));
            }
            let mut pt = [0u8; 1];
            if let Err(e) = tokio::io::AsyncReadExt::read_exact(&mut stream, &mut pt).await {
                return Ok(Some(Err(ServerError::Network(format!(
                    "riboCipher mito: failed to read protocol type: {e}"
                )))));
            }
            info!(
                protocol_type = format_args!("0x{:02X}", pt[0]),
                hmac = format_args!(
                    "{:02x}{:02x}{:02x}{:02x}",
                    hmac_tag[0], hmac_tag[1], hmac_tag[2], hmac_tag[3]
                ),
                "riboCipher mito-beacon signal accepted"
            );
            Ok(Some(
                handle_ribocipher_clear_unix(handler.clone(), stream, pt[0]).await,
            ))
        }
        ribocipher::NUCLEAR => {
            warn!("riboCipher nuclear-sealed tier not yet supported — rejecting");
            let reject = serde_json::json!({
                "jsonrpc": "2.0",
                "error": {"code": -32600, "message": "riboCipher tier 3 (nuclear) not yet supported"},
                "id": null
            });
            let mut buf = serde_json::to_vec(&reject).unwrap_or_default();
            buf.push(b'\n');
            let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, &buf).await;
            let _ = tokio::io::AsyncWriteExt::flush(&mut stream).await;
            Ok(Some(Ok(())))
        }
        _ => Err(stream),
    }
}

/// Handle a riboCipher clear-signalled Unix connection, routed by protocol type.
async fn handle_ribocipher_clear_unix(
    handler: Arc<JsonRpcHandler>,
    stream: UnixStream,
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
            handle_ndjson_unix(handler, &mut reader, &mut writer, String::new()).await
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
            handle_http_keepalive_unix(handler, &mut reader, &mut writer, first_line).await
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

/// HTTP/1.1 keep-alive loop: process multiple HTTP requests on a single connection.
///
/// Defaults to keep-alive per HTTP/1.1 spec. Closes only when the client sends
/// `Connection: close` or the connection reaches EOF.
pub(super) async fn handle_http_keepalive_unix(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    first_request_line: String,
) -> ServerResult<()> {
    let mut request_line = first_request_line;
    loop {
        let (headers, body) = read_http_request_continuation_unix(reader).await?;
        let response_body =
            process_request(&handler, &body, ConnectionTrustHints::UNIX_LOCAL).await?;

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
pub(super) async fn handle_ndjson_unix(
    handler: Arc<JsonRpcHandler>,
    reader: &mut BufReader<tokio::net::unix::OwnedReadHalf>,
    writer: &mut tokio::net::unix::OwnedWriteHalf,
    first_line: String,
) -> ServerResult<()> {
    let mut line = first_line;
    loop {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let response_body = process_request(
                &handler,
                trimmed.as_bytes(),
                ConnectionTrustHints::UNIX_LOCAL,
            )
            .await?;
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
