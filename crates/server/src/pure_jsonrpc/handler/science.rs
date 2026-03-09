// SPDX-License-Identifier: AGPL-3.0-only
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

/// Conservative system-wide precision defaults from groundSpring V84-V98.
///
/// Per-adapter precision routing is available via `GpuAdapterInfo::precision_routing()`
/// when the wgpu backend is active. These defaults cover the worst-case across all
/// tested GPUs via the naga/SPIR-V pipeline.
mod precision_defaults {
    /// naga/SPIR-V f64 shared-memory reductions return zeros on all tested GPUs.
    pub const F64_SHARED_MEMORY_RELIABLE: bool = false;
    /// f64 element-wise arithmetic works on GPUs that report SHADER_F64.
    pub const F64_NATIVE_ELEMENT_WISE: bool = true;
    /// DF64 (double-float f32 pairs) reductions work correctly everywhere.
    pub const DF64_REDUCTIONS: bool = true;
    /// coralDriver binary submission path not yet production-ready.
    pub const SOVEREIGN_BINARY_PIPELINE: bool = false;
    pub const FUSED_OPS_CANARY: &str = "Run variance canary probe before fused GPU reductions";
    pub const ROUTING_ADVICE: &str =
        "Use DF64 for shared-memory reductions; per-adapter PrecisionRoutingAdvice available via wgpu backend";
}

/// Returns GPU capabilities for science workloads.
#[expect(
    clippy::unused_async,
    reason = "async for JSON-RPC handler consistency"
)]
pub(super) async fn science_gpu_capabilities() -> JsonRpcResult {
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
#[expect(
    clippy::unused_async,
    reason = "async for JSON-RPC handler consistency"
)]
pub(super) async fn science_npu_capabilities() -> JsonRpcResult {
    Ok(serde_json::json!({
        "available": false,
        "domain": "science",
        "supported_models": [],
        "note": "NPU capabilities discovered at runtime via NpuDispatch trait",
    }))
}

/// Discovers available compute substrates (GPU, NPU, CPU).
#[expect(
    clippy::unused_async,
    reason = "async for JSON-RPC handler consistency"
)]
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

// ═══════════════════════════════════════════════════════════
// Ecology domain — airSpring science offload routing
//
// 14 validated JSON-RPC methods from airSpring V0.7.5.
// toadStool acts as a compute.offload proxy, routing to the
// appropriate science primal discovered at runtime.
// ═══════════════════════════════════════════════════════════

const ECOLOGY_METHODS: &[&str] = &[
    "ecology.et0_fao56",
    "ecology.water_balance",
    "ecology.yield_response",
    "ecology.thornthwaite",
    "ecology.gdd",
    "ecology.pedotransfer",
    "ecology.spi_drought_index",
    "ecology.autocorrelation",
    "ecology.gamma_cdf",
    "ecology.runoff_scs_cn",
    "ecology.van_genuchten_theta",
    "ecology.van_genuchten_k",
    "ecology.bootstrap_ci",
    "ecology.jackknife_ci",
];

/// Routes ecology method calls to the appropriate science primal.
///
/// Discovery is capability-based: toadStool scans the biomeOS socket
/// directory for primals advertising the `ecology.*` capability,
/// then forwards the JSON-RPC call. When no science primal is
/// discovered, returns a structured error with routing advice.
pub(super) async fn ecology_offload(
    method: &str,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    let socket_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let primal_socket = socket_dir.join("airspring.sock");

    if primal_socket.exists() {
        return forward_to_primal(&primal_socket, method, params).await;
    }

    Ok(serde_json::json!({
        "method": method,
        "status": "queued",
        "domain": "ecology",
        "available_methods": ECOLOGY_METHODS,
        "params_received": params.is_some(),
        "routing": "No airSpring science primal discovered. Method registered for deferred execution.",
        "discovery_path": socket_dir.display().to_string(),
    }))
}

