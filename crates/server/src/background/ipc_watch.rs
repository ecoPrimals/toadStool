// SPDX-License-Identifier: AGPL-3.0-or-later
//! Communication provider `ipc.watch` background poller.
//!
//! Polls the communication provider's `ipc.watch` method for `shader` capability registration events.
//! When a new shader provider registers (e.g. the shader compiler coming online), invalidates
//! the [`VisualizationClient`] cache so the dispatch handler re-discovers the provider
//! on its next call.
//!
//! This resolves GAP-HS-119: toadStool's dispatch path previously cached the first
//! discovery result permanently via `OnceCell`. If the shader compiler wasn't running at
//! toadStool startup, shader dispatch was permanently broken until restart.

use crate::visualization_client::SharedVisualizationClient;
use std::time::Duration;
use toadstool_common::primal_sockets::{SocketPathEnv, resolve_capability_socket_fallback};
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;
use tracing::{debug, info, warn};

const POLL_INTERVAL: Duration = Duration::from_secs(10);
const POLL_INTERVAL_NO_DISCOVERY: Duration = Duration::from_secs(30);
const SHADER_CAPABILITY: &str =
    toadstool_common::constants::primal_identity::capability::SHADER_COMPILER;

/// Mutable state carried across poll iterations.
struct WatcherState {
    revision: u64,
    discovery_available: bool,
}

/// Process a single `ipc.watch` response, returning `true` if a `registered`
/// event triggered cache invalidation.
///
/// Extracted from the event loop to enable unit testing of event processing.
async fn process_response(
    response: &serde_json::Value,
    state: &mut WatcherState,
    shader_client: &SharedVisualizationClient,
) -> bool {
    if let Some(new_rev) = response.get("revision").and_then(serde_json::Value::as_u64) {
        state.revision = new_rev;
    }

    let mut invalidated = false;

    if let Some(events) = response.get("events").and_then(|v| v.as_array()) {
        for event in events {
            let kind = event
                .get("kind")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let primal = event
                .get("primal")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            if kind == "registered" {
                info!(
                    primal,
                    kind,
                    state.revision,
                    "ipc.watch: shader provider registered — invalidating cache"
                );
                shader_client.invalidate().await;
                invalidated = true;
            } else {
                debug!(
                    primal,
                    kind, state.revision, "ipc.watch: shader provider event"
                );
            }
        }
    }

    invalidated
}

/// Run the `ipc.watch` background poller.
///
/// Connects to the discovery capability socket and polls `ipc.watch` for shader
/// capability registrations. On detection, invalidates the visualization client
/// cache to trigger re-discovery.
pub async fn run(shader_client: SharedVisualizationClient) {
    info!("ipc.watch background poller starting (watching for shader capability)");

    let mut state = WatcherState {
        revision: 0,
        discovery_available: false,
    };

    loop {
        let env = SocketPathEnv::from_env();
        let discovery_socket = resolve_capability_socket_fallback("discovery", &env);

        if !discovery_socket.exists() {
            if state.discovery_available {
                warn!(
                    path = %discovery_socket.display(),
                    "discovery service socket disappeared — will retry"
                );
                state.discovery_available = false;
            }
            tokio::time::sleep(POLL_INTERVAL_NO_DISCOVERY).await;
            continue;
        }

        let client = UnixJsonRpcClient::new(&discovery_socket);

        let params = serde_json::json!({
            "since_revision": state.revision,
            "capabilities": [SHADER_CAPABILITY],
        });

        match client
            .call_with_timeout("ipc.watch", params, Duration::from_secs(5))
            .await
        {
            Ok(response) => {
                if !state.discovery_available {
                    info!(
                        path = %discovery_socket.display(),
                        "discovery ipc.watch connected"
                    );
                    state.discovery_available = true;
                }

                process_response(&response, &mut state, &shader_client).await;
            }
            Err(e) => {
                if state.discovery_available {
                    debug!(
                        error = %e,
                        "ipc.watch poll failed — discovery service may be restarting"
                    );
                    state.discovery_available = false;
                }
            }
        }

        let interval = if state.discovery_available {
            POLL_INTERVAL
        } else {
            POLL_INTERVAL_NO_DISCOVERY
        };
        tokio::time::sleep(interval).await;
    }
}

#[cfg(test)]
#[path = "ipc_watch_tests.rs"]
mod tests;
