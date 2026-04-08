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
///
/// Wire Standard L1: includes `"status": "alive"` for biomeOS liveness probes.
#[expect(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn health(
    version: &str,
    start_time: std::time::Instant,
    error_count: &AtomicU64,
) -> JsonRpcResult {
    let uptime = start_time.elapsed();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "error count u64→usize is lossless on 64-bit"
    )]
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
    let mut value = serde_json::to_value(status)
        .map_err(|e| JsonRpcError::internal_error(format!("Serialization error: {e}")))?;
    // Wire Standard L1: biomeOS probes expect "status": "alive"
    if let Some(obj) = value.as_object_mut() {
        obj.insert("status".into(), serde_json::Value::String("alive".into()));
    }
    Ok(value)
}

/// Returns version and protocol information.
#[expect(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn version_info(version: &str) -> JsonRpcResult {
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

/// Builds the sorted flat methods list from direct routes + semantic registry.
fn all_callable_methods(semantic_registry: &SemanticMethodRegistry) -> Vec<&str> {
    let semantic_methods: Vec<&str> = semantic_registry.semantic_names().into_iter().collect();

    let mut methods = vec![
        "capabilities.list",
        "identity.get",
        "health.liveness",
        "health.readiness",
        "health.check",
        "toadstool.health",
        "toadstool.version",
        "toadstool.submit_workload",
        "toadstool.query_status",
        "toadstool.cancel_workload",
        "toadstool.list_workloads",
        "toadstool.query_capabilities",
        "toadstool.resources.estimate",
        "toadstool.resources.validate_availability",
        "toadstool.resources.suggest_optimizations",
        "resources.estimate",
        "resources.validate_availability",
        "resources.suggest_optimizations",
        "ai.local_inference",
        "ai.local_execute",
        "compute.health",
        "compute.version",
        "compute.capabilities",
        "compute.discover_capabilities",
        "compute.submit",
        "compute.status",
        "compute.result",
        "compute.cancel",
        "compute.list",
        "compute.dispatch.submit",
        "compute.dispatch.status",
        "compute.dispatch.result",
        "compute.dispatch.forward",
        "compute.dispatch.capabilities",
        "compute.hardware.observe",
        "compute.hardware.distill",
        "compute.hardware.apply",
        "compute.hardware.share_recipe",
        "compute.hardware.auto_init",
        "compute.hardware.auto_init_all",
        "compute.hardware.status",
        "compute.hardware.vfio_devices",
        "compute.performance_surface.report",
        "compute.performance_surface.query",
        "compute.performance_surface.list",
        "compute.route.multi_unit",
        "gpu.query_info",
        "gpu.query_memory",
        "gpu.query_telemetry",
        "gate.update",
        "gate.remove",
        "gate.list",
        "gate.route",
        "transport.discover",
        "transport.list",
        "transport.route",
        "transport.open",
        "transport.stream",
        "transport.status",
        "shader.dispatch",
        "ember.list",
        "ember.status",
        "provenance.query",
    ];

    for m in &semantic_methods {
        if !methods.contains(m) {
            methods.push(m);
        }
    }
    methods.sort_unstable();
    methods
}

/// Wire Standard L2 `capabilities.list` response.
///
/// Returns `{primal, version, methods, provided_capabilities}` per
/// `CAPABILITY_WIRE_STANDARD.md` v1.0.
#[expect(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn capabilities_list(
    semantic_registry: &SemanticMethodRegistry,
    version: &str,
) -> JsonRpcResult {
    let methods = all_callable_methods(semantic_registry);

    Ok(serde_json::json!({
        "primal": toadstool_common::constants::PRIMAL_NAME,
        "version": version,
        "methods": methods,
        "provided_capabilities": [
            {
                "type": "compute",
                "methods": ["submit", "status", "result", "cancel", "list",
                            "dispatch.submit", "dispatch.status", "dispatch.result",
                            "dispatch.forward", "dispatch.capabilities",
                            "hardware.observe", "hardware.distill", "hardware.apply",
                            "hardware.share_recipe", "hardware.auto_init",
                            "hardware.auto_init_all", "hardware.status",
                            "hardware.vfio_devices",
                            "performance_surface.report", "performance_surface.query",
                            "performance_surface.list", "route.multi_unit",
                            "health", "version", "capabilities", "discover_capabilities"],
                "version": version,
                "description": "GPU job queue, hardware dispatch, and performance routing"
            },
            {
                "type": "toadstool",
                "methods": ["submit_workload", "query_status", "cancel_workload",
                            "list_workloads", "query_capabilities", "health", "version",
                            "resources.estimate", "resources.validate_availability",
                            "resources.suggest_optimizations"],
                "version": version,
                "description": "High-level workload executor (multi-runtime)"
            },
            {
                "type": "gpu",
                "methods": ["query_info", "query_memory", "query_telemetry"],
                "description": "GPU hardware info and telemetry"
            },
            {
                "type": "gate",
                "methods": ["update", "remove", "list", "route"],
                "description": "Distributed cross-gate routing"
            },
            {
                "type": "transport",
                "methods": ["discover", "list", "route", "open", "stream", "status"],
                "description": "Hardware transport (DRM, V4L2, serial)"
            },
            {
                "type": "shader",
                "methods": ["dispatch"],
                "description": "Sovereign shader dispatch (VFIO/DRM passthrough)"
            },
            {
                "type": "ember",
                "methods": ["list", "status"],
                "description": "glowPlug/ember GPU device lifecycle"
            }
        ],
        "consumed_capabilities": [
            "security.sign",
            "security.verify",
            "storage.artifact.store",
            "storage.artifact.retrieve",
            "coordination.register",
            "coordination.discover"
        ],
        "protocol": "jsonrpc-2.0",
        "transport": ["uds", "tcp"]
    }))
}

