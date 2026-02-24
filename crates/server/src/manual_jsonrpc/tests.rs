//! Manual JSON-RPC tests (consolidated from mod and handlers)

#![allow(deprecated)]
use toadstool_common::interned_strings::primals;

use super::*;
use std::sync::Arc;

use crate::gpu_job_queue::JobQueueError;
use crate::rpc_types::{ComputeCapabilities, ExecutionMetrics, WorkloadResult, WorkloadStatus};
use crate::tarpc_server::{StandaloneExecutor, WorkloadExecutor};

fn test_server() -> ManualJsonRpcServer {
    let executor = Arc::new(StandaloneExecutor::new());
    ManualJsonRpcServer::new(executor, "test-1.0.0".to_string(), None)
}

fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: toadstool_common::constants::jsonrpc::VERSION.to_string(),
        method: method.to_string(),
        params,
        id: Some(serde_json::json!(id)),
    }
}

#[test]
fn test_jsonrpc_request_parsing() {
    let json = r#"{"jsonrpc":"2.0","method":"test","id":1}"#;
    let request: JsonRpcRequest = serde_json::from_str(json).unwrap();
    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.method, "test");
}

#[test]
fn test_success_response() {
    let server = test_server();
    let request = mk_request("test", None, 42);
    let result = server.success_response(serde_json::json!({"key": "value"}), &request);
    let obj = result.as_object().expect("object");
    assert_eq!(obj["jsonrpc"], "2.0");
    assert_eq!(obj["result"]["key"], "value");
}

#[test]
fn test_extract_job_id_valid() {
    let server = test_server();
    let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
    let request = mk_request("test", Some(serde_json::json!({"job_id": uuid_str})), 1);
    let job_id = server.extract_job_id(&request).expect("valid uuid");
    assert_eq!(job_id.to_string(), uuid_str);
}

