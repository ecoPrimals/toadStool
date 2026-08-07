// SPDX-License-Identifier: AGPL-3.0-or-later
//! TCP listener and per-connection handling for Pure JSON-RPC.
//!
//! Protocol dispatch delegates to `dispatch.rs` (G66 transport abstraction).

use std::sync::Arc;
use std::time::Duration;
use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};
use tracing::{debug, error, info, warn};

use crate::errors::{ServerError, ServerResult};
use crate::pure_jsonrpc::JsonRpcHandler;
use crate::pure_jsonrpc::handler::ConnectionTrustHints;
use toadstool_common::interned_strings::socket_env;

pub(crate) fn tcp_idle_timeout() -> Duration {
    let secs = std::env::var(socket_env::TOADSTOOL_TCP_IDLE_TIMEOUT_SECS)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(toadstool_config::defaults::network::TCP_IDLE_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// Serve JSON-RPC on a TCP listener (isomorphic fallback).
///
/// # Errors
///
/// Returns [`ServerError`] if getting local address fails.
pub async fn serve_tcp(handler: Arc<JsonRpcHandler>, listener: TcpListener) -> ServerResult<()> {
    let local_addr = listener
        .local_addr()
        .map_err(|e| ServerError::Network(e.to_string()))?;
    info!(
        "✅ Pure JSON-RPC 2.0 server listening on TCP: {}",
        local_addr
    );

    loop {
        match listener.accept().await {
            Ok((stream, addr)) => {
                let _ = stream.set_nodelay(true);
                let handler = Arc::clone(&handler);
                tokio::spawn(async move {
                    if let Err(e) = handle_tcp_connection(handler, stream).await {
                        debug!("TCP connection from {addr} ended: {e}");
                    }
                });
            }
            Err(e) => error!("TCP accept error: {}", e),
        }
    }
}

/// Handle a single TCP connection with persistent keep-alive.
///
/// Detects riboCipher transport signal before protocol dispatch per
/// `RIBOCIPHER_TRANSPORT_SIGNAL_STANDARD.md`.
pub(crate) async fn handle_tcp_connection(
    handler: Arc<JsonRpcHandler>,
    mut stream: TcpStream,
) -> ServerResult<()> {
    use super::ribocipher;

    let idle_timeout = tcp_idle_timeout();

    let mut first = [0u8; 1];
    let n = match tokio::time::timeout(
        idle_timeout,
        tokio::io::AsyncReadExt::read(&mut stream, &mut first),
    )
    .await
    {
        Ok(Ok(n)) => n,
        Ok(Err(e)) => return Err(ServerError::Network(e.to_string())),
        Err(_) => {
            return Err(ServerError::Network(
                "TCP idle timeout on initial read".into(),
            ));
        }
    };
    if n == 0 {
        return Ok(());
    }

    match first[0] {
        ribocipher::CLEAR => {
            let mut pt = [0u8; 1];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut pt)
                .await
                .map_err(|e| {
                    ServerError::Network(format!("riboCipher: failed to read protocol type: {e}"))
                })?;
            info!(
                protocol_type = format_args!("0x{:02X}", pt[0]),
                "riboCipher clear signal on TCP"
            );
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            return super::dispatch::handle_ribocipher_clear(
                handler,
                &mut reader,
                &mut writer,
                pt[0],
                ConnectionTrustHints::TCP,
                Some(idle_timeout),
            )
            .await;
        }
        ribocipher::MITO => {
            let mut hmac_tag = [0u8; 4];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut hmac_tag)
                .await
                .map_err(|e| {
                    ServerError::Network(format!("riboCipher mito: failed to read HMAC tag: {e}"))
                })?;
            let mut pt = [0u8; 1];
            tokio::io::AsyncReadExt::read_exact(&mut stream, &mut pt)
                .await
                .map_err(|e| {
                    ServerError::Network(format!(
                        "riboCipher mito: failed to read protocol type: {e}"
                    ))
                })?;
            info!(
                protocol_type = format_args!("0x{:02X}", pt[0]),
                hmac = format_args!(
                    "{:02x}{:02x}{:02x}{:02x}",
                    hmac_tag[0], hmac_tag[1], hmac_tag[2], hmac_tag[3]
                ),
                "riboCipher mito-beacon signal accepted on TCP"
            );
            let (reader, mut writer) = stream.into_split();
            let mut reader = BufReader::new(reader);
            return super::dispatch::handle_ribocipher_clear(
                handler,
                &mut reader,
                &mut writer,
                pt[0],
                ConnectionTrustHints::TCP,
                Some(idle_timeout),
            )
            .await;
        }
        ribocipher::NUCLEAR => {
            warn!("riboCipher nuclear tier not yet supported on TCP — rejecting");
            let reject = serde_json::json!({
                "jsonrpc": "2.0",
                "error": {"code": -32600, "message": "riboCipher nuclear tier not yet supported"},
                "id": null
            });
            let mut buf = serde_json::to_vec(&reject).unwrap_or_default();
            buf.push(b'\n');
            let _ = tokio::io::AsyncWriteExt::write_all(&mut stream, &buf).await;
            let _ = tokio::io::AsyncWriteExt::flush(&mut stream).await;
            return Ok(());
        }
        other => {
            debug!(
                first_byte = format_args!("0x{:02X}", other),
                "riboCipher TCP: unhandled signal byte, falling through to unsignalled rejection"
            );
        }
    }

    // Wave 113: REJECT unsignalled connections
    error!(
        first_byte = format_args!("0x{:02X}", first[0]),
        "REJECTED: unsignalled TCP connection (no riboCipher prefix). \
         Clients MUST prepend [0xEC, 0x01]."
    );
    let (_, mut writer) = stream.into_split();
    let _ = super::dispatch::write_reject_response(&mut writer).await;
    Ok(())
}
