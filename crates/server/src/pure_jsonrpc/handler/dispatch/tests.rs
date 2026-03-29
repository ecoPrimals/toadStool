// SPDX-License-Identifier: AGPL-3.0-only

use super::DispatchHandler;
use crate::pure_jsonrpc::types::JsonRpcError;

fn test_handler() -> DispatchHandler {
    DispatchHandler::new(crate::coral_reef_client::create_coral_reef_client())
}

fn submit_params(bdf: &str, dispatch_mode: &str) -> serde_json::Value {
    serde_json::json!({
        "binary": [1u8, 2, 3],
        "bdf": bdf,
        "dispatch_mode": dispatch_mode,
    })
}

#[tokio::test]
async fn dispatch_capabilities_returns_expected_structure() {
    let handler = test_handler();
    let result = handler
        .dispatch_capabilities(None)
        .await
        .expect("capabilities");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "capabilities");
    assert!(result["sovereign_pipeline"].as_bool().unwrap());
    assert!(result["dispatch_modes"].as_array().is_some());
    assert!(result["vfio_gpus"].as_array().is_some());
    assert!(result["drm_gpus"].as_array().is_some());
    assert!(result["total_dispatch_count"].as_u64().is_some());
    assert!(result["coral_reef_available"].is_boolean());
}

#[tokio::test]
async fn dispatch_capabilities_total_dispatch_count_increments_after_submit() {
    let handler = test_handler();
    let before = handler
        .dispatch_capabilities(None)
        .await
        .expect("capabilities")["total_dispatch_count"]
        .as_u64()
        .expect("total_dispatch_count");

    let params = submit_params("0000:03:00.0", "passthrough");
    handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");

    let after = handler
        .dispatch_capabilities(None)
        .await
        .expect("capabilities")["total_dispatch_count"]
        .as_u64()
        .expect("total_dispatch_count");
    assert_eq!(after, before + 1);
}

#[tokio::test]
async fn dispatch_submit_missing_params_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .dispatch_submit(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("binary"));
}

#[tokio::test]
async fn dispatch_submit_empty_binary_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "binary": [] });
    let err = handler
        .dispatch_submit(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("empty"));
}

#[tokio::test]
async fn dispatch_submit_missing_binary_field_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "bdf": "0000:03:00.0" });
    let err = handler
        .dispatch_submit(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_submit_binary_not_array_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": "not-an-array",
        "bdf": "0000:03:00.0",
    });
    let err = handler
        .dispatch_submit(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_submit_vfio_mode_without_coral_returns_failed_payload() {
    let coral = crate::coral_reef_client::create_coral_reef_client();
    if coral.is_available().await {
        return;
    }
    let handler = DispatchHandler::new(coral);
    let params = submit_params("0000:03:00.0", "vfio");
    let result = handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["status"], "failed");
    assert!(
        result["error"]
            .as_str()
            .is_some_and(|s| s.contains("coralReef"))
    );
}

#[tokio::test]
async fn dispatch_submit_drm_mode_without_coral_returns_failed_payload() {
    let coral = crate::coral_reef_client::create_coral_reef_client();
    if coral.is_available().await {
        return;
    }
    let handler = DispatchHandler::new(coral);
    let params = submit_params("0000:03:00.0", "drm");
    let result = handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");
    assert_eq!(result["status"], "failed");
    assert!(
        result["error"]
            .as_str()
            .is_some_and(|s| s.contains("coralReef"))
    );
}

#[tokio::test]
async fn dispatch_submit_custom_dispatch_mode_registers_job_for_status_and_result() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1u8, 2, 3],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
        "workgroup_size": [128, 2, 4],
        "buffers": [{ "name": "a", "size": 16 }],
        "timeout_ms": 9999u64,
    });
    let result = handler
        .dispatch_submit(Some(&params))
        .await
        .expect("submit");
    assert_eq!(result["domain"], "compute.dispatch");
    let job_id = result["job_id"].as_str().expect("job_id");
    assert_eq!(result["workgroup_size"], serde_json::json!([128, 2, 4]));

    let status = handler
        .dispatch_status(Some(&serde_json::json!({ "job_id": job_id })))
        .await
        .expect("status");
    assert_eq!(status["job_id"], job_id);
    assert!(status["status"].as_str().is_some());
    assert_eq!(status["bdf"], "0000:03:00.0");

    let got = handler
        .dispatch_result(Some(&serde_json::json!({ "job_id": job_id })))
        .await
        .expect("result");
    assert_eq!(got["job_id"], job_id);
}

#[tokio::test]
async fn dispatch_status_unknown_job_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": "nonexistent-uuid" });
    let err = handler
        .dispatch_status(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("not found"));
}

#[tokio::test]
async fn dispatch_status_missing_job_id_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .dispatch_status(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);

    let err = handler
        .dispatch_status(Some(&serde_json::json!({})))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_status_job_id_not_string_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": 12345 });
    let err = handler
        .dispatch_status(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_result_unknown_job_returns_error() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": "nonexistent-uuid" });
    let err = handler
        .dispatch_result(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("not found"));
}

#[tokio::test]
async fn dispatch_result_missing_job_id_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .dispatch_result(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_result_job_id_not_string_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": true });
    let err = handler
        .dispatch_result(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_forward_missing_params_returns_invalid_params() {
    let handler = test_handler();
    let err = handler
        .dispatch_forward(None)
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_forward_missing_endpoint_returns_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "binary": [1] });
    let err = handler
        .dispatch_forward(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("endpoint"));
}

#[tokio::test]
async fn dispatch_forward_unreachable_returns_internal_error() {
    let handler = test_handler();
    let params = serde_json::json!({
        "endpoint": "127.0.0.1:1",
        "binary": [1, 2],
        "bdf": "0000:03:00.0",
    });
    let err = handler
        .dispatch_forward(Some(&params))
        .await
        .expect_err("expected error");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
    assert!(err.message.contains("127.0.0.1:1") || err.message.contains("failed"));
}

#[tokio::test]
async fn dispatch_forward_uses_nested_params_when_present() {
    let handler = test_handler();
    let params = serde_json::json!({
        "endpoint": "127.0.0.1:1",
        "params": {
            "binary": [9],
            "bdf": "0000:03:00.0",
        },
    });
    let err = handler
        .dispatch_forward(Some(&params))
        .await
        .expect_err("expected transport error");
    assert_eq!(err.code, JsonRpcError::INTERNAL_ERROR);
}
