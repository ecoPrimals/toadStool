// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON newline BTSP handshake types, helpers, and line-level utilities.
//!
//! The full relay handshake lives in [`super::relay`]; the Phase 3 negotiate
//! handler lives in [`super::negotiate`]. This module provides the shared
//! types, error enum, line parsing, and socket resolution used by both.

use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::ToadStoolError;
use crate::constants::timeouts;
use crate::interned_strings::socket_env;

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

// ── Timeout helpers (pub(crate) so relay.rs can use them) ──────────────

pub(crate) fn handshake_timeout() -> Duration {
    std::env::var(socket_env::BTSP_HANDSHAKE_TIMEOUT_SECS)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(timeouts::BTSP_HANDSHAKE_TIMEOUT, Duration::from_secs)
}

pub(crate) fn rpc_timeout() -> Duration {
    std::env::var(socket_env::BTSP_RPC_TIMEOUT_SECS)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .map_or(timeouts::BTSP_RPC_TIMEOUT, Duration::from_secs)
}

// ── Line parsing ───────────────────────────────────────────────────────

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

// ── Socket resolution ──────────────────────────────────────────────────

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

// ── Shared helpers (pub(crate) for relay + negotiate) ──────────────────

pub(crate) async fn send_error_line<S: AsyncWrite + Unpin>(
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

pub(crate) fn parse_negotiated_cipher(raw: &str) -> Result<BtspCipher, BtspJsonLineError> {
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

pub(crate) fn require_str(
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

pub(crate) async fn require_str_line<S: AsyncWrite + Unpin>(
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

pub(crate) async fn send_json_line<S: AsyncWrite + Unpin>(
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
            r#"{"protocol":"btsp","version":1,"client_ephemeral_pub":"AAAA"}"#
        ));
    }

    #[test]
    fn line_detection_case_insensitive() {
        assert!(line_looks_like_btsp_client_hello(
            r#"{"protocol":"BTSP","version":1}"#
        ));
    }

    #[test]
    fn line_detection_extra_whitespace() {
        assert!(line_looks_like_btsp_client_hello(
            r#"  {"protocol": "btsp", "version": 1}  "#
        ));
    }

    #[test]
    fn line_detection_not_json() {
        assert!(!line_looks_like_btsp_client_hello("hello world"));
    }

    #[test]
    fn line_detection_not_btsp() {
        assert!(!line_looks_like_btsp_client_hello(
            r#"{"protocol":"http","version":1}"#
        ));
    }

    #[test]
    fn line_detection_no_protocol() {
        assert!(!line_looks_like_btsp_client_hello(r#"{"version":1}"#));
    }

    #[test]
    fn line_detection_empty() {
        assert!(!line_looks_like_btsp_client_hello(""));
    }

    #[test]
    fn line_detection_json_object_no_protocol_field() {
        assert!(!line_looks_like_btsp_client_hello(
            r#"{"method":"rpc","id":1}"#
        ));
    }

    #[test]
    fn parse_cipher_chacha() {
        assert_eq!(
            parse_negotiated_cipher("chacha20-poly1305").unwrap(),
            BtspCipher::Chacha20Poly1305
        );
    }

    #[test]
    fn parse_cipher_chacha_underscore() {
        assert_eq!(
            parse_negotiated_cipher("chacha20_poly1305").unwrap(),
            BtspCipher::Chacha20Poly1305
        );
    }

    #[test]
    fn parse_cipher_hmac() {
        assert_eq!(
            parse_negotiated_cipher("hmac_plain").unwrap(),
            BtspCipher::HmacPlain
        );
    }

    #[test]
    fn parse_cipher_null() {
        assert_eq!(parse_negotiated_cipher("null").unwrap(), BtspCipher::Null);
    }

    #[test]
    fn parse_cipher_unknown() {
        assert!(parse_negotiated_cipher("aes-gcm").is_err());
    }

    #[test]
    fn parse_cipher_case_insensitive() {
        assert_eq!(
            parse_negotiated_cipher("CHACHA20-POLY1305").unwrap(),
            BtspCipher::Chacha20Poly1305
        );
    }

    #[test]
    fn require_str_present() {
        let mut map = serde_json::Map::new();
        map.insert("key".to_string(), Value::String("value".to_string()));
        assert_eq!(require_str(&map, "key").unwrap(), "value");
    }

    #[test]
    fn require_str_missing() {
        let map = serde_json::Map::new();
        assert!(require_str(&map, "key").is_err());
    }

    #[test]
    fn require_str_empty() {
        let mut map = serde_json::Map::new();
        map.insert("key".to_string(), Value::String(String::new()));
        assert!(require_str(&map, "key").is_err());
    }

    #[test]
    fn require_str_not_string() {
        let mut map = serde_json::Map::new();
        map.insert("key".to_string(), Value::Number(42.into()));
        assert!(require_str(&map, "key").is_err());
    }

    #[tokio::test]
    async fn read_line_suffix_basic() {
        let data = b"hello world\n";
        let mut cursor = std::io::Cursor::new(data);
        let line = read_line_suffix(&mut cursor).await.unwrap();
        assert_eq!(line, "hello world");
    }

    #[tokio::test]
    async fn read_full_line_with_first_byte() {
        let data = b"ello\n";
        let mut cursor = std::io::Cursor::new(data);
        let line = read_full_line_after_first_byte(&mut cursor, b'h')
            .await
            .unwrap();
        assert_eq!(line, "hello");
    }

    #[tokio::test]
    async fn send_error_line_format() {
        let mut buf = Vec::new();
        send_error_line(&mut buf, "test error").await.unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.contains("handshake_failed"));
        assert!(s.contains("test error"));
        assert!(s.ends_with('\n'));
    }

    #[tokio::test]
    async fn send_json_line_format() {
        let mut buf = Vec::new();
        let val = serde_json::json!({"key": "value"});
        send_json_line(&mut buf, &val).await.unwrap();
        let s = String::from_utf8(buf).unwrap();
        assert!(s.ends_with('\n'));
        let parsed: Value = serde_json::from_str(s.trim()).unwrap();
        assert_eq!(parsed["key"], "value");
    }

    // ── NegotiateOutcome tests (from original json_line.rs) ────────────

    use super::super::negotiate::NegotiateOutcome;

    #[test]
    fn negotiate_outcome_debug_redacts_keys() {
        let dbg = format!("{:?}", NegotiateOutcome::NullCipher);
        assert_eq!(dbg, "NullCipher");
    }

    #[test]
    fn negotiate_outcome_not_negotiate_debug() {
        let dbg = format!("{:?}", NegotiateOutcome::NotNegotiate);
        assert_eq!(dbg, "NotNegotiate");
    }

    #[tokio::test]
    async fn try_handle_negotiate_empty_line() {
        let mut buf = Vec::new();
        let result = super::super::negotiate::try_handle_negotiate("", &mut buf, "seed").await;
        assert!(matches!(result, Ok(NegotiateOutcome::NotNegotiate)));
    }

    #[tokio::test]
    async fn try_handle_negotiate_non_json() {
        let mut buf = Vec::new();
        let result =
            super::super::negotiate::try_handle_negotiate("not json", &mut buf, "seed").await;
        assert!(matches!(result, Ok(NegotiateOutcome::NotNegotiate)));
    }

    #[tokio::test]
    async fn try_handle_negotiate_different_method() {
        let line = r#"{"jsonrpc":"2.0","method":"health.check","id":1}"#;
        let mut buf = Vec::new();
        let result = super::super::negotiate::try_handle_negotiate(line, &mut buf, "seed").await;
        assert!(matches!(result, Ok(NegotiateOutcome::NotNegotiate)));
    }

    #[tokio::test]
    async fn try_handle_negotiate_invalid_params() {
        let line = r#"{"jsonrpc":"2.0","method":"btsp.negotiate","params":"bad","id":1}"#;
        let mut buf = Vec::new();
        let result = super::super::negotiate::try_handle_negotiate(line, &mut buf, "seed").await;
        assert!(result.is_err());
        let written = String::from_utf8(buf).unwrap();
        assert!(written.contains("-32602"));
    }

    #[tokio::test]
    async fn try_handle_negotiate_null_cipher_no_chacha() {
        let line = r#"{"jsonrpc":"2.0","method":"btsp.negotiate","params":{"session_id":"s1","ciphers":["aes-gcm"],"client_nonce":"AAAA"},"id":1}"#;
        let mut buf = Vec::new();
        let result = super::super::negotiate::try_handle_negotiate(line, &mut buf, "seed").await;
        assert!(matches!(result, Ok(NegotiateOutcome::NullCipher)));
    }

    #[tokio::test]
    async fn try_handle_negotiate_null_cipher_no_nonce() {
        let line = r#"{"jsonrpc":"2.0","method":"btsp.negotiate","params":{"session_id":"s1","ciphers":["chacha20-poly1305"]},"id":1}"#;
        let mut buf = Vec::new();
        let result = super::super::negotiate::try_handle_negotiate(line, &mut buf, "seed").await;
        assert!(matches!(result, Ok(NegotiateOutcome::NullCipher)));
    }

    #[tokio::test]
    async fn try_handle_negotiate_full_e2e() {
        use base64::Engine;
        let nonce_b64 = base64::engine::general_purpose::STANDARD.encode([42u8; 32]);
        let line = format!(
            r#"{{"jsonrpc":"2.0","method":"btsp.negotiate","params":{{"session_id":"s1","ciphers":["chacha20-poly1305"],"client_nonce":"{nonce_b64}"}},"id":1}}"#
        );
        let mut buf = Vec::new();
        let result =
            super::super::negotiate::try_handle_negotiate(&line, &mut buf, "test-seed").await;
        match result {
            Ok(NegotiateOutcome::Negotiated(_keys)) => {
                let written = String::from_utf8(buf).unwrap();
                assert!(written.contains("chacha20-poly1305"));
                assert!(written.contains("server_nonce"));
            }
            other => panic!("Expected Negotiated, got {other:?}"),
        }
    }
}