/// Returns discovered capabilities including semantic methods.
///
/// Legacy `compute.discover_capabilities` — returns node capabilities
/// and merged method list.
#[expect(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn discover_capabilities(
    semantic_registry: &SemanticMethodRegistry,
    version: &str,
) -> JsonRpcResult {
    let methods = all_callable_methods(semantic_registry);

    Ok(serde_json::json!({
        "node_capabilities": [
            "compute", "workload", "orchestration",
            "gpu", "wasm", "container", "hardware_transport",
            "shader_dispatch", "hardware_learning"
        ],
        "methods": methods,
        "version": version,
        "primal": toadstool_common::constants::PRIMAL_NAME
    }))
}

/// Returns primal identity per Wire Standard L2 + `CAPABILITY_BASED_DISCOVERY_STANDARD.md`.
///
/// Every primal MUST implement `identity.get` so orchestrators and peers
/// can discover name, version, capabilities, and protocol.
#[expect(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn identity_get(
    version: &str,
    semantic_registry: &SemanticMethodRegistry,
) -> JsonRpcResult {
    let semantic_methods: Vec<&str> = semantic_registry.semantic_names().into_iter().collect();

    let capabilities: Vec<&str> = vec![
        "compute",
        "workload",
        "gpu",
        "wasm",
        "container",
        "hardware_transport",
        "shader_dispatch",
        "hardware_learning",
    ];

    Ok(serde_json::json!({
        "primal": toadstool_common::constants::PRIMAL_NAME,
        "version": version,
        "domain": "compute",
        "license": "AGPL-3.0-or-later",
        "protocol": "JSON-RPC 2.0",
        "capabilities": capabilities,
        "methods": semantic_methods,
        "transport": "unix-socket",
        "socket_name": format!("{}.sock", toadstool_common::constants::PRIMAL_NAME),
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
#[expect(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
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
#[expect(
    clippy::unused_async,
    reason = "handler signature requires async for uniform dispatch"
)]
pub(crate) async fn gpu_memory() -> JsonRpcResult {
    Ok(serde_json::json!({
        "devices": crate::gpu_system::query_gpu_memory(),
    }))
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant};

    use toadstool::semantic_methods::SemanticMethodRegistry;

    use super::*;

    #[tokio::test]
    async fn health_includes_version_uptime_error_count_and_wire_status() {
        let ver = "unit-test-9.9.9";
        let start = Instant::now()
            .checked_sub(Duration::from_secs(2))
            .expect("instant");
        let errors = AtomicU64::new(7);
        let v = health(ver, start, &errors).await.expect("health ok");
        assert_eq!(v["healthy"], true);
        assert_eq!(
            v["status"], "alive",
            "Wire Standard L1: status must be 'alive'"
        );
        assert_eq!(v["version"], ver);
        assert!(v["uptime_secs"].as_u64().unwrap() >= 2);
        assert_eq!(v["error_count"], 7);
        errors.fetch_add(1, Ordering::Relaxed);
        let v2 = health(ver, start, &errors).await.expect("health ok");
        assert_eq!(v2["error_count"], 8);
        assert_eq!(v2["status"], "alive");
    }

    #[tokio::test]
    async fn version_info_maps_expected_keys() {
        let v = version_info("v-x").await.expect("version_info");
        assert_eq!(v["version"], "v-x");
        assert_eq!(v["protocol"], "JSON-RPC 2.0");
        assert_eq!(v["service"], "ToadStool Compute");
        assert!(v["implementation"].as_str().unwrap().contains("Pure Rust"));
    }

    #[tokio::test]
    async fn discover_capabilities_merges_registry_and_sorts_methods() {
        let reg = SemanticMethodRegistry::new();
        let cap = discover_capabilities(&reg, "cap-ver-1")
            .await
            .expect("discover_capabilities");
        let methods = cap["methods"].as_array().expect("methods array");
        assert!(!methods.is_empty(), "methods should be non-empty");
        let as_strs: Vec<&str> = methods
            .iter()
            .map(|m| m.as_str().expect("string"))
            .collect();
        let mut sorted = as_strs.clone();
        sorted.sort_unstable();
        assert_eq!(as_strs, sorted, "methods must be sorted");
        assert!(
            as_strs.contains(&"compute.execute"),
            "registry-only semantic should be merged in"
        );
        assert_eq!(cap["version"], "cap-ver-1");
        let node_caps = cap["node_capabilities"].as_array().expect("node caps");
        assert!(node_caps.iter().any(|c| c.as_str() == Some("compute")));
    }

    #[tokio::test]
    async fn capabilities_list_returns_wire_standard_envelope() {
        let reg = SemanticMethodRegistry::new();
        let cap = capabilities_list(&reg, "wire-1.0")
            .await
            .expect("capabilities_list");
        assert_eq!(cap["primal"], toadstool_common::constants::PRIMAL_NAME);
        assert_eq!(cap["version"], "wire-1.0");

        let methods = cap["methods"].as_array().expect("methods array");
        assert!(!methods.is_empty());
        let strs: Vec<&str> = methods.iter().map(|m| m.as_str().unwrap()).collect();
        let mut sorted = strs.clone();
        sorted.sort_unstable();
        assert_eq!(strs, sorted, "methods must be sorted");

        assert!(
            strs.contains(&"capabilities.list"),
            "must advertise capabilities.list"
        );
        assert!(
            strs.contains(&"health.liveness"),
            "must advertise health.liveness"
        );
        assert!(
            strs.contains(&"identity.get"),
            "must advertise identity.get"
        );
        assert!(
            strs.contains(&"compute.submit"),
            "must advertise compute.submit"
        );
        assert!(
            strs.contains(&"shader.dispatch"),
            "must advertise shader.dispatch"
        );
        assert!(strs.contains(&"ember.list"), "must advertise ember.list");

        let groups = cap["provided_capabilities"]
            .as_array()
            .expect("provided_capabilities");
        assert!(groups.len() >= 5, "should have multiple capability groups");
        let group_types: Vec<&str> = groups.iter().map(|g| g["type"].as_str().unwrap()).collect();
        assert!(group_types.contains(&"compute"));
        assert!(group_types.contains(&"toadstool"));
        assert!(group_types.contains(&"gpu"));
        assert!(group_types.contains(&"transport"));
        assert!(group_types.contains(&"shader"));

        assert!(cap["consumed_capabilities"].as_array().is_some());
        assert_eq!(cap["protocol"], "jsonrpc-2.0");
    }

    #[tokio::test]
    async fn identity_get_includes_domain_and_license() {
        let reg = SemanticMethodRegistry::new();
        let id = identity_get("id-ver", &reg).await.expect("identity_get");
        assert_eq!(id["primal"], toadstool_common::constants::PRIMAL_NAME);
        assert_eq!(id["version"], "id-ver");
        assert_eq!(id["domain"], "compute", "Wire Standard L2: domain field");
        assert_eq!(
            id["license"], "AGPL-3.0-or-later",
            "Wire Standard L2: license field"
        );
        assert_eq!(id["protocol"], "JSON-RPC 2.0");
        assert_eq!(id["transport"], "unix-socket");
        let sock = id["socket_name"].as_str().expect("socket_name");
        assert!(
            std::path::Path::new(sock)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sock"))
        );
        let caps = id["capabilities"].as_array().expect("capabilities");
        assert!(caps.iter().any(|c| c.as_str() == Some("compute")));
        let methods = id["methods"].as_array().expect("methods");
        assert!(methods.len() > 3);
    }

    #[tokio::test]
    async fn gpu_info_returns_device_and_driver_shape() {
        let g = gpu_info().await.expect("gpu_info");
        assert!(g.get("devices").is_some());
        assert_eq!(g["driver"], "wgpu");
        assert!(g.get("compute_backends").is_some());
        assert!(g.get("spirv_codegen_safety").is_some());
        assert!(g.get("firmware_inventory").is_some());
    }

    #[tokio::test]
    async fn gpu_memory_returns_devices_array() {
        let m = gpu_memory().await.expect("gpu_memory");
        assert!(m.get("devices").is_some());
    }
}
