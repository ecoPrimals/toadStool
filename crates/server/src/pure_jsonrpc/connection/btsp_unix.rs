// SPDX-License-Identifier: AGPL-3.0-or-later
//! BTSP connection handling on Unix domain sockets.
//!
//! Separated from `unix.rs` to keep per-file complexity under 750 lines.
//! Both `btsp`-enabled and `btsp`-disabled variants live here so that
//! `unix.rs` only contains core JSON-RPC / riboCipher / HTTP logic.

use std::sync::Arc;
#[cfg(feature = "btsp")]
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::net::UnixStream;
use tracing::error;
#[cfg(feature = "btsp")]
use tracing::{info, warn};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::JsonRpcHandler;
#[cfg(feature = "btsp")]
use crate::pure_jsonrpc::handler::ConnectionTrustHints;

#[cfg(feature = "btsp")]
use super::process_request;
use super::unix::{is_plaintext_protocol_byte, try_ribocipher_dispatch};

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

    // Wave 113: REJECT unsignalled connections on BTSP socket
    error!(
        first_byte = format_args!("0x{:02X}", first[0]),
        "REJECTED: unsignalled connection on BTSP socket (no riboCipher prefix). \
         Clients MUST prepend [0xEC, protocol_type]."
    );
    if is_plaintext_protocol_byte(first[0]) {
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
        let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, &buf).await;
        let _ = tokio::io::AsyncWriteExt::flush(&mut stream).await;
    }
    let _ = tokio::io::AsyncWriteExt::shutdown(&mut stream).await;
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

    // Wave 113: REJECT unsignalled connections on BTSP socket (btsp feature disabled)
    error!(
        first_byte = format_args!("0x{:02X}", first[0]),
        "REJECTED: unsignalled connection on BTSP socket (no riboCipher prefix). \
         Clients MUST prepend [0xEC, protocol_type]."
    );
    if is_plaintext_protocol_byte(first[0]) {
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
        let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, &buf).await;
        let _ = tokio::io::AsyncWriteExt::flush(&mut stream).await;
    }
    let _ = tokio::io::AsyncWriteExt::shutdown(&mut stream).await;
    Ok(())
}

/// After a JSON-line BTSP handshake, read the first NDJSON line and check for
/// `btsp.negotiate` (Phase 3 cipher upgrade). If the client negotiates ChaCha20-Poly1305,
/// switch to encrypted length-prefixed framing. Otherwise continue with NDJSON.
#[cfg(feature = "btsp")]
#[expect(
    dead_code,
    reason = "will be used when BTSP-over-riboCipher routing is wired (0xEC 0x02/0x03)"
)]
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
            super::dispatch::handle_ndjson(
                &handler,
                reader,
                writer,
                String::new(),
                ConnectionTrustHints::UNIX_LOCAL,
                None,
            )
            .await
        }
        btsp::NegotiateOutcome::NotNegotiate => {
            super::dispatch::handle_ndjson(
                &handler,
                reader,
                writer,
                first_line,
                ConnectionTrustHints::UNIX_LOCAL,
                None,
            )
            .await
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
                let response_body =
                    process_request(&handler, &plaintext, ConnectionTrustHints::UNIX_BTSP).await?;
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
#[expect(
    dead_code,
    reason = "will be used when BTSP-over-riboCipher routing is wired (0xEC 0x02/0x03)"
)]
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