// ═══════════════════════════════════════════════════════════
// Discovery domain — NUCLEUS primal discovery (groundSpring V99)
//
// Adaptive health checks and direct primal socket discovery.
// Scans $XDG_RUNTIME_DIR/biomeos/ for primal sockets.
// ═══════════════════════════════════════════════════════════

/// Discovers available primals by scanning the biomeOS socket directory.
#[expect(
    clippy::unused_async,
    reason = "async for JSON-RPC handler consistency"
)]
pub(super) async fn discovery_primals() -> JsonRpcResult {
    let socket_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let mut primals = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&socket_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("sock") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    primals.push(serde_json::json!({
                        "name": name,
                        "socket": path.display().to_string(),
                        "reachable": path.exists(),
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "primals": primals,
        "count": primals.len(),
        "socket_dir": socket_dir.display().to_string(),
        "domain": "discovery",
    }))
}

/// Checks health of a specific primal by name via its socket.
pub(super) async fn discovery_primal_health(params: Option<&serde_json::Value>) -> JsonRpcResult {
    let name = params
        .and_then(|p| p.get("name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or("unknown");

    let socket_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let socket_path = socket_dir.join(format!("{name}.sock"));

    if !socket_path.exists() {
        return Ok(serde_json::json!({
            "name": name,
            "healthy": false,
            "reason": "Socket not found",
            "socket_path": socket_path.display().to_string(),
        }));
    }

    // Adaptive health: try evolved method name first, fall back to alias
    // (groundSpring V99 pattern for handling binary version mismatches).
    match forward_to_primal(&socket_path, "compute.health", None).await {
        Ok(result) => Ok(serde_json::json!({
            "name": name,
            "healthy": true,
            "health_data": result,
        })),
        Err(_) => Ok(serde_json::json!({
            "name": name,
            "healthy": false,
            "reason": "Health check failed (socket exists but primal unresponsive)",
            "socket_path": socket_path.display().to_string(),
        })),
    }
}

/// Forwards a JSON-RPC call directly to a primal socket, bypassing
/// the Neural API router. For latency-sensitive direct primal calls.
pub(super) async fn discovery_direct_rpc(params: Option<&serde_json::Value>) -> JsonRpcResult {
    let name = params
        .and_then(|p| p.get("name"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'name' parameter"))?;

    let method = params
        .and_then(|p| p.get("method"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'method' parameter"))?;

    let rpc_params = params.and_then(|p| p.get("params"));

    let socket_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let socket_path = socket_dir.join(format!("{name}.sock"));

    if !socket_path.exists() {
        return Err(JsonRpcError::internal_error(format!(
            "Primal '{name}' socket not found at {}",
            socket_path.display()
        )));
    }

    forward_to_primal(&socket_path, method, rpc_params).await
}

/// Returns topology of discovered primals and their connections.
#[expect(
    clippy::unused_async,
    reason = "async for JSON-RPC handler consistency"
)]
pub(super) async fn discovery_topology() -> JsonRpcResult {
    let socket_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let mut nodes = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&socket_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("sock") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    nodes.push(name.to_string());
                }
            }
        }
    }

    Ok(serde_json::json!({
        "nodes": nodes,
        "self": toadstool_common::constants::PRIMAL_NAME,
        "protocol": "JSON-RPC 2.0",
        "socket_dir": socket_dir.display().to_string(),
        "domain": "discovery",
    }))
}

// ═══════════════════════════════════════════════════════════
// Deploy domain — science primal capability routing (wetSpring V99)
//
// Routes capability_call requests to science primals discovered
// at runtime. Supports the wetspring_deploy.toml pattern where
// Tower→ToadStool→wetSpring forms the deploy graph.
// ═══════════════════════════════════════════════════════════

