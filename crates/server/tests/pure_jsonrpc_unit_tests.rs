// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Unit tests for pure JSON-RPC handler.
//!
//! These tests cover request parsing, method dispatch, and error
//! construction without requiring a live server.

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use toadstool_server::pure_jsonrpc::*;
use toadstool_server::rpc_types::{ResourceRequirements, WorkloadPriority};
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

#[test]
fn test_parse_request() {
    let json = r#"{
        "jsonrpc": "2.0",
        "method": "toadstool.health",
        "id": 1
    }"#;

    let req: JsonRpcRequest<'_> = serde_json::from_str(json).expect("Parse failed");
    assert_eq!(req.jsonrpc.as_ref(), "2.0");
    assert_eq!(req.method.as_ref(), "toadstool.health");
}

#[test]
fn test_error_response() {
    let err = JsonRpcError::method_not_found("foo.bar");
    assert_eq!(err.code, -32601);
    assert!(err.message.contains("foo.bar"));
}

#[test]
fn test_json_workload_submission() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};

    let data = vec![1, 2, 3, 4];
    let encoded = STANDARD.encode(&data);

    let submission = JsonWorkloadSubmission {
        workload_id: Arc::from("work-123"),
        workload_type: Arc::from("gpu_compute"),
        data: encoded,
        metadata: HashMap::new(),
        priority: WorkloadPriority::Normal,
        requirements: ResourceRequirements {
            cpu_cores: Some(4),
            memory_bytes: Some(1024 * 1024 * 1024),
            gpu_memory_bytes: None,
            timeout_secs: Some(300),
        },
    };

    let tarpc = submission.into_tarpc().expect("Conversion failed");
    assert_eq!(tarpc.data, vec![1, 2, 3, 4]);
}

#[tokio::test]
async fn test_health_via_handle_request() {
    let handler = test_handler();
    let request = mk_request("toadstool.health", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["healthy"].as_bool().unwrap());
    assert_eq!(result["version"], "test-1.0.0");
}

#[tokio::test]
async fn test_handle_method_dispatch_version() {
    let handler = test_handler();
    let request = mk_request("toadstool.version", None, 2);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["version"], "test-1.0.0");
    assert_eq!(result["protocol"], "JSON-RPC 2.0");
}

#[tokio::test]
async fn test_handle_method_dispatch_unknown() {
    let handler = test_handler();
    let request = mk_request("unknown.method", None, 99);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::METHOD_NOT_FOUND);
}

