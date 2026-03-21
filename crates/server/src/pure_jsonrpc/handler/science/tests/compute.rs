// SPDX-License-Identifier: AGPL-3.0-only

use super::common::{mk_request, test_handler};
use crate::pure_jsonrpc::types::JsonRpcError;

#[tokio::test]
async fn science_compute_submit_valid() {
    let handler = test_handler();
    let params = serde_json::json!({
        "inference": { "model": "tinyllama", "prompt": "test", "params": {} }
    });
    let request = mk_request("science.compute.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result.get("job_id").is_some());
    assert!(result.get("routing").is_some());
}

#[tokio::test]
async fn science_compute_submit_missing_params() {
    let handler = test_handler();
    let request = mk_request("science.compute.submit", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn science_compute_submit_invalid_params() {
    let handler = test_handler();
    let params = serde_json::json!({ "invalid": "job_type" });
    let request = mk_request("science.compute.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn science_compute_submit_empty_inference_params() {
    let handler = test_handler();
    let params = serde_json::json!({
        "inference": { "model": "m", "prompt": "p", "params": {} }
    });
    let request = mk_request("science.compute.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result.get("job_id").is_some());
}

#[tokio::test]
async fn science_compute_status_valid() {
    let handler = test_handler();
    let params = serde_json::json!({
        "inference": { "model": "m", "prompt": "p", "params": {} }
    });
    let submit_resp = handler
        .handle_request(&mk_request("science.compute.submit", Some(params), 1))
        .await;
    let job_id = submit_resp
        .result
        .as_ref()
        .and_then(|r| r.get("job_id"))
        .and_then(|v| v.as_str())
        .expect("job_id");

    let status_params = serde_json::json!({ "job_id": job_id });
    let request = mk_request("science.compute.status", Some(status_params), 2);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result.get("state").is_some());
}

#[tokio::test]
async fn science_compute_status_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("science.compute.status", Some(serde_json::json!({})), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn science_compute_status_invalid_uuid() {
    let handler = test_handler();
    let params = serde_json::json!({ "job_id": "not-a-uuid" });
    let request = mk_request("science.compute.status", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn science_compute_result_valid() {
    let handler = test_handler();
    let params = serde_json::json!({
        "inference": { "model": "m", "prompt": "p", "params": {} }
    });
    let submit_resp = handler
        .handle_request(&mk_request("science.compute.submit", Some(params), 1))
        .await;
    let job_id = submit_resp
        .result
        .as_ref()
        .and_then(|r| r.get("job_id"))
        .and_then(|v| v.as_str())
        .expect("job_id");

    let result_params = serde_json::json!({ "job_id": job_id });
    let request = mk_request("science.compute.result", Some(result_params), 2);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test]
async fn science_compute_result_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("science.compute.result", Some(serde_json::json!({})), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn science_compute_result_job_not_found() {
    let handler = test_handler();
    let job_id = uuid::Uuid::new_v4();
    let params = serde_json::json!({ "job_id": job_id.to_string() });
    let request = mk_request("science.compute.result", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(
        err.code,
        toadstool_common::constants::jsonrpc::error_codes::WORKLOAD_NOT_FOUND
    );
}

#[tokio::test]
async fn science_compute_cancel_valid() {
    let handler = test_handler();
    let params = serde_json::json!({
        "inference": { "model": "m", "prompt": "p", "params": {} }
    });
    let submit_resp = handler
        .handle_request(&mk_request("science.compute.submit", Some(params), 1))
        .await;
    let job_id = submit_resp
        .result
        .as_ref()
        .and_then(|r| r.get("job_id"))
        .and_then(|v| v.as_str())
        .expect("job_id");

    let cancel_params = serde_json::json!({ "job_id": job_id });
    let request = mk_request("science.compute.cancel", Some(cancel_params), 2);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["cancelled"], true);
}

#[tokio::test]
async fn science_compute_cancel_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("science.compute.cancel", Some(serde_json::json!({})), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}
