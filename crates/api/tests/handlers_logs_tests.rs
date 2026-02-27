//! Comprehensive tests for logs handlers
//!
//! ✅ MODERN CONCURRENT TESTING - Zero sleeps, fully concurrent
//! Tests log retrieval endpoints with various scenarios

use axum::extract::{Path, Query, State};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, RwLock};
use uuid::Uuid;

use toadstool::RuntimeType;
use toadstool_api::handlers::logs::{get_execution_logs, parse_log_line};
use toadstool_api::types::{ExecutionInfo, ExecutionStatus, TimeRange};
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

/// Create test execution
fn create_test_execution(id: Uuid) -> ExecutionInfo {
    ExecutionInfo {
        execution_id: id,
        status: ExecutionStatus::Running,
        runtime_type: RuntimeType::Container,
        submitted_at: std::time::SystemTime::now(),
        started_at: Some(std::time::SystemTime::now()),
        completed_at: None,
        duration_ms: None,
        progress: None,
        error_message: None,
        resource_usage: None,
        metadata: HashMap::new(),
    }
}

// ============================================================================
// GET EXECUTION LOGS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_logs_success() {
    // ✅ FULLY CONCURRENT: Get logs for existing execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    let time_range = TimeRange {
        start: std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(3600))
            .unwrap_or(std::time::UNIX_EPOCH),
        end: std::time::SystemTime::now(),
    };

    let result = get_execution_logs(State(state), Path(execution_id), Query(time_range)).await;
    assert!(result.is_ok(), "Should return logs for existing execution");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_logs_not_found() {
    // ✅ FULLY CONCURRENT: Get logs for non-existent execution
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    let time_range = TimeRange {
        start: std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(3600))
            .unwrap_or(std::time::UNIX_EPOCH),
        end: std::time::SystemTime::now(),
    };

    let result = get_execution_logs(State(state), Path(execution_id), Query(time_range)).await;
    assert!(
        result.is_err(),
        "Should return error for non-existent execution"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_execution_logs_with_time_range() {
    // ✅ FULLY CONCURRENT: Get logs with custom time range
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    let time_range = TimeRange {
        start: std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(86400))
            .unwrap_or(std::time::UNIX_EPOCH),
        end: std::time::SystemTime::now(),
    };

    let result = get_execution_logs(State(state), Path(execution_id), Query(time_range)).await;
    assert!(result.is_ok(), "Should accept custom time range");
}

