// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON newline BTSP handshake relay via BearDog JSON-RPC.

use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::ToadStoolError;
use crate::constants::timeouts;
use crate::interned_strings::socket_env;
use crate::unix_jsonrpc::ConnectedJsonRpcClient;

use super::types::BtspCipher;

/// Session metadata returned after a successful JSON-line BTSP relay.
#[derive(Debug, Clone)]
pub struct BtspSessionInfo {
    /// Opaque session id from BearDog (`btsp.session.verify`).
    pub session_id: String,
    /// Negotiated cipher suite.
    pub cipher: BtspCipher,
}

/// Errors from JSON-line BTSP relay.
#[derive(Debug, Error)]
pub enum BtspJsonLineError {
    /// I/O error on the peer stream.
    #[error("BTSP JSON-line I/O: {0}")]
    Io(#[from] std::io::Error),

    /// JSON parse/serialize error.
    #[error("BTSP JSON-line JSON: {0}")]
    Json(#[from] serde_json::Error),

    /// BearDog JSON-RPC or unexpected payload.
    #[error("BTSP JSON-line RPC: {0}")]
    Rpc(String),

    /// Protocol violation (version, missing field, etc.).
    #[error("BTSP JSON-line protocol: {0}")]
    Protocol(String),

    /// Handshake or RPC call exceeded its timeout budget.
    #[error("BTSP JSON-line timeout: {0}")]
    Timeout(String),
}

impl From<ToadStoolError> for BtspJsonLineError {
    fn from(value: ToadStoolError) -> Self {
        Self::Rpc(value.to_string())
    }
}

fn handshake_timeout() -> Duration {
    std::env::var(socket_env::BTSP_HANDSHAKE_TIMEOUT_SECS)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(timeouts::BTSP_HANDSHAKE_TIMEOUT, Duration::from_secs)
}

fn rpc_timeout() -> Duration {
    std::env::var(socket_env::BTSP_RPC_TIMEOUT_SECS)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(timeouts::BTSP_RPC_TIMEOUT, Duration::from_secs)
}

/// Check if a JSON line looks like a BTSP ClientHello.
///
/// The line must parse as JSON and carry `"protocol": "btsp"` (spacing-insensitive via serde).
#[must_use]
pub fn line_looks_like_btsp_client_hello(line: &str) -> bool {
    #[derive(Deserialize)]
    struct Probe {
        protocol: Option<String>,
    }

    let t = line.trim();
    if !t.starts_with('{') {
        return false;
    }
    let Ok(p) = serde_json::from_str::<Probe>(t) else {
        return false;
    };
    p.protocol
        .as_deref()
        .is_some_and(|s| s.eq_ignore_ascii_case("btsp"))
}

/// Read bytes until `\n`, returning the line **without** the newline (UTF-8).
///
/// Used after the first byte of a line was already consumed for protocol sniffing.
pub async fn read_line_suffix<S: AsyncRead + Unpin>(
    stream: &mut S,
) -> Result<String, std::io::Error> {
    let mut v = Vec::new();
    let mut b = [0u8; 1];
    loop {
        stream.read_exact(&mut b).await?;
        if b[0] == b'\n' {
            break;
        }
        v.push(b[0]);
    }
    String::from_utf8(v).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// First byte of the line was already read; append the rest and return the full line.
pub async fn read_full_line_after_first_byte<S: AsyncRead + Unpin>(
    stream: &mut S,
    first_byte: u8,
) -> Result<String, std::io::Error> {
    let mut s = String::new();
    s.push(char::from(first_byte));
    s.push_str(&read_line_suffix(stream).await?);
    Ok(s)
}

/// Resolve BearDog / security provider unix socket path.
///
/// Order: `SECURITY_PROVIDER_SOCKET`, `CRYPTO_PROVIDER_SOCKET`, `SECURITY_SOCKET`,
/// then capability-style fallback via [`crate::primal_sockets`].
pub fn resolve_security_socket_path() -> Result<PathBuf, BtspJsonLineError> {
    for key in [
        "SECURITY_PROVIDER_SOCKET",
        "CRYPTO_PROVIDER_SOCKET",
        "SECURITY_SOCKET",
    ] {
        if let Ok(p) = std::env::var(key)
            && !p.is_empty()
        {
            return Ok(PathBuf::from(p));
        }
    }

    let env = crate::primal_sockets::SocketPathEnv::from_env();
    Ok(crate::primal_sockets::resolve_capability_socket_fallback(
        "crypto", &env,
    ))
}

#[derive(Deserialize)]
struct JsonLineClientHello {
    protocol: String,
    version: u32,
    client_ephemeral_pub: String,
}

#[derive(Deserialize)]
struct JsonLineChallengeResponse {
    response: String,
    preferred_cipher: String,
}

async fn send_error_line<S: AsyncWrite + Unpin>(
    stream: &mut S,
    reason: &str,
) -> Result<(), std::io::Error> {
    let v = serde_json::json!({
        "error": "handshake_failed",
        "reason": reason,
    });
    let mut buf = serde_json::to_vec(&v)?;
    buf.push(b'\n');
    stream.write_all(&buf).await?;
    stream.flush().await
}

fn parse_negotiated_cipher(raw: &str) -> Result<BtspCipher, BtspJsonLineError> {
    let n = raw.trim().to_ascii_lowercase().replace('-', "_");
    match n.as_str() {
        "chacha20_poly1305" => Ok(BtspCipher::Chacha20Poly1305),
        "hmac_plain" => Ok(BtspCipher::HmacPlain),
        "null" => Ok(BtspCipher::Null),
        _ => Err(BtspJsonLineError::Protocol(format!(
            "unknown negotiated cipher: {raw:?}"
        ))),
    }
}

fn require_str(
    obj: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, BtspJsonLineError> {
    match obj.get(key) {
        Some(Value::String(s)) if !s.is_empty() => Ok(s.clone()),
        Some(v) => Err(BtspJsonLineError::Protocol(format!(
            "field {key} must be a non-empty string, got {v}"
        ))),
        None => Err(BtspJsonLineError::Protocol(format!(
            "missing string field {key}"
        ))),
    }
}

async fn require_str_line<S: AsyncWrite + Unpin>(
    stream: &mut S,
    obj: &serde_json::Map<String, Value>,
    key: &str,
) -> Result<String, BtspJsonLineError> {
    match require_str(obj, key) {
        Ok(s) => Ok(s),
        Err(e) => {
            let _ = send_error_line(stream, &e.to_string()).await;
            Err(e)
        }
    }
}

/// 4-step JSON-line BTSP handshake relay:
/// 1. Parse ClientHello from the first line (already read)
/// 2. Call BearDog `btsp.session.create` with `family_seed` (base64-encoded)
/// 3. Send ServerHello as JSON line (challenge FROM BearDog, not self-generated)
/// 4. Read ChallengeResponse JSON line from client
/// 5. Call BearDog `btsp.session.verify` with session_token, response, client_ephemeral_pub, preferred_cipher
/// 6. Send HandshakeComplete JSON line
///
/// The entire handshake is bounded by `BTSP_HANDSHAKE_TIMEOUT` (default 5s,
/// override via `BTSP_HANDSHAKE_TIMEOUT_SECS`). Each BearDog RPC call is
/// individually bounded by `BTSP_RPC_TIMEOUT` (default 3s, override via
/// `BTSP_RPC_TIMEOUT_SECS`).
///
/// On error at any step, sends an error JSON line and returns `Err`.
pub async fn relay_json_line_handshake<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    first_line: &str,
    family_seed: &str,
    security_socket: &str,
) -> Result<BtspSessionInfo, BtspJsonLineError> {
    let budget = handshake_timeout();
    let Ok(result) = tokio::time::timeout(
        budget,
        relay_json_line_handshake_inner(stream, first_line, family_seed, security_socket),
    )
    .await
    else {
        let msg = format!("BTSP handshake exceeded {budget:?} budget");
        tracing::warn!(target: "btsp", "{msg}");
        let _ = send_error_line(stream, &msg).await;
        return Err(BtspJsonLineError::Timeout(msg));
    };
    result
}

async fn relay_json_line_handshake_inner<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    first_line: &str,
    family_seed: &str,
    security_socket: &str,
) -> Result<BtspSessionInfo, BtspJsonLineError> {
    let t0 = std::time::Instant::now();
    tracing::info!(target: "btsp", "JSON-line BTSP: parsing ClientHello");

    let hello: JsonLineClientHello = match serde_json::from_str(first_line.trim()) {
        Ok(h) => h,
        Err(e) => {
            let _ = send_error_line(stream, &format!("invalid ClientHello JSON: {e}")).await;
            return Err(e.into());
        }
    };

    if hello.version != super::types::BTSP_VERSION {
        let msg = format!(
            "version mismatch: expected {}, got {}",
            super::types::BTSP_VERSION,
            hello.version
        );
        let _ = send_error_line(stream, &msg).await;
        return Err(BtspJsonLineError::Protocol(msg));
    }

    if !hello.protocol.eq_ignore_ascii_case("btsp") {
        let msg = "protocol field must be btsp".to_string();
        let _ = send_error_line(stream, &msg).await;
        return Err(BtspJsonLineError::Protocol(msg));
    }

    // Single BearDog connection for both RPCs (SOURDOUGH_BTSP_RELAY_PATTERN §Part 2).
    let t_connect = std::time::Instant::now();
    let mut rpc = ConnectedJsonRpcClient::connect(security_socket)
        .await
        .map_err(|e| {
            BtspJsonLineError::Rpc(format!("BearDog connect to {security_socket}: {e}"))
        })?;
    tracing::debug!(
        target: "btsp",
        elapsed_ms = t_connect.elapsed().as_millis() as u64,
        "BearDog connected"
    );

    tracing::info!(target: "btsp", "JSON-line BTSP: calling btsp.session.create");

    let rpc_budget = rpc_timeout();
    let family_seed_b64 = base64::engine::general_purpose::STANDARD.encode(family_seed.as_bytes());
    let create_params = serde_json::json!({ "family_seed": family_seed_b64 });
    let t_create = std::time::Instant::now();
    let create_result: Value = match rpc
        .call_with_timeout("btsp.session.create", create_params, rpc_budget)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            let msg = e.to_string();
            let _ = send_error_line(stream, &msg).await;
            return Err(BtspJsonLineError::from(e));
        }
    };
    tracing::debug!(
        target: "btsp",
        elapsed_ms = t_create.elapsed().as_millis() as u64,
        "btsp.session.create completed"
    );

