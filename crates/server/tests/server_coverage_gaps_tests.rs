// SPDX-License-Identifier: AGPL-3.0-or-later
//! Additional coverage tests for server crate edge cases and error paths.

#![allow(clippy::redundant_closure_for_method_calls)]
//!
//! Focus: JSON-RPC compute methods, GPU job queue edge cases, resource estimator,
//! tarpc executor, and graph error display.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use toadstool_server::GraphValidationError;
use toadstool_server::cross_gate::{GateGpuInfo, JobRouter, RoutingReason};
use toadstool_server::gpu_job_queue::{
    GpuJobQueue, JobQueueConfig, JobQueueError, JobState, JobType,
};
use toadstool_server::graph_types::{
    EdgeType, ExecutionGraph, GraphEdge, GraphNode, NodeResourceRequirements,
};
use toadstool_server::pure_jsonrpc::{JsonRpcError, JsonRpcHandler, JsonRpcRequest};
use toadstool_server::resource_estimator::{EstimationError, ResourceEstimator};
use toadstool_server::tarpc_server::{StandaloneExecutor, WorkloadExecutorDispatch};
use uuid::Uuid;

fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest<'static> {
    JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Owned(method.to_string()),
        params,
        id: Some(serde_json::json!(id)),
    }
}

// ───── Compute.submit with transform and custom job types ────────────────────

#[tokio::test]
async fn test_compute_submit_transform_job_type() {
    let handler = JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
    );
    let params = serde_json::json!({
        "transform": {
            "operation": "embed",
            "input": {"text": "hello world"}
        }
    });
    let request = mk_request("compute.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["job_id"].as_str().is_some());
}

#[tokio::test]
async fn test_compute_submit_custom_job_type() {
    let handler = JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
    );
    let params = serde_json::json!({
        "custom": {
            "plugin": "my_plugin",
            "payload": {"config": "value"}
        }
    });
    let request = mk_request("compute.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["job_id"].as_str().is_some());
}

#[tokio::test]
async fn test_compute_submit_inference_with_params() {
    let handler = JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
    );
    let params = serde_json::json!({
        "inference": {
            "model": "llama2",
            "prompt": "Hello world",
            "params": {"temperature": 0.7}
        }
    });
    let request = mk_request("compute.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(
        response.error.is_none(),
        "unexpected error: {:?}",
        response.error
    );
    let result = response.result.expect("result present");
    assert!(result["job_id"].as_str().is_some());
}

// ───── Compute status/result/cancel invalid job_id format ────────────────────

