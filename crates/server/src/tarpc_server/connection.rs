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
                info!("tarpc connection idle for {}s — closing", idle_timeout.as_secs());
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
        let family_seed = match resolve_family_seed_for_tarpc() {
            Ok(s) => s,
            Err(e) => {
                error!("BTSP: cannot resolve family seed for tarpc: {e}");
                return Err(());
            }
        };

        match toadstool_common::btsp::BtspServer::accept_handshake(&mut stream, &family_seed).await
        {
            Ok(session) => {
                info!(
                    "🔒 BTSP tarpc handshake complete: cipher={}, session_id={:02x?}",
                    session.cipher.as_str(),
                    &session.session_id[..4]
                );
                Ok(stream)
            }
            Err(e) => {
                warn!("🔒 BTSP handshake rejected (tarpc): {e}");
                let _ = toadstool_common::btsp::BtspServer::send_handshake_error(&mut stream).await;
                Err(())
            }
        }
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