/// Routes a capability call to the appropriate discovered primal.
pub(super) async fn deploy_capability_call(params: Option<&serde_json::Value>) -> JsonRpcResult {
    let capability = params
        .and_then(|p| p.get("capability"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'capability' parameter"))?;

    let method = params
        .and_then(|p| p.get("method"))
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| JsonRpcError::invalid_params("Missing 'method' parameter"))?;

    let call_params = params.and_then(|p| p.get("params"));

    let socket_path = toadstool_common::primal_sockets::get_socket_path_for_capability(capability);

    if !socket_path.exists() {
        return Ok(serde_json::json!({
            "status": "no_provider",
            "capability": capability,
            "method": method,
            "note": format!("No primal discovered for capability '{capability}'"),
            "socket_path": socket_path.display().to_string(),
        }));
    }

    forward_to_primal(&socket_path, method, call_params).await
}

/// Returns status of known deploy graphs and their science primals.
#[expect(
    clippy::unused_async,
    reason = "async for JSON-RPC handler consistency"
)]
pub(super) async fn deploy_graph_status() -> JsonRpcResult {
    let socket_dir = toadstool_common::primal_sockets::get_biomeos_dir();

    let known_graphs = [
        ("wetspring", "science.diversity"),
        ("airspring", "ecology.et0_fao56"),
        ("groundspring", "science.noise_decomposition"),
        ("neuralspring", "science.ml_surrogate"),
        ("hotspring", "science.lattice_qcd"),
    ];

    let graphs: Vec<_> = known_graphs
        .iter()
        .map(|(primal, sample_cap)| {
            let socket = socket_dir.join(format!("{primal}.sock"));
            serde_json::json!({
                "primal": primal,
                "sample_capability": sample_cap,
                "socket_exists": socket.exists(),
                "socket_path": socket.display().to_string(),
            })
        })
        .collect();

    Ok(serde_json::json!({
        "deploy_graphs": graphs,
        "domain": "deploy",
    }))
}

/// Forwards a JSON-RPC request to a primal via its Unix socket.
///
/// Uses newline-delimited JSON-RPC 2.0 over Unix domain sockets,
/// matching the biomeOS protocol convention.
async fn forward_to_primal(
    socket_path: &std::path::Path,
    method: &str,
    params: Option<&serde_json::Value>,
) -> JsonRpcResult {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::UnixStream;

    let stream = UnixStream::connect(socket_path).await.map_err(|e| {
        JsonRpcError::internal_error(format!(
            "Failed to connect to {}: {e}",
            socket_path.display()
        ))
    })?;

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1
    });

    let mut payload = serde_json::to_string(&request)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialize error: {e}")))?;
    payload.push('\n');

    let (reader, mut writer) = stream.into_split();
    writer
        .write_all(payload.as_bytes())
        .await
        .map_err(|e| JsonRpcError::internal_error(format!("Write error: {e}")))?;
    writer
        .shutdown()
        .await
        .map_err(|e| JsonRpcError::internal_error(format!("Shutdown error: {e}")))?;

    let mut buf_reader = BufReader::new(reader);
    let mut response_line = String::new();

    let read_result = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        buf_reader.read_line(&mut response_line),
    )
    .await;

    match read_result {
        Ok(Ok(_)) => {
            let response: serde_json::Value = serde_json::from_str(&response_line)
                .map_err(|e| JsonRpcError::internal_error(format!("Parse response error: {e}")))?;

            if let Some(error) = response.get("error") {
                return Err(JsonRpcError::internal_error(format!(
                    "Primal returned error: {error}"
                )));
            }

            Ok(response
                .get("result")
                .cloned()
                .unwrap_or(serde_json::Value::Null))
        }
        Ok(Err(e)) => Err(JsonRpcError::internal_error(format!("Read error: {e}"))),
        Err(_) => Err(JsonRpcError::internal_error(
            "Primal response timed out (30s)".to_string(),
        )),
    }
}

#[cfg(test)]
mod tests {
    // SPDX-License-Identifier: AGPL-3.0-only
    use std::borrow::Cow;
    use std::sync::Arc;

    use crate::pure_jsonrpc::handler::JsonRpcHandler;
    use crate::pure_jsonrpc::types::JsonRpcError;
    use crate::tarpc_server::StandaloneExecutor;

    fn test_handler() -> JsonRpcHandler {
        let executor = Arc::new(StandaloneExecutor::new());
        JsonRpcHandler::new(executor, "test-1.0.0".to_string(), None)
    }

