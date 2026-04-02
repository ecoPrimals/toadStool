// SPDX-License-Identifier: AGPL-3.0-only
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

/// Default coordination-capability socket path (legacy callers said “Songbird”).
///
/// biomeOS convention: `$XDG_RUNTIME_DIR/biomeos/coordination.sock`
pub fn get_default_songbird_socket() -> String {
    format!("{}/biomeos/coordination.sock", get_runtime_dir())
}

/// Register ToadStool with Songbird discovery service
///
/// # Errors
///
/// Returns error if Songbird is unreachable, JSON-RPC framing fails, or registration is rejected.
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
        "method": "capability.register",
        "params": {
            "primal_name": PRIMAL_NAME,
            "capabilities": [
                "compute", "workload", "orchestration", "ai_local",
                "gpu", "wasm", "container", "shader.dispatch"
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
///
/// # Errors
///
/// Returns error if Songbird is unreachable, the response is invalid, or resolution fails.
#[deprecated(
    since = "0.92.0",
    note = "Identity-based discovery — use find_by_capability() then connect via socket"
)]
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
        "method": "capability.resolve",
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
///
/// # Errors
///
/// Returns error if endpoint resolution or the Unix connection fails.
#[deprecated(
    since = "0.92.0",
    note = "Identity-based discovery — use find_by_capability() then connect via socket"
)]
#[allow(deprecated)]
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
///
/// # Errors
///
/// Returns error if Songbird is unreachable, the response is invalid, or the query fails.
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
        "method": "capability.find",
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
        assert!(socket.ends_with("coordination.sock"));
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
                    "expected socket to start with XDG path, got: {socket}"
                );
            },
        );
    }
}
