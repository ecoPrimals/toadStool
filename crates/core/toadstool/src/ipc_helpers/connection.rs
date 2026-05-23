// SPDX-License-Identifier: AGPL-3.0-or-later
//! Connection establishment and Unix socket helpers for primal-to-primal IPC

use serde_json::Value;
use serde_json::json;
use std::time::Duration;
use toadstool_common::uid_detector;
use tokio::net::UnixStream;
use tokio::time::timeout;
use tracing::{debug, info};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::PRIMAL_NAME;
use toadstool_common::constants::timeouts;
use toadstool_common::primal_sockets::{
    SocketPathEnv, resolve_capability_socket_fallback, resolve_toadstool_socket,
};

use super::framing;

/// Request timeout for IPC operations (from config defaults)
pub const IPC_TIMEOUT: Duration = timeouts::TCP_CONNECT_TIMEOUT;

/// Get runtime directory: `$XDG_RUNTIME_DIR` → `$BIOMEOS_RUNTIME_DIR` → `/run/user/$UID` → temp dir.
fn get_runtime_dir() -> String {
    std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("BIOMEOS_RUNTIME_DIR"))
        .unwrap_or_else(|_| {
            uid_detector::get_user_id().map_or_else(
                |_| {
                    std::env::temp_dir()
                        .join("biomeos-runtime")
                        .to_string_lossy()
                        .to_string()
                },
                |uid| format!("/run/user/{uid}"),
            )
        })
}

/// Default coordination-capability socket path.
///
/// biomeOS convention: `$XDG_RUNTIME_DIR/biomeos/coordination.sock`
pub fn get_default_coordination_socket() -> String {
    format!("{}/biomeos/coordination.sock", get_runtime_dir())
}

/// Self-register with Songbird via `DISCOVERY_SOCKET` (preferred) or coordination fallback.
///
/// Sends `ipc.register` so Songbird can resolve `toadstool` by capability
/// without the composition launcher doing it on our behalf. Fire-and-forget
/// at the call site — if this fails the primal continues in standalone mode.
///
/// # Errors
///
/// Returns error if the discovery service is unreachable, JSON-RPC framing
/// fails, or registration is rejected.
pub async fn register_with_discovery() -> ToadStoolResult<()> {
    let env = SocketPathEnv::from_env();
    let discovery_path = resolve_capability_socket_fallback("discovery", &env);
    let socket_path = discovery_path.to_string_lossy().to_string();

    info!("Self-registering with discovery service at {}", socket_path);

    let mut stream = timeout(IPC_TIMEOUT, UnixStream::connect(discovery_path.as_path()))
        .await
        .map_err(|_| ToadStoolError::integration("Timeout connecting to discovery service"))?
        .map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to connect to discovery service at {socket_path}: {e}"
            ))
        })?;

    let own_socket = resolve_toadstool_socket(&env);
    let endpoint = format!("unix://{}", own_socket.display());

    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.register",
        "params": {
            "primal_id": PRIMAL_NAME,
            "capabilities": ["compute.dispatch", "compute.capabilities"],
            "endpoint": endpoint
        },
        "id": 1
    });

    framing::write_json_rpc(&mut stream, &request).await?;
    let response: Value = framing::read_json_rpc(&mut stream).await?;

    if let Some(error) = response.get("error") {
        return Err(ToadStoolError::integration(format!(
            "Discovery service registration failed: {error}"
        )));
    }

    info!("Self-registered with discovery service ({})", endpoint);
    debug!("Registration response: {:?}", response);

    Ok(())
}

/// Register ToadStool with coordination/discovery service.
///
/// Delegates to [`register_with_discovery`], which uses `DISCOVERY_SOCKET`
/// (highest precedence, set by `composition_nucleus.sh` → Songbird) with
/// full fallback through `resolve_capability_socket_fallback("discovery", …)`.
///
/// # Errors
///
/// Returns error if the coordination service is unreachable, JSON-RPC framing
/// fails, or registration is rejected.
#[deprecated(note = "use register_with_discovery — aligns with DISCOVERY_SOCKET + ipc.register")]
pub async fn register_with_coordination() -> ToadStoolResult<()> {
    register_with_discovery().await
}

