// SPDX-License-Identifier: AGPL-3.0-or-later
//! Version and GPU query handlers (`toadstool.version`, `gpu.query_*`).

use super::JsonRpcResult;

/// Returns version and protocol information.
pub(crate) async fn version_info(version: &str) -> JsonRpcResult {
    Ok(serde_json::json!({
        "version": version,
        "protocol": "JSON-RPC 2.0",
        "service": "ToadStool Compute",
        "implementation": "Pure Rust (ecoPrimals sovereign pattern)"
    }))
}

/// Returns GPU device, backend, NVVM safety, and firmware information.
///
/// Includes `nvvm_transcendental_risk` for each device so springs
/// (hotSpring v0.6.26+) can make probe-time decisions without
/// repeating the driver classification locally.
///
/// Also includes `firmware_inventory` for NVIDIA chips so callers can
/// assess `compute_viable()` and `compute_blockers()` without local probing.
pub(crate) async fn gpu_info() -> JsonRpcResult {
    Ok(serde_json::json!({
        "devices": crate::gpu_system::query_gpu_devices(),
        "driver": "wgpu",
        "compute_backends": crate::gpu_system::query_available_backends(),
        "spirv_codegen_safety": crate::gpu_system::query_spirv_codegen_safety(),
        "firmware_inventory": crate::gpu_system::query_firmware_inventory(),
    }))
}

/// Returns GPU memory information per device.
pub(crate) async fn gpu_memory() -> JsonRpcResult {
    Ok(serde_json::json!({
        "devices": crate::gpu_system::query_gpu_memory(),
    }))
}

#[cfg(test)]
mod tests {
    use super::{gpu_info, gpu_memory, version_info};

    #[tokio::test]
    async fn version_info_includes_version_string() {
        let v = version_info("compute-test-9").await.expect("version");
        assert_eq!(v["version"], "compute-test-9");
    }

    #[tokio::test]
    async fn version_info_includes_protocol_and_service() {
        let v = version_info("x").await.expect("version");
        assert_eq!(v["protocol"], "JSON-RPC 2.0");
        assert_eq!(v["service"], "ToadStool Compute");
    }

    #[tokio::test]
    async fn gpu_info_returns_devices_and_driver() {
        let g = gpu_info().await.expect("gpu_info");
        assert!(g.get("devices").is_some());
        assert_eq!(g["driver"], "wgpu");
    }

    #[tokio::test]
    async fn gpu_memory_returns_devices_key() {
        let m = gpu_memory().await.expect("gpu_memory");
        assert!(m.get("devices").is_some());
    }
}
