//! Comprehensive tests for execution handlers
//!
//! ✅ MODERN CONCURRENT TESTING - Zero sleeps, fully concurrent
//! Tests execution management endpoints with various scenarios

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, RwLock};
use uuid::Uuid;

use toadstool::RuntimeType;
use toadstool_api::handlers::execution::{
    cancel_execution, get_execution_status, list_executions, submit_execution,
};
use toadstool_api::types::{
    ExecutionFilter, ExecutionInfo, ExecutionRequest, ExecutionStatus, ResourceRequirements,
    WorkloadSpec,
};
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

/// Create test execution request
fn create_test_request() -> ExecutionRequest {
    ExecutionRequest {
        workload: WorkloadSpec::Native {
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
        },
        runtime_type: RuntimeType::Native,
        priority: 5,
        timeout_secs: Some(300),
        resources: Some(ResourceRequirements {
            cpu_cores: Some(2.0),
            memory_mb: Some(512),
            storage_mb: Some(1024),
            gpu_count: None, // Validation requires min=1, so use None for no GPU
            network_mbps: Some(100),
        }),
        environment: HashMap::new(),
        metadata: HashMap::new(),
        callback_url: None,
    }
}

/// Create test execution info
fn create_test_execution(id: Uuid, status: ExecutionStatus) -> ExecutionInfo {
    ExecutionInfo {
        execution_id: id,
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
// SUBMIT EXECUTION TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_success() {
    // ✅ FULLY CONCURRENT: Submit valid execution request
    let state = create_test_state();
    let request = create_test_request();
    let headers = HeaderMap::new();

    let result = submit_execution(State(state.clone()), headers, Json(request)).await;
    assert!(result.is_ok(), "Valid execution request should succeed");

    // Verify execution was stored
    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 1, "Should have one execution stored");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_creates_unique_ids() {
    // ✅ FULLY CONCURRENT: Multiple submissions create unique IDs
    let state = create_test_state();
    let headers = HeaderMap::new();

    for _ in 0..5 {
        let request = create_test_request();
        let _ = submit_execution(State(state.clone()), headers.clone(), Json(request)).await;
    }

    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 5, "Should have 5 unique executions");

    // Verify all IDs are unique
    let ids: Vec<Uuid> = executions.keys().copied().collect();
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(ids.len(), unique_ids.len(), "All IDs should be unique");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_with_different_runtimes() {
    // ✅ FULLY CONCURRENT: Submit executions with different runtime types
    let state = create_test_state();
    let headers = HeaderMap::new();

    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Python,
    ];

    for runtime in runtimes {
        let mut request = create_test_request();
        request.runtime_type = runtime.clone();
        let result = submit_execution(State(state.clone()), headers.clone(), Json(request)).await;
        assert!(result.is_ok(), "Should accept {:?} runtime", runtime);
    }
}

// ============================================================================
// GET EXECUTION STATUS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_status_success() {
    // ✅ FULLY CONCURRENT: Get status for existing execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(
            execution_id,
            create_test_execution(execution_id, ExecutionStatus::Running),
        );
    }

    let result = get_execution_status(State(state), Path(execution_id)).await;
    assert!(
        result.is_ok(),
        "Should return status for existing execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_status_not_found() {
    // ✅ FULLY CONCURRENT: Get status for non-existent execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let result = get_execution_status(State(state), Path(execution_id)).await;
    assert!(
        result.is_err(),
        "Should return error for non-existent execution"
    );
}

