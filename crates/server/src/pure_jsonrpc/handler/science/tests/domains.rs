// SPDX-License-Identifier: AGPL-3.0-or-later

use super::common::{mk_request, test_handler};
use crate::pure_jsonrpc::types::JsonRpcError;

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
    assert!(result.get("discovered_count").is_some());
    assert!(result.get("socket_dir").is_some());
    assert_eq!(result["domain"], "deploy");
}