#[tokio::test]
async fn test_compute_status_invalid_uuid_format() {
    let handler = JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
    );
    let params = serde_json::json!({ "job_id": "not-a-valid-uuid" });
    let request = mk_request("compute.status", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_compute_result_invalid_uuid_format() {
    let handler = JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
    );
    let params = serde_json::json!({ "job_id": "garbage" });
    let request = mk_request("compute.result", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn test_compute_cancel_invalid_uuid_format() {
    let handler = JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
    );
    let params = serde_json::json!({ "job_id": 12345 });
    let request = mk_request("compute.cancel", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

// ───── Query status params not string ────────────────────────────────────────

#[tokio::test]
async fn test_query_status_params_not_string() {
    let handler = JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
    );
    let params = serde_json::json!(42);
    let request = mk_request("toadstool.query_status", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

// ───── GPU job queue QueueFull error type ─────────────────────────────────────

#[tokio::test]
async fn test_queue_full_error_variant() {
    let queue = GpuJobQueue::new(JobQueueConfig {
        max_queue_size: 1,
        max_concurrent: 1,
    });

    let _ = queue
        .submit(
            JobType::Custom {
                plugin: "a".to_string(),
                payload: serde_json::Value::Null,
            },
            0,
        )
        .await
        .unwrap();

    let err = queue
        .submit(
            JobType::Custom {
                plugin: "b".to_string(),
                payload: serde_json::Value::Null,
            },
            0,
        )
        .await
        .unwrap_err();

    assert!(matches!(err, JobQueueError::QueueFull { max } if max == 1));
    assert!(err.to_string().contains("full"));
}

// ───── GPU job queue cancel Cancelled job returns CannotCancel ────────────────

#[tokio::test]
async fn test_cancel_cancelled_job_returns_error() {
    let queue = GpuJobQueue::new(JobQueueConfig::default());
    let id = queue
        .submit(
            JobType::Custom {
                plugin: "x".to_string(),
                payload: serde_json::Value::Null,
            },
            0,
        )
        .await
        .unwrap();

    queue.cancel(id).await.unwrap();

    let err = queue.cancel(id).await.unwrap_err();
    assert!(
        matches!(err, JobQueueError::CannotCancel { state, .. } if state == JobState::Cancelled)
    );
}

// ───── Resource estimator cyclic graph ────────────────────────────────────────

#[test]
fn test_estimator_cyclic_graph_error() {
    use toadstool::resources::{CpuRequirements, MemoryRequirements};

    let estimator = ResourceEstimator::new();
    let graph = ExecutionGraph {
        id: "cycle".to_string(),
        nodes: vec![
            GraphNode {
                id: "a".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements {
                    cpu: Some(CpuRequirements::default()),
                    memory: Some(MemoryRequirements::default()),
                    ..Default::default()
                },
                metadata: std::collections::HashMap::new(),
            },
            GraphNode {
                id: "b".to_string(),
                primal: "toadstool".to_string(),
                operation: "cpu_compute".to_string(),
                duration: None,
                requirements: NodeResourceRequirements::default(),
                metadata: std::collections::HashMap::new(),
            },
        ],
        edges: vec![
            GraphEdge::new("a", "b"),
            GraphEdge::new("b", "a"), // cycle
        ],
        metadata: std::collections::HashMap::new(),
    };

    let result = estimator.estimate(&graph);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, EstimationError::CyclicGraph)
            || matches!(err, EstimationError::InvalidGraph(_)),
        "expected cycle-related error, got: {err}"
    );
}

// ───── GraphValidationError InvalidEdge display ────────────────────────────────

#[test]
fn test_graph_validation_error_invalid_edge_display() {
    let err = GraphValidationError::InvalidEdge {
        from: "a".to_string(),
        to: "b".to_string(),
        reason: "invalid connection".to_string(),
    };
    let msg = err.to_string();
    assert!(msg.contains('a'));
    assert!(msg.contains('b'));
    assert!(msg.contains("invalid"));
}

// ───── GraphEdge data_flow and control (graph_types) ───────────────────────────

#[test]
fn test_graph_edge_data_flow_and_control() {
    let edge_df = GraphEdge::data_flow("producer", "consumer");
    assert_eq!(edge_df.edge_type, EdgeType::DataFlow);
    let edge_ctrl = GraphEdge::control("init", "process");
    assert_eq!(edge_ctrl.edge_type, EdgeType::Control);
}

// ───── StandaloneExecutor basic operations ────────────────────────────────────

#[tokio::test]
async fn test_standalone_executor_execute() {
    use bytes::Bytes;
    use std::sync::Arc;
    use toadstool_server::rpc_types::{ResourceRequirements, WorkloadPriority, WorkloadSubmission};
    use toadstool_server::tarpc_server::WorkloadExecutor;

    let executor = StandaloneExecutor::new();
    let submission = WorkloadSubmission {
        workload_id: Arc::from("test-1"),
        workload_type: Arc::from("cpu_compute"),
        data: Bytes::from(vec![1, 2, 3]),
        metadata: std::collections::HashMap::new(),
        priority: WorkloadPriority::Normal,
        requirements: ResourceRequirements::default(),
    };

    let result = executor.execute(submission).await;
    assert!(result.is_ok());
    let res = result.unwrap();
    assert_eq!(res.workload_id.as_ref(), "test-1");
}

#[tokio::test]
async fn test_standalone_executor_cancel_nonexistent() {
    use toadstool_server::tarpc_server::WorkloadExecutor;

    let executor = StandaloneExecutor::new();
    let result = executor.cancel("nonexistent-id").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_standalone_executor_query_capabilities() {
    use toadstool_server::tarpc_server::WorkloadExecutor;

    let executor = StandaloneExecutor::new();
    let result = executor.query_capabilities().await;
    assert!(result.is_ok());
    let caps = result.unwrap();
    assert!(!caps.service_id.is_empty());
}

// ───── Cross-gate RoutingReason Local fallback ────────────────────────────────

#[test]
fn test_route_shortest_queue_when_no_vram_candidates() {
    let mut router = JobRouter::new("local");

    let gate = GateGpuInfo {
        gate_id: std::sync::Arc::from("remote"),
        gpu_model: "RTX 3060".to_string(),
        vram_total_mb: 12288,
        vram_available_mb: 100,
        loaded_models: vec![],
        queue_depth: 10,
        reachable: true,
        endpoint: None,
        is_owner: false,
    };
    router.update_gate(gate);

    // No gate has enough VRAM for 50GB; router picks shortest queue from reachable
    let decision = router.route("huge_model", 50000);
    assert!(matches!(
        decision.reason,
        RoutingReason::ShortestQueue | RoutingReason::Local
    ));
}

// ───── JsonRpcError invalid_params with data ───────────────────────────────────

#[test]
fn test_jsonrpc_error_invalid_params_constructor() {
    let err = JsonRpcError::invalid_params("field X required");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("field X"));
}

// ───── JobQueueError display variants ─────────────────────────────────────────

#[test]
fn test_job_queue_error_job_failed_display() {
    let id = Uuid::new_v4();
    let err = JobQueueError::JobFailed {
        id,
        error: "OOM".to_string(),
    };
    assert!(err.to_string().contains("OOM"));
}

#[test]
fn test_job_queue_error_job_not_complete_display() {
    let id = Uuid::new_v4();
    let err = JobQueueError::JobNotComplete { id };
    assert!(err.to_string().to_lowercase().contains("not complete"));
}

#[test]
fn test_job_queue_error_no_result_display() {
    let id = Uuid::new_v4();
    let err = JobQueueError::NoResult { id };
    assert!(err.to_string().to_lowercase().contains("no result"));
}

// ───── EdgeType Dependency default ────────────────────────────────────────────

#[test]
fn test_edge_type_dependency_is_default() {
    let default: EdgeType = EdgeType::default();
    assert_eq!(default, EdgeType::Dependency);
}
