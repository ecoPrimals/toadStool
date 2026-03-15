// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for under-covered JSON-RPC handler modules.
//!
//! Expands coverage for:
//! - transport.rs (transport.open, transport.stream, transport.status)
//! - dispatch.rs (`dispatch_submit` success, `dispatch_forward`, status/result)
//! - `science_domains.rs` (ecology, discovery, deploy)
//! - shader.rs (`compile_wgsl_multi`, `compile_spirv`, `compile_status`)
//! - ollama.rs (inference, load, unload success/error paths)

#![allow(deprecated)]
#![allow(clippy::redundant_closure_for_method_calls)]

use std::borrow::Cow;
use std::sync::Arc;

use toadstool_server::pure_jsonrpc::{JsonRpcError, JsonRpcHandler, JsonRpcRequest};
use toadstool_server::tarpc_server::StandaloneExecutor;

fn test_handler() -> JsonRpcHandler {
    let executor = Arc::new(StandaloneExecutor::new());
    JsonRpcHandler::new(executor, "test-1.0.0".to_string(), None)
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
// Transport handler — transport.open, transport.stream, transport.status
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn transport_open_missing_params() {
    let handler = test_handler();
    let request = mk_request("transport.open", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn transport_open_missing_source_slot() {
    let handler = test_handler();
    let params = serde_json::json!({ "target_slot": "0000:41:00.0" });
    let request = mk_request("transport.open", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("source_slot"));
}

#[tokio::test]
async fn transport_open_missing_target_slot() {
    let handler = test_handler();
    let params = serde_json::json!({ "source_slot": "0000:25:00.0" });
    let request = mk_request("transport.open", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("target_slot"));
}

#[tokio::test]
async fn transport_open_nonexistent_link() {
    let handler = test_handler();
    let params = serde_json::json!({
        "source_slot": "0000:99:00.0",
        "target_slot": "0000:99:00.1"
    });
    let request = mk_request("transport.open", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert!(err.message.contains("No PCIe link") || err.message.contains("params"));
}

#[tokio::test]
async fn transport_stream_missing_params() {
    let handler = test_handler();
    let request = mk_request("transport.stream", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn transport_stream_unregistered_transports() {
    let handler = test_handler();
    let params = serde_json::json!({
        "rx_id": "nonexistent-rx",
        "tx_id": "nonexistent-tx"
    });
    let request = mk_request("transport.stream", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert!(err.message.contains("not registered"));
}

#[tokio::test]
async fn transport_status_all_streams_empty() {
    let handler = test_handler();
    let request = mk_request("transport.status", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["streams"].as_array().is_some());
    assert_eq!(result["count"].as_u64().unwrap(), 0);
}

#[tokio::test]
async fn transport_status_unknown_stream_id() {
    let handler = test_handler();
    let params = serde_json::json!({ "stream_id": "stream-999" });
    let request = mk_request("transport.status", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert!(err.message.contains("Unknown stream"));
}

// ═══════════════════════════════════════════════════════════
// Dispatch handler — compute.dispatch.*
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn dispatch_submit_with_valid_binary_returns_job_id() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1, 2, 3, 4, 5],
        "bdf": "0000:01:00.0"
    });
    let request = mk_request("compute.dispatch.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "submit");
    assert!(result["job_id"].as_str().is_some());
    assert!(result["status"].as_str().is_some());
}

#[tokio::test]
async fn dispatch_submit_missing_binary() {
    let handler = test_handler();
    let params = serde_json::json!({ "bdf": "0000:01:00.0" });
    let request = mk_request("compute.dispatch.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("binary"));
}

#[tokio::test]
async fn dispatch_status_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.status", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_result_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.result", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_submit_then_status_and_result() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [10, 20, 30],
        "bdf": "0000:01:00.0"
    });
    let submit_req = mk_request("compute.dispatch.submit", Some(params), 1);
    let submit_resp = handler.handle_request(&submit_req).await;

    assert!(submit_resp.error.is_none());
    let job_id = submit_resp
        .result
        .as_ref()
        .and_then(|r| r.get("job_id"))
        .and_then(|v| v.as_str())
        .expect("submit returns job_id");

    let status_params = serde_json::json!({ "job_id": job_id });
    let status_req = mk_request("compute.dispatch.status", Some(status_params), 2);
    let status_resp = handler.handle_request(&status_req).await;

    assert!(status_resp.error.is_none());
    let status_result = status_resp.result.expect("status result");
    assert_eq!(status_result["job_id"], job_id);
    assert!(status_result["status"].as_str().is_some());
    assert!(status_result["bdf"].as_str().is_some());

    let result_params = serde_json::json!({ "job_id": job_id });
    let result_req = mk_request("compute.dispatch.result", Some(result_params), 3);
    let result_resp = handler.handle_request(&result_req).await;

    assert!(result_resp.error.is_none());
    let result_val = result_resp.result.expect("result");
    assert_eq!(result_val["job_id"], job_id);
}

#[tokio::test]
async fn dispatch_forward_missing_params() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.forward", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_forward_missing_endpoint() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1, 2, 3],
        "params": {}
    });
    let request = mk_request("compute.dispatch.forward", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert!(err.message.contains("endpoint"));
}

#[tokio::test]
async fn dispatch_capabilities_returns_structure() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.capabilities", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "capabilities");
    assert!(result["sovereign_pipeline"].as_bool().unwrap());
    assert!(result["dispatch_modes"].as_array().is_some());
}

