// SPDX-License-Identifier: AGPL-3.0-or-later
//! Targeted tests to expand server crate coverage toward 90%.
//!
//! Focus on error paths, edge cases, and state transitions.

#![allow(deprecated)]

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use toadstool::ExecutionStatus;
use toadstool::RuntimeType;
use toadstool_server::cross_gate::{GateGpuInfo, JobRouter, RoutingReason};
use toadstool_server::pure_jsonrpc::{JsonRpcError, JsonRpcHandler, JsonRpcRequest};
use toadstool_server::resource_estimator::EstimationError;
use toadstool_server::resource_validator::ValidationError;
use toadstool_server::state::ServerEvent;
use toadstool_server::tarpc_server::StandaloneExecutor;
use uuid::Uuid;

// ───── JSON-RPC invalid version and request ID ─────────────────────────────

#[tokio::test]
async fn test_invalid_jsonrpc_version_increments_error_count() {
    let error_count = Arc::new(AtomicU64::new(0));
    let handler = JsonRpcHandler::new(
        Arc::new(StandaloneExecutor::new()),
        "1.0.0".to_string(),
        Some(Arc::clone(&error_count)),
    );

    let request = JsonRpcRequest {
        jsonrpc: Cow::Owned("1.0".to_string()),
        method: Cow::Borrowed("toadstool.health"),
        params: None,
        id: Some(serde_json::json!(99)),
    };
    let _ = handler.handle_request(&request).await;

    assert_eq!(error_count.load(Ordering::Relaxed), 1);
}

#[tokio::test]
async fn test_request_with_null_id_preserves_null_in_response() {
    let handler = JsonRpcHandler::new(
        Arc::new(StandaloneExecutor::new()),
        "1.0.0".to_string(),
        None,
    );

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("unknown.method"),
        params: None,
        id: None,
    };
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_some());
    assert!(response.id == serde_json::Value::Null);
}

// ───── EstimationError display ──────────────────────────────────────────────

#[test]
fn test_estimation_error_display_cyclic() {
    let err = EstimationError::CyclicGraph;
    let msg = err.to_string();
    assert!(msg.contains("cycle"));
    assert!(msg.to_lowercase().contains("dag"));
}

#[test]
fn test_estimation_error_display_node_failed() {
    let err =
        EstimationError::NodeEstimationFailed("node_x".to_string(), "missing reqs".to_string());
    let msg = err.to_string();
    assert!(msg.contains("node_x"));
    assert!(msg.contains("missing reqs"));
}

#[test]
fn test_estimation_error_from_graph_validation() {
    use toadstool_server::graph_types::GraphValidationError;

    let validation_err = GraphValidationError::EmptyGraph;
    let est_err: EstimationError = validation_err.into();
    let msg = est_err.to_string();
    assert!(msg.to_lowercase().contains("empty") || msg.to_lowercase().contains("invalid"));
}

// ───── ValidationError display ────────────────────────────────────────────────

#[test]
fn test_validation_error_display_system_query_failed() {
    let err = ValidationError::SystemQueryFailed("disk io error".to_string());
    let msg = err.to_string();
    assert!(msg.contains("disk io error"));
}

#[test]
fn test_validation_error_display_invalid_configuration() {
    let err = ValidationError::InvalidConfiguration("bad path".to_string());
    let msg = err.to_string();
    assert!(msg.contains("bad path"));
}

// ───── ServerEvent ErrorOccurred with execution_id ────────────────────────────

#[test]
fn test_server_event_error_occurred_with_execution_id() {
    let exec_id = Uuid::new_v4();
    let event = ServerEvent::ErrorOccurred {
        error_type: "Execution".to_string(),
        message: "timeout".to_string(),
        execution_id: Some(exec_id),
        timestamp: std::time::SystemTime::now(),
    };
    let json = event.to_json();
    assert!(json.contains("error_occurred"));
    assert!(json.contains(&exec_id.to_string()));
}

// ───── Cross-gate routing edge cases ────────────────────────────────────────

#[test]
fn test_route_reachable_empty_returns_only_option() {
    let router = JobRouter::new("local");
    let decision = router.route("any_model", 1000);
    assert_eq!(decision.gate_id.as_ref(), "local");
    assert!(matches!(decision.reason, RoutingReason::OnlyOption));
}

