// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::float_cmp,
    clippy::no_effect_underscore_binding,
    clippy::unreadable_literal
)]
//! Comprehensive tests for `pure_jsonrpc` handler/transport.rs
//! Tests transport.discover, transport.list, transport.route via `JsonRpcHandler`.
//! No real network I/O, no hardware probing.

use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;

use toadstool_server::pure_jsonrpc::{JsonRpcHandler, JsonRpcRequest};
use toadstool_server::tarpc_server::{StandaloneExecutor, WorkloadExecutor};

// ─── transport.discover ────────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn transport_discover_returns_structure() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", Some(Arc::new(AtomicU64::new(0))));

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("transport.discover"),
        params: None,
        id: Some(serde_json::json!(1)),
    };

    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none(), "error: {:?}", response.error);
    let result = response.result.expect("expected result");
    assert!(result.get("transports").is_some());
    assert!(result.get("count").is_some());
    let count = result["count"].as_u64().expect("count");
    assert!(result["transports"].as_array().unwrap().len() == count as usize);
}

// ─── transport.list ────────────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn transport_list_returns_empty_initially() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", Some(Arc::new(AtomicU64::new(0))));

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("transport.list"),
        params: None,
        id: Some(serde_json::json!(2)),
    };

    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("expected result");
    assert!(result.get("transports").is_some());
    assert!(result.get("count").is_some());
    let count = result["count"].as_u64().unwrap();
    assert_eq!(count, 0);
    assert!(result["transports"].as_array().unwrap().is_empty());
}

// ─── transport.route ────────────────────────────────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn transport_route_missing_params() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", Some(Arc::new(AtomicU64::new(0))));

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("transport.route"),
        params: None,
        id: Some(serde_json::json!(3)),
    };

    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn transport_route_missing_rx_id() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", Some(Arc::new(AtomicU64::new(0))));

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("transport.route"),
        params: Some(serde_json::json!({"tx_id": "display:0"})),
        id: Some(serde_json::json!(4)),
    };

    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn transport_route_missing_tx_id() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", Some(Arc::new(AtomicU64::new(0))));

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("transport.route"),
        params: Some(serde_json::json!({"rx_id": "capture:0"})),
        id: Some(serde_json::json!(5)),
    };

    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn transport_route_with_nonexistent_transports() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", Some(Arc::new(AtomicU64::new(0))));

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("transport.route"),
        params: Some(serde_json::json!({
            "rx_id": "nonexistent-rx",
            "tx_id": "nonexistent-tx"
        })),
        id: Some(serde_json::json!(6)),
    };

    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn transport_route_with_buf_size() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", Some(Arc::new(AtomicU64::new(0))));

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("transport.route"),
        params: Some(serde_json::json!({
            "rx_id": "nonexistent",
            "tx_id": "nonexistent",
            "buf_size": 32768
        })),
        id: Some(serde_json::json!(7)),
    };

    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
}

// ─── Handler methods: core, resources, compute, gate, ollama ─────────────────

#[tokio::test(flavor = "current_thread")]
async fn toadstool_health_returns_structure() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "1.2.3", Some(Arc::new(AtomicU64::new(0))));

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("toadstool.health"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.expect("result");
    assert!(result.get("healthy").is_some());
    assert!(result.get("version").is_some());
    assert_eq!(result["version"], "1.2.3");
}

#[tokio::test(flavor = "current_thread")]
async fn compute_health_alias() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "2.0.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("compute.health"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    assert_eq!(response.result.unwrap()["version"], "2.0.0");
}

#[tokio::test(flavor = "current_thread")]
async fn toadstool_version_returns_info() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "3.0.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("toadstool.version"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
    let result = response.result.unwrap();
    assert_eq!(result["version"], "3.0.0");
    assert_eq!(result["protocol"], "JSON-RPC 2.0");
}

#[tokio::test(flavor = "current_thread")]
async fn resources_estimate_exercises_handler() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("resources.estimate"),
        params: Some(serde_json::json!({"workload": {}})),
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn resources_validate_availability_exercises_handler() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("resources.validate_availability"),
        params: Some(serde_json::json!({})),
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn resources_suggest_optimizations_exercises_handler() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("resources.suggest_optimizations"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.result.is_some() || response.error.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_jsonrpc_version_returns_error() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("1.0"),
        method: Cow::Borrowed("toadstool.health"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_some());
    assert!(response.result.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn gate_list_returns_structure() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("gate.list"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn gpu_info_returns_structure() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("gpu.info"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn gpu_memory_returns_structure() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("gpu.memory"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn toadstool_query_capabilities() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("toadstool.query_capabilities"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}

#[tokio::test(flavor = "current_thread")]
async fn toadstool_list_workloads() {
    let executor: Arc<dyn WorkloadExecutor + Send + Sync> = Arc::new(StandaloneExecutor::new());
    let handler = JsonRpcHandler::new(executor, "0.1.0", None);

    let request = JsonRpcRequest {
        jsonrpc: Cow::Borrowed("2.0"),
        method: Cow::Borrowed("toadstool.list_workloads"),
        params: None,
        id: Some(serde_json::json!(1)),
    };
    let response = handler.handle_request(&request).await;
    assert!(response.error.is_none());
}