// ============================================================================
// PARSE LOG LINE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_log_line_info() {
    // ✅ FULLY CONCURRENT: Parse INFO level log
    let log_line = "2025-12-02T10:00:00Z info [executor] Test message";
    let entry = parse_log_line(log_line);

    assert!(entry.is_some(), "Should parse valid log line");
    let entry = entry.unwrap();
    assert_eq!(entry.message, "Test message");
    assert_eq!(entry.source, "executor");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_log_line_error() {
    // ✅ FULLY CONCURRENT: Parse ERROR level log
    let log_line = "2025-12-02T10:00:00Z error [system] Error occurred";
    let entry = parse_log_line(log_line);

    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.message, "Error occurred");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_log_line_warn() {
    // ✅ FULLY CONCURRENT: Parse WARN level log
    let log_line = "2025-12-02T10:00:00Z warn [monitor] Warning message";
    let entry = parse_log_line(log_line);

    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.message, "Warning message");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_log_line_debug() {
    // ✅ FULLY CONCURRENT: Parse DEBUG level log
    let log_line = "2025-12-02T10:00:00Z debug [runtime] Debug info";
    let entry = parse_log_line(log_line);

    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.message, "Debug info");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_log_line_invalid() {
    // ✅ FULLY CONCURRENT: Parse invalid log line
    let log_line = "invalid log";
    let entry = parse_log_line(log_line);

    assert!(entry.is_none(), "Should return None for invalid log");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_log_line_empty() {
    // ✅ FULLY CONCURRENT: Parse empty log line
    let log_line = "";
    let entry = parse_log_line(log_line);

    assert!(entry.is_none(), "Should return None for empty log");
}

// ============================================================================
// CONCURRENT LOG TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_log_requests() {
    // ✅ FULLY CONCURRENT: Multiple log requests in parallel
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

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
            let time_range = TimeRange {
                start: std::time::SystemTime::now()
                    .checked_sub(std::time::Duration::from_secs(3600))
                    .unwrap_or(std::time::UNIX_EPOCH),
                end: std::time::SystemTime::now(),
            };
            get_execution_logs(State(state_clone), Path(exec_id), Query(time_range))
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

    assert_eq!(
        successes, 30,
        "All 30 concurrent log requests should succeed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_log_parsing() {
    // ✅ FULLY CONCURRENT: Multiple log parsing operations in parallel
    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for i in 0..50 {
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let log_line = format!("2025-12-02T10:00:00Z info [source{}] Message {}", i, i);
            parse_log_line(&log_line).is_some()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(
        successes, 50,
        "All 50 concurrent parsing operations should succeed"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_logs_while_executions_changing() {
    // ✅ FULLY CONCURRENT: Log requests while executions are being modified
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    let barrier = Arc::new(Barrier::new(40));
    let mut tasks = vec![];

    // Spawn 20 tasks reading logs
    for _ in 0..20 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);
        let exec_id = execution_id;

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let time_range = TimeRange {
                start: std::time::SystemTime::now()
                    .checked_sub(std::time::Duration::from_secs(3600))
                    .unwrap_or(std::time::UNIX_EPOCH),
                end: std::time::SystemTime::now(),
            };
            get_execution_logs(State(state_clone), Path(exec_id), Query(time_range))
                .await
                .is_ok()
        }));
    }

    // Spawn 20 tasks adding new executions
    for _ in 0..20 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let mut executions = state_clone.executions.write().await;
            let new_id = Uuid::new_v4();
            executions.insert(new_id, create_test_execution(new_id));
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
async fn test_stress_100_concurrent_log_requests() {
    // ✅ STRESS TEST: 100 concurrent log requests
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);
        let exec_id = execution_id;

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let time_range = TimeRange {
                start: std::time::SystemTime::now()
                    .checked_sub(std::time::Duration::from_secs(3600))
                    .unwrap_or(std::time::UNIX_EPOCH),
                end: std::time::SystemTime::now(),
            };
            get_execution_logs(State(state_clone), Path(exec_id), Query(time_range))
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

    assert_eq!(successes, 100, "All 100 log requests should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_parse_many_log_lines() {
    // ✅ STRESS TEST: Parse many log lines concurrently
    let barrier = Arc::new(Barrier::new(200));
    let mut tasks = vec![];

    for i in 0..200 {
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            let level = match i % 4 {
                0 => "info",
                1 => "error",
                2 => "warn",
                _ => "debug",
            };

            let log_line = format!("2025-12-02T10:00:00Z {} [source] Message {}", level, i);
            parse_log_line(&log_line).is_some()
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            successes += 1;
        }
    }

    assert_eq!(successes, 200, "All 200 parsing operations should succeed");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_logs_for_multiple_executions() {
    // ✅ FULLY CONCURRENT: Get logs for multiple different executions
    let state = create_test_state();
    let mut execution_ids = vec![];

    {
        let mut executions = state.executions.write().await;
        for _ in 0..10 {
            let id = Uuid::new_v4();
            executions.insert(id, create_test_execution(id));
            execution_ids.push(id);
        }
    }

    for execution_id in execution_ids {
        let time_range = TimeRange {
            start: std::time::SystemTime::now()
                .checked_sub(std::time::Duration::from_secs(3600))
                .unwrap_or(std::time::UNIX_EPOCH),
            end: std::time::SystemTime::now(),
        };
        let result =
            get_execution_logs(State(state.clone()), Path(execution_id), Query(time_range)).await;
        assert!(result.is_ok(), "Should get logs for each execution");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_log_line_with_multiword_message() {
    // ✅ FULLY CONCURRENT: Parse log with multi-word message
    let log_line = "2025-12-02T10:00:00Z info [executor] This is a long multi-word message";
    let entry = parse_log_line(log_line);

    assert!(entry.is_some());
    let entry = entry.unwrap();
    assert_eq!(entry.message, "This is a long multi-word message");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_parse_log_line_unknown_level() {
    // ✅ FULLY CONCURRENT: Parse log with unknown level (should default to info)
    let log_line = "2025-12-02T10:00:00Z unknown [source] Message";
    let entry = parse_log_line(log_line);

    assert!(entry.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_logs_performance() {
    // ✅ FULLY CONCURRENT: Log retrieval should be fast
    let state = create_test_state();
    let execution_id = Uuid::new_v4();

    {
        let mut executions = state.executions.write().await;
        executions.insert(execution_id, create_test_execution(execution_id));
    }

    let start = std::time::Instant::now();
    let time_range = TimeRange {
        start: std::time::SystemTime::now()
            .checked_sub(std::time::Duration::from_secs(3600))
            .unwrap_or(std::time::UNIX_EPOCH),
        end: std::time::SystemTime::now(),
    };
    let result = get_execution_logs(State(state), Path(execution_id), Query(time_range)).await;
    let duration = start.elapsed();

    assert!(result.is_ok());
    assert!(
        duration.as_millis() < 50,
        "Log retrieval should complete in <50ms"
    );
}
