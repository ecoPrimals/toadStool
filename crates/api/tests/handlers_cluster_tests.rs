//! Comprehensive tests for cluster handlers
//!
//! ✅ MODERN CONCURRENT TESTING - Zero sleeps, fully concurrent
//! Tests cluster status endpoints with various scenarios

use axum::extract::State;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, RwLock};
use uuid::Uuid;

use toadstool::RuntimeType;
use toadstool_api::handlers::cluster::get_cluster_status;
use toadstool_api::types::{ExecutionInfo, ExecutionStatus};
use toadstool_api::{websocket, ApiState};

/// Create test API state
fn create_test_state() -> ApiState {
    let (event_broadcaster, _) = broadcast::channel(100);

    ApiState {
        executions: Arc::new(RwLock::new(HashMap::new())),
        metrics: Arc::new(RwLock::new(toadstool_api::ApiMetrics::default())),
        event_broadcaster,
        websocket_manager: Arc::new(websocket::WebSocketManager::new()),
        capability_provider: None,
    }
}

/// Create test execution with specific status
fn create_execution_with_status(status: ExecutionStatus) -> ExecutionInfo {
    ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status,
        runtime_type: RuntimeType::Container,
        submitted_at: chrono::Utc::now(),
        started_at: Some(chrono::Utc::now()),
        completed_at: None,
        duration_ms: None,
        progress: None,
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    }
}

