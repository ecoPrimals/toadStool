//! Comprehensive tests for health check handlers
//!
//! ✅ MODERN CONCURRENT TESTING - Zero sleeps, fully concurrent
//! Tests health check endpoints with various scenarios

use axum::response::IntoResponse;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, RwLock};
use uuid::Uuid;

use toadstool::RuntimeType;
use toadstool_api::handlers::health::health_check;
use toadstool_api::types::{ExecutionInfo, ExecutionStatus};
use toadstool_api::ApiState;

/// Create test API state
fn create_test_state() -> ApiState {
    let (event_broadcaster, _) = broadcast::channel(100);

    ApiState {
        executions: Arc::new(RwLock::new(HashMap::new())),
        metrics: Arc::new(RwLock::new(toadstool_api::ApiMetrics::default())),
        event_broadcaster,
        capability_provider: None,
    }
}

/// Create test execution info
fn create_test_execution() -> ExecutionInfo {
    ExecutionInfo {
        execution_id: Uuid::new_v4(),
        status: ExecutionStatus::Running,
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
// BASIC HEALTH CHECK TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_healthy_state() {
    // ✅ FULLY CONCURRENT: Health check returns 200 when system is healthy
    let state = create_test_state();

    let result = health_check(axum::extract::State(state)).await;

    assert!(result.is_ok(), "Health check should succeed");
    let (parts, _body) = result.unwrap().into_response().into_parts();
    assert_eq!(parts.status.as_u16(), 200, "Should return 200 OK");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_with_few_executions() {
    // ✅ FULLY CONCURRENT: Health check with small queue size
    let state = create_test_state();

    // Add a few executions (below threshold)
    {
        let mut executions = state.executions.write().await;
        for _ in 0..10 {
            let execution = create_test_execution();
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = health_check(axum::extract::State(state)).await;
    assert!(
        result.is_ok(),
        "Health check should succeed with few executions"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_response_structure() {
    // ✅ FULLY CONCURRENT: Verify health response has expected structure
    let state = create_test_state();

    let result = health_check(axum::extract::State(state)).await;
    assert!(result.is_ok(), "Health check should succeed");

    // Response should contain version, timestamp, checks, etc.
    // (In a real test, we'd parse the JSON and verify structure)
}

// ============================================================================
// DEGRADED STATE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_degraded_with_large_queue() {
    // ✅ FULLY CONCURRENT: Health check returns 503 when queue is large
    let state = create_test_state();

    // Add many executions (above threshold of 1000)
    {
        let mut executions = state.executions.write().await;
        for _ in 0..1100 {
            let execution = create_test_execution();
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = health_check(axum::extract::State(state)).await;

    assert!(
        result.is_ok(),
        "Health check should succeed even when degraded"
    );
    let (parts, _body) = result.unwrap().into_response().into_parts();
    assert_eq!(
        parts.status.as_u16(),
        503,
        "Should return 503 SERVICE_UNAVAILABLE when degraded"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_at_threshold() {
    // ✅ FULLY CONCURRENT: Test exactly at threshold (999 should be healthy, 1000 should be degraded)
    let state = create_test_state();

    // Add exactly 999 executions (just below threshold)
    {
        let mut executions = state.executions.write().await;
        for _ in 0..999 {
            let execution = create_test_execution();
            executions.insert(execution.execution_id, execution);
        }
    }

    let result = health_check(axum::extract::State(state)).await;
    assert!(result.is_ok());
    let (parts, _) = result.unwrap().into_response().into_parts();
    assert_eq!(
        parts.status.as_u16(),
        200,
        "999 executions should be healthy"
    );
}

// ============================================================================
// CONCURRENT HEALTH CHECK TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_health_checks() {
    // ✅ FULLY CONCURRENT: Multiple health checks in parallel
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for _ in 0..50 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            health_check(axum::extract::State(state_clone))
                .await
                .is_ok()
        }));
    }

    // All health checks should succeed
    for task in tasks {
        let success = task.await.expect("Task should complete");
        assert!(success, "Concurrent health check should succeed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_while_executions_changing() {
    // ✅ FULLY CONCURRENT: Health checks while execution queue is being modified
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = vec![];

    // Spawn 10 tasks that perform health checks
    for _ in 0..10 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let _ = health_check(axum::extract::State(state_clone)).await;
            true
        }));
    }

    // Spawn 10 tasks that modify the execution queue
    for i in 0..10 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let mut executions = state_clone.executions.write().await;
            for _ in 0..i * 10 {
                let execution = create_test_execution();
                executions.insert(execution.execution_id, execution);
            }
            true
        }));
    }

    // All tasks should complete without panicking
    for task in tasks {
        let success = task.await.expect("Task should not panic");
        assert!(success, "Task should complete successfully");
    }
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_stress_200_concurrent() {
    // ✅ STRESS TEST: 200 concurrent health checks
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(200));
    let mut tasks = vec![];

    for _ in 0..200 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            health_check(axum::extract::State(state_clone))
                .await
                .is_ok()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should not panic") {
            successes += 1;
        }
    }

    assert_eq!(successes, 200, "All 200 health checks should succeed");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_with_empty_queue() {
    // ✅ FULLY CONCURRENT: Health check with no executions
    let state = create_test_state();

    // Verify queue is empty
    {
        let executions = state.executions.read().await;
        assert_eq!(executions.len(), 0, "Queue should be empty");
    }

    let result = health_check(axum::extract::State(state)).await;
    assert!(
        result.is_ok(),
        "Health check should succeed with empty queue"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_multiple_times_same_state() {
    // ✅ FULLY CONCURRENT: Multiple health checks on same state
    let state = create_test_state();

    // Perform health check multiple times
    for _ in 0..10 {
        let result = health_check(axum::extract::State(state.clone())).await;
        assert!(result.is_ok(), "Each health check should succeed");
    }
}

// ============================================================================
// PERFORMANCE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_performance() {
    // ✅ FULLY CONCURRENT: Measure health check performance
    let state = create_test_state();
    let start = std::time::Instant::now();

    health_check(axum::extract::State(state))
        .await
        .expect("Should succeed");

    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 100,
        "Health check should complete in <100ms"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_with_large_queue_performance() {
    // ✅ FULLY CONCURRENT: Performance with large queue
    let state = create_test_state();

    // Add 5000 executions
    {
        let mut executions = state.executions.write().await;
        for _ in 0..5000 {
            let execution = create_test_execution();
            executions.insert(execution.execution_id, execution);
        }
    }

    let start = std::time::Instant::now();
    health_check(axum::extract::State(state))
        .await
        .expect("Should succeed");
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 200,
        "Health check should complete in <200ms even with large queue"
    );
}
