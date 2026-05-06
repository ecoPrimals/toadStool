// SPDX-License-Identifier: AGPL-3.0-or-later
//! Phase 3 `btsp.negotiate` handler for JSON-RPC connections.
//!
//! Extracted from `json_line.rs` for cohesion: this module owns the
//! cipher negotiation flow that switches a connection from NDJSON to
//! encrypted length-prefixed framing.

use serde_json::Value;
use tokio::io::AsyncWrite;

use super::json_line::{BtspJsonLineError, send_json_line};

/// Outcome of attempting to handle a `btsp.negotiate` JSON-RPC line.
///
/// Used by the server connection loop to decide whether to switch to
/// encrypted framing or continue with NDJSON.
pub enum NegotiateOutcome {
    /// The line was `btsp.negotiate` and was handled. Response has been sent.
    /// Contains the derived session keys for encrypted framing.
    Negotiated(super::phase3::Phase3SessionKeys),

    /// The line was `btsp.negotiate` but cipher negotiation fell back to null.
    /// Response has been sent; continue with NDJSON.
    NullCipher,

    /// The line was not `btsp.negotiate`. Caller should process it as normal JSON-RPC.
    NotNegotiate,
}

impl std::fmt::Debug for NegotiateOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Negotiated(_) => f.write_str("Negotiated(<keys redacted>)"),
            Self::NullCipher => f.write_str("NullCipher"),
            Self::NotNegotiate => f.write_str("NotNegotiate"),
        }
    }
}

/// Try to handle a line as a `btsp.negotiate` JSON-RPC request.
///
/// If the line is a valid `btsp.negotiate` request and we can support the requested
/// cipher, derives Phase 3 session keys and sends the negotiate response.
///
/// The `family_seed` is the raw family seed string (same as used for handshake).
///
/// # Transport switch protocol
///
/// On `Ok(Negotiated(keys))`, the negotiate JSON-RPC response has already been
/// flushed as the **last NDJSON message** on the connection. The caller MUST
/// immediately switch to encrypted length-prefixed framing via
/// [`super::framing::read_encrypted_frame`] / [`super::framing::write_encrypted_frame`]
/// for all subsequent I/O.
///
/// **BufReader pipelining hazard**: if the peer sends additional bytes after the
/// `btsp.negotiate\n` line before waiting for the response, those bytes will sit
/// in the `BufReader` internal buffer and be interpreted as the length prefix of
/// the first encrypted frame — typically causing a decrypt failure or oversized
/// frame rejection. Well-behaved clients (primalSpring) wait for the negotiate
/// response before sending encrypted frames, so this is not a concern in normal
/// operation, but protocol fuzzers or misbehaving clients will see immediate
/// connection termination rather than silent data corruption.
///
/// # Returns
///
/// - `Ok(Negotiated(keys))` — negotiate succeeded; switch to encrypted framing
/// - `Ok(NullCipher)` — negotiate fell back to null cipher; stay on NDJSON
/// - `Ok(NotNegotiate)` — line is not `btsp.negotiate`; caller handles normally
///
/// # Errors
///
/// I/O or protocol errors during negotiate handling.
pub async fn try_handle_negotiate<S: AsyncWrite + Unpin>(
    line: &str,
    writer: &mut S,
    family_seed: &str,
) -> Result<NegotiateOutcome, BtspJsonLineError> {
    use base64::Engine;
    use base64::engine::general_purpose::STANDARD as BASE64;

    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Ok(NegotiateOutcome::NotNegotiate);
    }

    let parsed: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => return Ok(NegotiateOutcome::NotNegotiate),
    };

    let method = parsed
        .get("method")
        .and_then(Value::as_str)
        .unwrap_or_default();

    if method != "btsp.negotiate" {
        return Ok(NegotiateOutcome::NotNegotiate);
    }

    let id = parsed.get("id").cloned().unwrap_or(Value::Null);
    let Some(params) = parsed
        .get("params")
        .cloned()
        .and_then(|v| serde_json::from_value::<super::phase3::NegotiateParams>(v).ok())
    else {
        let err_resp = serde_json::json!({
            "jsonrpc": "2.0",
            "error": {"code": -32602, "message": "btsp.negotiate: invalid or missing params"},
            "id": id,
        });
        send_json_line(writer, &err_resp).await?;
        return Err(BtspJsonLineError::Protocol(
            "btsp.negotiate: invalid params".into(),
        ));
    };

    let requested_cipher = if let Some(c) = params
        .ciphers
        .iter()
        .find(|c| c.as_str() == "chacha20-poly1305")
    {
        c.clone()
    } else if params
        .preferred_cipher
        .as_deref()
        .is_some_and(|c| c == "chacha20-poly1305" || c == "chacha20_poly1305")
    {
        "chacha20-poly1305".to_owned()
    } else {
        tracing::info!(target: "btsp", "btsp.negotiate: no supported cipher, returning null");
        let resp = serde_json::json!({
            "jsonrpc": "2.0",
            "result": {"cipher": "null"},
            "id": id,
        });
        send_json_line(writer, &resp).await?;
        return Ok(NegotiateOutcome::NullCipher);
    };

    let client_nonce_bytes = if let Some(ref nonce_str) = params.client_nonce {
        BASE64.decode(nonce_str).map_err(|e| {
            BtspJsonLineError::Protocol(format!("btsp.negotiate: client_nonce decode: {e}"))
        })?
    } else {
        tracing::info!(target: "btsp", "btsp.negotiate: no client_nonce, returning null cipher");
        let resp = serde_json::json!({
            "jsonrpc": "2.0",
            "result": {"cipher": "null"},
            "id": id,
        });
        send_json_line(writer, &resp).await?;
        return Ok(NegotiateOutcome::NullCipher);
    };

    let handshake_key =
        super::phase3::derive_handshake_key(family_seed.as_bytes()).map_err(|e| {
            BtspJsonLineError::Rpc(format!("btsp.negotiate: handshake key derivation: {e}"))
        })?;

    let server_nonce = super::phase3::generate_negotiate_nonce();
    let server_nonce_b64 = BASE64.encode(server_nonce);

    let keys = super::phase3::Phase3SessionKeys::derive(
        &handshake_key,
        &client_nonce_bytes,
        &server_nonce,
        true,
    )
    .map_err(|e| BtspJsonLineError::Rpc(format!("btsp.negotiate: key derivation: {e}")))?;

    let resp = serde_json::json!({
        "jsonrpc": "2.0",
        "result": {
            "cipher": requested_cipher,
            "server_nonce": server_nonce_b64,
        },
        "id": id,
    });
    send_json_line(writer, &resp).await?;

    tracing::info!(
        target: "btsp",
        session_id = %params.session_id,
        cipher = %requested_cipher,
        "BTSP Phase 3 negotiate complete — switching to encrypted framing"
    );

    Ok(NegotiateOutcome::Negotiated(keys))
}