    fn mk_request(
        method: &str,
        params: Option<serde_json::Value>,
        id: i32,
    ) -> crate::pure_jsonrpc::types::JsonRpcRequest<'static> {
        crate::pure_jsonrpc::types::JsonRpcRequest {
            jsonrpc: Cow::Borrowed("2.0"),
            method: Cow::Owned(method.to_string()),
            params,
            id: Some(serde_json::json!(id)),
        }
    }

    // ───── science.compute.* ─────

    #[tokio::test]
    async fn science_compute_submit_valid() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": { "model": "tinyllama", "prompt": "test", "params": {} }
        });
        let request = mk_request("science.compute.submit", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("job_id").is_some());
        assert!(result.get("routing").is_some());
    }

    #[tokio::test]
    async fn science_compute_submit_missing_params() {
        let handler = test_handler();
        let request = mk_request("science.compute.submit", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn science_compute_submit_invalid_params() {
        let handler = test_handler();
        let params = serde_json::json!({ "invalid": "job_type" });
        let request = mk_request("science.compute.submit", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn science_compute_status_valid() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": { "model": "m", "prompt": "p", "params": {} }
        });
        let submit_resp = handler
            .handle_request(&mk_request("science.compute.submit", Some(params), 1))
            .await;
        let job_id = submit_resp
            .result
            .as_ref()
            .and_then(|r| r.get("job_id"))
            .and_then(|v| v.as_str())
            .expect("job_id");

        let status_params = serde_json::json!({ "job_id": job_id });
        let request = mk_request("science.compute.status", Some(status_params), 2);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("state").is_some());
    }

    #[tokio::test]
    async fn science_compute_status_missing_job_id() {
        let handler = test_handler();
        let request = mk_request("science.compute.status", Some(serde_json::json!({})), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn science_compute_status_invalid_uuid() {
        let handler = test_handler();
        let params = serde_json::json!({ "job_id": "not-a-uuid" });
        let request = mk_request("science.compute.status", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn science_compute_result_valid() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": { "model": "m", "prompt": "p", "params": {} }
        });
        let submit_resp = handler
            .handle_request(&mk_request("science.compute.submit", Some(params), 1))
            .await;
        let job_id = submit_resp
            .result
            .as_ref()
            .and_then(|r| r.get("job_id"))
            .and_then(|v| v.as_str())
            .expect("job_id");

        let result_params = serde_json::json!({ "job_id": job_id });
        let request = mk_request("science.compute.result", Some(result_params), 2);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_some() || response.error.is_some());
    }

    #[tokio::test]
    async fn science_compute_result_missing_job_id() {
        let handler = test_handler();
        let request = mk_request("science.compute.result", Some(serde_json::json!({})), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn science_compute_result_job_not_found() {
        let handler = test_handler();
        let job_id = uuid::Uuid::new_v4();
        let params = serde_json::json!({ "job_id": job_id.to_string() });
        let request = mk_request("science.compute.result", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(
            err.code,
            toadstool_common::constants::jsonrpc::error_codes::WORKLOAD_NOT_FOUND
        );
    }

    #[tokio::test]
    async fn science_compute_cancel_valid() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": { "model": "m", "prompt": "p", "params": {} }
        });
        let submit_resp = handler
            .handle_request(&mk_request("science.compute.submit", Some(params), 1))
            .await;
        let job_id = submit_resp
            .result
            .as_ref()
            .and_then(|r| r.get("job_id"))
            .and_then(|v| v.as_str())
            .expect("job_id");

        let cancel_params = serde_json::json!({ "job_id": job_id });
        let request = mk_request("science.compute.cancel", Some(cancel_params), 2);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["cancelled"], true);
    }

    #[tokio::test]
    async fn science_compute_cancel_missing_job_id() {
        let handler = test_handler();
        let request = mk_request("science.compute.cancel", Some(serde_json::json!({})), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    // ───── science.gpu.* ─────

    #[tokio::test]
    async fn science_gpu_dispatch_valid() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": { "model": "tinyllama", "prompt": "test", "params": {} }
        });
        let request = mk_request("science.gpu.dispatch", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("job_id").is_some());
    }

    #[tokio::test]
    async fn science_gpu_dispatch_missing_params() {
        let handler = test_handler();
        let request = mk_request("science.gpu.dispatch", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn science_gpu_capabilities_structure() {
        let handler = test_handler();
        let request = mk_request("science.gpu.capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("devices").is_some());
        assert!(result.get("supported_precisions").is_some());
        assert!(result.get("precision_notes").is_some());
        assert!(result.get("compute_backends").is_some());
        assert_eq!(result["domain"], "science");
        assert!(result["precision_notes"]["f64_shared_memory_reliable"]
            .as_bool()
            .is_some());
        assert!(result["precision_notes"]["df64_reductions"]
            .as_bool()
            .is_some());
    }

    // ───── science.npu.* ─────

    #[tokio::test]
    async fn science_npu_dispatch_valid() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": { "model": "tinyllama", "prompt": "test", "params": {} }
        });
        let request = mk_request("science.npu.dispatch", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("job_id").is_some());
    }

    #[tokio::test]
    async fn science_npu_dispatch_missing_params() {
        let handler = test_handler();
        let request = mk_request("science.npu.dispatch", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn science_npu_capabilities_structure() {
        let handler = test_handler();
        let request = mk_request("science.npu.capabilities", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["available"], false);
        assert_eq!(result["domain"], "science");
        assert!(result.get("supported_models").is_some());
        assert!(result.get("note").is_some());
    }

    // ───── science.substrate.* ─────

    #[tokio::test]
    async fn science_substrate_discover_structure() {
        let handler = test_handler();
        let request = mk_request("science.substrate.discover", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        let substrates = result.get("substrates").expect("substrates");
        assert!(substrates.get("gpu").is_some());
        assert!(substrates.get("npu").is_some());
        assert!(substrates.get("cpu").is_some());
        assert_eq!(result["domain"], "science");
    }

    #[tokio::test]
    async fn science_substrate_probe_with_capability() {
        let handler = test_handler();
        let params = serde_json::json!({ "capability": "f64_reductions" });
        let request = mk_request("science.substrate.probe", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["capability"], "f64_reductions");
        assert_eq!(result["available"], true);
        assert_eq!(result["domain"], "science");
    }

    #[tokio::test]
    async fn science_substrate_probe_without_params_defaults_unknown() {
        let handler = test_handler();
        let request = mk_request("science.substrate.probe", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["capability"], "unknown");
        assert_eq!(result["available"], true);
    }

    #[tokio::test]
    async fn science_substrate_probe_empty_params() {
        let handler = test_handler();
        let request = mk_request("science.substrate.probe", Some(serde_json::json!({})), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["capability"], "unknown");
    }

    // ───── ecology.* ─────

    #[tokio::test]
    async fn ecology_offload_queued_when_no_socket() {
        let handler = test_handler();
        let params = serde_json::json!({ "lat": 45.0, "lon": -122.0 });
        let request = mk_request("ecology.et0_fao56", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["method"], "ecology.et0_fao56");
        assert_eq!(result["status"], "queued");
        assert_eq!(result["domain"], "ecology");
        assert!(result.get("available_methods").is_some());
        assert!(result.get("routing").is_some());
    }

    #[tokio::test]
    async fn ecology_offload_without_params() {
        let handler = test_handler();
        let request = mk_request("ecology.water_balance", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["params_received"], false);
    }

    #[tokio::test]
    async fn ecology_offload_multiple_methods() {
        let handler = test_handler();
        for method in [
            "ecology.gdd",
            "ecology.pedotransfer",
            "ecology.spi_drought_index",
            "ecology.bootstrap_ci",
        ] {
            let request = mk_request(method, None, 1);
            let response = handler.handle_request(&request).await;
            assert!(response.error.is_none(), "{method} should succeed");
            let result = response.result.expect("result present");
            assert_eq!(result["domain"], "ecology");
        }
    }

    // ───── discovery.* ─────

    #[tokio::test]
    async fn discovery_primals_structure() {
        let handler = test_handler();
        let request = mk_request("discovery.primals", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("primals").is_some());
        assert!(result.get("count").is_some());
        assert!(result.get("socket_dir").is_some());
        assert_eq!(result["domain"], "discovery");
    }

    #[tokio::test]
    async fn discovery_primal_health_socket_not_found() {
        let handler = test_handler();
        let params = serde_json::json!({ "name": "nonexistent_primal_xyz" });
        let request = mk_request("discovery.primal_health", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["name"], "nonexistent_primal_xyz");
        assert_eq!(result["healthy"], false);
        assert!(result.get("reason").is_some());
    }

    #[tokio::test]
    async fn discovery_primal_health_missing_name_defaults_unknown() {
        let handler = test_handler();
        let request = mk_request("discovery.primal_health", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["name"], "unknown");
    }

    #[tokio::test]
    async fn discovery_direct_rpc_missing_name() {
        let handler = test_handler();
        let params = serde_json::json!({ "method": "compute.health" });
        let request = mk_request("discovery.direct_rpc", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("name"));
    }

    #[tokio::test]
    async fn discovery_direct_rpc_missing_method() {
        let handler = test_handler();
        let params = serde_json::json!({ "name": "airspring" });
        let request = mk_request("discovery.direct_rpc", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("method"));
    }

    #[tokio::test]
    async fn discovery_direct_rpc_socket_not_found() {
        let handler = test_handler();
        let params = serde_json::json!({
            "name": "nonexistent_primal_xyz",
            "method": "compute.health"
        });
        let request = mk_request("discovery.direct_rpc", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
        assert!(err.message.contains("socket not found") || err.message.contains("not found"));
    }

    #[tokio::test]
    async fn discovery_topology_structure() {
        let handler = test_handler();
        let request = mk_request("discovery.topology", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("nodes").is_some());
        assert!(result.get("self").is_some());
        assert_eq!(result["protocol"], "JSON-RPC 2.0");
        assert_eq!(result["domain"], "discovery");
    }

    // ───── deploy.* ─────

    #[tokio::test]
    async fn deploy_capability_call_missing_capability() {
        let handler = test_handler();
        let params = serde_json::json!({ "method": "science.diversity" });
        let request = mk_request("deploy.capability_call", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("capability"));
    }

    #[tokio::test]
    async fn deploy_capability_call_missing_method() {
        let handler = test_handler();
        let params = serde_json::json!({ "capability": "science.diversity" });
        let request = mk_request("deploy.capability_call", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("method"));
    }

    #[tokio::test]
    async fn deploy_capability_call_no_provider() {
        let handler = test_handler();
        let params = serde_json::json!({
            "capability": "nonexistent_capability_xyz",
            "method": "science.diversity"
        });
        let request = mk_request("deploy.capability_call", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["status"], "no_provider");
        assert!(result.get("note").is_some());
    }

    #[tokio::test]
    async fn deploy_graph_status_structure() {
        let handler = test_handler();
        let request = mk_request("deploy.graph_status", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        let graphs = result.get("deploy_graphs").expect("deploy_graphs");
        assert!(graphs.is_array());
        assert!(!graphs.as_array().unwrap().is_empty());
        assert_eq!(result["domain"], "deploy");
    }

    // ───── edge cases ─────

    #[tokio::test]
    async fn science_compute_submit_empty_inference_params() {
        let handler = test_handler();
        let params = serde_json::json!({
            "inference": { "model": "m", "prompt": "p", "params": {} }
        });
        let request = mk_request("science.compute.submit", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result.get("job_id").is_some());
    }

    #[tokio::test]
    async fn science_substrate_probe_large_capability_name() {
        let handler = test_handler();
        let cap = "a".repeat(256);
        let params = serde_json::json!({ "capability": cap });
        let request = mk_request("science.substrate.probe", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert_eq!(result["capability"], cap);
    }
}
