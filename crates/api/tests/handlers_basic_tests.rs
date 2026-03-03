// SPDX-License-Identifier: AGPL-3.0-or-later
//! JSON-RPC 2.0 integration tests for the API layer.
//!
//! All API functionality is served via `/jsonrpc`. These tests exercise
//! each JSON-RPC method through the HTTP transport.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use toadstool_api::ApiState;
use tokio::sync::{broadcast, RwLock};
use tower::ServiceExt;

fn create_test_state() -> ApiState {
    let (tx, _rx) = broadcast::channel(100);
    ApiState {
        executions: Arc::new(RwLock::new(HashMap::new())),
        metrics: Arc::new(RwLock::new(toadstool_api::ApiMetrics::default())),
        event_broadcaster: tx,
        capability_provider: None,
    }
}

#[allow(clippy::unwrap_used)]
fn jsonrpc_request(method: &str, params: Value) -> Request<Body> {
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": method,
        "params": params,
    });
    Request::builder()
        .method("POST")
        .uri("/jsonrpc")
        .header("content-type", "application/json")
        .body(Body::from(serde_json::to_string(&body).unwrap()))
        .unwrap()
}

#[allow(clippy::unwrap_used)]
async fn call_jsonrpc(method: &str, params: Value) -> (StatusCode, Value) {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);
    let response = app.oneshot(jsonrpc_request(method, params)).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
        .await
        .unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, body)
}

#[tokio::test]
async fn test_root_handler() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);
    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_jsonrpc_health() {
    let (status, body) = call_jsonrpc("api.health", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["status"].as_str().is_some());
}

#[tokio::test]
async fn test_jsonrpc_execution_submit() {
    let (status, body) = call_jsonrpc(
        "api.execution.submit",
        json!({"workload": "test", "runtime": "native"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["execution_id"].as_str().is_some());
}

#[tokio::test]
async fn test_jsonrpc_execution_submit_missing_workload() {
    let (status, body) = call_jsonrpc("api.execution.submit", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_execution_list() {
    let (status, body) = call_jsonrpc("api.execution.list", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["executions"].is_array());
}

#[tokio::test]
async fn test_jsonrpc_execution_status_missing_id() {
    let (status, body) = call_jsonrpc("api.execution.status", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_execution_status_invalid_id() {
    let (status, body) = call_jsonrpc(
        "api.execution.status",
        json!({"execution_id": "not-a-uuid"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_execution_cancel_not_found() {
    let (status, body) = call_jsonrpc(
        "api.execution.cancel",
        json!({"execution_id": "00000000-0000-0000-0000-000000000001"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_execution_logs_not_found() {
    let (status, body) = call_jsonrpc(
        "api.execution.logs",
        json!({"execution_id": "00000000-0000-0000-0000-000000000001"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_execution_metrics_not_found() {
    let (status, body) = call_jsonrpc(
        "api.execution.metrics",
        json!({"execution_id": "00000000-0000-0000-0000-000000000001"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_api_metrics() {
    let (status, body) = call_jsonrpc("api.metrics", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["total_requests"].is_number());
}

#[tokio::test]
async fn test_jsonrpc_cluster_status() {
    let (status, body) = call_jsonrpc("api.cluster.status", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["result"]["status"].as_str().is_some());
}

#[tokio::test]
async fn test_jsonrpc_workload_execute_no_provider() {
    let (status, body) = call_jsonrpc(
        "api.workload.execute",
        json!({"workload": "test", "runtime": "native"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    // Falls back to execution submit when deserialization as WorkloadRequest fails
    assert!(body["result"].is_object() || body["error"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_unknown_method() {
    let (status, body) = call_jsonrpc("nonexistent.method", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["error"].is_object());
}

#[tokio::test]
async fn test_jsonrpc_submit_then_status() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let submit_body = json!({
        "jsonrpc": "2.0", "id": 1,
        "method": "api.execution.submit",
        "params": {"workload": "test", "runtime": "native"},
    });
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jsonrpc")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&submit_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
        .await
        .unwrap();
    let submit_result: Value = serde_json::from_slice(&bytes).unwrap();
    let exec_id = submit_result["result"]["execution_id"]
        .as_str()
        .expect("should have execution_id");

    let status_body = json!({
        "jsonrpc": "2.0", "id": 2,
        "method": "api.execution.status",
        "params": {"execution_id": exec_id},
    });
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/jsonrpc")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&status_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let bytes = axum::body::to_bytes(response.into_body(), 1_048_576)
        .await
        .unwrap();
    let status_result: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        status_result["result"]["execution_id"].as_str().unwrap(),
        exec_id
    );
}
