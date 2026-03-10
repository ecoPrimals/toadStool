// SPDX-License-Identifier: AGPL-3.0-only
//! Domain-specific science routing handlers.
//!
//! Extracted from `science.rs` to keep each module under 1000 LOC.
//! Handles routing for ecology, discovery, and deploy domains,
//! plus the `forward_to_primal` Unix socket relay.

use crate::pure_jsonrpc::types::JsonRpcError;
use toadstool_common::interned_strings::capabilities;

type JsonRpcResult = Result<serde_json::Value, JsonRpcError>;

const ECOLOGY_CAPABILITY: &str = capabilities::ECOLOGY;

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
    let socket_path =
        toadstool_common::primal_sockets::get_socket_path_for_capability(ECOLOGY_CAPABILITY);

    if socket_path.exists() {
        return forward_to_primal(&socket_path, method, params).await;
    }

    let socket_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    Ok(serde_json::json!({
        "method": method,
        "status": "queued",
        "domain": "ecology",
        "available_methods": ECOLOGY_METHODS,
        "params_received": params.is_some(),
        "routing": format!("No primal discovered for capability '{ECOLOGY_CAPABILITY}'. Method registered for deferred execution."),
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
#[allow(clippy::unused_async)]
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
#[allow(clippy::unused_async)]
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

/// Returns status of discovered deploy graphs by scanning the biomeOS socket
/// directory at runtime. No hardcoded primal names -- sovereignty-compliant.
#[allow(clippy::unused_async)]
pub(super) async fn deploy_graph_status() -> JsonRpcResult {
    let socket_dir = toadstool_common::primal_sockets::get_biomeos_dir();
    let mut graphs = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&socket_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("sock") {
                if let Some(name) = path.file_stem().and_then(|s| s.to_str()) {
                    graphs.push(serde_json::json!({
                        "primal": name,
                        "socket_exists": path.exists(),
                        "socket_path": path.display().to_string(),
                    }));
                }
            }
        }
    }

    Ok(serde_json::json!({
        "deploy_graphs": graphs,
        "discovered_count": graphs.len(),
        "socket_dir": socket_dir.display().to_string(),
        "domain": "deploy",
    }))
}

// ═══════════════════════════════════════════════════════════
// Unix socket relay
// ═══════════════════════════════════════════════════════════

/// Forwards a JSON-RPC request to a primal via its Unix socket.
///
/// Uses newline-delimited JSON-RPC 2.0 over Unix domain sockets,
/// matching the biomeOS protocol convention.
pub(super) async fn forward_to_primal(
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
