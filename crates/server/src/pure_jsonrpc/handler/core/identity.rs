// SPDX-License-Identifier: AGPL-3.0-or-later
//! Identity, capability discovery, and Wire Standard L3 `capabilities.list`.

use toadstool::semantic_methods::SemanticMethodRegistry;
use toadstool_common::constants::PRIMAL_NAME;

use super::DIRECT_JSONRPC_METHODS;
use super::JsonRpcResult;
use super::wire_l3::{cost_estimates, operation_dependencies};

/// Builds the sorted flat methods list from direct routes + semantic registry.
fn all_callable_methods(semantic_registry: &SemanticMethodRegistry) -> Vec<&str> {
    let mut methods: Vec<&str> = DIRECT_JSONRPC_METHODS.to_vec();

    for m in semantic_registry.semantic_names() {
        if !methods.contains(&m) {
            methods.push(m);
        }
    }
    methods.sort_unstable();
    methods
}

/// Wire Standard L3 `capabilities.list` response.
///
/// Returns `{primal, version, methods, provided_capabilities, cost_estimates,
/// operation_dependencies, consumed_capabilities}` per
/// `CAPABILITY_WIRE_STANDARD.md` v1.0.
///
/// Cost model: energy/time/compute, not monetary. Dollar value is an
/// end-user concern built on top of these primitives.
#[allow(
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
                "methods": ["execute", "submit", "status", "result", "cancel", "list",
                            "dispatch.submit", "dispatch.status", "dispatch.result",
                            "dispatch.forward", "dispatch.capabilities",
                            "dispatch.pipeline.submit", "dispatch.pipeline.status",
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
                "type": PRIMAL_NAME,
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
        "cost_estimates": cost_estimates(),
        "operation_dependencies": operation_dependencies(),
        "protocol": "jsonrpc-2.0",
        "transport": ["uds", "tcp"]
    }))
}

/// Returns discovered capabilities including semantic methods.
///
/// Legacy `compute.discover_capabilities` — returns node capabilities
/// and merged method list.
#[allow(
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
#[allow(
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
        "socket_name": format!("{}.sock", toadstool_common::constants::CAPABILITY_DOMAIN),
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_registry() -> SemanticMethodRegistry {
        SemanticMethodRegistry::default()
    }

    #[test]
    fn test_all_callable_methods_includes_direct_routes() {
        let reg = empty_registry();
        let methods = all_callable_methods(&reg);
        assert!(methods.contains(&"capabilities.list"));
        assert!(methods.contains(&"health.liveness"));
        assert!(methods.contains(&"identity.get"));
    }

    #[test]
    fn test_all_callable_methods_sorted() {
        let reg = empty_registry();
        let methods = all_callable_methods(&reg);
        let mut sorted = methods.clone();
        sorted.sort_unstable();
        assert_eq!(methods, sorted);
    }

    #[tokio::test]
    async fn test_capabilities_list_structure() {
        let reg = empty_registry();
        let result = capabilities_list(&reg, "0.1.0").await.unwrap();
        assert_eq!(result["primal"], PRIMAL_NAME);
        assert_eq!(result["version"], "0.1.0");
        assert_eq!(result["protocol"], "jsonrpc-2.0");
        assert!(result["methods"].is_array());
        assert!(result["provided_capabilities"].is_array());
        assert!(result["consumed_capabilities"].is_array());
        assert!(result["cost_estimates"].is_object());
        assert!(result["operation_dependencies"].is_object());
        let transport = result["transport"].as_array().unwrap();
        assert!(transport.iter().any(|t| t == "uds"));
        assert!(transport.iter().any(|t| t == "tcp"));
    }

    #[tokio::test]
    async fn test_capabilities_list_provided_types() {
        let reg = empty_registry();
        let result = capabilities_list(&reg, "0.1.0").await.unwrap();
        let provided = result["provided_capabilities"].as_array().unwrap();
        let types: Vec<&str> = provided.iter().filter_map(|c| c["type"].as_str()).collect();
        assert!(types.contains(&"compute"));
        assert!(types.contains(&PRIMAL_NAME));
        assert!(types.contains(&"gpu"));
        assert!(types.contains(&"gate"));
        assert!(types.contains(&"transport"));
        assert!(types.contains(&"shader"));
        assert!(types.contains(&"ember"));
    }

    #[tokio::test]
    async fn test_discover_capabilities_structure() {
        let reg = empty_registry();
        let result = discover_capabilities(&reg, "0.1.0").await.unwrap();
        assert_eq!(result["primal"], PRIMAL_NAME);
        assert_eq!(result["version"], "0.1.0");
        let caps = result["node_capabilities"].as_array().unwrap();
        assert!(caps.iter().any(|c| c == "compute"));
        assert!(caps.iter().any(|c| c == "gpu"));
        assert!(caps.iter().any(|c| c == "wasm"));
    }

    #[tokio::test]
    async fn test_identity_get_structure() {
        let reg = empty_registry();
        let result = identity_get("0.1.0", &reg).await.unwrap();
        assert_eq!(result["primal"], PRIMAL_NAME);
        assert_eq!(result["version"], "0.1.0");
        assert_eq!(result["domain"], "compute");
        assert_eq!(result["license"], "AGPL-3.0-or-later");
        assert_eq!(result["protocol"], "JSON-RPC 2.0");
        assert_eq!(result["transport"], "unix-socket");
        let caps = result["capabilities"].as_array().unwrap();
        assert!(caps.iter().any(|c| c == "compute"));
    }

    #[tokio::test]
    async fn test_identity_get_socket_name() {
        let reg = empty_registry();
        let result = identity_get("0.1.0", &reg).await.unwrap();
        let sock = result["socket_name"].as_str().unwrap();
        assert!(
            std::path::Path::new(sock)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sock"))
        );
        assert!(sock.contains("compute"));
    }
}