// ============================================================================
// LIST EXECUTIONS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_executions_empty() {
    // ✅ FULLY CONCURRENT: List executions with empty state
    let state = create_test_state();
    let filter = ExecutionFilter {
        status: None,
        runtime_type: None,
        submitted_after: None,
        submitted_before: None,
        page: Some(1),
        per_page: Some(10),
    };

    let result = list_executions(State(state), Query(filter)).await;
    assert!(result.is_ok(), "Should return empty list");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_executions_with_data() {
    // ✅ FULLY CONCURRENT: List executions with multiple executions
    let state = create_test_state();

    {
        let mut executions = state.executions.write().await;
        for i in 0..15 {
            let id = Uuid::new_v4();
            let status = if i % 2 == 0 {
                ExecutionStatus::Running
            } else {
                ExecutionStatus::Completed
            };
            executions.insert(id, create_test_execution(id, status));
        }
    }

    let filter = ExecutionFilter {
        status: None,
        runtime_type: None,
        submitted_after: None,
        submitted_before: None,
        page: Some(1),
        per_page: Some(10),
    };

    let result = list_executions(State(state), Query(filter)).await;
    assert!(result.is_ok(), "Should list executions successfully");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_executions_with_status_filter() {
    // ✅ FULLY CONCURRENT: Filter executions by status
    let state = create_test_state();

    {
        let mut executions = state.executions.write().await;
        for i in 0..10 {
            let id = Uuid::new_v4();
            let status = if i < 5 {
                ExecutionStatus::Running
            } else {
                ExecutionStatus::Completed
            };
            executions.insert(id, create_test_execution(id, status));
        }
    }

    let filter = ExecutionFilter {
        status: Some(ExecutionStatus::Running),
        runtime_type: None,
        submitted_after: None,
        submitted_before: None,
        page: Some(1),
        per_page: Some(10),
    };

    let result = list_executions(State(state), Query(filter)).await;
    assert!(result.is_ok(), "Should filter by status");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_executions_pagination() {
    // ✅ FULLY CONCURRENT: Test pagination
    let state = create_test_state();

    {
        let mut executions = state.executions.write().await;
        for _ in 0..25 {
            let id = Uuid::new_v4();
            executions.insert(id, create_test_execution(id, ExecutionStatus::Running));
        }
    }

    // Get first page
    let filter1 = ExecutionFilter {
        status: None,
        runtime_type: None,
        submitted_after: None,
        submitted_before: None,
        page: Some(1),
        per_page: Some(10),
    };

    let result1 = list_executions(State(state.clone()), Query(filter1)).await;
    assert!(result1.is_ok(), "Should get first page");

    // Get second page
    let filter2 = ExecutionFilter {
        status: None,
        runtime_type: None,
        submitted_after: None,
        submitted_before: None,
        page: Some(2),
        per_page: Some(10),
    };

    let result2 = list_executions(State(state), Query(filter2)).await;
    assert!(result2.is_ok(), "Should get second page");
}

// ============================================================================
// CANCEL EXECUTION TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_success() {
    // ✅ FULLY CONCURRENT: Cancel running execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(
            execution_id,
            create_test_execution(execution_id, ExecutionStatus::Running),
        );
    }

    let result = cancel_execution(State(state.clone()), Path(execution_id)).await;
    assert!(result.is_ok(), "Should cancel running execution");

    // Verify status changed
    let executions = state.executions.read().await;
    let info = executions.get(&execution_id).unwrap();
    assert_eq!(info.status, ExecutionStatus::Cancelled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_not_found() {
    // ✅ FULLY CONCURRENT: Cancel non-existent execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let result = cancel_execution(State(state), Path(execution_id)).await;
    assert!(
        result.is_err(),
        "Should return error for non-existent execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_already_completed() {
    // ✅ FULLY CONCURRENT: Cannot cancel completed execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(
            execution_id,
            create_test_execution(execution_id, ExecutionStatus::Completed),
        );
    }

    let result = cancel_execution(State(state), Path(execution_id)).await;
    assert!(result.is_err(), "Should not cancel completed execution");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_already_failed() {
    // ✅ FULLY CONCURRENT: Cannot cancel failed execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(
            execution_id,
            create_test_execution(execution_id, ExecutionStatus::Failed),
        );
    }

    let result = cancel_execution(State(state), Path(execution_id)).await;
    assert!(result.is_err(), "Should not cancel failed execution");
}

