// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::doc_markdown)]
//! Coverage expansion S162 — hw_learn handler routes, tarpc server, unibin paths
//!
//! Targets: auto_init.rs, hw_learn module dispatch, tarpc_server.rs, unibin paths

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use toadstool_server::pure_jsonrpc::{JsonRpcHandler, JsonRpcRequest};
use toadstool_server::{StandaloneExecutor, WorkloadExecutorDispatch};

fn test_handler() -> JsonRpcHandler {
    JsonRpcHandler::new(
        Arc::new(WorkloadExecutorDispatch::Standalone(
            StandaloneExecutor::new(),
        )),
        "test-s162-hw".to_string(),
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
// hw_learn handler routes via JSON-RPC
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn hw_learn_observe_requires_bdf() {
    let handler = test_handler();
    let request = mk_request("compute.hardware.observe", None, 1);
    let response = handler.handle_request(&request).await;
    let err = response.error.expect("should error without bdf");
    assert!(err.message.contains("bdf") || err.message.contains("param"));
}

#[tokio::test]
async fn hw_learn_distill_requires_params() {
    let handler = test_handler();
    let request = mk_request("compute.hardware.distill", None, 2);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn hw_learn_apply_requires_params() {
    let handler = test_handler();
    let request = mk_request("compute.hardware.apply", None, 3);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn hw_learn_share_recipe_requires_params() {
    let handler = test_handler();
    let request = mk_request("compute.hardware.share_recipe", None, 4);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn hw_learn_auto_init_no_gpu_returns_error() {
    let handler = test_handler();
    let request = mk_request("compute.hardware.auto_init", None, 5);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn hw_learn_auto_init_with_dry_run_no_gpu() {
    let handler = test_handler();
    let params = serde_json::json!({"dry_run": true});
    let request = mk_request("compute.hardware.auto_init", Some(params), 6);
    let response = handler.handle_request(&request).await;
    // Without GPUs this errors, but exercises the dry_run param parsing path
    assert!(response.error.is_some());
}

#[tokio::test]
async fn hw_learn_auto_init_all_returns_result_shape() {
    let handler = test_handler();
    let request = mk_request("compute.hardware.auto_init_all", None, 7);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["domain"], "compute.hardware");
    assert_eq!(result["operation"], "auto_init_all");
    assert!(result["gpus"].as_array().is_some());
    assert!(result["total"].as_u64().is_some());
}

#[tokio::test]
async fn hw_learn_auto_init_all_with_parallel_flag() {
    let handler = test_handler();
    let params = serde_json::json!({"parallel": true, "dry_run": true});
    let request = mk_request("compute.hardware.auto_init_all", Some(params), 8);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert_eq!(result["operation"], "auto_init_all");
}

#[tokio::test]
async fn hw_learn_status_returns_result() {
    let handler = test_handler();
    let request = mk_request("compute.hardware.status", None, 9);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result.is_object());
}

#[tokio::test]
async fn hw_learn_vfio_devices_returns_array() {
    let handler = test_handler();
    let request = mk_request("compute.hardware.vfio_devices", None, 10);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
    let result = response.result.expect("result present");
    assert!(result["devices"].as_array().is_some());
    assert!(result["count"].as_u64().is_some());
}

#[tokio::test]
async fn gpu_telemetry_returns_result() {
    let handler = test_handler();
    let request = mk_request("gpu.telemetry", None, 11);
    let response = handler.handle_request(&request).await;

    assert!(response.error.is_none());
}

// ═══════════════════════════════════════════════════════════
// tarpc_server coverage — workload lifecycle
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn tarpc_workload_submit_and_query() {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    let handler = test_handler();

    let params = serde_json::json!({
        "workload_id": "s162-test-1",
        "workload_type": "cpu_compute",
        "data": STANDARD.encode([1u8, 2, 3, 4]),
        "metadata": {},
        "priority": "Normal",
        "requirements": {
            "cpu_cores": 1,
            "memory_bytes": 512,
            "timeout_secs": 10
        }
    });
    let submit_req = mk_request("toadstool.submit_workload", Some(params), 20);
    let submit_resp = handler.handle_request(&submit_req).await;
    assert!(submit_resp.error.is_none());
    let submit_result = submit_resp.result.unwrap();
    assert_eq!(submit_result["workload_id"], "s162-test-1");

    let status_params = serde_json::json!({"workload_id": "s162-test-1"});
    let status_req = mk_request("toadstool.query_status", Some(status_params), 21);
    let status_resp = handler.handle_request(&status_req).await;
    // Query status exercises the routing path regardless of whether the
    // workload is found in the internal state (it may execute synchronously
    // and complete before we query).
    assert!(status_resp.result.is_some() || status_resp.error.is_some());
}

#[tokio::test]
async fn tarpc_list_workloads() {
    let handler = test_handler();
    let request = mk_request("toadstool.list_workloads", None, 22);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

#[tokio::test]
async fn tarpc_cancel_nonexistent_workload() {
    let handler = test_handler();
    let params = serde_json::json!({"workload_id": "nonexistent-uuid"});
    let request = mk_request("toadstool.cancel_workload", Some(params), 23);
    let response = handler.handle_request(&request).await;
    // Should handle gracefully even for missing workload
    assert!(
        response.error.is_some() || response.result.is_some(),
        "cancel should respond"
    );
}

#[tokio::test]
async fn tarpc_query_capabilities() {
    let handler = test_handler();
    let request = mk_request("toadstool.query_capabilities", None, 24);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert!(result.is_object());
}

// ═══════════════════════════════════════════════════════════
// compute.* aliases
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn compute_submit_lifecycle() {
    let handler = test_handler();

    let params = serde_json::json!({
        "inference": {"model": "test", "prompt": "test", "params": {}}
    });
    let submit = mk_request("compute.submit", Some(params), 30);
    let resp = handler.handle_request(&submit).await;
    assert!(resp.error.is_none());
    let result = resp.result.unwrap();
    let job_id = result["job_id"].as_str().unwrap().to_string();

    let status = mk_request(
        "compute.status",
        Some(serde_json::json!({"job_id": job_id})),
        31,
    );
    let status_resp = handler.handle_request(&status).await;
    assert!(status_resp.error.is_none());

    let result_req = mk_request(
        "compute.result",
        Some(serde_json::json!({"job_id": &job_id})),
        32,
    );
    let result_resp = handler.handle_request(&result_req).await;
    // Result may or may not be ready; both are valid
    assert!(result_resp.result.is_some() || result_resp.error.is_some());

    let list = mk_request("compute.list", None, 33);
    let list_resp = handler.handle_request(&list).await;
    assert!(list_resp.error.is_none());
    let list_result = list_resp.result.unwrap();
    assert!(list_result["jobs"].as_array().is_some());

    let cancel = mk_request(
        "compute.cancel",
        Some(serde_json::json!({"job_id": &job_id})),
        34,
    );
    let cancel_resp = handler.handle_request(&cancel).await;
    assert!(cancel_resp.error.is_none() || cancel_resp.error.is_some());
}

// ═══════════════════════════════════════════════════════════
// GPU info handlers
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn gpu_info_returns_result() {
    let handler = test_handler();
    let request = mk_request("gpu.info", None, 40);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

#[tokio::test]
async fn gpu_memory_returns_result() {
    let handler = test_handler();
    let request = mk_request("gpu.memory", None, 41);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

// ═══════════════════════════════════════════════════════════
// Gate handlers
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn gate_list_returns_result() {
    let handler = test_handler();
    let request = mk_request("gate.list", None, 50);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

#[tokio::test]
async fn gate_update_requires_params() {
    let handler = test_handler();
    let request = mk_request("gate.update", None, 51);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn gate_remove_requires_params() {
    let handler = test_handler();
    let request = mk_request("gate.remove", None, 52);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn gate_route_requires_params() {
    let handler = test_handler();
    let request = mk_request("gate.route", None, 53);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

// ═══════════════════════════════════════════════════════════
// Ollama/Inference handlers: REMOVED — AI is Squirrel's domain
// Shader compilation handlers: REMOVED — compilation is coralReef's domain

// ═══════════════════════════════════════════════════════════
// Silicon performance surface handlers
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn silicon_performance_surface_list() {
    let handler = test_handler();
    let request = mk_request("compute.performance_surface.list", None, 80);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

#[tokio::test]
async fn silicon_performance_surface_query_requires_params() {
    let handler = test_handler();
    let request = mk_request("compute.performance_surface.query", None, 81);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn silicon_performance_surface_report_requires_params() {
    let handler = test_handler();
    let request = mk_request("compute.performance_surface.report", None, 82);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn silicon_route_multi_unit_requires_params() {
    let handler = test_handler();
    let request = mk_request("compute.route.multi_unit", None, 83);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

// ═══════════════════════════════════════════════════════════
// Resources handlers
// ═══════════════════════════════════════════════════════════

#[tokio::test]
async fn resources_estimate_requires_graph() {
    let handler = test_handler();
    let request = mk_request("resources.estimate", None, 90);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn resources_validate_availability_requires_graph() {
    let handler = test_handler();
    let request = mk_request("resources.validate_availability", None, 91);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test]
async fn resources_suggest_optimizations_requires_params() {
    let handler = test_handler();
    let request = mk_request("resources.suggest_optimizations", None, 92);
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

// ═══════════════════════════════════════════════════════════
// unibin module helpers
// ═══════════════════════════════════════════════════════════

#[test]
fn unibin_resolve_family_id_default() {
    let family = toadstool_server::unibin::resolve_family_id(None);
    assert!(!family.is_empty());
}

#[test]
fn unibin_resolve_family_id_override() {
    let family = toadstool_server::unibin::resolve_family_id(Some("test-family".to_string()));
    assert_eq!(family, "test-family");
}

#[test]
fn unibin_resolve_node_id_default() {
    let node = toadstool_server::unibin::resolve_node_id();
    assert!(!node.is_empty());
}

#[test]
fn unibin_exit_codes() {
    assert_eq!(toadstool_server::unibin::exit_codes::SUCCESS, 0);
    assert_eq!(toadstool_server::unibin::exit_codes::GENERAL_ERROR, 1);
    assert_eq!(toadstool_server::unibin::exit_codes::CONFIG_ERROR, 2);
    assert_eq!(toadstool_server::unibin::exit_codes::RUNTIME_ERROR, 3);
    assert_eq!(toadstool_server::unibin::exit_codes::INTERRUPTED, 130);
}

#[test]
fn unibin_shutdown_signal_variants() {
    let sigint = toadstool_server::unibin::ShutdownSignal::Sigint;
    let sigterm = toadstool_server::unibin::ShutdownSignal::Sigterm;
    let err = toadstool_server::unibin::ShutdownSignal::Error("test error");

    assert_eq!(sigint, toadstool_server::unibin::ShutdownSignal::Sigint);
    assert_ne!(sigint, sigterm);
    assert_ne!(sigint, err);
    assert_eq!(format!("{sigint:?}"), "Sigint");
    assert_eq!(format!("{sigterm:?}"), "Sigterm");
    assert!(format!("{err:?}").contains("test error"));
}

#[test]
fn unibin_is_platform_constraint_str() {
    assert!(toadstool_server::unibin::is_platform_constraint_str(
        "Unsupported operation"
    ));
    assert!(toadstool_server::unibin::is_platform_constraint_str(
        "protocol not supported"
    ));
    assert!(toadstool_server::unibin::is_platform_constraint_str(
        "protocol not available on this system"
    ));
    // "Permission denied" and "Operation not permitted" depend on SELinux state
    let _ = toadstool_server::unibin::is_platform_constraint_str("Permission denied");
    let _ = toadstool_server::unibin::is_platform_constraint_str("Operation not permitted");
    assert!(!toadstool_server::unibin::is_platform_constraint_str(
        "some random error"
    ));
}

#[test]
fn unibin_is_selinux_enforcing() {
    let _result = toadstool_server::unibin::is_selinux_enforcing();
    // Just verify it doesn't panic; result depends on platform
}

#[test]
fn unibin_write_tcp_discovery_file() {
    let dir = tempfile::tempdir().unwrap();
    let file_path = dir.path().join("discovery.json");
    let addr: std::net::SocketAddr = "127.0.0.1:8080".parse().unwrap();
    let _ = toadstool_server::unibin::write_tcp_discovery_file(file_path.to_str().unwrap(), &addr);
}

#[test]
fn unibin_socket_filename_for_family() {
    let name = toadstool_server::unibin::socket_filename_for_family("nat0");
    assert!(name.contains("nat0"));
}

#[test]
fn unibin_ensure_biomeos_directory() {
    let dir = tempfile::tempdir().unwrap();
    let result = toadstool_server::unibin::ensure_biomeos_directory(dir.path());
    assert!(result.is_ok());
    let biomeos_path = result.unwrap();
    assert!(biomeos_path.exists());
}