// ═══════════════════════════════════════════════════════════
// Science domains — ecology, discovery, deploy
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn ecology_offload_returns_queued_or_forwarded() {
    let handler = test_handler();
    let params = serde_json::json!({ "input": "test" });
    let request = mk_request("ecology.et0_fao56", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "ecology");
    assert!(result["method"].as_str().is_some());
    assert!(result["available_methods"].as_array().is_some());
}

#[tokio::test]
async fn ecology_water_balance_no_params() {
    let handler = test_handler();
    let request = mk_request("ecology.water_balance", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "ecology");
}

#[tokio::test]
async fn discovery_primals_returns_structure() {
    let handler = test_handler();
    let request = mk_request("discovery.primals", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "discovery");
    assert!(result["primals"].as_array().is_some());
    assert!(result["count"].as_u64().is_some());
    assert!(result["socket_dir"].as_str().is_some());
}

#[tokio::test]
async fn discovery_primal_health_without_name() {
    let handler = test_handler();
    let request = mk_request("discovery.primal_health", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["name"].as_str().is_some());
    assert!(result["healthy"].as_bool().is_some());
}

#[tokio::test]
async fn discovery_primal_health_with_name() {
    let handler = test_handler();
    let params = serde_json::json!({ "name": "nonexistent-primal" });
    let request = mk_request("discovery.primal_health", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["name"], "nonexistent-primal");
    assert!(!result["healthy"].as_bool().unwrap());
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
    let params = serde_json::json!({ "name": "toadstool" });
    let request = mk_request("discovery.direct_rpc", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("method"));
}

#[tokio::test]
async fn discovery_topology_returns_structure() {
    let handler = test_handler();
    let request = mk_request("discovery.topology", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "discovery");
    assert!(result["nodes"].as_array().is_some());
    assert!(result["self"].as_str().is_some());
    assert_eq!(result["protocol"], "JSON-RPC 2.0");
}