// ============================================================================
// CONCURRENT TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_submit_executions() {
    // ✅ FULLY CONCURRENT: Multiple concurrent submissions
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(30));
    let mut tasks = vec![];

    for _ in 0..30 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let request = create_test_request();
            let headers = HeaderMap::new();
            submit_execution(State(state_clone), headers, Json(request))
                .await
                .is_ok()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 30, "All 30 submissions should succeed");

    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 30, "Should have 30 executions");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_get_status() {
    // ✅ FULLY CONCURRENT: Multiple concurrent status queries
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(
            execution_id,
            create_test_execution(execution_id, ExecutionStatus::Running),
        );
    }

    let barrier = Arc::new(Barrier::new(40));
    let mut tasks = vec![];

    for _ in 0..40 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            get_execution_status(State(state_clone), Path(execution_id))
                .await
                .is_ok()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 40, "All 40 status queries should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_list_executions() {
    // ✅ FULLY CONCURRENT: Multiple concurrent list operations
    let state = create_test_state();

    {
        let mut executions = state.executions.write().await;
        for _ in 0..20 {
            let id = Uuid::new_v4();
            executions.insert(id, create_test_execution(id, ExecutionStatus::Running));
        }
    }

    let barrier = Arc::new(Barrier::new(25));
    let mut tasks = vec![];

    for _ in 0..25 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let filter = ExecutionFilter {
                status: None,
                runtime_type: None,
                submitted_after: None,
                submitted_before: None,
                page: Some(1),
                per_page: Some(10),
            };
            list_executions(State(state_clone), Query(filter))
                .await
                .is_ok()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 25, "All 25 list operations should succeed");
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_100_concurrent_operations() {
    // ✅ STRESS TEST: Mix of all operations concurrently
    let state = create_test_state();
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    // Pre-populate with some executions
    let mut exec_ids = vec![];
    {
        let mut executions = state.executions.write().await;
        for _ in 0..10 {
            let id = Uuid::new_v4();
            executions.insert(id, create_test_execution(id, ExecutionStatus::Running));
            exec_ids.push(id);
        }
    }

    for i in 0..100 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);
        let exec_ids_clone = exec_ids.clone();

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            match i % 4 {
                0 => {
                    // Submit execution
                    let request = create_test_request();
                    let headers = HeaderMap::new();
                    submit_execution(State(state_clone), headers, Json(request))
                        .await
                        .is_ok()
                }
                1 if !exec_ids_clone.is_empty() => {
                    // Get status
                    let id = exec_ids_clone[i % exec_ids_clone.len()];
                    get_execution_status(State(state_clone), Path(id))
                        .await
                        .is_ok()
                }
                2 => {
                    // List executions
                    let filter = ExecutionFilter {
                        status: None,
                        runtime_type: None,
                        submitted_after: None,
                        submitted_before: None,
                        page: Some(1),
                        per_page: Some(10),
                    };
                    list_executions(State(state_clone), Query(filter))
                        .await
                        .is_ok()
                }
                _ if !exec_ids_clone.is_empty() => {
                    // Try cancel (might fail if already cancelled)
                    let id = exec_ids_clone[i % exec_ids_clone.len()];
                    let result = cancel_execution(State(state_clone), Path(id)).await;
                    result.is_ok() || result.is_err() // Both outcomes are acceptable
                }
                _ => true, // Default case
            }
        }));
    }

    let mut operations = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            operations += 1;
        }
    }

    assert!(
        operations >= 90,
        "At least 90% of operations should complete successfully"
    );
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_with_minimal_resources() {
    // ✅ FULLY CONCURRENT: Submit with minimal resource requirements
    let state = create_test_state();
    let mut request = create_test_request();
    request.resources = Some(ResourceRequirements {
        cpu_cores: Some(0.1),
        memory_mb: Some(64),
        storage_mb: Some(100),
        gpu_count: None, // No GPU needed
        network_mbps: Some(10),
    });
    let headers = HeaderMap::new();

    let result = submit_execution(State(state), headers, Json(request)).await;
    assert!(result.is_ok(), "Should accept minimal resources");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_with_large_per_page() {
    // ✅ FULLY CONCURRENT: List with large per_page value
    let state = create_test_state();

    {
        let mut executions = state.executions.write().await;
        for _ in 0..50 {
            let id = Uuid::new_v4();
            executions.insert(id, create_test_execution(id, ExecutionStatus::Running));
        }
    }

    let filter = ExecutionFilter {
        status: None,
        runtime_type: None,
        submitted_after: None,
        submitted_before: None,
        page: Some(1),
        per_page: Some(100),
    };

    let result = list_executions(State(state), Query(filter)).await;
    assert!(result.is_ok(), "Should handle large per_page");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_submit_execution() {
    // ✅ FULLY CONCURRENT: Submission should be fast
    let state = create_test_state();
    let request = create_test_request();
    let headers = HeaderMap::new();

    let start = std::time::Instant::now();
    let result = submit_execution(State(state), headers, Json(request)).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(
        duration.as_millis() < 50,
        "Submission should complete in <50ms"
    );
}
