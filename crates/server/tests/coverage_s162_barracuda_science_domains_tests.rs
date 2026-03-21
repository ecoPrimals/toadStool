// SPDX-License-Identifier: AGPL-3.0-only
//! Coverage expansion S162 — barracuda + science_domains + deploy + discovery
//!
//! Targets: barracuda.rs (0%), science_domains.rs (51.89%), dispatch.rs branches

use std::borrow::Cow;
use std::sync::Arc;

use toadstool_server::StandaloneExecutor;
use toadstool_server::pure_jsonrpc::{JsonRpcError, JsonRpcHandler, JsonRpcRequest};

fn test_handler() -> JsonRpcHandler {
    JsonRpcHandler::new(
        Arc::new(StandaloneExecutor::new()),
        "test-s162".to_string(),
        None,
    )
}

fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest<'static> {
    JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Owned(method.to_string()),
        params,
        id: Some(serde_json::json!(id)),
    }
}

// ═══════════════════════════════════════════════════════════
// barracuda.rs — science.activations.list, science.rng.capabilities,
//                science.special.functions
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn barracuda_activations_list_returns_expected_structure() {
    let handler = test_handler();
    let request = mk_request("science.activations.list", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none(), "activations.list should succeed");
    let result = response.result.expect("result present");

    let activations = result["activations"].as_array().expect("activations array");
    assert!(!activations.is_empty());
    let names: Vec<&str> = activations
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert!(names.contains(&"sigmoid"));
    assert!(names.contains(&"relu"));
    assert!(names.contains(&"gelu"));
    assert!(names.contains(&"swish"));
    assert!(names.contains(&"mish"));

    let batch_variants = result["batch_variants"]
        .as_array()
        .expect("batch_variants array");
    assert!(!batch_variants.is_empty());

    assert_eq!(result["precision"], "f64");
    assert_eq!(result["domain"], "science");
    assert!(result["provider"].as_str().is_some());
}

#[tokio::test]
async fn barracuda_rng_capabilities_returns_expected_structure() {
    let handler = test_handler();
    let request = mk_request("science.rng.capabilities", None, 2);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none(), "rng.capabilities should succeed");
    let result = response.result.expect("result present");

    let cpu_prng = result.get("cpu_prng").expect("cpu_prng present");
    assert!(cpu_prng.get("lcg").is_some());
    assert!(cpu_prng.get("uniform_f64").is_some());

    let gpu_prng = result.get("gpu_prng").expect("gpu_prng present");
    assert!(gpu_prng.get("xoshiro128ss").is_some());

    assert_eq!(result["domain"], "science");
}

#[tokio::test]
async fn barracuda_special_functions_returns_expected_structure() {
    let handler = test_handler();
    let request = mk_request("science.special.functions", None, 3);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none(), "special.functions should succeed");
    let result = response.result.expect("result present");

    let functions = result["functions"].as_array().expect("functions array");
    assert!(!functions.is_empty());
    let fn_names: Vec<&str> = functions
        .iter()
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert!(fn_names.contains(&"tridiagonal_ql"));
    assert!(fn_names.contains(&"plasma_dispersion_z"));

    let categories = result.get("categories").expect("categories present");
    assert!(categories.get("eigensolver").is_some());
    assert!(categories.get("plasma_physics").is_some());
    assert!(categories.get("pharmacology").is_some());

    assert_eq!(result["domain"], "science");
    assert!(result["provider"].as_str().is_some());
}

// ═══════════════════════════════════════════════════════════
// science_domains.rs — ecology offload, discovery, deploy
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn ecology_offload_queued_when_no_primal() {
    let handler = test_handler();
    let params = serde_json::json!({"temperature": 25.0, "humidity": 0.6});
    let request = mk_request("ecology.et0_fao56", Some(params), 10);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["status"], "queued");
    assert_eq!(result["domain"], "ecology");
    assert_eq!(result["method"], "ecology.et0_fao56");
    assert!(result["params_received"].as_bool().unwrap());
    assert!(result["available_methods"].as_array().is_some());
    assert!(result["routing"].as_str().unwrap().contains("No primal"));
    assert!(result["discovery_path"].as_str().is_some());
}

