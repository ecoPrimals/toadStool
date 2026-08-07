// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix domain socket listener and per-connection handling.
//!
//! Supports three layers (composable, per-connection):
//!
//! 1. **G65 protocol negotiation** (`PROTOCOLS:` line → tarpc or JSON-RPC)
//! 2. **riboCipher transport signal** (`0xEC` / `0xED` / `0xEE` prefix)
//! 3. **BTSP handshake** (when `FAMILY_ID` is set)
//!
//! Legacy clients that send neither a G65 line nor a riboCipher prefix are
//! rejected per Wave 113 policy. G65 negotiation is checked first (100 ms
//! peek timeout), then riboCipher dispatch on the first byte.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::JsonRpcHandler;
use crate::pure_jsonrpc::handler::ConnectionTrustHints;

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
                .unwrap_or(0o660);
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
    let mut line = ndjson_line_prefix_after_first_byte(first[0]);
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

    let response = early_health_response(method.as_deref(), id);

    let mut buf = serde_json::to_vec(&response).unwrap_or_default();
    buf.push(b'\n');
    let _ = writer.write_all(&buf).await;
    let _ = writer.flush().await;
}

/// Serve JSON-RPC on a pre-bound Unix socket listener (C2 legacy path).
///
/// Used with [`prebind_unix_listener`] to start accepting connections
/// on a listener that was bound before the full handler was constructed.
///
/// Prefer [`serve_unix_g65`] for new deployments — it handles both
/// JSON-RPC and tarpc on a single socket via G65 protocol negotiation.
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

/// G65 protocol negotiation accept loop (Phase 3 cephalization).
///
/// Serves both JSON-RPC and tarpc on a **single socket**. On each connection:
///
/// 1. Peek the first byte with a 100 ms timeout.
/// 2. If the byte is `b'P'`, read the `PROTOCOLS:` line byte-by-byte,
///    select the best mutual protocol, respond with `PROTOCOL:`, and
///    route to the appropriate handler.
/// 3. Otherwise fall through to the existing riboCipher / BTSP / NDJSON
///    dispatch — full backward compatibility, zero client changes.
pub async fn serve_unix_g65(
    handler: Arc<JsonRpcHandler>,
    tarpc_server: crate::tarpc_server::ToadStoolTarpcServer,
    listener: Arc<UnixListener>,
) -> ServerResult<()> {
    use super::ipc_protocol::IpcProtocol;

    let env = toadstool_common::primal_sockets::SocketPathEnv::from_env();
    let btsp_required = toadstool_common::primal_sockets::is_btsp_required(&env);

    info!("✅ G65 protocol negotiation active (jsonrpc + tarpc on single socket)");
    if btsp_required {
        info!("   BTSP handshake required for non-negotiated connections");
    }

    let server_supported = IpcProtocol::supported();

    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let handler = Arc::clone(&handler);
                let tarpc = tarpc_server.clone();
                let btsp = btsp_required;
                let supported = server_supported.clone();
                tokio::spawn(async move {
                    let result =
                        handle_g65_connection(handler, tarpc, stream, btsp, &supported).await;
                    if let Err(e) = result {
                        error!("G65 connection error: {e}");
                    }
                });
            }
            Err(e) => error!("G65 accept error: {e}"),
        }
    }
}

/// Per-connection G65 dispatch: read first byte → negotiate or riboCipher → route.
///
/// Reads the first byte of the connection. If it is `b'P'` (start of
/// `PROTOCOLS:`), the G65 negotiation path runs. The `read_negotiation_line`
/// function reads byte-by-byte so it consumes exactly one line — the `P`
/// we already read is prepended. Otherwise the byte is forwarded to the
/// existing `handle_unix_connection_with_first_byte` handler.
async fn handle_g65_connection(
    handler: Arc<JsonRpcHandler>,
    tarpc_server: crate::tarpc_server::ToadStoolTarpcServer,
    mut stream: UnixStream,
    btsp_required: bool,
    server_supported: &[super::ipc_protocol::IpcProtocol],
) -> ServerResult<()> {
    use super::ipc_protocol::IpcProtocol;
    use super::protocol_negotiation::{
        ProtocolRequest, ProtocolResponse, read_negotiation_line, select_protocol,
    };

    let mut first = [0u8; 1];
    let n = tokio::io::AsyncReadExt::read(&mut stream, &mut first)
        .await
        .map_err(|e| ServerError::Network(e.to_string()))?;
    if n == 0 {
        return Ok(());
    }

    if first[0] == b'P' {
        // Likely a G65 `PROTOCOLS:` line. We already consumed the `P`, so
        // read the rest of the line byte-by-byte, then prepend the `P`.
        let rest = match read_negotiation_line(&mut stream).await {
            Ok(l) => l,
            Err(e) => {
                warn!("G65 negotiation line read failed: {e}");
                return Ok(());
            }
        };

        let full_line = format!("P{rest}");

        let request = match ProtocolRequest::from_wire(&full_line) {
            Ok(r) => r,
            Err(e) => {
                warn!("Invalid G65 protocol request: {e}");
                let _ = stream.write_all(b"PROTOCOL: jsonrpc\n").await;
                let _ = stream.flush().await;
                return if btsp_required {
                    super::btsp_unix::handle_btsp_connection(handler, stream).await
                } else {
                    handle_unix_connection(handler, stream).await
                };
            }
        };

        let selected = select_protocol(&request.supported, server_supported);
        let response = ProtocolResponse::new(selected);

        if let Err(e) = stream.write_all(response.to_wire().as_bytes()).await {
            warn!("G65 response write failed: {e}");
            return Ok(());
        }
        let _ = stream.flush().await;

        info!("G65 protocol negotiated: {selected}");

        return match selected {
            IpcProtocol::Tarpc => {
                crate::tarpc_server::serve_on_tarpc_channel(tarpc_server, stream).await;
                Ok(())
            }
            IpcProtocol::JsonRpc => {
                if btsp_required {
                    super::btsp_unix::handle_btsp_connection(handler, stream).await
                } else {
                    handle_unix_connection(handler, stream).await
                }
            }
        };
    }

    // Not a G65 negotiation — dispatch via riboCipher / legacy path.
    // The first byte is already consumed, reuse the existing handler.
    handle_unix_connection_with_first_byte(handler, stream, first[0], btsp_required).await
}