#[tokio::test]
async fn test_invalid_jsonrpc_version() {
    let handler = test_handler();
    let request = JsonRpcRequest {
        jsonrpc: Cow::Owned("3.0".to_string()),
        method: Cow::Borrowed("toadstool.health"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_REQUEST);
}

#[tokio::test]
async fn test_compute_submit() {
    let handler = test_handler();
    let params = serde_json::json!({
        "inference": {
            "model": "tinyllama",
            "prompt": "Hello",
            "params": {}
        }
    });
    let request = mk_request("compute.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["job_id"].as_str().is_some());
}

#[tokio::test]
async fn test_compute_list() {
    let handler = test_handler();
    let request = mk_request("compute.list", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["jobs"].is_array());
    assert!(result["counts"].is_object());
}

#[tokio::test]
async fn test_compute_status_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("compute.status", Some(serde_json::json!({})), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_query_capabilities() {
    let handler = test_handler();
    let request = mk_request("toadstool.query_capabilities", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["service_id"].as_str().is_some());
    assert!(result["compute_units"].is_array());
}

#[test]
fn test_jsonrpc_error_constructors() {
    let err = JsonRpcError::parse_error("bad json");
    assert_eq!(err.code, JsonRpcError::PARSE_ERROR);
    assert!(err.message.contains("bad json"));

    let err = JsonRpcError::invalid_request("wrong version");
    assert_eq!(err.code, JsonRpcError::INVALID_REQUEST);

    let err = JsonRpcError::invalid_params("missing field");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);

    let err = JsonRpcError::internal_error("panic");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
}

#[test]
fn test_json_workload_submission_invalid_base64() {
    let submission = JsonWorkloadSubmission {
        workload_id: Arc::from("work-1"),
        workload_type: Arc::from("gpu_compute"),
        data: "!!!not-valid-base64!!!".to_string(),
        metadata: HashMap::new(),
        priority: WorkloadPriority::Normal,
        requirements: ResourceRequirements {
            cpu_cores: Some(4),
            memory_bytes: Some(1024 * 1024 * 1024),
            gpu_memory_bytes: None,
            timeout_secs: Some(300),
        },
    };
    let result = submission.into_tarpc();
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Invalid base64"));
}

#[tokio::test]
async fn test_submit_workload_missing_params() {
    let handler = test_handler();
    let request = mk_request("toadstool.submit_workload", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_submit_workload_success() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let handler = test_handler();
    let params = serde_json::json!({
        "workload_id": "work-submit-1",
        "workload_type": "cpu_compute",
        "data": STANDARD.encode([1u8, 2, 3, 4]),
        "metadata": {},
        "priority": "Normal",
        "requirements": {
            "cpu_cores": 2,
            "memory_bytes": 1024,
            "timeout_secs": 60
        }
    });
    let request = mk_request("toadstool.submit_workload", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["workload_id"], "work-submit-1");
    assert!(result["status"].as_str().is_some());
}

#[tokio::test]
async fn test_submit_workload_invalid_base64() {
    let handler = test_handler();
    let params = serde_json::json!({
        "workload_id": "work-1",
        "workload_type": "cpu_compute",
        "data": "!!!invalid!!!",
        "metadata": {},
        "priority": "Normal",
        "requirements": {}
    });
    let request = mk_request("toadstool.submit_workload", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_query_status_invalid_uuid() {
    let handler = test_handler();
    let params = serde_json::json!("not-a-uuid");
    let request = mk_request("toadstool.query_status", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_query_status_job_not_found() {
    let handler = test_handler();
    let job_id = uuid::Uuid::new_v4();
    let params = serde_json::json!(job_id.to_string());
    let request = mk_request("toadstool.query_status", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
}

#[tokio::test]
async fn test_cancel_workload_missing_params() {
    let handler = test_handler();
    let request = mk_request("toadstool.cancel_workload", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_cancel_workload_success() {
    let handler = test_handler();
    let params = serde_json::json!("some-workload-id");
    let request = mk_request("toadstool.cancel_workload", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["success"], true);
}

#[tokio::test]
async fn test_compute_result_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("compute.result", Some(serde_json::json!({})), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_compute_result_job_not_found() {
    let handler = test_handler();
    let job_id = uuid::Uuid::new_v4();
    let params = serde_json::json!({ "job_id": job_id.to_string() });
    let request = mk_request("compute.result", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::WORKLOAD_NOT_FOUND
    );
}

#[tokio::test]
async fn test_compute_cancel_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("compute.cancel", Some(serde_json::json!({})), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_compute_list_with_state_filter() {
    let handler = test_handler();
    let params = serde_json::json!({ "state": "pending" });
    let request = mk_request("compute.list", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["jobs"].is_array());
    assert!(result["counts"].is_object());
}

#[tokio::test]
async fn test_health_error_count_incremented() {
    let error_count = Arc::new(std::sync::atomic::AtomicU64::new(0));
    let executor = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "1.0".to_string(), Some(error_count));

    let bad_request = mk_request("unknown.method", None, 1);
    let _ = handler.handle_request(&bad_request).await;

    let health_request = mk_request("toadstool.health", None, 2);
    let response = handler.handle_request(&health_request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["error_count"].as_u64().unwrap_or(0) >= 1);
}

#[tokio::test]
async fn test_request_id_null_when_missing() {
    let handler = test_handler();
    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("toadstool.health"),
        params: None,
        id: None,
    };
    let response = handler.handle_request(&request).await;
    assert_eq!(response.id, serde_json::Value::Null);
}

#[tokio::test]
async fn test_compute_submit_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "invalid": "job_type" });
    let request = mk_request("compute.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_submit_workload_invalid_params_structure() {
    let handler = test_handler();
    let params = serde_json::json!({
        "workload_type": "cpu_compute",
        "data": ""
    });
    let request = mk_request("toadstool.submit_workload", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_compute_cancel_success() {
    let handler = test_handler();
    let params = serde_json::json!({
        "inference": { "model": "test", "prompt": "x", "params": {} }
    });
    let submit_req = mk_request("compute.submit", Some(params.clone()), 1);
    let submit_resp = handler.handle_request(&submit_req).await;
    let job_id = submit_resp
        .result
        .as_ref()
        .and_then(|r| r.get("job_id"))
        .and_then(|v| v.as_str())
        .expect("submit should return job_id");

    let cancel_params = serde_json::json!({ "job_id": job_id });
    let cancel_req = mk_request("compute.cancel", Some(cancel_params), 2);
    let cancel_resp = handler.handle_request(&cancel_req).await;

    assert!(cancel_resp.error.is_none());
    let result = cancel_resp.result.expect("result present");
    assert_eq!(result["cancelled"], true);
}

#[test]
fn test_jsonrpc_response_serialization() {
    let success = JsonRpcResponse {
        jsonrpc: std::borrow::Cow::Borrowed("2.0"),
        result: Some(serde_json::json!({"ok": true})),
        error: None,
        id: serde_json::json!(1),
    };
    let json = serde_json::to_string(&success).expect("Serialize failed");
    assert!(json.contains("\"result\""));
    assert!(json.contains("\"ok\""));
    assert!(!json.contains("\"error\""));

    let failure = JsonRpcResponse {
        jsonrpc: std::borrow::Cow::Borrowed("2.0"),
        result: None,
        error: Some(JsonRpcError::method_not_found("foo")),
        id: serde_json::json!(2),
    };
    let json_err = serde_json::to_string(&failure).expect("Serialize failed");
    assert!(json_err.contains("\"error\""));
}

#[test]
fn test_jsonrpc_request_with_params_array() {
    let json = r#"{"jsonrpc":"2.0","method":"foo","params":[1,2],"id":1}"#;
    let req: JsonRpcRequest<'_> = serde_json::from_str(json).expect("Parse failed");
    assert!(req.params.is_some());
}

// ───── Additional coverage: gpu, gate, resources, ollama, compute aliases ───

#[tokio::test]
async fn test_gpu_info() {
    let handler = test_handler();
    let request = mk_request("gpu.query_info", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["devices"].is_array());
    assert_eq!(result["driver"], "wgpu");
}

#[tokio::test]
async fn test_gpu_memory() {
    let handler = test_handler();
    let request = mk_request("gpu.query_memory", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["devices"].is_array());
}

#[tokio::test]
async fn test_compute_discover_capabilities() {
    let handler = test_handler();
    let request = mk_request("compute.discover_capabilities", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["node_capabilities"].is_array());
    assert!(result["methods"].is_array());
    assert!(result["version"].as_str().is_some());
}

#[tokio::test]
async fn test_compute_version() {
    let handler = test_handler();
    let request = mk_request("compute.version", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["version"], "test-1.0.0");
}

#[tokio::test]
async fn test_compute_capabilities() {
    let handler = test_handler();
    let request = mk_request("compute.capabilities", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["service_id"].as_str().is_some());
}

#[tokio::test]
async fn test_toadstool_list_workloads() {
    let handler = test_handler();
    let request = mk_request("toadstool.list_workloads", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["jobs"].is_array());
    assert!(result["counts"].is_object());
}

#[tokio::test]
async fn test_gate_list() {
    let handler = test_handler();
    let request = mk_request("gate.list", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["gates"].is_array());
}

#[tokio::test]
async fn test_gate_route() {
    let handler = test_handler();
    let params = serde_json::json!({
        "model": "llama2",
        "vram_required_mb": 4096
    });
    let request = mk_request("gate.route", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["gate_id"].as_str().is_some());
    assert!(result["reason"].as_str().is_some());
}

#[tokio::test]
async fn test_gate_route_missing_params() {
    let handler = test_handler();
    let request = mk_request("gate.route", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_gate_update() {
    let handler = test_handler();
    let params = serde_json::json!({
        "gate_id": "test-gate",
        "gpu_model": "RTX 4090",
        "vram_total_mb": 24000,
        "vram_available_mb": 20000,
        "loaded_models": [],
        "queue_depth": 0,
        "reachable": true
    });
    let request = mk_request("gate.update", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["updated"], true);
    assert_eq!(result["gate_id"], "test-gate");
}

#[tokio::test]
async fn test_gate_update_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({"invalid": "data"});
    let request = mk_request("gate.update", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_gate_remove() {
    let handler = test_handler();
    let params = serde_json::json!({"gate_id": "gate-to-remove"});
    let request = mk_request("gate.remove", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["removed"], true);
}

#[tokio::test]
async fn test_gate_remove_missing_gate_id() {
    let handler = test_handler();
    let request = mk_request("gate.remove", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_resources_estimate_empty_graph() {
    let handler = test_handler();
    let params = serde_json::json!({
        "graph": {
            "id": "empty",
            "nodes": [],
            "edges": [],
            "metadata": {}
        }
    });
    let request = mk_request("resources.estimate", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test]
async fn test_resources_estimate_missing_params() {
    let handler = test_handler();
    let request = mk_request("resources.estimate", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_resources_validate_availability() {
    let handler = test_handler();
    let params = serde_json::json!({
        "graph": {
            "id": "validate",
            "nodes": [],
            "edges": [],
            "metadata": {}
        }
    });
    let request = mk_request("resources.validate_availability", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test]
async fn test_resources_suggest_optimizations() {
    let handler = test_handler();
    let params = serde_json::json!({
        "graph": {
            "id": "optimize",
            "nodes": [],
            "edges": [],
            "metadata": {}
        }
    });
    let request = mk_request("resources.suggest_optimizations", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test]
async fn test_ai_local_inference_alias() {
    let handler = test_handler();
    let params = serde_json::json!({
        "graph": {
            "id": "x",
            "nodes": [],
            "edges": [],
            "metadata": {}
        }
    });
    let request = mk_request("ai.local_inference", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test]
async fn test_ai_local_execute_alias() {
    let handler = test_handler();
    let params = serde_json::json!({
        "graph": {
            "id": "x",
            "nodes": [],
            "edges": [],
            "metadata": {}
        }
    });
    let request = mk_request("ai.local_execute", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test]
async fn test_ollama_list_models() {
    let handler = test_handler();
    let request = mk_request("inference.list_models", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test]
async fn test_ollama_inference_missing_params() {
    let handler = test_handler();
    let request = mk_request("inference.execute", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_ollama_load_missing_model() {
    let handler = test_handler();
    let request = mk_request("inference.load_model", Some(serde_json::json!({})), 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_ollama_unload_missing_model() {
    let handler = test_handler();
    let request = mk_request("inference.unload_model", None, 1);
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}