#[tokio::test]
async fn ecology_offload_queued_without_params() {
    let handler = test_handler();
    let request = mk_request("ecology.water_balance", None, 11);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["status"], "queued");
    assert!(!result["params_received"].as_bool().unwrap());
}

#[tokio::test]
async fn ecology_offload_various_methods_all_return_queued() {
    let handler = test_handler();
    let methods = [
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

    for (i, method) in methods.iter().enumerate() {
        let request = mk_request(method, None, (100 + i) as i32);
        let response = handler.handle_request(&request).await;
        assert!(response.error.is_none(), "{method} should not error");
        let result = response.result.unwrap();
        assert_eq!(result["status"], "queued", "{method} should be queued");
        assert_eq!(result["method"], *method);
    }
}

#[tokio::test]
async fn discovery_primals_returns_expected_structure() {
    let handler = test_handler();
    let request = mk_request("discovery.primals", None, 20);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["primals"].as_array().is_some());
    assert!(result["count"].as_u64().is_some());
    assert!(result["socket_dir"].as_str().is_some());
    assert_eq!(result["domain"], "discovery");
}

#[tokio::test]
async fn discovery_primal_health_socket_not_found() {
    let handler = test_handler();
    let params = serde_json::json!({"name": "nonexistent_primal"});
    let request = mk_request("discovery.primal_health", Some(params), 21);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["name"], "nonexistent_primal");
    assert!(!result["healthy"].as_bool().unwrap());
    assert_eq!(result["reason"], "Socket not found");
    assert!(result["socket_path"].as_str().is_some());
}

#[tokio::test]
async fn discovery_primal_health_default_name_when_no_params() {
    let handler = test_handler();
    let request = mk_request("discovery.primal_health", None, 22);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["name"], "unknown");
    assert!(!result["healthy"].as_bool().unwrap());
}

#[tokio::test]
async fn discovery_direct_rpc_missing_name_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({"method": "compute.health"});
    let request = mk_request("discovery.direct_rpc", Some(params), 23);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("name"));
}

#[tokio::test]
async fn discovery_direct_rpc_missing_method_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({"name": "toadstool"});
    let request = mk_request("discovery.direct_rpc", Some(params), 24);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("method"));
}

#[tokio::test]
async fn discovery_direct_rpc_socket_not_found_returns_internal_error() {
    let handler = test_handler();
    let params = serde_json::json!({"name": "nonexistent", "method": "compute.health"});
    let request = mk_request("discovery.direct_rpc", Some(params), 25);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("socket not found"));
}

#[tokio::test]
async fn discovery_topology_returns_expected_structure() {
    let handler = test_handler();
    let request = mk_request("discovery.topology", None, 30);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["nodes"].as_array().is_some());
    assert!(result["self"].as_str().is_some());
    assert_eq!(result["protocol"], "JSON-RPC 2.0");
    assert!(result["socket_dir"].as_str().is_some());
    assert_eq!(result["domain"], "discovery");
}

// ═══════════════════════════════════════════════════════════
// deploy domain
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn deploy_capability_call_no_provider_flat_format() {
    let handler = test_handler();
    let params = serde_json::json!({
        "capability": "biology",
        "method": "phylo.infer",
        "params": {"tree": "data"}
    });
    let request = mk_request("deploy.capability_call", Some(params), 40);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["status"], "no_provider");
    assert_eq!(result["capability"], "biology");
    assert_eq!(result["method"], "phylo.infer");
    assert!(
        result["note"]
            .as_str()
            .unwrap()
            .contains("No primal discovered")
    );
}

#[tokio::test]
async fn deploy_capability_call_qualified_method_format() {
    let handler = test_handler();
    let params = serde_json::json!({
        "qualified_method": "biology.phylo.infer",
        "params": {"tree": "data"}
    });
    let request = mk_request("deploy.capability_call", Some(params), 41);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["status"], "no_provider");
    assert_eq!(result["capability"], "biology");
    assert_eq!(result["method"], "phylo.infer");
}

#[tokio::test]
async fn deploy_capability_call_qualified_method_no_dot_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({"qualified_method": "nodot"});
    let request = mk_request("deploy.capability_call", Some(params), 42);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("'.'"));
}

#[tokio::test]
async fn deploy_capability_call_missing_all_params_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({});
    let request = mk_request("deploy.capability_call", Some(params), 43);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn deploy_capability_call_capability_without_method_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({"capability": "biology"});
    let request = mk_request("deploy.capability_call", Some(params), 44);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("method"));
}

