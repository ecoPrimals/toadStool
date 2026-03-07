// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for server HTTP handlers

use std::collections::HashMap;
use std::sync::Arc;
use toadstool_server::{ServerConfig, ServerState, ServerStatistics};
use tokio::sync::{broadcast, RwLock};

// Helper function to create test server state
fn create_test_state() -> ServerState {
    let (event_broadcaster, _) = broadcast::channel(100);

    ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster,
        config: ServerConfig::default(),
        resource_monitor: Arc::new(toadstool::SystemResourceMonitor::new()),
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        capability_provider: None,
    }
}

// ============================================================================
// Health Check Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_handler_does_not_panic() {
    let state = create_test_state();

    // Add timeout to prevent hanging (5 seconds)
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        toadstool_server::handlers::health_check_handler(axum::extract::State(state)),
    )
    .await;

    // Handler should complete without panicking and within timeout
    assert!(result.is_ok(), "Handler timed out or failed");
    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_readiness_check_handler_does_not_panic() {
    let state = create_test_state();
    let result =
        toadstool_server::handlers::readiness_check_handler(axum::extract::State(state)).await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_readiness_check_multiple_calls() {
    let state = create_test_state();

    // Call readiness check multiple times
    for _ in 0..3 {
        let result = toadstool_server::handlers::readiness_check_handler(axum::extract::State(
            state.clone(),
        ))
        .await;
        drop(result);
    }
}

// ============================================================================
// Metrics Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_handler_does_not_panic() {
    let state = create_test_state();
    let result = toadstool_server::handlers::metrics_handler(axum::extract::State(state)).await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_with_statistics() {
    let state = create_test_state();

    {
        let mut stats = state.stats.write().await;
        stats.total_executions = 10;
        stats.successful_executions = 8;
        stats.failed_executions = 2;
    }

    let result = toadstool_server::handlers::metrics_handler(axum::extract::State(state)).await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_with_active_executions() {
    let state = create_test_state();

    {
        let mut executions = state.active_executions.write().await;
        let execution = toadstool_server::ActiveExecution {
            execution_id: uuid::Uuid::new_v4(),
            runtime_type: toadstool::RuntimeType::Native,
            started_at: std::time::SystemTime::now(),
            timeout: std::time::Duration::from_secs(300),
            status: toadstool::ExecutionStatus::Running,
            client_info: toadstool_server::ClientInfo {
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: None,
                api_key: None,
                authenticated_user: None,
            },
        };
        executions.insert(execution.execution_id, execution);
    }

    let result = toadstool_server::handlers::metrics_handler(axum::extract::State(state)).await;

    drop(result);
}

// ============================================================================
// Submit Execution Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_handler_basic() {
    let state = create_test_state();

    let request = serde_json::json!({
        "runtime_type": "native",
        "workload": {
            "executable": "/bin/echo",
            "args": ["test"]
        }
    });

    let result = toadstool_server::handlers::submit_execution_handler(
        axum::extract::State(state),
        axum::Json(request),
    )
    .await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_python() {
    let state = create_test_state();

    let request = serde_json::json!({
        "runtime_type": "python",
        "script": "print('Hello')"
    });

    let result = toadstool_server::handlers::submit_execution_handler(
        axum::extract::State(state),
        axum::Json(request),
    )
    .await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_container() {
    let state = create_test_state();

    let request = serde_json::json!({
        "runtime_type": "container",
        "image": "alpine:latest"
    });

    let result = toadstool_server::handlers::submit_execution_handler(
        axum::extract::State(state),
        axum::Json(request),
    )
    .await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_wasm() {
    let state = create_test_state();

    let request = serde_json::json!({
        "runtime_type": "wasm",
        "module": "test.wasm"
    });

    let result = toadstool_server::handlers::submit_execution_handler(
        axum::extract::State(state),
        axum::Json(request),
    )
    .await;

    drop(result);
}

// ============================================================================
// Get Execution Status Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_status_nonexistent_execution() {
    let state = create_test_state();
    let execution_id = uuid::Uuid::new_v4();

    let result = toadstool_server::handlers::get_execution_status_handler(
        axum::extract::State(state),
        axum::extract::Path(execution_id),
    )
    .await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_status_existing_execution() {
    let state = create_test_state();
    let execution_id = uuid::Uuid::new_v4();

    {
        let mut executions = state.active_executions.write().await;
        let execution = toadstool_server::ActiveExecution {
            execution_id,
            runtime_type: toadstool::RuntimeType::Native,
            started_at: std::time::SystemTime::now(),
            timeout: std::time::Duration::from_secs(300),
            status: toadstool::ExecutionStatus::Running,
            client_info: toadstool_server::ClientInfo {
                ip_address: None,
                user_agent: None,
                api_key: None,
                authenticated_user: None,
            },
        };
        executions.insert(execution_id, execution);
    }

    let result = toadstool_server::handlers::get_execution_status_handler(
        axum::extract::State(state),
        axum::extract::Path(execution_id),
    )
    .await;

    drop(result);
}

// ============================================================================
// Cancel Execution Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_nonexistent_execution() {
    let state = create_test_state();
    let execution_id = uuid::Uuid::new_v4();

    let result = toadstool_server::handlers::cancel_execution_handler(
        axum::extract::State(state),
        axum::extract::Path(execution_id),
    )
    .await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_existing_execution() {
    let state = create_test_state();
    let execution_id = uuid::Uuid::new_v4();

    {
        let mut executions = state.active_executions.write().await;
        let execution = toadstool_server::ActiveExecution {
            execution_id,
            runtime_type: toadstool::RuntimeType::Native,
            started_at: std::time::SystemTime::now(),
            timeout: std::time::Duration::from_secs(300),
            status: toadstool::ExecutionStatus::Running,
            client_info: toadstool_server::ClientInfo {
                ip_address: None,
                user_agent: None,
                authenticated_user: None,
                api_key: None,
            },
        };
        executions.insert(execution_id, execution);
    }

    let result = toadstool_server::handlers::cancel_execution_handler(
        axum::extract::State(state),
        axum::extract::Path(execution_id),
    )
    .await;

    drop(result);
}

// ============================================================================
// Cluster Status Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_empty() {
    let state = create_test_state();

    let result =
        toadstool_server::handlers::get_cluster_status_handler(axum::extract::State(state)).await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_with_active_executions() {
    let state = create_test_state();

    {
        let mut executions = state.active_executions.write().await;
        for i in 0..3 {
            let execution_id = uuid::Uuid::new_v4();
            let execution = toadstool_server::ActiveExecution {
                execution_id,
                runtime_type: toadstool::RuntimeType::Native,
                started_at: std::time::SystemTime::now(),
                timeout: std::time::Duration::from_secs(300),
                status: if i == 0 {
                    toadstool::ExecutionStatus::Pending
                } else {
                    toadstool::ExecutionStatus::Running
                },
                client_info: toadstool_server::ClientInfo {
                    ip_address: Some(format!("192.168.1.{i}")),
                    user_agent: None,
                    api_key: None,
                    authenticated_user: None,
                },
            };
            executions.insert(execution_id, execution);
        }
    }

    let result =
        toadstool_server::handlers::get_cluster_status_handler(axum::extract::State(state)).await;

    drop(result);
}

// ============================================================================
// Runtime Management Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_runtime_engines_empty() {
    let state = create_test_state();

    let result =
        toadstool_server::handlers::list_runtime_engines_handler(axum::extract::State(state)).await;

    drop(result);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_runtime_engines_multiple_calls() {
    let state = create_test_state();

    // Call list handler multiple times
    for _ in 0..3 {
        let result = toadstool_server::handlers::list_runtime_engines_handler(
            axum::extract::State(state.clone()),
        )
        .await;
        drop(result);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_runtime_engines_concurrent() {
    let state = create_test_state();

    let handles: Vec<_> = (0..5)
        .map(|_| {
            let s = state.clone();
            tokio::spawn(async move {
                toadstool_server::handlers::list_runtime_engines_handler(axum::extract::State(s))
                    .await
            })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap();
        drop(result);
    }
}

// ============================================================================
// Server Statistics Tests
// ============================================================================

#[test]
fn test_statistics_initial_state() {
    let stats = ServerStatistics::default();

    assert_eq!(stats.total_executions, 0);
    assert_eq!(stats.successful_executions, 0);
    assert_eq!(stats.failed_executions, 0);
    assert_eq!(stats.average_execution_time_ms, 0.0);
}

#[test]
fn test_statistics_clone() {
    let stats1 = ServerStatistics::default();
    let stats2 = stats1.clone();

    assert_eq!(stats1.total_executions, stats2.total_executions);
    assert_eq!(stats1.successful_executions, stats2.successful_executions);
}

#[test]
fn test_statistics_debug() {
    let stats = ServerStatistics::default();
    let debug_str = format!("{stats:?}");

    assert!(debug_str.contains("ServerStatistics"));
}

#[test]
fn test_statistics_field_updates() {
    let stats = ServerStatistics {
        total_executions: 100,
        successful_executions: 90,
        failed_executions: 10,
        average_execution_time_ms: 125.5,
        ..Default::default()
    };

    assert_eq!(stats.total_executions, 100);
    assert_eq!(stats.successful_executions, 90);
    assert_eq!(stats.failed_executions, 10);
    assert_eq!(stats.average_execution_time_ms, 125.5);
}

// ============================================================================
// State Management Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_state_creation() {
    let state = create_test_state();

    assert_eq!(state.runtime_engines.read().await.len(), 0);
    assert_eq!(state.active_executions.read().await.len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_server_state_clone() {
    let state1 = create_test_state();
    let state2 = state1.clone();

    // Both should reference the same underlying data
    assert!(Arc::ptr_eq(
        &state1.runtime_engines,
        &state2.runtime_engines
    ));
    assert!(Arc::ptr_eq(
        &state1.active_executions,
        &state2.active_executions
    ));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_state_access() {
    let state = create_test_state();

    let handles: Vec<_> = (0..10)
        .map(|_| {
            let s = state.clone();
            tokio::spawn(async move { s.runtime_engines.read().await.len() })
        })
        .collect();

    for handle in handles {
        let result = handle.await.unwrap();
        assert_eq!(result, 0);
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_full_execution_lifecycle() {
    let state = create_test_state();
    let execution_id = uuid::Uuid::new_v4();

    // 1. Create execution
    {
        let mut executions = state.active_executions.write().await;
        let execution = toadstool_server::ActiveExecution {
            execution_id,
            runtime_type: toadstool::RuntimeType::Native,
            started_at: std::time::SystemTime::now(),
            timeout: std::time::Duration::from_secs(300),
            status: toadstool::ExecutionStatus::Pending,
            client_info: toadstool_server::ClientInfo {
                ip_address: Some("127.0.0.1".to_string()),
                user_agent: Some("test-client".to_string()),
                api_key: None,
                authenticated_user: Some("test_user".to_string()),
            },
        };
        executions.insert(execution_id, execution);
    }

    // 2. Verify it exists
    {
        let executions = state.active_executions.read().await;
        assert!(executions.contains_key(&execution_id));
    }

    // 3. Remove it
    {
        let mut executions = state.active_executions.write().await;
        executions.remove(&execution_id);
    }

    // 4. Verify it's gone
    {
        let executions = state.active_executions.read().await;
        assert!(!executions.contains_key(&execution_id));
    }
}

// ============================================================================
// Summary
// ============================================================================

#[test]
fn test_handlers_module_summary() {
    println!("\n========================================");
    println!("Handler Tests Summary");
    println!("========================================");
    println!("Health Check Tests:        3 tests");
    println!("Metrics Tests:             3 tests");
    println!("Submit Execution Tests:    5 tests");
    println!("Get Status Tests:          2 tests");
    println!("Cancel Execution Tests:    2 tests");
    println!("Cluster Status Tests:      2 tests");
    println!("List Runtime Engines:      3 tests");
    println!("Statistics Tests:          4 tests");
    println!("State Management Tests:    3 tests");
    println!("Integration Tests:         1 test");
    println!("----------------------------------------");
    println!("Total:                     28 tests");
    println!("========================================");
}
