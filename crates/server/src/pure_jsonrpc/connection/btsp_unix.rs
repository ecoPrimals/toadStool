// SPDX-License-Identifier: AGPL-3.0-or-later
//! BTSP connection handling on Unix domain sockets.
//!
//! Separated from `unix.rs` to keep per-file complexity under 750 lines.
//! Both `btsp`-enabled and `btsp`-disabled variants live here so that
//! `unix.rs` only contains core JSON-RPC / riboCipher / HTTP logic.

use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tracing::{error, info, warn};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::JsonRpcHandler;
use crate::pure_jsonrpc::handler::ConnectionTrustHints;

use super::process_request;
use super::unix::{
    handle_http_keepalive_unix, handle_ndjson_unix, is_plaintext_protocol_byte,
    try_ribocipher_dispatch,
};

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

    // riboCipher detection — BEFORE legacy BTSP/plaintext peek
    let mut stream = match try_ribocipher_dispatch(&handler, stream, first[0]).await {
        Ok(Some(result)) => return result,
        Ok(None) => {
            return Err(ServerError::Internal(
                "riboCipher dispatch returned Ok(None) — invariant violation".into(),
            ));
        }
        Err(s) => s,
    };

    // Legacy BTSP detection with ERROR (Wave 112) for unsignalled connections
    error!(
        first_byte = format_args!("0x{:02X}", first[0]),
        "DEPRECATED: unsignalled connection on BTSP socket (no riboCipher prefix). \
         Clients MUST prepend [0xEC, protocol_type] per RIBOCIPHER_TRANSPORT_SIGNAL_STANDARD. \
         Wave 113 will REJECT unsignalled connections."
    );

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
                let response_body =
                    process_request(&handler, &frame, ConnectionTrustHints::UNIX_BTSP).await?;
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

    // riboCipher detection — BEFORE legacy peek
    let mut stream = match try_ribocipher_dispatch(&handler, stream, first[0]).await {
        Ok(Some(result)) => return result,
        Ok(None) => {
            return Err(ServerError::Internal(
                "riboCipher dispatch returned Ok(None) — invariant violation".into(),
            ));
        }
        Err(s) => s,
    };

    error!(
        first_byte = format_args!("0x{:02X}", first[0]),
        "DEPRECATED: unsignalled connection on BTSP socket (no riboCipher prefix). \
         Clients MUST prepend [0xEC, protocol_type] per RIBOCIPHER_TRANSPORT_SIGNAL_STANDARD. \
         Wave 113 will REJECT unsignalled connections."
    );

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
                let response_body = process_request(
                    &handler,
                    &plaintext,
                    ConnectionTrustHints::UNIX_BTSP,
                )
                .await?;
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
    if let Ok(seed) = std::env::var(toadstool_common::interned_strings::socket_env::FAMILY_SEED) {
        return Ok(seed.into_bytes());
    }

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
