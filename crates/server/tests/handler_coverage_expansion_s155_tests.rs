// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for under-covered JSON-RPC handler modules.
//!
//! Expands coverage for:
//! - transport.rs (transport.open, transport.stream, transport.status)
//! - dispatch.rs (`dispatch_submit` success, `dispatch_forward`, status/result)

#![allow(deprecated)]
#![allow(clippy::redundant_closure_for_method_calls)]

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use toadstool_server::pure_jsonrpc::{JsonRpcError, JsonRpcHandler, JsonRpcRequest};
use toadstool_server::tarpc_server::{StandaloneExecutor, WorkloadExecutorDispatch};

fn test_handler() -> JsonRpcHandler {
    let executor = Arc::new(WorkloadExecutorDispatch::Standalone(
        StandaloneExecutor::new(),
    ));
    JsonRpcHandler::new(
        executor,
        "test-1.0.0".to_string(),
        None,
        Arc::new(AtomicBool::new(true)),
        None,
    )
}

fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest<'static> {
    JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Owned(method.to_string()),
        params,
        id: Some(serde_json::json!(id)),
    }
}

// ═══════════════════════════════════════════════════════════
// Transport handler — transport.open, transport.stream, transport.status
// ═══════════════════════════════════════════════════════════

#[cfg(all(target_os = "linux", feature = "display"))]
mod transport_handler_tests {
    use super::*;

    #[tokio::test]
    async fn transport_open_missing_params() {
        let handler = test_handler();
        let request = mk_request("transport.open", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn transport_open_missing_source_slot() {
        let handler = test_handler();
        let params = serde_json::json!({ "target_slot": "0000:41:00.0" });
        let request = mk_request("transport.open", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("source_slot"));
    }

    #[tokio::test]
    async fn transport_open_missing_target_slot() {
        let handler = test_handler();
        let params = serde_json::json!({ "source_slot": "0000:25:00.0" });
        let request = mk_request("transport.open", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
        assert!(err.message.contains("target_slot"));
    }

    #[tokio::test]
    async fn transport_open_nonexistent_link() {
        let handler = test_handler();
        let params = serde_json::json!({
            "source_slot": "0000:99:00.0",
            "target_slot": "0000:99:00.1"
        });
        let request = mk_request("transport.open", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert!(err.message.contains("No PCIe link") || err.message.contains("params"));
    }

    #[tokio::test]
    async fn transport_stream_missing_params() {
        let handler = test_handler();
        let request = mk_request("transport.stream", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    }

    #[tokio::test]
    async fn transport_stream_unregistered_transports() {
        let handler = test_handler();
        let params = serde_json::json!({
            "rx_id": "nonexistent-rx",
            "tx_id": "nonexistent-tx"
        });
        let request = mk_request("transport.stream", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert!(err.message.contains("not registered"));
    }

    #[tokio::test]
    async fn transport_status_all_streams_empty() {
        let handler = test_handler();
        let request = mk_request("transport.status", None, 1);
        let response = handler.handle_request(&request).await;

        assert!(response.error.is_none());
        let result = response.result.expect("result present");
        assert!(result["streams"].as_array().is_some());
        assert_eq!(result["count"].as_u64().unwrap(), 0);
    }

    #[tokio::test]
    async fn transport_status_unknown_stream_id() {
        let handler = test_handler();
        let params = serde_json::json!({ "stream_id": "stream-999" });
        let request = mk_request("transport.status", Some(params), 1);
        let response = handler.handle_request(&request).await;

        assert!(response.result.is_none());
        let err = response.error.expect("error present");
        assert!(err.message.contains("Unknown stream"));
    }
}

// ═══════════════════════════════════════════════════════════
// Dispatch handler — compute.dispatch.*
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn dispatch_submit_with_valid_binary_returns_job_id() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1, 2, 3, 4, 5],
        "bdf": "0000:01:00.0"
    });
    let request = mk_request("compute.dispatch.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(
        response.error.is_none(),
        "unexpected JSON-RPC error: {:?}",
        response.error
    );
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "submit");
    assert!(result["job_id"].as_str().is_some());
    assert!(result["status"].as_str().is_some());
}

#[tokio::test]
async fn dispatch_submit_missing_binary() {
    let handler = test_handler();
    let params = serde_json::json!({ "bdf": "0000:01:00.0" });
    let request = mk_request("compute.dispatch.submit", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
    assert!(err.message.contains("binary"));
}

#[tokio::test]
async fn dispatch_status_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.status", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_result_missing_job_id() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.result", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_submit_then_status_and_result() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [10, 20, 30],
        "bdf": "0000:01:00.0"
    });
    let submit_req = mk_request("compute.dispatch.submit", Some(params), 1);
    let submit_resp = handler.handle_request(&submit_req).await;

    assert!(submit_resp.error.is_none());
    let job_id = submit_resp
        .result
        .as_ref()
        .and_then(|r| r.get("job_id"))
        .and_then(|v| v.as_str())
        .expect("submit returns job_id");

    let status_params = serde_json::json!({ "job_id": job_id });
    let status_req = mk_request("compute.dispatch.status", Some(status_params), 2);
    let status_resp = handler.handle_request(&status_req).await;

    assert!(status_resp.error.is_none());
    let status_result = status_resp.result.expect("status result");
    assert_eq!(status_result["job_id"], job_id);
    assert!(status_result["status"].as_str().is_some());
    assert!(status_result["metadata"]["bdf"].as_str().is_some());

    let result_params = serde_json::json!({ "job_id": job_id });
    let result_req = mk_request("compute.dispatch.result", Some(result_params), 3);
    let result_resp = handler.handle_request(&result_req).await;

    assert!(result_resp.error.is_none());
    let result_val = result_resp.result.expect("result");
    assert_eq!(result_val["job_id"], job_id);
}

#[tokio::test]
async fn dispatch_forward_missing_params() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.forward", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::INVALID_PARAMS);
}

#[tokio::test]
async fn dispatch_forward_missing_endpoint() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [1, 2, 3],
        "params": {}
    });
    let request = mk_request("compute.dispatch.forward", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert!(err.message.contains("endpoint"));
}

#[tokio::test]
async fn dispatch_capabilities_returns_structure() {
    let handler = test_handler();
    let request = mk_request("compute.dispatch.capabilities", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "capabilities");
    assert!(result["output"]["sovereign_pipeline"].as_bool().unwrap());
    assert!(result["output"]["dispatch_modes"].as_array().is_some());
}

// Science domains tests: REMOVED — ecology/discovery/deploy are biomeOS's domain
// Shader compile tests: REMOVED — compilation is coralReef's domain
// Ollama tests: REMOVED — AI inference is Squirrel's domain
