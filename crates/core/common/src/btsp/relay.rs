// SPDX-License-Identifier: AGPL-3.0-or-later
//! 4-step JSON-line BTSP handshake relay via crypto provider JSON-RPC.
//!
//! Extracted from `json_line.rs` for cohesion: this module owns the full
//! relay flow (parse `ClientHello` → crypto `btsp.session.create` →
//! `ServerHello` → `ChallengeResponse` → `btsp.session.verify` → complete).

#[cfg(unix)]
use base64::Engine;
#[cfg(unix)]
use serde::Deserialize;
#[cfg(unix)]
use serde_json::Value;
#[cfg(unix)]
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

#[cfg(unix)]
use crate::unix_jsonrpc::ConnectedJsonRpcClient;

#[cfg(unix)]
use super::json_line::{
    BtspJsonLineError, BtspSessionInfo, handshake_timeout, parse_negotiated_cipher, require_str,
    require_str_line, rpc_timeout, send_error_line,
};

#[cfg(unix)]
#[derive(Deserialize)]
struct JsonLineClientHello {
    protocol: String,
    version: u32,
    client_ephemeral_pub: String,
}

#[cfg(unix)]
#[derive(Deserialize)]
struct JsonLineChallengeResponse {
    response: String,
    preferred_cipher: String,
}

/// 4-step JSON-line BTSP handshake relay:
/// 1. Parse `ClientHello` from the first line (already read)
/// 2. Call crypto provider `btsp.session.create` with `family_seed` (base64-encoded)
/// 3. Send `ServerHello` as JSON line (challenge from crypto provider, not self-generated)
/// 4. Read `ChallengeResponse` JSON line from client
/// 5. Call crypto provider `btsp.session.verify` with session_token, response, client_ephemeral_pub, preferred_cipher
/// 6. Send `HandshakeComplete` JSON line
///
/// The entire handshake is bounded by `BTSP_HANDSHAKE_TIMEOUT` (default 5s,
/// override via `BTSP_HANDSHAKE_TIMEOUT_SECS`). Each crypto RPC call is
/// individually bounded by `BTSP_RPC_TIMEOUT` (default 3s, override via
/// `BTSP_RPC_TIMEOUT_SECS`).
///
/// On error at any step, sends an error JSON line and returns `Err`.
#[cfg(unix)]
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

#[cfg(unix)]
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

    // Single crypto provider connection for both RPCs (SOURDOUGH_BTSP_RELAY_PATTERN §Part 2).
    let t_connect = std::time::Instant::now();
    let mut rpc = ConnectedJsonRpcClient::connect(security_socket)
        .await
        .map_err(|e| {
            BtspJsonLineError::Rpc(format!("crypto provider connect to {security_socket}: {e}"))
        })?;
    tracing::debug!(
        target: "btsp",
        elapsed_ms = t_connect.elapsed().as_millis() as u64,
        "crypto provider connected"
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
