// SPDX-License-Identifier: AGPL-3.0-or-later

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::JsonRpcHandler;
use crate::pure_jsonrpc::types::{JsonRpcError, JsonRpcRequest};

fn test_handler() -> JsonRpcHandler {
    let executor = Arc::new(crate::tarpc_server::WorkloadExecutorDispatch::Standalone(
        crate::tarpc_server::StandaloneExecutor::new(),
    ));
    JsonRpcHandler::new(executor, "test-1.0.0".to_string(), None, Arc::new(AtomicBool::new(true)))
}

fn mk_request(method: &str, params: Option<serde_json::Value>, id: i32) -> JsonRpcRequest<'static> {
    JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Owned(method.to_string()),
        params,
        id: Some(serde_json::json!(id)),
    }
}

#[tokio::test]
async fn test_health_returns_valid_status() {
    let handler = test_handler();
    let request = mk_request("toadstool.health", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["healthy"].as_bool().unwrap());
    assert!(result["version"].as_str().is_some());
    assert!(result["uptime_secs"].as_u64().is_some());
    assert!(result["error_count"].as_u64().is_some());
}

#[tokio::test]
async fn test_health_triad_liveness_readiness_check() {
    let handler = test_handler();

    let live = handler
        .handle_request(&mk_request("health.liveness", None, 10))
        .await;
    assert!(live.error.is_none());
    let r = live.result.expect("liveness");
    assert_eq!(r["status"], "alive");
    assert!(r.get("healthy").is_none(), "liveness must be minimal");

    let ready = handler
        .handle_request(&mk_request("health.readiness", None, 11))
        .await;
    assert!(ready.error.is_none());
    let r = ready.result.expect("readiness");
    assert_eq!(r["status"], "ready");
    assert_eq!(r["version"], "test-1.0.0");

    let check = handler
        .handle_request(&mk_request("health.check", None, 12))
        .await;
    assert!(check.error.is_none());
    let r = check.result.expect("check");
    assert!(r["healthy"].as_bool().unwrap());
    assert_eq!(r["status"], "alive");
}

/// PG-62: health.liveness returns "starting" when the readiness flag is false.
#[tokio::test]
async fn test_health_liveness_returns_starting_before_ready() {
    use std::sync::atomic::Ordering;

    let ready = Arc::new(AtomicBool::new(false));
    let executor = Arc::new(crate::tarpc_server::WorkloadExecutorDispatch::Standalone(
        crate::tarpc_server::StandaloneExecutor::new(),
    ));
    let handler = JsonRpcHandler::new(executor, "test-1.0.0".to_string(), None, Arc::clone(&ready));

    let live = handler
        .handle_request(&mk_request("health.liveness", None, 20))
        .await;
    assert!(live.error.is_none());
    let r = live.result.expect("liveness starting");
    assert_eq!(r["status"], "starting");

    let rdns = handler
        .handle_request(&mk_request("health.readiness", None, 21))
        .await;
    assert!(rdns.error.is_none());
    let r = rdns.result.expect("readiness starting");
    assert_eq!(r["status"], "starting");

    ready.store(true, Ordering::Release);

    let live = handler
        .handle_request(&mk_request("health.liveness", None, 22))
        .await;
    assert!(live.error.is_none());
    let r = live.result.expect("liveness alive");
    assert_eq!(r["status"], "alive");

    let rdns = handler
        .handle_request(&mk_request("health.readiness", None, 23))
        .await;
    assert!(rdns.error.is_none());
    let r = rdns.result.expect("readiness ready");
    assert_eq!(r["status"], "ready");
    assert_eq!(r["version"], "test-1.0.0");
}

#[tokio::test]
async fn test_version_info_returns_expected_fields() {
    let handler = test_handler();
    let request = mk_request("toadstool.version", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["version"], "test-1.0.0");
    assert_eq!(result["protocol"], "JSON-RPC 2.0");
    assert_eq!(result["service"], "ToadStool Compute");
    assert!(result["implementation"].as_str().is_some());
}

#[tokio::test]
async fn test_handle_method_returns_method_not_found_for_unknown() {
    let handler = test_handler();
    let request = mk_request("unknown.nonexistent.method", None, 99);
    let response = handler.handle_request(&request).await;

    assert!(response.result.is_none());
    let err = response.error.expect("error present");
    assert_eq!(err.code, JsonRpcError::METHOD_NOT_FOUND);
    assert!(err.message.contains("unknown.nonexistent.method"));
}

#[tokio::test]
async fn test_discover_capabilities_includes_shader_methods() {
    let handler = test_handler();
    let request = mk_request("compute.discover_capabilities", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    let methods = result["methods"].as_array().expect("methods is array");
    let has_shader_dispatch = methods
        .iter()
        .any(|m| m.as_str() == Some("shader.dispatch"));
    assert!(
        has_shader_dispatch,
        "methods should include shader.dispatch"
    );
}

#[tokio::test]
async fn test_shader_dispatch_routes_and_returns_domain() {
    let handler = test_handler();
    let params = serde_json::json!({
        "binary": [0xDE, 0xAD, 0xBE, 0xEF],
        "bdf": "0000:03:00.0",
        "dispatch_mode": "passthrough",
    });
    let request = mk_request("shader.dispatch", Some(params), 1);
    let response = handler.handle_request(&request).await;
    assert!(
        response.error.is_none(),
        "shader.dispatch should route without error"
    );
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "compute.dispatch");
    assert_eq!(result["operation"], "shader");
    assert!(result["job_id"].as_str().is_some());
    assert_eq!(result["metadata"]["binary_size"], 4);
}

#[tokio::test]
async fn test_auth_check_returns_allowed_permissive() {
    let handler = test_handler();
    let params = serde_json::json!({"method": "compute.dispatch.submit"});
    let request = mk_request("auth.check", Some(params), 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result");
    assert_eq!(result["allowed"], true);
    assert_eq!(result["visibility"], "protected");
    assert_eq!(result["mode"], "permissive");
    assert_eq!(result["method"], "compute.dispatch.submit");
}

#[tokio::test]
async fn test_auth_mode_returns_permissive() {
    let handler = test_handler();
    let request = mk_request("auth.mode", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result");
    assert_eq!(result["mode"], "permissive");
}

#[tokio::test]
async fn test_auth_peer_info_returns_unknown() {
    let handler = test_handler();
    let request = mk_request("auth.peer_info", None, 1);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result");
    assert_eq!(result["transport"], "unknown");
    assert_eq!(result["authenticated"], false);
}
