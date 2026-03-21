// SPDX-License-Identifier: AGPL-3.0-only

use super::super::job::JobHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

type JsonRpcResult = Result<serde_json::Value, JsonRpcError>;

mod precision_defaults {
    pub const F64_SHARED_MEMORY_RELIABLE: bool = false;
    pub const F64_NATIVE_ELEMENT_WISE: bool = true;
    pub const DF64_REDUCTIONS: bool = true;
    pub const SOVEREIGN_BINARY_PIPELINE: bool = true;
    pub const FUSED_OPS_CANARY: &str = "Run variance canary probe before fused GPU reductions";
    pub const ROUTING_ADVICE: &str = "Use DF64 for shared-memory reductions; per-adapter PrecisionRoutingAdvice available via wgpu backend";
    pub const NVVM_POISONING_WARNING: &str = "NVIDIA proprietary: DF64/F64Precise exp/log compilation permanently invalidates wgpu device. Use HardwareCalibration.";
}

pub(crate) async fn science_gpu_dispatch(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_submit(params).await
}

#[allow(clippy::unused_async)] // async for JSON-RPC handler consistency
pub(crate) async fn science_gpu_capabilities() -> JsonRpcResult {
    let gpu_info = crate::gpu_system::query_gpu_devices();
    let available_backends = crate::gpu_system::query_available_backends();

    Ok(serde_json::json!({
        "devices": gpu_info,
        "supported_precisions": ["f32", "f64", "df64"],
        "precision_notes": {
            "f64_shared_memory_reliable": precision_defaults::F64_SHARED_MEMORY_RELIABLE,
            "f64_native_element_wise": precision_defaults::F64_NATIVE_ELEMENT_WISE,
            "df64_reductions": precision_defaults::DF64_REDUCTIONS,
            "fused_ops_canary": precision_defaults::FUSED_OPS_CANARY,
            "routing_advice": precision_defaults::ROUTING_ADVICE,
        },
        "compute_backends": available_backends,
        "sovereign_binary_pipeline": precision_defaults::SOVEREIGN_BINARY_PIPELINE,
        "spirv_codegen_safety": {
            "warning": precision_defaults::NVVM_POISONING_WARNING,
            "root_cause": "naga SPIR-V codegen (not NVVM — renamed per hotSpring v0.6.30)",
            "affected_drivers": ["nvidia (proprietary)"],
            "safe_drivers": ["nvk", "radv", "anv"],
            "affected_tiers": ["F64Precise", "Df64"],
            "affected_operations": ["exp", "log", "transcendentals"],
            "mitigation": "Use HardwareCalibration::from_adapter_info() for safe tier probing",
        },
        "domain": "science",
    }))
}