#[tokio::test]
async fn deploy_graph_status_returns_expected_structure() {
    let handler = test_handler();
    let request = mk_request("deploy.graph_status", None, 50);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["deploy_graphs"].as_array().is_some());
    assert!(result["discovered_count"].as_u64().is_some());
    assert!(result["socket_dir"].as_str().is_some());
    assert_eq!(result["domain"], "deploy");
}

// ═══════════════════════════════════════════════════════════
// dispatch.rs — expanded branch coverage
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn dispatch_submit_missing_binary_field() {
    let handler = test_handler();
    let params = serde_json::json!({"bdf": "0000:01:00.0"});
    let request = mk_request("compute.dispatch.submit", Some(params), 60);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("binary"));
}

#[tokio::test]
async fn dispatch_submit_with_valid_binary_and_bdf() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [0x01, 0x02, 0x03, 0x04],
        "bdf": "0000:01:00.0",
        "workgroup_size": [64, 1, 1],
        "buffers": [{"size": 1024}],
        "timeout_ms": 5000,
        "dispatch_mode": "drm"
    });
    let request = mk_request("compute.dispatch.submit", Some(params), 61);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "submit");
    assert!(result["job_id"].as_str().is_some());
    assert_eq!(result["bdf"], "0000:01:00.0");
    assert_eq!(result["binary_size"], 4);
}

#[tokio::test]
async fn dispatch_submit_coral_not_available_vfio_mode() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [0x01, 0x02],
        "bdf": "0000:25:00.0",
        "dispatch_mode": "vfio"
    });
    let request = mk_request("compute.dispatch.submit", Some(params), 62);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["status"], "failed");
    assert!(
        result["error"]
            .as_str()
            .unwrap()
            .contains("coralReef not available")
    );
}

#[tokio::test]
async fn dispatch_forward_missing_endpoint() {
    let handler = test_handler();
    let params = serde_json::json!({"binary": [1, 2, 3]});
    let request = mk_request("compute.dispatch.forward", Some(params), 63);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("endpoint"));
}

#[tokio::test]
async fn dispatch_forward_to_unreachable_endpoint() {
    let handler = test_handler();
    let params = serde_json::json!({
        "endpoint": "http://127.0.0.1:99999",
        "binary": [1, 2, 3]
    });
    let request = mk_request("compute.dispatch.forward", Some(params), 64);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("failed"));
}

#[tokio::test]
async fn dispatch_forward_missing_params_entirely() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.forward", None, 65);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_status_missing_job_id() {
    let handler = test_handler();
    let params = serde_json::json!({});
    let request = mk_request("compute.dispatch.status", Some(params), 66);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("job_id"));
}

#[tokio::test]
async fn dispatch_result_missing_job_id() {
    let handler = test_handler();
    let params = serde_json::json!({});
    let request = mk_request("compute.dispatch.result", Some(params), 67);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("job_id"));
}

#[tokio::test]
async fn dispatch_capabilities_structure_validated() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.capabilities", None, 68);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "capabilities");
    assert!(result["sovereign_pipeline"].as_bool().unwrap());
    let modes = result["dispatch_modes"].as_array().unwrap();
    assert!(modes.iter().any(|m| m == "vfio"));
    assert!(modes.iter().any(|m| m == "drm"));
    assert!(result["vfio_gpus"].as_array().is_some());
    assert!(result["drm_gpus"].as_array().is_some());
    assert_eq!(result["coral_reef_available"], false);
}

// ═══════════════════════════════════════════════════════════
// transport.rs — expanded validation paths
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn transport_discover_returns_expected_structure() {
    let handler = test_handler();
    let request = mk_request("transport.discover", None, 70);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["transports"].as_array().is_some());
    assert!(result["count"].as_u64().is_some());
}