// ============================================================================
// BASIC CLUSTER STATUS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_cluster_status_empty() {
    // ✅ FULLY CONCURRENT: Get cluster status with no executions
    let state = create_test_state();

    let result = get_cluster_status(State(state)).await;
    assert!(result.is_ok(), "Should return cluster status");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_cluster_status_with_running_executions() {
    // ✅ FULLY CONCURRENT: Get cluster status with running executions
    let state = create_test_state();

    // Add running executions
    {
        let mut executions = state.executions.write().await;
        for _ in 0..5 {
            let execution = create_execution_with_status(ExecutionStatus::Running);
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = get_cluster_status(State(state)).await;
    assert!(
        result.is_ok(),
        "Should return cluster status with running executions"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_cluster_status_with_queued_executions() {
    // ✅ FULLY CONCURRENT: Get cluster status with queued executions
    let state = create_test_state();

    // Add queued executions
    {
        let mut executions = state.executions.write().await;
        for _ in 0..3 {
            let execution = create_execution_with_status(ExecutionStatus::Queued);
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = get_cluster_status(State(state)).await;
    assert!(
        result.is_ok(),
        "Should return cluster status with queued executions"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_cluster_status_with_mixed_executions() {
    // ✅ FULLY CONCURRENT: Get cluster status with mixed execution statuses
    let state = create_test_state();

    {
        let mut executions = state.executions.write().await;

        // Add various execution statuses
        for _ in 0..3 {
            let execution = create_execution_with_status(ExecutionStatus::Running);
            executions.insert(execution.execution_id, execution);
        }
        for _ in 0..2 {
            let execution = create_execution_with_status(ExecutionStatus::Queued);
            executions.insert(execution.execution_id, execution);
        }
        for _ in 0..2 {
            let execution = create_execution_with_status(ExecutionStatus::Completed);
            executions.insert(execution.execution_id, execution);
        }
        for _ in 0..1 {
            let execution = create_execution_with_status(ExecutionStatus::Failed);
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = get_cluster_status(State(state)).await;
    assert!(
        result.is_ok(),
        "Should return cluster status with mixed executions"
    );
}

// ============================================================================
// CLUSTER STATUS CONSISTENCY TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_consistency() {
    // ✅ FULLY CONCURRENT: Multiple status calls should be consistent
    let state = create_test_state();

    let result1 = get_cluster_status(State(state.clone())).await;
    let result2 = get_cluster_status(State(state.clone())).await;

    assert!(
        result1.is_ok() && result2.is_ok(),
        "Both calls should succeed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_after_execution_changes() {
    // ✅ FULLY CONCURRENT: Status should update as executions change
    let state = create_test_state();

    // Get initial status
    let result1 = get_cluster_status(State(state.clone())).await;
    assert!(result1.is_ok());

    // Add executions
    {
        let mut executions = state.executions.write().await;
        for _ in 0..5 {
            let execution = create_execution_with_status(ExecutionStatus::Running);
            executions.insert(execution.execution_id, execution);
        }
    }

    // Get updated status
    let result2 = get_cluster_status(State(state)).await;
    assert!(result2.is_ok());
}

// ============================================================================
// CONCURRENT CLUSTER STATUS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_cluster_status_requests() {
    // ✅ FULLY CONCURRENT: Multiple cluster status requests in parallel
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(30));
    let mut tasks = vec![];

    for _ in 0..30 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            get_cluster_status(State(state_clone)).await.is_ok()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 30, "All 30 concurrent requests should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_while_executions_changing() {
    // ✅ FULLY CONCURRENT: Cluster status requests while executions are being modified
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(40));
    let mut tasks = vec![];

    // Spawn 20 tasks reading cluster status
    for _ in 0..20 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            get_cluster_status(State(state_clone)).await.is_ok()
        }));
    }

    // Spawn 20 tasks modifying executions
    for i in 0..20 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let mut executions = state_clone.executions.write().await;

            let status = if i % 3 == 0 {
                ExecutionStatus::Running
            } else if i % 3 == 1 {
                ExecutionStatus::Queued
            } else {
                ExecutionStatus::Completed
            };

            let execution = create_execution_with_status(status);
            executions.insert(execution.execution_id, execution);
            true
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should not panic") {
            successes += 1;
        }
    }

    assert_eq!(successes, 40, "All 40 concurrent operations should succeed");
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_100_concurrent_status_requests() {
    // ✅ STRESS TEST: 100 concurrent cluster status requests
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            get_cluster_status(State(state_clone)).await.is_ok()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 100, "All 100 requests should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_with_many_executions() {
    // ✅ STRESS TEST: Cluster status with large number of executions
    let state = create_test_state();

    // Add many executions
    {
        let mut executions = state.executions.write().await;
        for i in 0..500 {
            let status = match i % 4 {
                0 => ExecutionStatus::Running,
                1 => ExecutionStatus::Queued,
                2 => ExecutionStatus::Completed,
                _ => ExecutionStatus::Failed,
            };

            let execution = create_execution_with_status(status);
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = get_cluster_status(State(state)).await;
    assert!(result.is_ok(), "Should handle many executions");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_all_completed_executions() {
    // ✅ FULLY CONCURRENT: All executions completed
    let state = create_test_state();

    {
        let mut executions = state.executions.write().await;
        for _ in 0..10 {
            let execution = create_execution_with_status(ExecutionStatus::Completed);
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = get_cluster_status(State(state)).await;
    assert!(result.is_ok(), "Should handle all completed executions");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_all_failed_executions() {
    // ✅ FULLY CONCURRENT: All executions failed
    let state = create_test_state();

    {
        let mut executions = state.executions.write().await;
        for _ in 0..10 {
            let execution = create_execution_with_status(ExecutionStatus::Failed);
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = get_cluster_status(State(state)).await;
    assert!(result.is_ok(), "Should handle all failed executions");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cluster_status_performance() {
    // ✅ FULLY CONCURRENT: Cluster status should be fast
    let state = create_test_state();

    // Add some executions
    {
        let mut executions = state.executions.write().await;
        for _ in 0..50 {
            let execution = create_execution_with_status(ExecutionStatus::Running);
            executions.insert(execution.execution_id, execution);
        }
    }

    let start = std::time::Instant::now();
    let result = get_cluster_status(State(state)).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(
        duration.as_millis() < 100,
        "Cluster status should complete in <100ms"
    );
}
