// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core handlers for JSON-RPC: health, version, capabilities, GPU info.
//!
//! Provides health checks, version information, capability discovery,
//! and GPU device/memory queries.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use toadstool::semantic_methods::SemanticMethodRegistry;

use crate::pure_jsonrpc::types::JsonRpcError;
use crate::rpc_types::HealthStatus;

type JsonRpcResult = Result<serde_json::Value, JsonRpcError>;

/// Returns health status with uptime and error count.
#[allow(clippy::unused_async)]
pub(super) async fn health(
    version: &str,
    start_time: std::time::Instant,
    error_count: &AtomicU64,
) -> JsonRpcResult {
    let uptime = start_time.elapsed();
    #[allow(clippy::cast_possible_truncation)]
    let error_count_val = error_count.load(Ordering::Relaxed) as usize;
    let status = HealthStatus {
        healthy: true,
        version: version.to_string(),
        uptime_secs: uptime.as_secs(),
        active_workloads: 0,
        queued_workloads: 0,
        error_count: error_count_val,
        resource_utilization: 0.0,
    };
    serde_json::to_value(status)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}")))
}

/// Returns version and protocol information.
#[allow(clippy::unused_async)]
pub(super) async fn version_info(version: &str) -> JsonRpcResult {
    let mut info = HashMap::new();
    info.insert(String::from("version"), version.to_string());
    info.insert(String::from("protocol"), String::from("JSON-RPC 2.0"));
    info.insert(String::from("service"), String::from("ToadStool Compute"));
    info.insert(
        String::from("implementation"),
        String::from("Pure Rust (ecoPrimals sovereign pattern)"),
    );
    Ok(serde_json::json!(info))
}

/// Returns discovered capabilities including semantic methods.
#[allow(clippy::unused_async)]
pub(super) async fn discover_capabilities(
    semantic_registry: &SemanticMethodRegistry,
    version: &str,
) -> JsonRpcResult {
    let semantic_methods: Vec<&str> = semantic_registry.semantic_names().into_iter().collect();

    let mut direct_methods = vec![
        "toadstool.health",
        "toadstool.version",
        "toadstool.query_capabilities",
        "toadstool.resources.estimate",
        "toadstool.resources.validate_availability",
        "toadstool.resources.suggest_optimizations",
        "resources.estimate",
        "resources.validate_availability",
        "resources.suggest_optimizations",
        "compute.health",
        "compute.version",
        "compute.capabilities",
        "compute.discover_capabilities",
        "compute.submit",
        "compute.status",
        "compute.result",
        "compute.cancel",
        "compute.list",
        "ai.local_inference",
        "ai.local_execute",
        "gpu.info",
        "gpu.memory",
        "ollama.list_models",
        "ollama.inference",
        "ollama.load",
        "ollama.unload",
        "gate.update",
        "gate.remove",
        "gate.list",
        "gate.route",
        "transport.discover",
        "transport.list",
        "transport.route",
    ];

    for m in &semantic_methods {
        if !direct_methods.contains(m) {
            direct_methods.push(m);
        }
    }
    direct_methods.sort_unstable();

    let capabilities = serde_json::json!({
        "node_capabilities": [
            "compute", "workload", "orchestration", "ai_local",
            "gpu", "wasm", "container", "hardware_transport",
            "science", "shader", "ecology", "discovery", "deploy"
        ],
        "methods": direct_methods,
        "version": version,
        "primal": toadstool_common::constants::PRIMAL_NAME
    });
    Ok(capabilities)
}

/// Returns GPU device and backend information.
#[allow(clippy::unused_async)]
pub(super) async fn gpu_info() -> JsonRpcResult {
    Ok(serde_json::json!({
        "devices": crate::gpu_system::query_gpu_devices(),
        "driver": "wgpu",
        "compute_backends": crate::gpu_system::query_available_backends(),
    }))
}

/// Returns GPU memory information per device.
#[allow(clippy::unused_async)]
pub(super) async fn gpu_memory() -> JsonRpcResult {
    Ok(serde_json::json!({
        "devices": crate::gpu_system::query_gpu_memory(),
    }))
}