#[test]
fn test_route_unreachable_gates_excluded() {
    let mut router = JobRouter::new("local");

    let gate = GateGpuInfo {
        gate_id: std::sync::Arc::from("remote"),
        gpu_model: "RTX 4090".to_string(),
        vram_total_mb: 24000,
        vram_available_mb: 20000,
        loaded_models: vec!["model".to_string()],
        queue_depth: 0,
        reachable: false,
        endpoint: None,
    };
    router.update_gate(gate);

    let decision = router.route("model", 1000);
    assert_eq!(decision.gate_id.as_ref(), "local");
}

#[test]
fn test_route_local_fallback_when_no_vram_candidates() {
    let mut router = JobRouter::new("local");

    let gate = GateGpuInfo {
        gate_id: std::sync::Arc::from("small_gpu"),
        gpu_model: "RTX 3060".to_string(),
        vram_total_mb: 12 * 1024,
        vram_available_mb: 100,
        loaded_models: vec![],
        queue_depth: 5,
        reachable: true,
        endpoint: None,
    };
    router.update_gate(gate);

    let decision = router.route("huge_model", 50000);
    assert!(matches!(
        decision.reason,
        RoutingReason::ShortestQueue | RoutingReason::Local
    ));
}

// ───── JsonRpcError serialization roundtrip ───────────────────────────────────

#[test]
fn test_jsonrpc_error_serialization_roundtrip() {
    let err = JsonRpcError {
        code: -32600,
        message: Cow::Borrowed("Invalid Request"),
        data: Some(serde_json::json!({"details": "bad"})),
    };
    let json = serde_json::to_string(&err).unwrap();
    let restored: JsonRpcError = serde_json::from_str(&json).unwrap();
    assert_eq!(err.code, restored.code);
    assert_eq!(err.message.as_ref(), restored.message.as_ref());
}

// ───── Method not found increments error count ────────────────────────────────

#[tokio::test]
async fn test_method_not_found_increments_error_count() {
    let error_count = Arc::new(AtomicU64::new(0));
    let handler = JsonRpcHandler::new(
        Arc::new(StandaloneExecutor::new()),
        "1.0.0".to_string(),
        Some(Arc::clone(&error_count)),
    );

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("nonexistent.method"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let _ = handler.handle_request(&request).await;

    assert_eq!(error_count.load(Ordering::Relaxed), 1);
}

// ───── ServerEvent additional variants ───────────────────────────────────────

#[test]
fn test_server_event_runtime_engine_registered_to_json() {
    let event = ServerEvent::RuntimeEngineRegistered {
        runtime_type: RuntimeType::Wasm,
        timestamp: std::time::SystemTime::now(),
    };
    let json = event.to_json();
    assert!(json.contains("runtime_engine_registered"));
    assert!(json.contains("runtime_type"));
}

#[test]
fn test_server_event_resource_usage_update_to_json() {
    let event = ServerEvent::ResourceUsageUpdate {
        cpu_usage_percent: 45.5,
        memory_usage_percent: 62.3,
        active_executions: 3,
        timestamp: std::time::SystemTime::now(),
    };
    let json = event.to_json();
    assert!(json.contains("resource_usage_update"));
    assert!(json.contains("45.5"));
    assert!(json.contains("62.3"));
}

#[test]
fn test_server_event_execution_completed_to_json() {
    let exec_id = Uuid::new_v4();
    let event = ServerEvent::ExecutionCompleted {
        execution_id: exec_id,
        status: ExecutionStatus::Failed {
            error: "test error".into(),
        },
        duration_ms: 5000,
        timestamp: std::time::SystemTime::now(),
    };
    let json = event.to_json();
    assert!(json.contains("execution_completed"));
    assert!(json.contains(&exec_id.to_string()));
    assert!(json.contains("5000"));
}

// ───── ValidationError EstimationFailed display ───────────────────────────────

#[test]
fn test_validation_error_display_estimation_failed() {
    let est_err = EstimationError::CyclicGraph;
    let err = ValidationError::EstimationFailed(est_err);
    let msg = err.to_string();
    assert!(msg.to_lowercase().contains("estimation") || msg.to_lowercase().contains("cycle"));
}