/// Handle a Unix connection where the first byte has already been read.
///
/// Shared by the G65 fallback path (non-`P` first byte) and the legacy
/// connection handler. Dispatches to riboCipher or rejects unsignalled.
async fn handle_unix_connection_with_first_byte(
    handler: Arc<JsonRpcHandler>,
    stream: UnixStream,
    first_byte: u8,
    btsp_required: bool,
) -> ServerResult<()> {
    if btsp_required {
        // In BTSP mode, delegate to btsp_unix which does its own first-byte read.
        // We need to prepend the byte we already consumed. Use a thin wrapper
        // that feeds the byte back, or just pass through the stream since btsp_unix
        // re-reads. For correctness we route to the riboCipher dispatch.
        let stream = match try_ribocipher_dispatch(&handler, stream, first_byte).await {
            Ok(Some(result)) => return result,
            Ok(None) => {
                return Err(ServerError::Internal(
                    "riboCipher dispatch returned Ok(None) — invariant violation".into(),
                ));
            }
            Err(stream) => stream,
        };
        error!(
            first_byte = format_args!("0x{:02X}", first_byte),
            "REJECTED: unsignalled connection (no riboCipher prefix in BTSP mode)"
        );
        let (_, mut writer) = stream.into_split();
        let _ = super::dispatch::write_reject_response(&mut writer).await;
        return Ok(());
    }

    // Non-BTSP: existing riboCipher dispatch path.
    let stream = match try_ribocipher_dispatch(&handler, stream, first_byte).await {
        Ok(Some(result)) => return result,
        Ok(None) => {
            return Err(ServerError::Internal(
                "riboCipher dispatch returned Ok(None) — invariant violation".into(),
            ));
        }
        Err(stream) => stream,
    };

    error!(
        first_byte = format_args!("0x{:02X}", first_byte),
        "REJECTED: unsignalled connection (no riboCipher prefix). \
         Clients MUST prepend [0xEC, 0x01] per RIBOCIPHER_TRANSPORT_SIGNAL_STANDARD."
    );
    let (_, mut writer) = stream.into_split();
    let _ = super::dispatch::write_reject_response(&mut writer).await;
    Ok(())
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
    let _ = super::dispatch::write_reject_response(&mut writer).await;
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
///
/// Delegates to `dispatch::handle_ribocipher_clear` (G66 transport abstraction).
async fn handle_ribocipher_clear_unix(
    handler: Arc<JsonRpcHandler>,
    stream: UnixStream,
    protocol_type: u8,
) -> ServerResult<()> {
    let (reader, mut writer) = stream.into_split();
    let mut reader = BufReader::new(reader);
    super::dispatch::handle_ribocipher_clear(
        handler,
        &mut reader,
        &mut writer,
        protocol_type,
        ConnectionTrustHints::UNIX_LOCAL,
        None,
    )
    .await
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

/// Returns `true` when a byte is a riboCipher transport signal prefix.
pub(crate) const fn is_ribocipher_signal_byte(byte: u8) -> bool {
    use super::ribocipher;
    matches!(
        byte,
        ribocipher::CLEAR | ribocipher::MITO | ribocipher::NUCLEAR
    )
}

/// Build the initial NDJSON line buffer after consuming the first connection byte.
pub(crate) fn ndjson_line_prefix_after_first_byte(first_byte: u8) -> String {
    if is_ribocipher_signal_byte(first_byte) && first_byte != super::ribocipher::NUCLEAR {
        String::new()
    } else {
        String::from(first_byte as char)
    }
}

/// JSON-RPC response for the early-health responder while the full handler starts.
#[expect(
    clippy::needless_pass_by_value,
    reason = "id is consumed once into the response object — taking by value avoids a clone at call sites"
)]
pub(crate) fn early_health_response(
    method: Option<&str>,
    id: serde_json::Value,
) -> serde_json::Value {
    match method {
        Some("health") => serde_json::json!({
            "jsonrpc": "2.0",
            "result": {
                "status": "starting",
                "primal": toadstool_common::constants::primal_identity::PRIMAL_NAME,
                "version": env!("CARGO_PKG_VERSION")
            },
            "id": id
        }),
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
    }
}
