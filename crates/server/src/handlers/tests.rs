//! Handler unit tests

use super::*;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use crate::ServerState;
use toadstool_common::constants::timeouts::WORKLOAD_EXECUTION_TIMEOUT;

fn create_test_state() -> ServerState {
    let (event_broadcaster, _) = broadcast::channel(100);
    ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster,
        config: crate::ServerConfig::default(),
        resource_monitor: Arc::new(toadstool::SystemResourceMonitor::new()),
        stats: Arc::new(RwLock::new(crate::ServerStatistics::default())),
        capability_provider: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_handler() {
    let state = create_test_state();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        health_check_handler(State(state)),
    )
    .await;
    assert!(result.is_ok(), "Health check handler timed out");
    let response = result.expect("ok").into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_readiness_check_handler_not_ready() {
    let state = create_test_state();
    let response = readiness_check_handler(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_handler() {
    let state = create_test_state();
    let response = metrics_handler(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_handler() {
    let state = create_test_state();
    let request = serde_json::json!({"runtime_type": "native", "workload": "test"});
    let response = submit_execution_handler(State(state), Json(request))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_status_handler_not_found() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    let response = get_execution_status_handler(State(state), Path(execution_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_status_handler_found() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    {
        let mut executions = state.active_executions.write().await;
        executions.insert(
            execution_id,
            crate::ActiveExecution {
                execution_id,
                runtime_type: toadstool::RuntimeType::Native,
                started_at: std::time::SystemTime::now(),
                timeout: WORKLOAD_EXECUTION_TIMEOUT,
                status: toadstool::ExecutionStatus::Running,
                client_info: crate::ClientInfo {
                    ip_address: None,
                    user_agent: None,
                    api_key: None,
                    authenticated_user: None,
                },
            },
        );
    }
    let response = get_execution_status_handler(State(state), Path(execution_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_handler_not_found() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    let response = cancel_execution_handler(State(state), Path(execution_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_handler_success() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    {
        let mut executions = state.active_executions.write().await;
        executions.insert(
            execution_id,
            crate::ActiveExecution {
                execution_id,
                runtime_type: toadstool::RuntimeType::Native,
                started_at: std::time::SystemTime::now(),
                timeout: WORKLOAD_EXECUTION_TIMEOUT,
                status: toadstool::ExecutionStatus::Running,
                client_info: crate::ClientInfo {
                    ip_address: None,
                    user_agent: None,
                    api_key: None,
                    authenticated_user: None,
                },
            },
        );
    }
    let response = cancel_execution_handler(State(state.clone()), Path(execution_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
    let executions = state.active_executions.read().await;
    let execution = executions.get(&execution_id).expect("exists");
    assert!(matches!(
        execution.status,
        toadstool::ExecutionStatus::Cancelled
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_cluster_status_handler() {
    let state = create_test_state();
    let response = get_cluster_status_handler(State(state))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_runtime_engines_handler_empty() {
    let state = create_test_state();
    let response = list_runtime_engines_handler(State(state))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_dashboard_html_constant() {
    assert!(DASHBOARD_HTML.contains("ToadStool Server Dashboard"));
    assert!(DASHBOARD_HTML.contains("System Health"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_dashboard_handler() {
    let response = dashboard_handler().await.into_response();
    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_handler_already_completed() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    {
        let mut executions = state.active_executions.write().await;
        executions.insert(
            execution_id,
            crate::ActiveExecution {
                execution_id,
                runtime_type: toadstool::RuntimeType::Native,
                started_at: std::time::SystemTime::now(),
                timeout: WORKLOAD_EXECUTION_TIMEOUT,
                status: toadstool::ExecutionStatus::Success,
                client_info: crate::ClientInfo {
                    ip_address: None,
                    user_agent: None,
                    api_key: None,
                    authenticated_user: None,
                },
            },
        );
    }
    let response = cancel_execution_handler(State(state), Path(execution_id))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_handler_invalid_json() {
    let state = create_test_state();
    let request = serde_json::json!({"runtime_type": "wasm", "workload": "test.wasm"});
    let response = submit_execution_handler(State(state), Json(request))
        .await
        .into_response();
    assert_eq!(response.status(), StatusCode::ACCEPTED);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_readiness_check_handler_with_engines() {
    let (event_broadcaster, _) = broadcast::channel(100);
    let state = ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster,
        config: crate::ServerConfig::default(),
        resource_monitor: Arc::new(toadstool::SystemResourceMonitor::new()),
        stats: Arc::new(RwLock::new(crate::ServerStatistics::default())),
        capability_provider: None,
    };
    {
        let mut engines = state.runtime_engines.write().await;
        engines.insert(
            toadstool::RuntimeType::Native,
            Box::new(toadstool_testing::mocks::MockRuntimeEngine::new()),
        );
    }
    let response = readiness_check_handler(State(state)).await.into_response();
    assert_eq!(response.status(), StatusCode::OK);
}
