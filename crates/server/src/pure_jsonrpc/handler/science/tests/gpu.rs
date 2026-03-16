// SPDX-License-Identifier: AGPL-3.0-only

use super::common::{mk_request, test_handler};
use crate::pure_jsonrpc::types::JsonRpcError;

#[tokio::test]
async fn science_gpu_dispatch_valid() {
    let handler = test_handler();
    let params = serde_json::json!({
        "inference": { "model": "tinyllama", "prompt": "test", "params": {} }
    });
    let request = mk_request("science.gpu.dispatch", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result.get("job_id").is_some());
}

#[tokio::test]
async fn science_gpu_dispatch_missing_params() {
    let handler = test_handler();
    let request = mk_request("science.gpu.dispatch", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn science_gpu_capabilities_structure() {
    let handler = test_handler();
    let request = mk_request("science.gpu.capabilities", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result.get("devices").is_some());
    assert!(result.get("supported_precisions").is_some());
    assert!(result.get("precision_notes").is_some());
    assert!(result.get("compute_backends").is_some());
    assert_eq!(result["domain"], "science");
    assert!(result["precision_notes"]["f64_shared_memory_reliable"]
        .as_bool()
        .is_some());
    assert!(result["precision_notes"]["df64_reductions"]
        .as_bool()
        .is_some());
}