    let Some(create_obj) = create_result.as_object() else {
        let msg = "btsp.session.create result must be object";
        let _ = send_error_line(stream, msg).await;
        return Err(BtspJsonLineError::Protocol(msg.into()));
    };

    let session_token = match require_str(create_obj, "session_token") {
        Ok(t) => t,
        Err(_) => require_str_line(stream, create_obj, "session_id").await?,
    };
    let server_ephemeral_pub = require_str_line(stream, create_obj, "server_ephemeral_pub").await?;
    let challenge = require_str_line(stream, create_obj, "challenge").await?;

    let server_hello = serde_json::json!({
        "version": super::types::BTSP_VERSION,
        "server_ephemeral_pub": server_ephemeral_pub,
        "challenge": challenge,
    });
    let mut sh = serde_json::to_vec(&server_hello)?;
    sh.push(b'\n');
    stream.write_all(&sh).await?;
    stream.flush().await?;

    tracing::info!(target: "btsp", "JSON-line BTSP: reading ChallengeResponse line");

    let mut line = String::new();
    let mut b = [0u8; 1];
    loop {
        stream.read_exact(&mut b).await?;
        if b[0] == b'\n' {
            break;
        }
        line.push(char::from(b[0]));
    }

    let cr: JsonLineChallengeResponse = match serde_json::from_str(line.trim()) {
        Ok(v) => v,
        Err(e) => {
            let _ = send_error_line(stream, &format!("invalid ChallengeResponse: {e}")).await;
            return Err(e.into());
        }
    };

