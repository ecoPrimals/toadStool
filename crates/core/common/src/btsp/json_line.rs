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
}
