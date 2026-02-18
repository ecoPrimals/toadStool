//! Comprehensive tests for metrics handlers
//!
//! ✅ MODERN CONCURRENT TESTING - Zero sleeps, fully concurrent
//! Tests metrics endpoints with various scenarios

use axum::extract::{Path, Query, State};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, RwLock};
use uuid::Uuid;

use toadstool::RuntimeType;
use toadstool_api::handlers::metrics::{get_api_metrics, get_execution_metrics};
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
fn create_test_execution(id: Uuid) -> ExecutionInfo {
    ExecutionInfo {
        execution_id: id,
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
// GET EXECUTION METRICS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_metrics_success() {
    // ✅ FULLY CONCURRENT: Get metrics for existing execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    // Add execution
    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    let params: HashMap<String, String> = HashMap::new();
    let result = get_execution_metrics(State(state), Path(execution_id), Query(params)).await;

    assert!(
        result.is_ok(),
        "Should return metrics for existing execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_metrics_not_found() {
    // ✅ FULLY CONCURRENT: Get metrics for non-existent execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let params: HashMap<String, String> = HashMap::new();
    let result = get_execution_metrics(State(state), Path(execution_id), Query(params)).await;

    assert!(
        result.is_err(),
        "Should return error for non-existent execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_metrics_with_time_range() {
    // ✅ FULLY CONCURRENT: Get metrics with custom time range
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    // Add execution
    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    // Add time range parameters
    let now = chrono::Utc::now();
    let mut params: HashMap<String, String> = HashMap::new();
    params.insert(
        "start".to_string(),
        (now - chrono::Duration::hours(2)).to_rfc3339(),
    );
    params.insert("end".to_string(), now.to_rfc3339());

    let result = get_execution_metrics(State(state), Path(execution_id), Query(params)).await;

    assert!(result.is_ok(), "Should accept valid time range");
}

// ============================================================================
// GET API METRICS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_api_metrics_default() {
    // ✅ FULLY CONCURRENT: Get API metrics with default state
    let state = create_test_state();

    let result = get_api_metrics(State(state)).await;
    assert!(result.is_ok(), "Should return API metrics");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_api_metrics_with_executions() {
    // ✅ FULLY CONCURRENT: Get API metrics with active executions
    let state = create_test_state();

    // Add some executions
    {
        let mut executions = state.executions.write().await;
        for _ in 0..5 {
            let id = Uuid::new_v4();
            executions.insert(id, create_test_execution(id));
        }
    }

    // Update metrics
    {
        let mut metrics = state.metrics.write().await;
        metrics.total_requests = 100;
        metrics.successful_requests = 95;
        metrics.failed_requests = 5;
        metrics.average_response_time_ms = 125.5;
    }

    let result = get_api_metrics(State(state)).await;
    assert!(result.is_ok(), "Should return API metrics with data");
}

// ============================================================================
// CONCURRENT METRICS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_api_metrics_requests() {
    // ✅ FULLY CONCURRENT: Multiple API metrics requests in parallel
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for _ in 0..50 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            get_api_metrics(State(state_clone)).await.is_ok()
        }));
    }

    // All requests should succeed
    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(
        successes, 50,
        "All 50 concurrent metrics requests should succeed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_execution_metrics_requests() {
    // ✅ FULLY CONCURRENT: Multiple execution metrics requests in parallel
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    // Add execution
    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    let barrier = Arc::new(Barrier::new(30));
    let mut tasks = vec![];

    for _ in 0..30 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);
        let exec_id = execution_id;

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let params: HashMap<String, String> = HashMap::new();
            get_execution_metrics(State(state_clone), Path(exec_id), Query(params))
                .await
                .is_ok()
        }));
    }

    // All requests should succeed
    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(
        successes, 30,
        "All 30 concurrent metrics requests should succeed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metrics_while_state_updating() {
    // ✅ FULLY CONCURRENT: Metrics requests while state is being updated
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(40));
    let mut tasks = vec![];

    // Spawn 20 tasks reading API metrics
    for _ in 0..20 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            get_api_metrics(State(state_clone)).await.is_ok()
        }));
    }

    // Spawn 20 tasks updating metrics
    for i in 0..20 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let mut metrics = state_clone.metrics.write().await;
            metrics.total_requests += i;
            metrics.successful_requests += i;
            true
        }));
    }

    // All tasks should complete
    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should not panic") {
            successes += 1;
        }
    }

    assert_eq!(successes, 40, "All 40 concurrent operations should succeed");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_metrics_for_multiple_executions() {
    // ✅ FULLY CONCURRENT: Get metrics for different executions
    let state = create_test_state();
    let mut execution_ids = vec![];

    // Create 10 executions
    {
        let mut executions = state.executions.write().await;
        for _ in 0..10 {
            let id = Uuid::new_v4();
            executions.insert(id, create_test_execution(id));
            execution_ids.push(id);
        }
    }

    // Get metrics for each execution
    for execution_id in execution_ids {
        let params: HashMap<String, String> = HashMap::new();
        let result =
            get_execution_metrics(State(state.clone()), Path(execution_id), Query(params)).await;

        assert!(result.is_ok(), "Should get metrics for each execution");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_api_metrics_with_zero_requests() {
    // ✅ FULLY CONCURRENT: API metrics with no requests
    let state = create_test_state();

    let result = get_api_metrics(State(state.clone())).await;
    assert!(result.is_ok());

    // Verify zero metrics
    let metrics = state.metrics.read().await;
    assert_eq!(metrics.total_requests, 0);
    assert_eq!(metrics.successful_requests, 0);
    assert_eq!(metrics.failed_requests, 0);
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_100_concurrent_metrics_requests() {
    // ✅ STRESS TEST: 100 concurrent API metrics requests
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            get_api_metrics(State(state_clone)).await.is_ok()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 100, "All 100 concurrent requests should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_mixed_metrics_operations() {
    // ✅ STRESS TEST: Mix of API and execution metrics requests
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    // Add execution
    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for i in 0..100 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);
        let exec_id = execution_id;

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            if i % 2 == 0 {
                // API metrics
                get_api_metrics(State(state_clone)).await.is_ok()
            } else {
                // Execution metrics
                let params: HashMap<String, String> = HashMap::new();
                get_execution_metrics(State(state_clone), Path(exec_id), Query(params))
                    .await
                    .is_ok()
            }
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 100, "All 100 mixed requests should succeed");
}
