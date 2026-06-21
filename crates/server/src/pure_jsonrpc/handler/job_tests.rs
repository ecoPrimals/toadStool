// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

fn handler() -> JobHandler {
    JobHandler::new(Arc::new(GateOwnership::new("local-test")))
}

#[tokio::test]
async fn test_gate_update_and_list_endpoint_serializes() {
    let handler = handler();
    let gate_info = serde_json::json!({
        "gate_id": "remote-gate",
        "gpu_model": "RTX 4090",
        "vram_total_mb": 24576,
        "vram_available_mb": 20000,
        "loaded_models": [],
        "queue_depth": 0,
        "reachable": true,
        "endpoint": "/tmp/remote-gate.sock"
    });
    handler.gate_update(Some(&gate_info)).await.unwrap();
    let list = handler.gate_list().await.unwrap();
    let gates = list["gates"].as_array().expect("gates array");
    let remote = gates
        .iter()
        .find(|g| g["gate_id"] == "remote-gate")
        .expect("remote-gate in list");
    assert_eq!(remote["endpoint"].as_str(), Some("/tmp/remote-gate.sock"));
}

#[test]
fn extract_job_id_missing_params() {
    let err = JobHandler::extract_job_id(None).unwrap_err();
    assert!(err.message.contains("Missing params"));
}

#[test]
fn extract_job_id_missing_field() {
    let params = serde_json::json!({});
    let err = JobHandler::extract_job_id(Some(&params)).unwrap_err();
    assert!(err.message.contains("job_id"));
}

#[test]
fn extract_job_id_invalid_uuid() {
    let params = serde_json::json!({"job_id": "not-a-uuid"});
    let err = JobHandler::extract_job_id(Some(&params)).unwrap_err();
    assert!(err.message.contains("Invalid"));
}

#[test]
fn extract_job_id_valid() {
    let id = Uuid::new_v4();
    let params = serde_json::json!({"job_id": id.to_string()});
    let parsed = JobHandler::extract_job_id(Some(&params)).unwrap();
    assert_eq!(parsed, id);
}

#[test]
fn job_queue_error_not_found_code() {
    let err = JobQueueError::JobNotFound { id: Uuid::new_v4() };
    let rpc_err = JobHandler::job_queue_error(&err);
    assert_eq!(
        rpc_err.code,
        toadstool_common::constants::jsonrpc::error_codes::WORKLOAD_NOT_FOUND
    );
}

#[tokio::test]
async fn gate_remove_missing_id() {
    let h = handler();
    let err = h.gate_remove(None).await.unwrap_err();
    assert!(err.message.contains("gate_id"));
}

#[tokio::test]
async fn gate_remove_valid() {
    let h = handler();
    let gate_info = serde_json::json!({
        "gate_id": "to-remove",
        "gpu_model": "A100",
        "vram_total_mb": 40960,
        "vram_available_mb": 40960,
        "loaded_models": [],
        "queue_depth": 0,
        "reachable": true,
        "endpoint": "/tmp/remove.sock"
    });
    h.gate_update(Some(&gate_info)).await.unwrap();
    let result = h
        .gate_remove(Some(&serde_json::json!({"gate_id": "to-remove"})))
        .await
        .unwrap();
    assert_eq!(result["removed"], true);
}

#[tokio::test]
async fn gate_route_defaults() {
    let h = handler();
    let params = serde_json::json!({});
    let result = h.gate_route(Some(&params)).await.unwrap();
    assert!(result["gate_id"].is_string());
}

#[tokio::test]
async fn compute_list_empty() {
    let h = handler();
    let result = h.compute_list(None).await.unwrap();
    assert!(result["jobs"].is_array());
    assert!(result["counts"].is_object());
}

#[tokio::test]
async fn list_workloads_empty() {
    let h = handler();
    let result = h.list_workloads(None).await.unwrap();
    assert!(result["jobs"].is_array());
}

#[tokio::test]
async fn query_status_missing_params() {
    let h = handler();
    let err = h.query_status(None).await.unwrap_err();
    assert!(err.message.contains("Missing params"));
}

#[tokio::test]
async fn query_status_invalid_id() {
    let h = handler();
    let err = h
        .query_status(Some(&serde_json::json!("not-a-uuid")))
        .await
        .unwrap_err();
    assert!(err.message.contains("Invalid"));
}

#[tokio::test]
async fn compute_status_not_found() {
    let h = handler();
    let id = Uuid::new_v4();
    let params = serde_json::json!({"job_id": id.to_string()});
    let err = h.compute_status(Some(&params)).await.unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::WORKLOAD_NOT_FOUND
    );
}

#[tokio::test]
async fn compute_cancel_not_found() {
    let h = handler();
    let id = Uuid::new_v4();
    let params = serde_json::json!({"job_id": id.to_string()});
    let err = h.compute_cancel(Some(&params)).await.unwrap_err();
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::WORKLOAD_NOT_FOUND
    );
}

#[tokio::test]
async fn gate_update_is_owner_sets_hardware_owner() {
    let h = handler();
    assert_eq!(
        h.gate_ownership.hardware_owner_gate_id().await.as_ref(),
        "local-test"
    );

    let gate_info = serde_json::json!({
        "gate_id": "remote-owner",
        "gpu_model": "RTX 4090",
        "vram_total_mb": 24576,
        "vram_available_mb": 20000,
        "loaded_models": [],
        "queue_depth": 0,
        "reachable": true,
        "is_owner": true,
    });
    h.gate_update(Some(&gate_info)).await.unwrap();
    assert_eq!(
        h.gate_ownership.hardware_owner_gate_id().await.as_ref(),
        "remote-owner"
    );
}

