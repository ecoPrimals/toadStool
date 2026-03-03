// SPDX-License-Identifier: AGPL-3.0-or-later
//! Connection establishment and Unix socket helpers for primal-to-primal IPC

use serde_json::json;
use serde_json::Value;
use std::time::Duration;
use toadstool_common::uid_detector;
use tokio::net::UnixStream;
use tokio::time::timeout;
use tracing::{debug, info};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::timeouts;
use toadstool_common::constants::PRIMAL_NAME;

use super::framing;

/// Request timeout for IPC operations (from config defaults)
pub(crate) const IPC_TIMEOUT: Duration = timeouts::TCP_CONNECT_TIMEOUT;

/// Get runtime directory: `$XDG_RUNTIME_DIR` → `$BIOMEOS_RUNTIME_DIR` → `/run/user/$UID` → temp dir.
fn get_runtime_dir() -> String {
    std::env::var("XDG_RUNTIME_DIR")
        .or_else(|_| std::env::var("BIOMEOS_RUNTIME_DIR"))
        .unwrap_or_else(|_| {
            if let Ok(uid) = uid_detector::get_user_id() {
                format!("/run/user/{uid}")
            } else {
                std::env::temp_dir()
                    .join("biomeos-runtime")
                    .to_string_lossy()
                    .to_string()
            }
        })
}

/// Get default Songbird socket path using biomeOS standard.
///
/// biomeOS socket standard: `$XDG_RUNTIME_DIR/biomeos/songbird.sock`
pub fn get_default_songbird_socket() -> String {
    format!("{}/biomeos/songbird.sock", get_runtime_dir())
}

/// Register ToadStool with Songbird discovery service
pub async fn register_with_songbird() -> ToadStoolResult<()> {
    let socket_path =
        std::env::var("SONGBIRD_SOCKET").unwrap_or_else(|_| get_default_songbird_socket());

    info!("🌍 Registering with Songbird at {}", socket_path);

    let mut stream = timeout(IPC_TIMEOUT, UnixStream::connect(&socket_path))
        .await
        .map_err(|_| ToadStoolError::integration("Timeout connecting to Songbird"))?
        .map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to connect to Songbird at {socket_path}: {e}. Is Songbird running?"
            ))
        })?;

    let socket_endpoint = std::env::var("TOADSTOOL_SOCKET").unwrap_or_else(|_| {
        // XDG-compliant biomeOS standard path: $XDG_RUNTIME_DIR/biomeos/toadstool.sock
        let runtime_dir = get_runtime_dir();
        format!("{runtime_dir}/biomeos/{PRIMAL_NAME}.sock")
    });

    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.register",
        "params": {
            "primal_name": PRIMAL_NAME,
            // biomeOS Node Atomic capabilities (node_atomic_compute.toml):
            //   compute, workload, orchestration, ai_local
            // Plus implementation-level capabilities advertised to consumers:
            //   gpu, wasm, container
            "capabilities": [
                "compute", "workload", "orchestration", "ai_local",
                "gpu", "wasm", "container"
            ],
            "endpoint": socket_endpoint
        },
        "id": 1
    });

    framing::write_json_rpc(&mut stream, &request).await?;
    let response: Value = framing::read_json_rpc(&mut stream).await?;

    if let Some(error) = response.get("error") {
        return Err(ToadStoolError::integration(format!(
            "Songbird registration failed: {error}"
        )));
    }

    info!("✅ Successfully registered with Songbird discovery service");
    debug!("   Registration response: {:?}", response);

    Ok(())
}

/// Resolve a primal's endpoint via Songbird
pub async fn resolve_primal(primal_name: &str) -> ToadStoolResult<String> {
    let socket_path =
        std::env::var("SONGBIRD_SOCKET").unwrap_or_else(|_| get_default_songbird_socket());

    debug!("🔍 Resolving {} via Songbird", primal_name);

    let mut stream = timeout(IPC_TIMEOUT, UnixStream::connect(&socket_path))
        .await
        .map_err(|_| ToadStoolError::integration("Timeout connecting to Songbird"))?
        .map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to connect to Songbird: {e}. Is Songbird running?"
            ))
        })?;

    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.resolve",
        "params": {
            "primal_name": primal_name
        },
        "id": 1
    });

    framing::write_json_rpc(&mut stream, &request).await?;
    let response: Value = framing::read_json_rpc(&mut stream).await?;

    if let Some(error) = response.get("error") {
        return Err(ToadStoolError::integration(format!(
            "Failed to resolve {primal_name}: {error}"
        )));
    }

    let endpoint = response
        .get("result")
        .and_then(|r| r.get("endpoint"))
        .and_then(|e| e.as_str())
        .ok_or_else(|| {
            ToadStoolError::integration(format!(
                "Invalid response from Songbird: missing endpoint for {primal_name}"
            ))
        })?
        .to_string();

    debug!("✅ Resolved {} -> {}", primal_name, endpoint);

    Ok(endpoint)
}

/// Connect to another primal
pub async fn connect_to_primal(primal_name: &str) -> ToadStoolResult<UnixStream> {
    let endpoint = resolve_primal(primal_name).await?;

    info!("🔗 Connecting to {} at {}", primal_name, endpoint);

    let stream = timeout(IPC_TIMEOUT, UnixStream::connect(&endpoint))
        .await
        .map_err(|_| {
            ToadStoolError::integration(format!(
                "Timeout connecting to {primal_name} at {endpoint}"
            ))
        })?
        .map_err(|e| {
            ToadStoolError::integration(format!(
                "Failed to connect to {primal_name} at {endpoint}: {e}"
            ))
        })?;

    debug!("✅ Connected to {}", primal_name);

    Ok(stream)
}

/// Find primals by capability
pub async fn find_by_capability(capability: &str) -> ToadStoolResult<Vec<String>> {
    let socket_path =
        std::env::var("SONGBIRD_SOCKET").unwrap_or_else(|_| get_default_songbird_socket());

    debug!("🔍 Finding primals with capability: {}", capability);

    let mut stream = timeout(IPC_TIMEOUT, UnixStream::connect(&socket_path))
        .await
        .map_err(|_| ToadStoolError::integration("Timeout connecting to Songbird"))?
        .map_err(|e| ToadStoolError::integration(format!("Failed to connect to Songbird: {e}")))?;

    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.capabilities",
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
        "✅ Found {} primals with capability {}",
        primals.len(),
        capability
    );

    Ok(primals)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_default_songbird_socket_format() {
        let socket = get_default_songbird_socket();
        assert!(socket.contains("biomeos"));
        assert!(socket.ends_with("songbird.sock"));
        assert!(!socket.is_empty());
    }

    #[test]
    fn test_get_default_songbird_socket_with_xdg_runtime_dir() {
        temp_env::with_vars(
            [
                ("BIOMEOS_RUNTIME_DIR", None),
                ("XDG_RUNTIME_DIR", Some("/tmp/xdg-socket-test")),
            ],
            || {
                let socket = get_default_songbird_socket();
                assert!(
                    socket.starts_with("/tmp/xdg-socket-test"),
                    "expected socket to start with XDG path, got: {}",
                    socket
                );
            },
        );
    }
}
