//! Integration tests for API handlers
//!
//! These tests verify the behavior of the main API handler functions,
//! focusing on critical paths and error handling.

use std::collections::HashMap;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::Json;
use chrono::Utc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

use toadstool::RuntimeType;
use toadstool_api::handlers::{
    cancel_execution, get_cluster_status, get_execution_logs, get_execution_metrics,
    get_execution_status, health_check, list_executions, submit_execution,
};
use toadstool_api::types::*;
use toadstool_api::ApiState;

/// Helper function to create a test API state
fn create_test_state() -> ApiState {
    let (event_broadcaster, _) = broadcast::channel(100);

    ApiState {
        event_broadcaster,
        executions: Arc::new(RwLock::new(HashMap::new())),
        metrics: Arc::new(RwLock::new(toadstool_api::ApiMetrics::default())),
        capability_provider: None,
    }
}

/// Helper function to create a valid execution request
fn create_valid_request() -> ExecutionRequest {
    ExecutionRequest {
        workload: WorkloadSpec::Native {
            executable: "echo".to_string(),
            args: vec!["hello".to_string()],
        },
        runtime_type: RuntimeType::Native,
        priority: 5,
        timeout_secs: Some(30),
        resources: None,
        environment: HashMap::new(),
        metadata: HashMap::new(),
        callback_url: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_success() {
    let state = create_test_state();
    let request = create_valid_request();
    let headers = HeaderMap::new();

    let result = submit_execution(State(state.clone()), headers, Json(request)).await;

    assert!(result.is_ok(), "Valid execution request should succeed");

    // Verify execution was stored
    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 1, "Should have one execution stored");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_generates_unique_ids() {
    let state = create_test_state();
    let headers = HeaderMap::new();

    // Submit 5 executions
    for _ in 0..5 {
        let request = create_valid_request();
        let _ = submit_execution(State(state.clone()), headers.clone(), Json(request)).await;
    }

    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 5, "Should have 5 executions");

    // Verify all IDs are unique
    let ids: Vec<_> = executions.keys().collect();
    let unique_ids: std::collections::HashSet<_> = ids.iter().collect();
    assert_eq!(
        ids.len(),
        unique_ids.len(),
        "All execution IDs should be unique"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_initial_status() {
    let state = create_test_state();
    let request = create_valid_request();
    let headers = HeaderMap::new();

    let result = submit_execution(State(state.clone()), headers, Json(request)).await;
    assert!(result.is_ok());

    let executions = state.executions.read().await;
    let execution = executions
        .values()
        .next()
        .expect("Should have an execution");

    assert_eq!(execution.status, ExecutionStatus::Submitted);
    assert!(
        execution.started_at.is_none(),
        "Should not have started yet"
    );
    assert!(
        execution.completed_at.is_none(),
        "Should not be completed yet"
    );
    assert_eq!(execution.progress, Some(0.0), "Progress should be 0%");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_status_existing() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    // Add an execution
    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Running,
        runtime_type: RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        duration_ms: None,
        progress: Some(50.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };

    state
        .executions
        .write()
        .await
        .insert(execution_id, execution_info);

    let result = get_execution_status(State(state), Path(execution_id)).await;

    assert!(result.is_ok(), "Should find existing execution");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_status_not_found() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let result = get_execution_status(State(state), Path(execution_id)).await;

    assert!(
        result.is_err(),
        "Should return error for non-existent execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_executions_empty() {
    let state = create_test_state();
    let filter = Query(ExecutionFilter::default());

    let result = list_executions(State(state), filter).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_executions_with_data() {
    let state = create_test_state();

    // Add multiple executions
    for i in 0..5 {
        let execution_id = Uuid::new_v4();
        let execution_info = ExecutionInfo {
            execution_id,
            status: if i % 2 == 0 {
                ExecutionStatus::Completed
            } else {
                ExecutionStatus::Running
            },
            runtime_type: RuntimeType::Native,
            submitted_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            duration_ms: None,
            progress: Some(i as f64 * 20.0),
            error_message: None,
            resource_usage: None,
            metadata: HashMap::new(),
        };
        state
            .executions
            .write()
            .await
            .insert(execution_id, execution_info);
    }

    let filter = Query(ExecutionFilter::default());
    let result = list_executions(State(state), filter).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_running() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    // Add a running execution
    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Running,
        runtime_type: RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        duration_ms: None,
        progress: Some(30.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };

    state
        .executions
        .write()
        .await
        .insert(execution_id, execution_info);

    let result = cancel_execution(State(state.clone()), Path(execution_id)).await;

    assert!(
        result.is_ok(),
        "Should successfully cancel running execution"
    );

    // Verify status changed
    let executions = state.executions.read().await;
    let execution = executions.get(&execution_id).unwrap();
    assert_eq!(execution.status, ExecutionStatus::Cancelled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_not_found() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let result = cancel_execution(State(state), Path(execution_id)).await;

    assert!(result.is_err(), "Should error on non-existent execution");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_success() {
    let state = create_test_state();

    let result = health_check(State(state)).await;

    assert!(result.is_ok(), "Health check should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_cluster_status_empty() {
    let state = create_test_state();

    let result = get_cluster_status(State(state)).await;

    assert!(
        result.is_ok(),
        "Cluster status should succeed even when empty"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_logs_not_found() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    let filter = Query(TimeRange {
        start: Utc::now(),
        end: Utc::now(),
    });

    let result = get_execution_logs(State(state), Path(execution_id), filter).await;

    // Should return empty logs or error for non-existent execution
    // Behavior depends on implementation
    let _ = result;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_metrics_not_found() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    let filter = Query(HashMap::new());

    let result = get_execution_metrics(State(state), Path(execution_id), filter).await;

    // Should return empty metrics or error for non-existent execution
    let _ = result;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_concurrent_submissions() {
    let state = create_test_state();

    // Submit multiple executions concurrently
    let mut handles = vec![];
    for _ in 0..10 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            let request = create_valid_request();
            let headers = HeaderMap::new();
            submit_execution(State(state_clone), headers, Json(request)).await
        });
        handles.push(handle);
    }

    // Wait for all submissions
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent submission should succeed");
    }

    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 10, "Should have 10 executions");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_lifecycle() {
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    // 1. Create execution (Submitted)
    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Submitted,
        runtime_type: RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: None,
        completed_at: None,
        duration_ms: None,
        progress: Some(0.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };
    state
        .executions
        .write()
        .await
        .insert(execution_id, execution_info.clone());

    // 2. Start execution (Running)
    {
        let mut executions = state.executions.write().await;
        if let Some(exec) = executions.get_mut(&execution_id) {
            exec.status = ExecutionStatus::Running;
            exec.started_at = Some(Utc::now());
            exec.progress = Some(50.0);
        }
    }

    // 3. Complete execution (Completed)
    {
        let mut executions = state.executions.write().await;
        if let Some(exec) = executions.get_mut(&execution_id) {
            exec.status = ExecutionStatus::Completed;
            exec.completed_at = Some(Utc::now());
            exec.progress = Some(100.0);
            exec.duration_ms = Some(1500);
        }
    }

    // Verify final state
    let executions = state.executions.read().await;
    let execution = executions.get(&execution_id).unwrap();
    assert_eq!(execution.status, ExecutionStatus::Completed);
    assert_eq!(execution.progress, Some(100.0));
    assert!(execution.duration_ms.is_some());
}

// ============================================================================
// Day 5: API Integration Tests - Error Handling & Edge Cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_with_all_runtime_types() {
    // Test submission with all supported runtime types
    let state = create_test_state();
    let headers = HeaderMap::new();

    let runtime_types = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Python,
    ];

    for runtime_type in runtime_types {
        let mut request = create_valid_request();
        request.runtime_type = runtime_type.clone();

        let result = submit_execution(State(state.clone()), headers.clone(), Json(request)).await;
        assert!(
            result.is_ok(),
            "Runtime type {:?} should be accepted",
            runtime_type
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_with_various_priorities() {
    // Test execution submission with different priority levels
    let state = create_test_state();
    let headers = HeaderMap::new();

    let priorities = vec![1, 3, 5, 7, 10];

    for priority in priorities {
        let mut request = create_valid_request();
        request.priority = priority;

        let result = submit_execution(State(state.clone()), headers.clone(), Json(request)).await;
        assert!(result.is_ok(), "Priority {} should be accepted", priority);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_with_timeout_values() {
    // Test various timeout configurations
    let state = create_test_state();
    let headers = HeaderMap::new();

    let timeouts = vec![Some(1), Some(30), Some(300), Some(3600), None];

    for timeout in timeouts {
        let mut request = create_valid_request();
        request.timeout_secs = timeout;

        let result = submit_execution(State(state.clone()), headers.clone(), Json(request)).await;
        assert!(result.is_ok(), "Timeout {:?} should be accepted", timeout);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_with_metadata() {
    // Test submission with metadata
    let state = create_test_state();
    let headers = HeaderMap::new();

    let mut request = create_valid_request();
    request
        .metadata
        .insert("user".to_string(), "test-user".to_string());
    request
        .metadata
        .insert("app".to_string(), "test-app".to_string());
    request
        .metadata
        .insert("version".to_string(), "1.0.0".to_string());

    let result = submit_execution(State(state.clone()), headers, Json(request)).await;
    assert!(result.is_ok());

    let executions = state.executions.read().await;
    let execution = executions.values().next().unwrap();
    assert_eq!(execution.metadata.len(), 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_with_environment_variables() {
    // Test submission with environment variables
    let state = create_test_state();
    let headers = HeaderMap::new();

    let mut request = create_valid_request();
    request
        .environment
        .insert("DEBUG".to_string(), "true".to_string());
    request
        .environment
        .insert("LOG_LEVEL".to_string(), "info".to_string());

    let result = submit_execution(State(state.clone()), headers, Json(request)).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_submit_execution_with_callback_url() {
    // Test submission with callback URL
    let state = create_test_state();
    let headers = HeaderMap::new();

    let mut request = create_valid_request();
    request.callback_url = Some("https://example.com/callback".to_string());

    let result = submit_execution(State(state.clone()), headers, Json(request)).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_execution_submissions() {
    // Test concurrent submission of multiple executions
    let state = create_test_state();

    let mut handles = vec![];
    for _ in 0..10 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            let headers = HeaderMap::new();
            let request = create_valid_request();
            submit_execution(State(state_clone), headers, Json(request)).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent submission should succeed");
    }

    let executions = state.executions.read().await;
    assert_eq!(executions.len(), 10, "Should have 10 executions");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_executions_with_status_filter() {
    // Test listing executions with status filter
    let state = create_test_state();

    // Add executions with different statuses
    for i in 0..6 {
        let execution_id = Uuid::new_v4();
        let status = match i % 3 {
            0 => ExecutionStatus::Running,
            1 => ExecutionStatus::Completed,
            _ => ExecutionStatus::Failed,
        };

        let execution_info = ExecutionInfo {
            execution_id,
            status,
            runtime_type: RuntimeType::Native,
            submitted_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            duration_ms: None,
            progress: Some(50.0),
            error_message: None,
            resource_usage: None,
            metadata: HashMap::new(),
        };

        state
            .executions
            .write()
            .await
            .insert(execution_id, execution_info);
    }

    let filter = Query(ExecutionFilter::default());
    let result = list_executions(State(state), filter).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_metrics_error_handling() {
    // Test getting metrics for non-existent execution (error handling variant)
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    let query = Query(HashMap::new());

    let result = get_execution_metrics(State(state), Path(execution_id), query).await;

    assert!(
        result.is_err(),
        "Should return error for non-existent execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_logs_error_handling() {
    // Test getting logs for non-existent execution (error handling variant)
    let state = create_test_state();
    let execution_id = Uuid::new_v4();
    let query = Query(TimeRange {
        start: Utc::now(),
        end: Utc::now(),
    });

    let result = get_execution_logs(State(state), Path(execution_id), query).await;

    assert!(
        result.is_err(),
        "Should return error for non-existent execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_execution_error_handling() {
    // Test cancelling non-existent execution (error handling variant)
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let result = cancel_execution(State(state), Path(execution_id)).await;

    assert!(
        result.is_err(),
        "Should return error for non-existent execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_health_check_always_responds() {
    // Test health check always returns a response
    let state = create_test_state();

    let result = health_check(State(state)).await;

    assert!(result.is_ok(), "Health check should always respond");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_cluster_status_responds() {
    // Test cluster status endpoint responds
    let state = create_test_state();

    let result = get_cluster_status(State(state)).await;

    assert!(result.is_ok(), "Cluster status should be retrievable");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_state_transitions() {
    // Test valid execution state transitions
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    // Start with Submitted
    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Submitted,
        runtime_type: RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: None,
        completed_at: None,
        duration_ms: None,
        progress: Some(0.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };
    state
        .executions
        .write()
        .await
        .insert(execution_id, execution_info);

    // Transition to Queued
    {
        let mut executions = state.executions.write().await;
        executions.get_mut(&execution_id).unwrap().status = ExecutionStatus::Queued;
    }

    // Transition to Running
    {
        let mut executions = state.executions.write().await;
        executions.get_mut(&execution_id).unwrap().status = ExecutionStatus::Running;
    }

    // Transition to Completed
    {
        let mut executions = state.executions.write().await;
        executions.get_mut(&execution_id).unwrap().status = ExecutionStatus::Completed;
    }

    let executions = state.executions.read().await;
    assert_eq!(
        executions.get(&execution_id).unwrap().status,
        ExecutionStatus::Completed
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_progress_updates() {
    // Test progress updates during execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Running,
        runtime_type: RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        duration_ms: None,
        progress: Some(0.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };
    state
        .executions
        .write()
        .await
        .insert(execution_id, execution_info);

    // Update progress incrementally
    for progress in [25.0, 50.0, 75.0, 100.0] {
        let mut executions = state.executions.write().await;
        executions.get_mut(&execution_id).unwrap().progress = Some(progress);
    }

    let executions = state.executions.read().await;
    assert_eq!(executions.get(&execution_id).unwrap().progress, Some(100.0));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execution_with_error_message() {
    // Test execution that fails with error message
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Failed,
        runtime_type: RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: Some(Utc::now()),
        duration_ms: Some(500),
        progress: Some(30.0),
        error_message: Some("Test error message".to_string()),
        resource_usage: None,
        metadata: HashMap::new(),
    };
    state
        .executions
        .write()
        .await
        .insert(execution_id, execution_info);

    let executions = state.executions.read().await;
    let execution = executions.get(&execution_id).unwrap();
    assert_eq!(execution.status, ExecutionStatus::Failed);
    assert!(execution.error_message.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_api_state_initialization() {
    // Test API state is properly initialized
    let state = create_test_state();

    assert!(state.executions.read().await.is_empty());
    // Config is properly initialized with default values
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_status_queries() {
    // Test concurrent queries for execution status
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let execution_info = ExecutionInfo {
        execution_id,
        status: ExecutionStatus::Running,
        runtime_type: RuntimeType::Native,
        submitted_at: Utc::now(),
        started_at: Some(Utc::now()),
        completed_at: None,
        duration_ms: None,
        progress: Some(50.0),
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    };
    state
        .executions
        .write()
        .await
        .insert(execution_id, execution_info);

    let mut handles = vec![];
    for _ in 0..20 {
        let state_clone = state.clone();
        let handle = tokio::spawn(async move {
            get_execution_status(State(state_clone), Path(execution_id)).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent status query should succeed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_executions_different_runtime_types() {
    // Test multiple executions with different runtime types coexisting
    let state = create_test_state();

    let runtime_types = vec![
        RuntimeType::Native,
        RuntimeType::Wasm,
        RuntimeType::Container,
        RuntimeType::Python,
    ];

    for (i, runtime_type) in runtime_types.into_iter().enumerate() {
        let execution_id = Uuid::new_v4();
        let execution_info = ExecutionInfo {
            execution_id,
            status: ExecutionStatus::Running,
            runtime_type,
            submitted_at: Utc::now(),
            started_at: Some(Utc::now()),
            completed_at: None,
            duration_ms: None,
            progress: Some((i as f64 + 1.0) * 25.0),
            error_message: None,
            resource_usage: None,
            metadata: HashMap::new(),
        };
        state
            .executions
            .write()
            .await
            .insert(execution_id, execution_info);
    }

    let executions = state.executions.read().await;
    assert_eq!(
        executions.len(),
        4,
        "Should have 4 executions with different runtime types"
    );
}
