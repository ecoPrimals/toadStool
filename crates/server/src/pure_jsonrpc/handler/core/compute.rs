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