    tracing::info!(target: "btsp", "JSON-line BTSP: calling btsp.session.verify");

    let verify_params = serde_json::json!({
        "session_token": session_token,
        "response": cr.response,
        "client_ephemeral_pub": hello.client_ephemeral_pub,
        "preferred_cipher": cr.preferred_cipher,
    });
    let t_verify = std::time::Instant::now();
    let verify_result: Value = match rpc
        .call_with_timeout("btsp.session.verify", verify_params, rpc_budget)
        .await
    {
        Ok(v) => v,
        Err(e) => {
            let msg = e.to_string();
            let _ = send_error_line(stream, &msg).await;
            return Err(BtspJsonLineError::from(e));
        }
    };
    tracing::debug!(
        target: "btsp",
        elapsed_ms = t_verify.elapsed().as_millis() as u64,
        "btsp.session.verify completed"
    );

    let Some(verify_obj) = verify_result.as_object() else {
        let msg = "btsp.session.verify result must be object";
        let _ = send_error_line(stream, msg).await;
        return Err(BtspJsonLineError::Protocol(msg.into()));
    };

    let verified = verify_obj
        .get("verified")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    if !verified {
        let reason = verify_obj
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let msg = format!("verify failed: {reason}");
        let _ = send_error_line(stream, &msg).await;
        return Err(BtspJsonLineError::Protocol(msg));
    }