#[tokio::test]
async fn compute_submit_missing_params() {
    let h = handler();
    let err = h.compute_submit(None).await.unwrap_err();
    assert!(err.message.contains("Missing params"));
}

fn handler_with_queue_size(max: usize) -> JobHandler {
    let mut h = handler();
    h.job_queue = GpuJobQueue::new(JobQueueConfig {
        max_queue_size: max,
        max_concurrent: 4,
    });
    h
}

fn inference_params(model: &str) -> serde_json::Value {
    serde_json::json!({
        "inference": {
            "model": model,
            "prompt": "test",
            "params": {}
        }
    })
}

#[tokio::test]
async fn compute_submit_queue_full_rejection() {
    let h = handler_with_queue_size(1);
    h.compute_submit(Some(&inference_params("m1")))
        .await
        .unwrap();
    let err = h
        .compute_submit(Some(&inference_params("m2")))
        .await
        .unwrap_err();
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("Queue is full"));
}

#[tokio::test]
async fn compute_submit_invalid_job_type() {
    let h = handler();
    let err = h
        .compute_submit(Some(&serde_json::json!({"not_a_job": true})))
        .await
        .unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("Invalid job type"));
}

#[tokio::test]
async fn job_status_transitions_pending_running_completed() {
    let h = handler();
    let submit = h
        .compute_submit(Some(&inference_params("lifecycle")))
        .await
        .unwrap();
    let job_id = Uuid::parse_str(submit["job_id"].as_str().unwrap()).unwrap();

    let pending = h
        .compute_status(Some(&serde_json::json!({"job_id": job_id.to_string()})))
        .await
        .unwrap();
    assert_eq!(pending["state"], "pending");

    h.job_queue.mark_running(job_id).await.unwrap();
    let running = h
        .compute_status(Some(&serde_json::json!({"job_id": job_id.to_string()})))
        .await
        .unwrap();
    assert_eq!(running["state"], "running");

    h.job_queue
        .mark_completed(job_id, serde_json::json!({"tokens": 42}))
        .await
        .unwrap();
    let completed = h
        .compute_status(Some(&serde_json::json!({"job_id": job_id.to_string()})))
        .await
        .unwrap();
    assert_eq!(completed["state"], "completed");
}

#[tokio::test]
async fn compute_result_after_completion() {
    let h = handler();
    let submit = h
        .compute_submit(Some(&inference_params("result-test")))
        .await
        .unwrap();
    let job_id = Uuid::parse_str(submit["job_id"].as_str().unwrap()).unwrap();
    h.job_queue
        .mark_completed(job_id, serde_json::json!({"answer": "ok"}))
        .await
        .unwrap();

    let result = h
        .compute_result(Some(&serde_json::json!({"job_id": job_id.to_string()})))
        .await
        .unwrap();
    assert_eq!(result["answer"], "ok");
}

#[tokio::test]
async fn compute_result_not_complete_returns_error() {
    let h = handler();
    let submit = h
        .compute_submit(Some(&inference_params("pending")))
        .await
        .unwrap();
    let job_id = Uuid::parse_str(submit["job_id"].as_str().unwrap()).unwrap();
    let err = h
        .compute_result(Some(&serde_json::json!({"job_id": job_id.to_string()})))
        .await
        .unwrap_err();
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("not complete"));
}

#[tokio::test]
async fn compute_cancel_while_running() {
    let h = handler();
    let submit = h
        .compute_submit(Some(&inference_params("cancel-running")))
        .await
        .unwrap();
    let job_id = Uuid::parse_str(submit["job_id"].as_str().unwrap()).unwrap();
    h.job_queue.mark_running(job_id).await.unwrap();

    let cancel = h
        .compute_cancel(Some(&serde_json::json!({"job_id": job_id.to_string()})))
        .await
        .unwrap();
    assert_eq!(cancel["cancelled"], true);

    let status = h
        .compute_status(Some(&serde_json::json!({"job_id": job_id.to_string()})))
        .await
        .unwrap();
    assert_eq!(status["state"], "cancelled");
}

#[tokio::test]
async fn compute_cancel_completed_job_rejected() {
    let h = handler();
    let submit = h
        .compute_submit(Some(&inference_params("done")))
        .await
        .unwrap();
    let job_id = Uuid::parse_str(submit["job_id"].as_str().unwrap()).unwrap();
    h.job_queue
        .mark_completed(job_id, serde_json::json!({}))
        .await
        .unwrap();

    let err = h
        .compute_cancel(Some(&serde_json::json!({"job_id": job_id.to_string()})))
        .await
        .unwrap_err();
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("Cannot cancel"));
}

#[tokio::test]
async fn compute_result_malformed_job_id() {
    let h = handler();
    let err = h
        .compute_result(Some(&serde_json::json!({"job_id": "bad-id"})))
        .await
        .unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn compute_cancel_malformed_job_id() {
    let h = handler();
    let err = h
        .compute_cancel(Some(&serde_json::json!({"job_id": ""})))
        .await
        .unwrap_err();
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn job_queue_error_non_not_found_uses_internal_code() {
    let err = JobQueueError::QueueFull { max: 10 };
    let rpc_err = JobHandler::job_queue_error(&err);
    assert_eq!(rpc_err.code, JsonRpcError::INTERNAL_ERROR);
}