#[tokio::test]
async fn test_method_dispatch_health() {
    let server = test_server();
    let request = mk_request("toadstool.health", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert_eq!(obj["jsonrpc"], "2.0");
    assert!(obj["result"]["healthy"].as_bool().unwrap());
    assert_eq!(obj["result"]["service"], primals::TOADSTOOL);
}

#[tokio::test]
async fn test_handle_health_response_format() {
    let server = test_server();
    let request = mk_request("compute.health", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    let result = &obj["result"];
    assert!(result["healthy"].as_bool().expect("healthy") == true);
    assert!(result["version"].as_str().is_some());
    assert!(result["error_count"].as_u64().is_some());
    assert!(result["uptime_secs"].as_u64().is_some());
}

#[tokio::test]
async fn test_method_dispatch_version() {
    let server = test_server();
    let request = mk_request("toadstool.version", None, 2);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert_eq!(obj["result"]["version"], "test-1.0.0");
}

#[tokio::test]
async fn test_handle_version_response_format() {
    let server = test_server();
    let request = mk_request("compute.version", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert_eq!(obj["result"]["version"], "test-1.0.0");
    assert_eq!(obj["result"]["protocol"], "json-rpc-2.0");
}

#[tokio::test]
async fn test_handle_discover_capabilities_structure() {
    let server = test_server();
    let request = mk_request("compute.discover_capabilities", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    let result = &obj["result"];
    assert!(result["node_capabilities"].is_array());
    assert!(result["methods"].is_array());
    assert!(result["version"].as_str().is_some());
    assert_eq!(result["primal"], primals::TOADSTOOL);
    let methods = result["methods"].as_array().expect("methods array");
    assert!(methods.iter().any(|m| m == "gate.route"));
    assert!(methods.iter().any(|m| m == "toadstool.health"));
}

#[tokio::test]
async fn test_handle_gpu_info_response_format() {
    let server = test_server();
    let request = mk_request("gpu.info", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    let result = &obj["result"];
    assert!(result["devices"].is_array());
    assert_eq!(result["driver"], "wgpu");
    assert!(result["compute_backends"].is_array());
}

#[tokio::test]
async fn test_handle_gpu_memory_response_format() {
    let server = test_server();
    let request = mk_request("gpu.memory", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    assert!(obj["result"]["devices"].is_array());
}

#[tokio::test]
async fn test_handle_query_capabilities_success() {
    let server = test_server();
    let request = mk_request("compute.capabilities", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    let result = &obj["result"];
    assert!(result["service_id"].as_str().is_some());
    assert!(result["compute_units"].is_array());
    assert!(result["supported_workload_types"].is_array());
}

struct FailingCapabilitiesExecutor;

#[async_trait::async_trait]
impl WorkloadExecutor for FailingCapabilitiesExecutor {
    async fn execute(
        &self,
        submission: crate::rpc_types::WorkloadSubmission,
    ) -> Result<WorkloadResult, String> {
        Ok(WorkloadResult {
            workload_id: submission.workload_id,
            status: WorkloadStatus::Completed,
            data: None,
            error: None,
            metrics: ExecutionMetrics {
                queued_duration_secs: 0.0,
                execution_duration_secs: 0.0,
                cpu_cores_used: 1,
                memory_used_bytes: 0,
                gpu_memory_used_bytes: None,
            },
        })
    }

    async fn query_capabilities(&self) -> Result<ComputeCapabilities, String> {
        Err("capabilities query failed".to_string())
    }

    async fn cancel(&self, _workload_id: &str) -> Result<(), String> {
        Ok(())
    }
}

#[tokio::test]
async fn test_handle_query_capabilities_executor_error() {
    let executor = Arc::new(FailingCapabilitiesExecutor);
    let server = ManualJsonRpcServer::new(executor, "test-1.0.0".to_string(), None);
    let request = mk_request("toadstool.query_capabilities", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("error"));
    assert_eq!(obj["error"]["code"], INTERNAL_ERROR);
    assert!(obj["error"]["message"]
        .as_str()
        .expect("message")
        .contains("capabilities query failed"));
}

#[tokio::test]
async fn test_method_dispatch_unknown() {
    let server = test_server();
    let request = mk_request("unknown.method", None, 99);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert_eq!(obj["error"]["code"], METHOD_NOT_FOUND);
}

#[tokio::test]
async fn test_handle_compute_submit_inference() {
    let server = test_server();
    let params = serde_json::json!({
        "inference": {
            "model": "tinyllama",
            "prompt": "Hello",
            "params": {}
        }
    });
    let request = mk_request("compute.submit", Some(params), 1);
    let response = server.handle_compute_submit(request).await;
    let obj = response.as_object().expect("object");
    assert_eq!(obj["jsonrpc"], "2.0");
    assert!(obj["result"]["job_id"].as_str().is_some());
}

#[test]
fn test_job_queue_error_response_job_not_found() {
    let server = test_server();
    let request = mk_request("test", None, 1);
    let err = JobQueueError::JobNotFound {
        id: uuid::Uuid::nil(),
    };
    let result = server.job_queue_error_response(err, &request);
    let obj = result.as_object().expect("object");
    assert_eq!(obj["error"]["code"], METHOD_NOT_FOUND);
}

// ── gate.* handlers (handlers_cluster) ───────────────────────────────────────

#[tokio::test]
async fn test_gate_update_missing_params() {
    let server = test_server();
    let request = mk_request("gate.update", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("error"));
    assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    assert!(obj["error"]["message"]
        .as_str()
        .expect("message")
        .contains("Missing params"));
}

#[tokio::test]
async fn test_gate_update_invalid_gate_info() {
    let server = test_server();
    let params = serde_json::json!({"gate_id": "g1"}); // missing required fields
    let request = mk_request("gate.update", Some(params), 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("error"));
    assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    assert!(obj["error"]["message"]
        .as_str()
        .expect("message")
        .contains("Invalid gate info"));
}

#[tokio::test]
async fn test_gate_update_success() {
    let server = test_server();
    let params = serde_json::json!({
        "gate_id": "tower",
        "gpu_model": "RTX 4070",
        "vram_total_mb": 12288,
        "vram_available_mb": 8000,
        "loaded_models": ["llama2"],
        "queue_depth": 2,
        "reachable": true
    });
    let request = mk_request("gate.update", Some(params), 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    assert_eq!(obj["result"]["updated"], true);
    assert_eq!(obj["result"]["gate_id"], "tower");
}

#[tokio::test]
async fn test_gate_remove_missing_gate_id() {
    let server = test_server();
    let request = mk_request("gate.remove", Some(serde_json::json!({})), 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("error"));
    assert_eq!(obj["error"]["code"], INVALID_PARAMS);
    assert!(obj["error"]["message"]
        .as_str()
        .expect("message")
        .contains("gate_id"));
}

#[tokio::test]
async fn test_gate_remove_success() {
    let server = test_server();
    let request = mk_request(
        "gate.remove",
        Some(serde_json::json!({"gate_id": "nonexistent"})),
        1,
    );
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    assert_eq!(obj["result"]["removed"], true);
    assert_eq!(obj["result"]["gate_id"], "nonexistent");
}

#[tokio::test]
async fn test_gate_list_empty() {
    let server = test_server();
    let request = mk_request("gate.list", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    assert!(obj["result"]["gates"].as_array().expect("array").is_empty());
}

#[tokio::test]
async fn test_gate_list_after_update() {
    let server = test_server();
    let params = serde_json::json!({
        "gate_id": "gate1",
        "gpu_model": "RTX 3090",
        "vram_total_mb": 24576,
        "vram_available_mb": 20000,
        "loaded_models": [],
        "queue_depth": 0,
        "reachable": true
    });
    let _ = server
        .handle_jsonrpc_request(mk_request("gate.update", Some(params), 0))
        .await;
    let request = mk_request("gate.list", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    let gates = obj["result"]["gates"].as_array().expect("array");
    assert_eq!(gates.len(), 1);
    assert_eq!(gates[0]["gate_id"], "gate1");
}

#[tokio::test]
async fn test_gate_route_missing_params() {
    let server = test_server();
    let request = mk_request("gate.route", None, 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("error"));
    assert_eq!(obj["error"]["code"], INVALID_PARAMS);
}

#[tokio::test]
async fn test_gate_route_success_defaults() {
    let server = test_server();
    let request = mk_request("gate.route", Some(serde_json::json!({})), 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    assert!(obj["result"]["gate_id"].as_str().is_some());
    assert!(obj["result"]["reason"].as_str().is_some());
    assert!(obj["result"]["estimated_wait_ms"].as_u64().is_some());
}

#[tokio::test]
async fn test_gate_route_with_model_and_vram() {
    let server = test_server();
    let params = serde_json::json!({
        "model": "llama3",
        "vram_required_mb": 8192
    });
    let request = mk_request("gate.route", Some(params), 1);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert!(obj.contains_key("result"));
    assert!(obj["result"]["gate_id"].as_str().is_some());
    assert_eq!(obj["result"]["reason"], "only_option");
}
