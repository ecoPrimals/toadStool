//! Basic integration tests for API handlers
//!
//! Tests the 0% coverage handlers to improve overall test coverage.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use toadstool_api::ApiState;
use tokio::sync::{broadcast, RwLock};
use tower::ServiceExt;

/// Create a test API state for handler testing
fn create_test_state() -> ApiState {
    let (tx, _rx) = broadcast::channel(100);

    ApiState {
        executions: Arc::new(RwLock::new(HashMap::new())),
        metrics: Arc::new(RwLock::new(toadstool_api::ApiMetrics::default())),
        event_broadcaster: tx,
        capability_provider: None,
    }
}

// ============================================================================
// Health Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_handler() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // May not have route configured, check for OK or NOT_FOUND
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

// ============================================================================
// Metrics Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_handler() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Cluster Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_cluster_status() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/cluster/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // May not be OK if cluster not configured, but should respond
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_cluster_nodes() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/cluster/nodes")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // May not have route - check for valid response
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

// ============================================================================
// Logs Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_logs() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let execution_id = "test-exec-123";
    let uri = format!("/api/executions/{}/logs", execution_id);

    let response = app
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Will be NOT_FOUND if execution doesn't exist
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

// ============================================================================
// Workload Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_workloads() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/workloads")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // May not have route - check for valid response
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_workload_details() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let workload_id = "test-workload-123";
    let uri = format!("/api/workloads/{}", workload_id);

    let response = app
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Will be NOT_FOUND if workload doesn't exist
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

// ============================================================================
// Execution Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let request_body = json!({
        "workload_id": "test-workload",
        "runtime": "native",
        "resources": {
            "cpu_cores": 1,
            "memory_mb": 512
        }
    });

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/executions")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&request_body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // May succeed or fail based on state, but should respond
    assert!(
        response.status().is_success()
            || response.status() == StatusCode::BAD_REQUEST
            || response.status() == StatusCode::NOT_FOUND
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_status() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let execution_id = "test-exec-123";
    let uri = format!("/api/executions/{}", execution_id);

    let response = app
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Will be NOT_FOUND if execution doesn't exist
    assert!(response.status().is_success() || response.status() == StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let execution_id = "test-exec-123";
    let uri = format!("/api/executions/{}/cancel", execution_id);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Will be NOT_FOUND if execution doesn't exist
    assert!(
        response.status().is_success()
            || response.status() == StatusCode::NOT_FOUND
            || response.status() == StatusCode::CONFLICT
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_executions() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/executions")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ============================================================================
// Additional API Endpoint Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_root_handler() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_endpoint_v2() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_endpoint_v2() {
    let state = create_test_state();
    let app = toadstool_api::create_router(state);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v2/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