    let session_id = require_str_line(stream, verify_obj, "session_id").await?;
    let cipher_raw = require_str_line(stream, verify_obj, "cipher").await?;
    let cipher = match parse_negotiated_cipher(&cipher_raw) {
        Ok(c) => c,
        Err(e) => {
            let _ = send_error_line(stream, &e.to_string()).await;
            return Err(e);
        }
    };

    let complete = serde_json::json!({
        "status": "ok",
        "session_id": session_id,
        "cipher": cipher_raw,
    });
    let mut done = serde_json::to_vec(&complete)?;
    done.push(b'\n');
    stream.write_all(&done).await?;
    stream.flush().await?;

    tracing::info!(
        target: "btsp",
        session_id = %session_id,
        ?cipher,
        total_ms = t0.elapsed().as_millis() as u64,
        "JSON-line BTSP: handshake complete"
    );

    Ok(BtspSessionInfo { session_id, cipher })
}

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

async fn send_json_line<S: AsyncWrite + Unpin>(
    writer: &mut S,
    value: &Value,
) -> Result<(), BtspJsonLineError> {
    let mut buf = serde_json::to_vec(value)?;
    buf.push(b'\n');
    writer.write_all(&buf).await?;
    writer.flush().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_detection_positive() {
        assert!(line_looks_like_btsp_client_hello(
            r#"{"protocol":"btsp","version":1,"client_ephemeral_pub":"YQ=="}"#
        ));
        assert!(line_looks_like_btsp_client_hello(
            r#"{"protocol": "btsp", "version": 1, "client_ephemeral_pub": "abc"}"#
        ));
    }

    #[test]
    fn line_detection_negative() {
        assert!(!line_looks_like_btsp_client_hello(
            r#"{"jsonrpc":"2.0","method":"x"}"#
        ));
        assert!(!line_looks_like_btsp_client_hello(
            r#"{"protocol":"http","version":1}"#
        ));
        assert!(!line_looks_like_btsp_client_hello("not json"));
        assert!(!line_looks_like_btsp_client_hello(""));
    }

    #[tokio::test]
    async fn negotiate_chacha20_returns_negotiated_with_keys() {
        use base64::Engine;
        let client_nonce = super::super::phase3::generate_negotiate_nonce();
        let client_nonce_b64 = base64::engine::general_purpose::STANDARD.encode(client_nonce);

        let negotiate_line = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "btsp.negotiate",
            "params": {
                "session_id": "test-session-1",
                "ciphers": ["chacha20-poly1305"],
                "client_nonce": client_nonce_b64,
            },
            "id": 42
        })
        .to_string();

        let family_seed = "test-family-seed-for-negotiate";
        let mut response_buf: Vec<u8> = Vec::new();

        let outcome = try_handle_negotiate(&negotiate_line, &mut response_buf, family_seed)
            .await
            .expect("negotiate should succeed");

        assert!(
            matches!(outcome, NegotiateOutcome::Negotiated(_)),
            "expected Negotiated, got {outcome:?}",
        );

        let resp_str = String::from_utf8_lossy(&response_buf);
        let resp: serde_json::Value =
            serde_json::from_str(resp_str.trim()).expect("response should be valid JSON");
        assert_eq!(resp["id"], 42);
        assert_eq!(resp["result"]["cipher"], "chacha20-poly1305");
        assert!(
            resp["result"]["server_nonce"].is_string(),
            "server_nonce should be present"
        );
    }

    #[tokio::test]
    async fn negotiate_null_cipher_when_unsupported() {
        let negotiate_line = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "btsp.negotiate",
            "params": {
                "session_id": "test-session-2",
                "ciphers": ["aes-256-gcm"],
            },
            "id": 7
        })
        .to_string();

        let mut response_buf: Vec<u8> = Vec::new();
        let outcome = try_handle_negotiate(&negotiate_line, &mut response_buf, "seed")
            .await
            .expect("negotiate should succeed");

        assert!(matches!(outcome, NegotiateOutcome::NullCipher));

        let resp_str = String::from_utf8_lossy(&response_buf);
        let resp: serde_json::Value = serde_json::from_str(resp_str.trim()).expect("valid JSON");
        assert_eq!(resp["result"]["cipher"], "null");
    }

    #[tokio::test]
    async fn negotiate_not_negotiate_for_other_methods() {
        let line = r#"{"jsonrpc":"2.0","method":"health.liveness","id":1}"#;
        let mut buf: Vec<u8> = Vec::new();
        let outcome = try_handle_negotiate(line, &mut buf, "seed")
            .await
            .expect("should succeed");
        assert!(matches!(outcome, NegotiateOutcome::NotNegotiate));
        assert!(
            buf.is_empty(),
            "no response should be written for non-negotiate"
        );
    }

    #[tokio::test]
    async fn negotiate_not_negotiate_for_empty_line() {
        let mut buf: Vec<u8> = Vec::new();
        let outcome = try_handle_negotiate("", &mut buf, "seed")
            .await
            .expect("should succeed");
        assert!(matches!(outcome, NegotiateOutcome::NotNegotiate));
    }

    #[tokio::test]
    async fn negotiate_null_cipher_when_no_client_nonce() {
        let negotiate_line = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "btsp.negotiate",
            "params": {
                "session_id": "test-session-3",
                "ciphers": ["chacha20-poly1305"],
            },
            "id": 9
        })
        .to_string();

        let mut response_buf: Vec<u8> = Vec::new();
        let outcome = try_handle_negotiate(&negotiate_line, &mut response_buf, "seed")
            .await
            .expect("should succeed");
        assert!(matches!(outcome, NegotiateOutcome::NullCipher));
    }

    #[tokio::test]
    async fn negotiate_preferred_cipher_hyphen_variant() {
        use base64::Engine;
        let nonce = super::super::phase3::generate_negotiate_nonce();
        let nonce_b64 = base64::engine::general_purpose::STANDARD.encode(nonce);

        let line = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "btsp.negotiate",
            "params": {
                "session_id": "s4",
                "ciphers": [],
                "preferred_cipher": "chacha20-poly1305",
                "client_nonce": nonce_b64,
            },
            "id": 10
        })
        .to_string();

        let mut buf: Vec<u8> = Vec::new();
        let outcome = try_handle_negotiate(&line, &mut buf, "family-seed")
            .await
            .expect("should succeed");
        assert!(matches!(outcome, NegotiateOutcome::Negotiated(_)));
    }

    #[tokio::test]
    async fn negotiate_preferred_cipher_underscore_variant() {
        use base64::Engine;
        let nonce = super::super::phase3::generate_negotiate_nonce();
        let nonce_b64 = base64::engine::general_purpose::STANDARD.encode(nonce);

        let line = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "btsp.negotiate",
            "params": {
                "session_id": "s5",
                "ciphers": [],
                "preferred_cipher": "chacha20_poly1305",
                "client_nonce": nonce_b64,
            },
            "id": 11
        })
        .to_string();

        let mut buf: Vec<u8> = Vec::new();
        let outcome = try_handle_negotiate(&line, &mut buf, "family-seed")
            .await
            .expect("should succeed");
        assert!(matches!(outcome, NegotiateOutcome::Negotiated(_)));
    }

    /// Full E2E: negotiate → derive client keys → encrypted frame exchange.
    ///
    /// Simulates the complete Phase 3 transport switch: the server handles
    /// `btsp.negotiate`, both sides derive keys, and subsequent messages use
    /// encrypted framing exclusively.
    #[tokio::test]
    async fn negotiate_then_encrypted_frame_exchange() {
        use super::super::framing;
        use super::super::phase3;
        use base64::Engine;

        let family_seed = "e2e-test-family-seed";
        let client_nonce = phase3::generate_negotiate_nonce();
        let client_nonce_b64 = base64::engine::general_purpose::STANDARD.encode(client_nonce);

        let negotiate_req = serde_json::json!({
            "jsonrpc": "2.0",
            "method": "btsp.negotiate",
            "params": {
                "session_id": "e2e-session",
                "ciphers": ["chacha20-poly1305"],
                "client_nonce": client_nonce_b64,
            },
            "id": 100
        })
        .to_string();

        // --- Server side: handle negotiate ---
        let mut server_response: Vec<u8> = Vec::new();
        let outcome = try_handle_negotiate(&negotiate_req, &mut server_response, family_seed)
            .await
            .expect("negotiate");

        let server_keys = match outcome {
            NegotiateOutcome::Negotiated(keys) => keys,
            other => panic!("expected Negotiated, got {other:?}"),
        };

        // --- Client side: parse response, derive keys ---
        let resp_str = String::from_utf8_lossy(&server_response);
        let resp: serde_json::Value = serde_json::from_str(resp_str.trim()).expect("parse");
        let server_nonce_b64 = resp["result"]["server_nonce"]
            .as_str()
            .expect("server_nonce");
        let server_nonce = base64::engine::general_purpose::STANDARD
            .decode(server_nonce_b64)
            .expect("decode nonce");

        let handshake_key = phase3::derive_handshake_key(family_seed.as_bytes()).expect("hk");
        let client_keys =
            phase3::Phase3SessionKeys::derive(&handshake_key, &client_nonce, &server_nonce, false)
                .expect("client derive");

        // --- Verify key symmetry ---
        assert_eq!(
            server_keys.encrypt_key, client_keys.decrypt_key,
            "server encrypt = client decrypt"
        );
        assert_eq!(
            server_keys.decrypt_key, client_keys.encrypt_key,
            "server decrypt = client encrypt"
        );

        // --- Client sends encrypted JSON-RPC request ---
        let request = b"{\"jsonrpc\":\"2.0\",\"method\":\"health.liveness\",\"id\":1}";
        let mut wire = Vec::new();
        framing::write_encrypted_frame(&mut wire, &client_keys, request)
            .await
            .expect("client write");

        // --- Server reads and decrypts ---
        let mut cursor = std::io::Cursor::new(wire);
        let server_plaintext = framing::read_encrypted_frame(&mut cursor, &server_keys)
            .await
            .expect("server read");
        assert_eq!(server_plaintext, request);

        // --- Server sends encrypted response ---
        let response = b"{\"jsonrpc\":\"2.0\",\"result\":{\"status\":\"alive\"},\"id\":1}";
        let mut wire2 = Vec::new();
        framing::write_encrypted_frame(&mut wire2, &server_keys, response)
            .await
            .expect("server write");

        // --- Client reads and decrypts ---
        let mut cursor2 = std::io::Cursor::new(wire2);
        let client_plaintext = framing::read_encrypted_frame(&mut cursor2, &client_keys)
            .await
            .expect("client read");
        assert_eq!(client_plaintext, response);
    }
}