#[tokio::test]
async fn transport_list_returns_empty_initially() {
    let handler = test_handler();
    let request = mk_request("transport.list", None, 71);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["count"], 0);
    assert!(result["transports"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn transport_route_missing_params_returns_error() {
    let handler = test_handler();
    let request = mk_request("transport.route", None, 72);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn transport_route_missing_rx_id() {
    let handler = test_handler();
    let params = serde_json::json!({"tx_id": "tx-1"});
    let request = mk_request("transport.route", Some(params), 73);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("rx_id"));
}

#[tokio::test]
async fn transport_route_missing_tx_id() {
    let handler = test_handler();
    let params = serde_json::json!({"rx_id": "rx-1"});
    let request = mk_request("transport.route", Some(params), 74);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("tx_id"));
}

#[tokio::test]
async fn transport_open_missing_params_returns_error() {
    let handler = test_handler();
    let request = mk_request("transport.open", None, 75);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn transport_open_missing_source_slot() {
    let handler = test_handler();
    let params = serde_json::json!({"target_slot": "0000:41:00.0"});
    let request = mk_request("transport.open", Some(params), 76);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("source_slot"));
}

#[tokio::test]
async fn transport_open_missing_target_slot() {
    let handler = test_handler();
    let params = serde_json::json!({"source_slot": "0000:25:00.0"});
    let request = mk_request("transport.open", Some(params), 77);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("target_slot"));
}

#[tokio::test]
async fn transport_open_no_pcie_link_found() {
    let handler = test_handler();
    let params = serde_json::json!({
        "source_slot": "0000:ff:00.0",
        "target_slot": "0000:fe:00.0"
    });
    let request = mk_request("transport.open", Some(params), 78);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("No PCIe link found"));
}

#[tokio::test]
async fn transport_stream_missing_params() {
    let handler = test_handler();
    let request = mk_request("transport.stream", None, 79);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn transport_stream_unregistered_rx() {
    let handler = test_handler();
    let params = serde_json::json!({"rx_id": "fake-rx", "tx_id": "fake-tx"});
    let request = mk_request("transport.stream", Some(params), 80);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("rx transport not registered"));
}

#[tokio::test]
async fn transport_status_no_streams_returns_empty() {
    let handler = test_handler();
    let request = mk_request("transport.status", None, 81);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["count"], 0);
    assert!(result["streams"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn transport_status_unknown_stream_id() {
    let handler = test_handler();
    let params = serde_json::json!({"stream_id": "nonexistent"});
    let request = mk_request("transport.status", Some(params), 82);
    let response = handler.handle_request(&request).await;

    let err = response.error.expect("should error");
    assert!(err.message.contains("Unknown stream"));
}

// ═══════════════════════════════════════════════════════════
// Semantic method resolution coverage
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn semantic_dispatch_resolves_known_methods() {
    let handler = test_handler();

    type CheckFn = Box<dyn Fn(&serde_json::Value)>;
    let methods_and_checks: Vec<(&str, CheckFn)> = vec![
        (
            "compute.health",
            Box::new(|r: &serde_json::Value| {
                assert!(r["healthy"].as_bool().is_some());
            }),
        ),
        (
            "compute.version",
            Box::new(|r: &serde_json::Value| {
                assert_eq!(r["protocol"], "JSON-RPC 2.0");
            }),
        ),
        (
            "compute.capabilities",
            Box::new(|r: &serde_json::Value| {
                assert!(r.is_object(), "capabilities should return an object");
            }),
        ),
    ];

    for (method, check) in methods_and_checks {
        let request = mk_request(method, None, 90);
        let response = handler.handle_request(&request).await;
        assert!(
            response.error.is_none(),
            "{method} should resolve and succeed"
        );
        check(response.result.as_ref().unwrap());
    }
}

#[tokio::test]
async fn invalid_jsonrpc_version_returns_error() {
    let handler = test_handler();
    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("1.0"),
        method: Cow::Borrowed("toadstool.health"),
        params: None,
        id: Some(serde_json::json!(99)),
    };
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("should error");
    assert!(err.message.contains("2.0"));
}

// ═══════════════════════════════════════════════════════════
// Provenance domain
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn provenance_query_returns_expected_structure() {
    let handler = test_handler();

    for method in ["provenance.query", "provenance.get", "toadstool.provenance"] {
        let request = mk_request(method, None, 95);
        let response = handler.handle_request(&request).await;
        assert!(response.error.is_none(), "{method} should succeed");
        let result = response.result.unwrap();
        assert!(
            result.get("total_flows").is_some(),
            "{method} should return provenance with total_flows"
        );
        assert!(result["springs"].as_array().is_some());
        assert!(result["flows"].as_array().is_some());
    }
}