/// Find primals by capability via discovery/coordination service
///
/// Uses `DISCOVERY_SOCKET` (highest precedence) with full fallback chain.
///
/// # Errors
///
/// Returns error if the coordination service is unreachable, the response is
/// invalid, or the query fails.
pub async fn find_by_capability(capability: &str) -> ToadStoolResult<Vec<String>> {
    let env = SocketPathEnv::from_env();
    let discovery_path = resolve_capability_socket_fallback("discovery", &env);
    let socket_path = discovery_path.to_string_lossy().to_string();

    debug!("Finding primals with capability: {}", capability);

    let mut stream = timeout(IPC_TIMEOUT, UnixStream::connect(discovery_path.as_path()))
        .await
        .map_err(|_| ToadStoolError::integration("Timeout connecting to discovery service"))?
        .map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to connect to discovery service at {socket_path}: {e}"
            ))
        })?;

    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.find_capability",
        "params": {
            "capability": capability
        },
        "id": 1
    });

    framing::write_json_rpc(&mut stream, &request).await?;
    let response: Value = framing::read_json_rpc(&mut stream).await?;

    if let Some(error) = response.get("error") {
        return Err(ToadStoolError::integration(format!(
            "Failed to find capability {capability}: {error}"
        )));
    }

    let primals: Vec<String> = response
        .get("result")
        .and_then(|r| r.get("services"))
        .and_then(|s| s.as_array())
        .map(|services| {
            services
                .iter()
                .filter_map(|service| {
                    service
                        .get("primal_name")
                        .and_then(|n| n.as_str())
                        .map(|s| s.to_string())
                })
                .collect()
        })
        .unwrap_or_default();

    debug!(
        "Found {} primals with capability {}",
        primals.len(),
        capability
    );

    Ok(primals)
}

/// Self-announce to biomeOS Neural API via `primal.announce` JSON-RPC.
///
/// Sends capabilities, methods, signal tier, cost hints, and latency estimates
/// so the Neural API can build routing weights and utilization tracking.
/// Fire-and-forget at call site — if biomeOS is unreachable we continue
/// in standalone mode.
///
/// # Errors
///
/// Returns error if the Neural API socket is unreachable or JSON-RPC framing fails.
pub async fn self_announce_to_biomeos(
    methods: &[&str],
    socket_path: &str,
) -> ToadStoolResult<()> {
    let biomeos_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let neural_sock = biomeos_dir.join("neural-api-ecoPrimal.sock");
    let neural_path = neural_sock.to_string_lossy().to_string();

    info!("Self-announcing to biomeOS Neural API at {}", neural_path);

    let mut stream = match timeout(IPC_TIMEOUT, UnixStream::connect(neural_sock.as_path())).await {
        Ok(Ok(s)) => s,
        Ok(Err(e)) => {
            return Err(ToadStoolError::integration(format!(
                "Failed to connect to Neural API at {neural_path}: {e}"
            )));
        }
        Err(_) => {
            return Err(ToadStoolError::integration(
                "Timeout connecting to Neural API socket",
            ));
        }
    };

    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "primal.announce",
        "params": {
            "primal": PRIMAL_NAME,
            "capabilities": ["compute", "science", "inference"],
            "methods": methods,
            "socket": socket_path,
            "signal_tiers": ["node"],
            "cost_hints": {
                "compute": 100.0,
                "science": 50.0,
                "inference": 80.0
            },
            "latency_estimates": {
                "compute": 200,
                "science": 100,
                "inference": 150
            }
        },
        "id": 1
    });

    framing::write_json_rpc(&mut stream, &request).await?;
    let response: Value = framing::read_json_rpc(&mut stream).await?;

    if let Some(error) = response.get("error") {
        return Err(ToadStoolError::integration(format!(
            "Neural API announce rejected: {error}"
        )));
    }

    info!("Self-announced to biomeOS Neural API (capabilities: compute, science, inference)");
    debug!("Neural API announce response: {:?}", response);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_coordination_socket_format() {
        let socket = get_default_coordination_socket();
        assert!(socket.contains("biomeos"));
        assert!(socket.ends_with("coordination.sock"));
        assert!(!socket.is_empty());
    }

    #[test]
    fn test_get_default_coordination_socket_with_xdg_runtime_dir() {
        temp_env::with_vars(
            [
                ("BIOMEOS_RUNTIME_DIR", None),
                ("XDG_RUNTIME_DIR", Some("/tmp/xdg-socket-test")),
            ],
            || {
                let socket = get_default_coordination_socket();
                assert!(
                    socket.starts_with("/tmp/xdg-socket-test"),
                    "expected socket to start with XDG path, got: {socket}"
                );
            },
        );
    }
}
