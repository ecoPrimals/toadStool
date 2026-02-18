//! Manual JSON-RPC tests (consolidated from mod and handlers)

#![allow(deprecated)]
use toadstool_common::interned_strings::primals;

use super::*;
use std::sync::Arc;

use crate::gpu_job_queue::JobQueueError;
use crate::tarpc_server::StandaloneExecutor;

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
async fn test_method_dispatch_version() {
    let server = test_server();
    let request = mk_request("toadstool.version", None, 2);
    let response = server.handle_jsonrpc_request(request).await;
    let obj = response.as_object().expect("object");
    assert_eq!(obj["result"]["version"], "test-1.0.0");
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