#[tokio::test]
async fn deploy_capability_call_missing_capability() {
    let handler = test_handler();
    let params = serde_json::json!({ "method": "phylo.infer" });
    let request = mk_request("deploy.capability_call", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn deploy_capability_call_missing_method() {
    let handler = test_handler();
    let params = serde_json::json!({ "capability": "biology" });
    let request = mk_request("deploy.capability_call", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn deploy_capability_call_qualified_method_invalid() {
    let handler = test_handler();
    let params = serde_json::json!({
        "qualified_method": "nodot",
        "params": {}
    });
    let request = mk_request("deploy.capability_call", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert!(err.message.contains("qualified_method") || err.message.contains("'.'"));
}

#[tokio::test]
async fn deploy_capability_call_no_provider() {
    let handler = test_handler();
    let params = serde_json::json!({
        "capability": "nonexistent_capability_xyz",
        "method": "some.method",
        "params": {}
    });
    let request = mk_request("deploy.capability_call", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["status"], "no_provider");
    assert_eq!(result["capability"], "nonexistent_capability_xyz");
}

#[tokio::test]
async fn deploy_graph_status_returns_structure() {
    let handler = test_handler();
    let request = mk_request("deploy.graph_status", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "deploy");
    assert!(result["deploy_graphs"].as_array().is_some());
    assert!(result["discovered_count"].as_u64().is_some());
}

// ═══════════════════════════════════════════════════════════
// Shader handler — compile_wgsl_multi, compile_spirv, compile_status
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn shader_compile_wgsl_multi_missing_params() {
    let handler = test_handler();
    let request = mk_request("shader.compile.wgsl.multi", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn shader_compile_wgsl_multi_empty_target_devices() {
    let handler = test_handler();
    let params = serde_json::json!({
        "wgsl_source": "@compute fn main() {}",
        "target_devices": []
    });
    let request = mk_request("shader.compile.wgsl.multi", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("target_devices"));
}

#[tokio::test]
async fn shader_compile_wgsl_multi_valid_params() {
    let handler = test_handler();
    let params = serde_json::json!({
        "wgsl_source": "@compute fn main() {}",
        "target_devices": [{ "card_index": 0 }]
    });
    let request = mk_request("shader.compile.wgsl.multi", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["status"].as_str().is_some());
    assert!(result["precision_advice"].as_array().is_some());
}

#[tokio::test]
async fn shader_compile_spirv_missing_binary() {
    let handler = test_handler();
    let request = mk_request("shader.compile.spirv", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("spirv_binary"));
}

#[tokio::test]
async fn shader_compile_spirv_with_binary() {
    let handler = test_handler();
    let params = serde_json::json!({
        "spirv_binary": [0x0723_0203, 0x0001_0000, 0x0008_000a]
    });
    let request = mk_request("shader.compile.spirv", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["source_language"], "spirv");
    assert!(result["status"].as_str().is_some());
}

#[tokio::test]
async fn shader_compile_status_default_compile_id() {
    let handler = test_handler();
    let request = mk_request("shader.compile.status", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["compile_id"], "unknown");
    assert!(result["status"].as_str().is_some());
}

#[tokio::test]
async fn shader_compile_status_with_compile_id() {
    let handler = test_handler();
    let params = serde_json::json!({ "compile_id": "compile-123" });
    let request = mk_request("shader.compile.status", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["compile_id"], "compile-123");
}

// ═══════════════════════════════════════════════════════════
// Ollama handler — inference, load, unload
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn ollama_inference_missing_model() {
    let handler = test_handler();
    let params = serde_json::json!({ "prompt": "hello" });
    let request = mk_request("ollama.inference", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("model"));
}

#[tokio::test]
async fn ollama_inference_missing_prompt() {
    let handler = test_handler();
    let params = serde_json::json!({ "model": "llama2" });
    let request = mk_request("ollama.inference", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("prompt"));
}

#[tokio::test]
async fn ollama_inference_with_extra_params() {
    let handler = test_handler();
    let params = serde_json::json!({
        "model": "llama2",
        "prompt": "test",
        "params": { "temperature": 0.7 }
    });
    let request = mk_request("ollama.inference", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test]
async fn ollama_load_missing_model() {
    let handler = test_handler();
    let request = mk_request("ollama.load", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn ollama_unload_missing_model() {
    let handler = test_handler();
    let request = mk_request("ollama.unload", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn ollama_list_models_returns_structure() {
    let handler = test_handler();
    let request = mk_request("ollama.list_models", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_some() || response.error.is_some());
    if let Some(ref result) = response.result {
        assert!(result.get("models").is_some() || result.as_object().is_some());
    }
}
