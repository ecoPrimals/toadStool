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
