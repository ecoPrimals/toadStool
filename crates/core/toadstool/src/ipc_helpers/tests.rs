//! IPC helpers tests

use super::connection::IPC_TIMEOUT;
use super::*;
use serde_json::json;

#[test]
fn test_constants() {
    assert_eq!(IPC_TIMEOUT.as_secs(), 5);
}

#[tokio::test]
async fn test_register_with_songbird_graceful_failure() {
    let result = register_with_songbird().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_msg = format!("{}", err);
    assert!(err_msg.contains("Songbird") || err_msg.contains("connection"));
}

#[tokio::test]
async fn test_resolve_primal_graceful_failure() {
    let result = resolve_primal("beardog").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connect_to_primal_graceful_failure() {
    let result = connect_to_primal("beardog").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_find_by_capability_graceful_failure() {
    let result = find_by_capability("crypto").await;
    assert!(result.is_err());
}

#[test]
fn test_json_rpc_request_format() {
    let request = json!({
        "jsonrpc": toadstool_common::constants::jsonrpc::VERSION,
        "method": "ipc.register",
        "params": {
            "primal_name": "toadstool",
            "capabilities": ["compute"]
        },
        "id": 1
    });
    assert_eq!(request.get("jsonrpc").unwrap(), "2.0");
    assert_eq!(request.get("method").unwrap(), "ipc.register");
    assert!(request.get("params").is_some());
    assert_eq!(request.get("id").unwrap(), 1);
}

#[test]
fn test_resolve_semantic_to_implementation() {
    assert_eq!(resolve_method_name("compute.execute"), "execute_workload");
    assert_eq!(resolve_method_name("resource.health.check"), "check_health");
    assert_eq!(
        resolve_method_name("storage.artifact.store"),
        "store_artifact"
    );
}

#[test]
fn test_resolve_implementation_passthrough() {
    assert_eq!(resolve_method_name("execute_workload"), "execute_workload");
    assert_eq!(resolve_method_name("check_health"), "check_health");
}

#[test]
fn test_resolve_unknown_semantic() {
    assert_eq!(resolve_method_name("unknown.method"), "unknown.method");
    assert_eq!(resolve_method_name("future.api.call"), "future.api.call");
}

#[test]
fn test_is_semantic_method() {
    assert!(is_semantic_method("compute.execute"));
    assert!(is_semantic_method("resource.cpu.get_usage"));
    assert!(!is_semantic_method("execute_workload"));
    assert!(!is_semantic_method("single_word"));
}

#[test]
fn test_get_semantic_name() {
    assert_eq!(
        get_semantic_name("execute_workload"),
        Some("compute.execute".to_string())
    );
    assert_eq!(
        get_semantic_name("check_health"),
        Some("resource.health.check".to_string())
    );
    assert_eq!(get_semantic_name("unknown_method"), None);
}

#[test]
fn test_list_semantic_methods() {
    let methods = list_semantic_methods();
    assert!(methods.len() > 40);
    assert!(methods.contains(&"compute.execute".to_string()));
    assert!(methods.contains(&"resource.health.check".to_string()));
    assert!(methods.contains(&"storage.artifact.store".to_string()));
    assert!(methods.contains(&"network.configure".to_string()));
    assert!(methods.contains(&"security.policy.apply".to_string()));
}

#[test]
fn test_semantic_resolution_bidirectional() {
    let impl_name = resolve_method_name("compute.execute");
    assert_eq!(impl_name, "execute_workload");
    let semantic_name = get_semantic_name(&impl_name);
    assert_eq!(semantic_name, Some("compute.execute".to_string()));
}

#[test]
fn test_runtime_variant_resolution() {
    assert_eq!(
        resolve_method_name("compute.container.run"),
        "run_container"
    );
    assert_eq!(
        resolve_method_name("compute.wasm.execute"),
        "start_wasm_module"
    );
    assert_eq!(
        resolve_method_name("compute.python.execute"),
        "run_python_script"
    );
    assert_eq!(
        resolve_method_name("compute.native.execute"),
        "run_native_binary"
    );
    assert_eq!(
        resolve_method_name("compute.gpu.execute"),
        "run_gpu_compute"
    );
}

#[test]
fn test_all_domains_covered() {
    let methods = list_semantic_methods();
    assert!(methods.iter().any(|m| m.starts_with("compute.")));
    assert!(methods.iter().any(|m| m.starts_with("resource.")));
    assert!(methods.iter().any(|m| m.starts_with("storage.")));
    assert!(methods.iter().any(|m| m.starts_with("network.")));
    assert!(methods.iter().any(|m| m.starts_with("security.")));
    assert!(methods.iter().any(|m| m.starts_with("runtime.")));
}

#[test]
fn test_all_semantic_to_implementation_mappings() {
    assert_eq!(resolve_method_name("compute.execute"), "execute_workload");
    assert_eq!(resolve_method_name("compute.stop"), "stop_workload");
    assert_eq!(
        resolve_method_name("compute.container.run"),
        "run_container"
    );
    assert_eq!(
        resolve_method_name("compute.wasm.execute"),
        "start_wasm_module"
    );
    assert_eq!(
        resolve_method_name("resource.cpu.get_usage"),
        "get_cpu_usage"
    );
    assert_eq!(resolve_method_name("resource.health.check"), "check_health");
    assert_eq!(
        resolve_method_name("storage.artifact.store"),
        "store_artifact"
    );
    assert_eq!(
        resolve_method_name("network.configure"),
        "configure_networking"
    );
    assert_eq!(
        resolve_method_name("security.policy.apply"),
        "apply_security_policies"
    );
    assert_eq!(
        resolve_method_name("runtime.engine.list"),
        "list_runtime_engines"
    );
}

#[test]
fn test_unknown_semantic_names_pass_through() {
    assert_eq!(resolve_method_name("unknown.method"), "unknown.method");
    assert_eq!(resolve_method_name("future.api.call"), "future.api.call");
}

#[test]
fn test_implementation_names_pass_through() {
    assert_eq!(resolve_method_name("execute_workload"), "execute_workload");
    assert_eq!(resolve_method_name("check_health"), "check_health");
    assert_eq!(resolve_method_name("store_artifact"), "store_artifact");
}

#[test]
fn test_is_semantic_method_all_known() {
    let methods = list_semantic_methods();
    for method in &methods {
        assert!(is_semantic_method(method), "{} should be semantic", method);
    }
}

#[test]
fn test_is_semantic_method_non_semantic() {
    assert!(!is_semantic_method("execute_workload"));
    assert!(!is_semantic_method("single_word"));
    assert!(!is_semantic_method(""));
}

#[test]
fn test_get_semantic_name_all_implementations() {
    let pairs = [
        ("execute_workload", "compute.execute"),
        ("check_health", "resource.health.check"),
        ("store_artifact", "storage.artifact.store"),
    ];
    for (impl_name, expected) in pairs {
        assert_eq!(
            get_semantic_name(impl_name),
            Some(expected.to_string()),
            "{}",
            impl_name
        );
    }
}

#[test]
fn test_get_semantic_name_unknown_returns_none() {
    assert_eq!(get_semantic_name("unknown_method"), None);
}

#[test]
fn test_list_semantic_methods_count_and_contents() {
    let methods = list_semantic_methods();
    assert_eq!(methods.len(), 56);
    assert!(methods.contains(&"compute.execute".to_string()));
}

#[test]
fn test_resolution_consistency_roundtrip() {
    let methods = list_semantic_methods();
    for semantic in &methods {
        let impl_name = resolve_method_name(semantic);
        let back = get_semantic_name(&impl_name);
        assert_eq!(back, Some(semantic.clone()), "roundtrip: {}", semantic);
    }
}

#[test]
fn test_edge_cases_semantic_resolution() {
    assert_eq!(resolve_method_name(""), "");
    assert!(!is_semantic_method(""));
    assert_eq!(get_semantic_name(""), None);
    assert_eq!(resolve_method_name(" "), " ");
    assert_eq!(resolve_method_name("."), ".");
    assert!(is_semantic_method("."));
}
