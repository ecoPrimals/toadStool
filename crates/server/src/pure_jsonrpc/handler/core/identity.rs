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
pub(crate) async fn capabilities_list(
    semantic_registry: &SemanticMethodRegistry,
    version: &str,
) -> JsonRpcResult {
    let methods = all_callable_methods(semantic_registry);

    let capabilities = serde_json::json!([
        {
            "type": "compute",
            "methods": ["execute", "submit", "status", "result", "cancel", "list",
                        "dispatch.submit", "dispatch.status", "dispatch.result",
                        "dispatch.forward", "dispatch.capabilities",
                        "dispatch.pipeline.submit", "dispatch.pipeline.status",
                        "fan_out",
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
                        "list_workloads", "validate", "query_capabilities", "health", "version",
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
            "methods": ["list", "status", "reacquire", "swap", "warm_catch"],
            "description": "glowPlug/ember GPU device lifecycle"
        },
        {
            "type": "device",
            "methods": ["vfio.open", "vfio.roundtrip", "gr.init"],
            "description": "VFIO device management and GR context initialization"
        },
        {
            "type": "mmio",
            "methods": ["read32", "write32", "batch", "pramin.read32", "bar0.probe", "falcon.status"],
            "description": "BAR0 register access and falcon diagnostics"
        },
        {
            "type": "btsp",
            "methods": ["capabilities"],
            "description": "BTSP transport security (ChaCha20-Poly1305 + HKDF)"
        }
    ]);
    let cap_count = capabilities.as_array().map_or(0, Vec::len);

    Ok(serde_json::json!({
        "primal": toadstool_common::constants::PRIMAL_NAME,
        "version": version,
        "capabilities": capabilities,
        "count": cap_count,
        "methods": methods,
        "provided_capabilities": capabilities,
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

/// `primal.announce` — self-registration broadcast for Neural API discovery.
///
/// Returns the primal's identity, capabilities, socket path, cost hints,
/// and latency estimates so biomeOS Neural API can build routing weights
/// and utilization tracking. Wire format per Wave 42/43 Neural API standard.
pub(crate) async fn primal_announce(
    version: &str,
    semantic_registry: &SemanticMethodRegistry,
    bound_socket: Option<&std::path::Path>,
) -> JsonRpcResult {
    use toadstool_common::interned_strings::socket_env;

    let methods = all_callable_methods(semantic_registry);
    let socket_name = format!("{}.sock", toadstool_common::constants::CAPABILITY_DOMAIN);
    let socket = if let Some(path) = bound_socket {
        path.to_string_lossy().into_owned()
    } else if let Ok(dir) = std::env::var(socket_env::BIOMEOS_SOCKET_DIR) {
        format!("{dir}/{socket_name}")
    } else {
        std::env::var(socket_env::XDG_RUNTIME_DIR).map_or_else(
            |_| std::env::temp_dir().join("biomeos").join(&socket_name).to_string_lossy().into_owned(),
            |d| format!("{d}/biomeos/{socket_name}"),
        )
    };

    let devices = crate::glowplug_client::discover_gpu_bdfs();

    Ok(serde_json::json!({
        "primal": PRIMAL_NAME,
        "version": version,
        "domain": "compute",
        "capabilities": ["compute", "science", "inference"],
        "methods": methods,
        "count": methods.len(),
        "protocol": "jsonrpc-2.0",
        "transport": ["uds", "tcp"],
        "socket": socket,
        "socket_name": socket_name,
        "signal_tiers": ["node"],
        "cost_hints": {
            "compute": 100.0,
            "science": 50.0,
            "inference": 80.0,
        },
        "latency_estimates": {
            "compute": 200,
            "science": 100,
            "inference": 150,
        },
        "devices": devices,
        "status": "ready",
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
        let result = capabilities_list(&reg, "0.1.0")
            .await
            .expect("capabilities_list");
        assert_eq!(result["primal"], PRIMAL_NAME);
        assert_eq!(result["version"], "0.1.0");
        assert_eq!(result["protocol"], "jsonrpc-2.0");
        assert!(result["methods"].is_array());
        assert!(result["provided_capabilities"].is_array());
        assert!(result["consumed_capabilities"].is_array());
        assert!(result["cost_estimates"].is_object());
        assert!(result["operation_dependencies"].is_object());
        let transport = result["transport"].as_array().expect("transport array");
        assert!(transport.iter().any(|t| t == "uds"));
        assert!(transport.iter().any(|t| t == "tcp"));
    }

    #[tokio::test]
    async fn test_capabilities_list_provided_types() {
        let reg = empty_registry();
        let result = capabilities_list(&reg, "0.1.0")
            .await
            .expect("capabilities_list");
        let provided = result["provided_capabilities"]
            .as_array()
            .expect("provided_capabilities");
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
        let result = discover_capabilities(&reg, "0.1.0")
            .await
            .expect("discover_capabilities");
        assert_eq!(result["primal"], PRIMAL_NAME);
        assert_eq!(result["version"], "0.1.0");
        let caps = result["node_capabilities"]
            .as_array()
            .expect("node_capabilities");
        assert!(caps.iter().any(|c| c == "compute"));
        assert!(caps.iter().any(|c| c == "gpu"));
        assert!(caps.iter().any(|c| c == "wasm"));
    }

    #[tokio::test]
    async fn test_identity_get_structure() {
        let reg = empty_registry();
        let result = identity_get("0.1.0", &reg).await.expect("identity_get");
        assert_eq!(result["primal"], PRIMAL_NAME);
        assert_eq!(result["version"], "0.1.0");
        assert_eq!(result["domain"], "compute");
        assert_eq!(result["license"], "AGPL-3.0-or-later");
        assert_eq!(result["protocol"], "JSON-RPC 2.0");
        assert_eq!(result["transport"], "unix-socket");
        let caps = result["capabilities"].as_array().expect("capabilities");
        assert!(caps.iter().any(|c| c == "compute"));
    }

    #[tokio::test]
    async fn test_identity_get_socket_name() {
        let reg = empty_registry();
        let result = identity_get("0.1.0", &reg).await.expect("identity_get");
        let sock = result["socket_name"].as_str().expect("socket_name");
        assert!(
            std::path::Path::new(sock)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("sock"))
        );
        assert!(sock.contains("compute"));
    }

    #[tokio::test]
    async fn test_capabilities_list_stadial_envelope() {
        let reg = empty_registry();
        let result = capabilities_list(&reg, "0.2.0")
            .await
            .expect("capabilities_list");
        assert!(result["capabilities"].is_array());
        let count = result["count"].as_u64().expect("count");
        assert!(count > 0);
        let capabilities = result["capabilities"].as_array().expect("capabilities");
        assert_eq!(count as usize, capabilities.len());
        let cap_types: Vec<&str> = capabilities
            .iter()
            .filter_map(|c| c["type"].as_str())
            .collect();
        assert!(cap_types.contains(&"btsp"));
        assert!(cap_types.contains(&"device"));
    }

    #[tokio::test]
    async fn test_primal_announce_structure() {
        let reg = empty_registry();
        let result = primal_announce("0.2.0", &reg, None).await.expect("primal_announce");
        assert_eq!(result["primal"], PRIMAL_NAME);
        assert_eq!(result["version"], "0.2.0");
        assert_eq!(result["domain"], "compute");
        assert_eq!(result["status"], "ready");
        assert!(result["capabilities"].is_array());
        assert!(result["methods"].is_array());
        let count = result["count"].as_u64().expect("count");
        let methods = result["methods"].as_array().expect("methods");
        assert_eq!(count as usize, methods.len());
        let transport = result["transport"].as_array().expect("transport");
        assert!(transport.iter().any(|t| t == "uds"));
    }

    #[tokio::test]
    async fn test_primal_announce_reports_bound_socket_path() {
        let reg = empty_registry();
        let bound = std::path::Path::new("/tmp/custom-bound/compute.sock");
        let result = primal_announce("0.2.0", &reg, Some(bound))
            .await
            .expect("primal_announce");
        assert_eq!(result["socket"].as_str(), Some("/tmp/custom-bound/compute.sock"));
    }

    #[tokio::test]
    async fn test_primal_announce_wave43_neural_api_fields() {
        let reg = empty_registry();
        let result = primal_announce("0.2.0", &reg, None).await.expect("primal_announce");

        assert!(result["socket"].is_string());
        let socket = result["socket"].as_str().expect("socket path");
        assert!(socket.contains("compute.sock"));

        let caps = result["capabilities"].as_array().expect("capabilities");
        let cap_strs: Vec<&str> = caps.iter().filter_map(|v| v.as_str()).collect();
        assert!(cap_strs.contains(&"compute"));
        assert!(cap_strs.contains(&"science"));
        assert!(cap_strs.contains(&"inference"));

        let tiers = result["signal_tiers"].as_array().expect("signal_tiers");
        assert!(tiers.iter().any(|t| t == "node"));

        let cost = &result["cost_hints"];
        assert_eq!(cost["compute"], 100.0);
        assert_eq!(cost["science"], 50.0);
        assert_eq!(cost["inference"], 80.0);

        let latency = &result["latency_estimates"];
        assert_eq!(latency["compute"], 200);
        assert_eq!(latency["science"], 100);
        assert_eq!(latency["inference"], 150);
    }
}
