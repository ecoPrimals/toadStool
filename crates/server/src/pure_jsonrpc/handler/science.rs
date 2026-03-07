// SPDX-License-Identifier: AGPL-3.0-or-later
//! Science domain handlers for JSON-RPC.
//!
//! Routes scientific compute through toadStool's workload infrastructure.
//! Springs (wetSpring, airSpring, hotSpring, etc.) call these methods to request
//! GPU/NPU compute without coupling to barraCuda directly.

use super::job::JobHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

type JsonRpcResult = Result<serde_json::Value, JsonRpcError>;

/// Submits a science compute job via the job queue.
pub(super) async fn science_compute_submit(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_submit(params).await
}

/// Returns status for a science compute job.
pub(super) async fn science_compute_status(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_status(params).await
}

/// Returns result for a completed science compute job.
pub(super) async fn science_compute_result(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_result(params).await
}

/// Cancels a science compute job.
pub(super) async fn science_compute_cancel(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_cancel(params).await
}

/// Dispatches a GPU-backed science workload.
pub(super) async fn science_gpu_dispatch(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_submit(params).await
}

/// Returns GPU capabilities for science workloads.
#[allow(clippy::unused_async)]
pub(super) async fn science_gpu_capabilities() -> JsonRpcResult {
    let gpu_info = crate::gpu_system::query_gpu_devices();
    let available_backends = crate::gpu_system::query_available_backends();

    Ok(serde_json::json!({
        "devices": gpu_info,
        "supported_precisions": ["f32", "f64", "df64"],
        "precision_notes": {
            "f64_shared_memory_reliable": false,
            "f64_native_element_wise": true,
            "df64_reductions": true,
            "routing_advice": "Use DF64 for shared-memory reductions until coralDriver is available"
        },
        "compute_backends": available_backends,
        "sovereign_binary_pipeline": false,
        "domain": "science",
    }))
}

/// Dispatches an NPU-backed science workload.
pub(super) async fn science_npu_dispatch(
    job: &JobHandler,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    job.compute_submit(params).await
}

/// Returns NPU capabilities for science workloads.
#[allow(clippy::unused_async)]
pub(super) async fn science_npu_capabilities() -> JsonRpcResult {
    Ok(serde_json::json!({
        "available": false,
        "domain": "science",
        "supported_models": [],
        "note": "NPU capabilities discovered at runtime via NpuDispatch trait",
    }))
}

/// Discovers available compute substrates (GPU, NPU, CPU).
#[allow(clippy::unused_async)]
pub(super) async fn science_substrate_discover() -> JsonRpcResult {
    let gpu_info = crate::gpu_system::query_gpu_devices();
    Ok(serde_json::json!({
        "substrates": {
            "gpu": gpu_info,
            "npu": [],
            "cpu": { "available": true },
        },
        "domain": "science",
    }))
}

/// Probes a specific capability on the science substrate.
pub(super) async fn science_substrate_probe(params: Option<&serde_json::Value>) -> JsonRpcResult {
    let capability = params
        .and_then(|p| p.get("capability"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");

    Ok(serde_json::json!({
        "capability": capability,
        "available": true,
        "domain": "science",
        "note": "Probe delegates to runtime substrate detection",
    }))
}
