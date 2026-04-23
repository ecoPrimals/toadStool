// SPDX-License-Identifier: AGPL-3.0-or-later
//! Unix/TCP accept path: BTSP optional handshake, then length-delimited tarpc framing.

use futures::StreamExt;
use tarpc::server::{BaseChannel, Channel};
use tokio_serde::formats::Json;
use tracing::{error, info, warn};

use crate::errors::{ServerError, ServerResult};
use toadstool_integration_protocols::tarpc_service::ToadStoolComputeRpc;

use super::ToadStoolTarpcServer;

/// Run tarpc on an already-connected byte stream (length-delimited JSON).
pub(super) async fn serve_on_tarpc_channel<S>(server: ToadStoolTarpcServer, stream: S)
where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static,
{
    let framed = tokio_util::codec::LengthDelimitedCodec::builder().new_framed(stream);
    let transport = tokio_serde::Framed::new(framed, Json::<_, _>::default());

    let channel = BaseChannel::with_defaults(transport);
    channel
        .execute(server.serve())
        .for_each(|rpc| async {
            tokio::spawn(rpc);
        })
        .await;
}

/// Like [`serve_on_tarpc_channel`] but closes the connection if no RPC arrives
/// within `idle_timeout`. The timer resets after each RPC, so active connections
/// are never killed — only truly idle ones.
pub(super) async fn serve_on_tarpc_channel_with_idle_timeout<S>(
    server: ToadStoolTarpcServer,
    stream: S,
    idle_timeout: std::time::Duration,
) where
    S: tokio::io::AsyncRead + tokio::io::AsyncWrite + Send + Unpin + 'static,
{
    let framed = tokio_util::codec::LengthDelimitedCodec::builder().new_framed(stream);
    let transport = tokio_serde::Framed::new(framed, Json::<_, _>::default());

    let channel = BaseChannel::with_defaults(transport);
    let mut rpcs = std::pin::pin!(channel.execute(server.serve()));

    loop {
        match tokio::time::timeout(idle_timeout, rpcs.next()).await {
            Ok(Some(rpc)) => {
                tokio::spawn(rpc);
            }
            Ok(None) => break,
            Err(_) => {
                info!(
                    "tarpc connection idle for {}s — closing",
                    idle_timeout.as_secs()
                );
                break;
            }
        }
    }
}

/// When `FAMILY_ID` is set, run BTSP before exposing tarpc length-delimited framing (BTSP Phase 2).
pub(super) async fn unix_maybe_btsp_before_tarpc(
    mut stream: tokio::net::UnixStream,
    btsp_required: bool,
) -> Result<tokio::net::UnixStream, ()> {
    if !btsp_required {
        return Ok(stream);
    }

    #[cfg(feature = "btsp")]
    {
        use toadstool_common::btsp;
        use tokio::io::AsyncReadExt;

        let mut first = [0u8; 1];
        let n = match stream.read(&mut first).await {
            Ok(n) => n,
            Err(e) => {
                warn!("BTSP tarpc: read first byte: {e}");
                return Err(());
            }
        };
        if n == 0 {
            return Err(());
        }

        let stream = if first[0] >= 0x09 {
            let first_line =
                match btsp::read_full_line_after_first_byte(&mut stream, first[0]).await {
                    Ok(l) => l,
                    Err(e) => {
                        warn!("BTSP tarpc: read first line: {e}");
                        return Err(());
                    }
                };
            if btsp::line_looks_like_btsp_client_hello(&first_line) {
                let family_seed_b64 = match btsp::load_family_seed_for_btsp() {
                    Ok(s) => s,
                    Err(e) => {
                        error!("BTSP tarpc: family seed: {e}");
                        return Err(());
                    }
                };
                let sec = match btsp::resolve_security_socket_path() {
                    Ok(p) => p,
                    Err(e) => {
                        error!("BTSP tarpc: security socket: {e}");
                        return Err(());
                    }
                };
                let sec_s = sec.to_string_lossy().into_owned();
                match btsp::relay_json_line_handshake(
                    &mut stream,
                    first_line.trim_end(),
                    &family_seed_b64,
                    &sec_s,
                )
                .await
                {
                    Ok(btsp_info) => {
                        info!(
                            target: "btsp",
                            "🔒 BTSP JSON-line tarpc handshake complete: cipher={}, session_id={}",
                            btsp_info.cipher.as_str(),
                            btsp_info.session_id
                        );
                    }
                    Err(e) => {
                        warn!("BTSP tarpc JSON-line handshake failed: {e}");
                        return Err(());
                    }
                }
                stream
            } else {
                warn!(
                    target: "btsp",
                    "BTSP tarpc: plaintext first line is not JSON-line BTSP — closing"
                );
                return Err(());
            }
        } else {
            let family_seed = match resolve_family_seed_for_tarpc() {
                Ok(s) => s,
                Err(e) => {
                    error!("BTSP: cannot resolve family seed for tarpc: {e}");
                    return Err(());
                }
            };

            let mut wrapped = btsp::framing::PrependByte::new(first[0], stream);
            match btsp::BtspServer::accept_handshake(&mut wrapped, &family_seed).await {
                Ok(session) => {
                    info!(
                        "🔒 BTSP tarpc handshake complete: cipher={}, session_id={:02x?}",
                        session.cipher.as_str(),
                        &session.session_id[..4]
                    );
                }
                Err(e) => {
                    warn!("🔒 BTSP handshake rejected (tarpc): {e}");
                    let _ = btsp::BtspServer::send_handshake_error(&mut wrapped).await;
                    return Err(());
                }
            }
            wrapped.into_inner()
        };

        Ok(stream)
    }

    #[cfg(not(feature = "btsp"))]
    {
        warn!("BTSP required but `btsp` feature not enabled — closing tarpc connection");
        Err(())
    }
}

#[cfg(feature = "btsp")]
fn resolve_family_seed_for_tarpc() -> ServerResult<Vec<u8>> {
    if let Ok(seed) = std::env::var("FAMILY_SEED") {
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
