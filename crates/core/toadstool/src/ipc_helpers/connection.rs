// SPDX-License-Identifier: AGPL-3.0-or-later
//! Connection establishment and Unix socket helpers for primal-to-primal IPC

use serde_json::Value;
use serde_json::json;
use std::time::Duration;
use tokio::net::UnixStream;
use tokio::time::timeout;
use tracing::{debug, info};

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_common::constants::PRIMAL_NAME;
use toadstool_common::constants::timeouts;
use toadstool_common::primal_sockets::{
    SocketPathEnv, get_runtime_dir, resolve_capability_socket_fallback, resolve_toadstool_socket,
};

use super::framing;

/// Request timeout for IPC operations (from config defaults)
pub const IPC_TIMEOUT: Duration = timeouts::TCP_CONNECT_TIMEOUT;

/// Capabilities advertised to the discovery service via `ipc.register`.
/// Must stay aligned with the `primal.announce` handler in `identity.rs`.
pub const DISCOVERY_CAPABILITIES: &[&str] = &[
    "compute",
    "workload",
    "orchestration",
    "gpu",
    "wasm",
    "container",
    "hardware_transport",
    "shader_dispatch",
    "hardware_learning",
];

/// Default coordination-capability socket path.
///
/// biomeOS convention: `$XDG_RUNTIME_DIR/biomeos/coordination.sock`
pub fn get_default_coordination_socket() -> String {
    format!("{}/biomeos/coordination.sock", get_runtime_dir())
}

/// Enumerate PCI GPU/NPU devices from sysfs for registration payloads.
///
/// Returns a JSON array of device descriptors. Runs synchronously (sysfs reads
/// are fast) and is safe to call during startup before wgpu initialization.
fn discover_hardware_inventory() -> Vec<Value> {
    let pci_dir = std::path::Path::new("/sys/bus/pci/devices");
    let Ok(entries) = std::fs::read_dir(pci_dir) else {
        return Vec::new();
    };

    entries
        .flatten()
        .filter_map(|entry| {
            let class_path = entry.path().join("class");
            let class = std::fs::read_to_string(class_path).ok()?;
            let class_trimmed = class.trim();
            // VGA: 0x030000, 3D: 0x030200
            let device_type = if class_trimmed.starts_with("0x0302") {
                "gpu_3d"
            } else if class_trimmed.starts_with("0x0300") {
                "gpu_vga"
            } else {
                return None;
            };

            let bdf = entry.file_name().to_str()?.to_string();
            let vendor = std::fs::read_to_string(entry.path().join("vendor"))
                .ok()
                .map(|v| v.trim().to_string());
            let device = std::fs::read_to_string(entry.path().join("device"))
                .ok()
                .map(|d| d.trim().to_string());
            let driver = entry
                .path()
                .join("driver")
                .read_link()
                .ok()
                .and_then(|p| p.file_name().map(|n| n.to_string_lossy().to_string()));

            Some(json!({
                "bdf": bdf,
                "type": device_type,
                "vendor_id": vendor,
                "device_id": device,
                "driver": driver,
            }))
        })
        .collect()
}

/// Self-register with the discovery service via `DISCOVERY_SOCKET` (preferred) or coordination fallback.
///
/// Sends `ipc.register` with capability list and hardware inventory so the
/// coordination plane can resolve `toadstool` by capability and know what
/// compute devices are available. Fire-and-forget at the call site — if
/// this fails the primal continues in standalone mode.
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

    framing::write_ribocipher_signal(&mut stream).await?;

    let own_socket = resolve_toadstool_socket(&env);
    let endpoint = format!("unix://{}", own_socket.display());

    let devices = discover_hardware_inventory();
    let device_count = devices.len();

    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.register",
        "params": {
            "primal_id": PRIMAL_NAME,
            "capabilities": DISCOVERY_CAPABILITIES,
            "endpoint": endpoint,
            "devices": devices
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

    info!(
        "Self-registered with discovery service ({}, {} device(s))",
        endpoint, device_count
    );
    debug!("Registration response: {:?}", response);

    Ok(())
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

    framing::write_ribocipher_signal(&mut stream).await?;

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
pub async fn self_announce_to_biomeos(methods: &[&str], socket_path: &str) -> ToadStoolResult<()> {
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

    framing::write_ribocipher_signal(&mut stream).await?;

    let devices = discover_hardware_inventory();

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
            },
            "devices": devices
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

    #[test]
    fn discover_hardware_inventory_returns_vec() {
        let devices = discover_hardware_inventory();
        // May be empty on CI / non-GPU hosts — structural check only
        for dev in &devices {
            assert!(dev.get("bdf").is_some(), "device must have bdf field");
            assert!(dev.get("type").is_some(), "device must have type field");
            let dev_type = dev["type"].as_str().unwrap();
            assert!(
                dev_type == "gpu_vga" || dev_type == "gpu_3d",
                "unexpected type: {dev_type}"
            );
        }
    }

    #[test]
    fn discover_hardware_inventory_includes_vendor_and_device_ids() {
        let devices = discover_hardware_inventory();
        for dev in &devices {
            if let Some(vid) = dev.get("vendor_id").and_then(|v| v.as_str()) {
                assert!(vid.starts_with("0x"), "vendor_id should be hex: {vid}");
            }
            if let Some(did) = dev.get("device_id").and_then(|v| v.as_str()) {
                assert!(did.starts_with("0x"), "device_id should be hex: {did}");
            }
        }
    }
}
