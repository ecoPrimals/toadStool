//! Comprehensive tests for workload execution handlers
//!
//! ✅ MODERN CONCURRENT TESTING - Zero sleeps, fully concurrent
//! Tests workload execution via primal capability system

use axum::extract::State;
use axum::Json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, Barrier, RwLock};
use uuid::Uuid;

use toadstool_api::handlers::workload::execute_workload;
use toadstool_api::{websocket, ApiState};
use toadstool_distributed::primal_capabilities::workload::{
    WorkloadResourceRequirements, WorkloadType,
};
use toadstool_distributed::primal_capabilities::WorkloadRequest;

/// Create test API state WITHOUT capability provider
fn create_test_state_no_provider() -> ApiState {
    let (event_broadcaster, _) = broadcast::channel(100);

    ApiState {
        executions: Arc::new(RwLock::new(HashMap::new())),
        metrics: Arc::new(RwLock::new(toadstool_api::ApiMetrics::default())),
        event_broadcaster,
        websocket_manager: Arc::new(websocket::WebSocketManager::new()),
        capability_provider: None, // No provider
    }
}

/// Create test workload request
fn create_test_request() -> WorkloadRequest {
    WorkloadRequest {
        request_id: Uuid::new_v4().to_string(),
        from_primal: "songbird".to_string(),
        required_capability: "compute".to_string(),
        workload_type: WorkloadType::Container {
            image: "test:latest".to_string(),
            command: None,
            args: None,
        },
        resource_requirements: WorkloadResourceRequirements {
            cpu_cores: Some(2),
            memory_mb: Some(512),
            gpu_required: false,
            gpu_memory_mb: None,
        },
        environment: HashMap::new(),
        timeout_seconds: Some(300),
        priority: "normal".to_string(),
    }
}

// ============================================================================
// WORKLOAD EXECUTION TESTS (Without Provider)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_workload_no_provider() {
    // ✅ FULLY CONCURRENT: Should fail when capability provider not configured
    let state = create_test_state_no_provider();
    let request = create_test_request();

    let result = execute_workload(State(state), Json(request)).await;
    assert!(result.is_err(), "Should fail without capability provider");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_workload_from_different_primals() {
    // ✅ FULLY CONCURRENT: Test workload requests from different primals
    let state = create_test_state_no_provider();

    let primals = vec!["songbird", "squirrel", "beardog", "nestgate"];

    for primal in primals {
        let mut request = create_test_request();
        request.from_primal = primal.to_string();

        let result = execute_workload(State(state.clone()), Json(request)).await;
        assert!(
            result.is_err(),
            "Should fail without provider regardless of primal"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_execute_workload_different_capabilities() {
    // ✅ FULLY CONCURRENT: Test different capability requirements
    let state = create_test_state_no_provider();

    let capabilities = vec!["compute", "storage", "network", "gpu"];

    for capability in capabilities {
        let mut request = create_test_request();
        request.required_capability = capability.to_string();

        let result = execute_workload(State(state.clone()), Json(request)).await;
        assert!(
            result.is_err(),
            "Should fail without provider for any capability"
        );
    }
}

// ============================================================================
// CONCURRENT WORKLOAD TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_workload_requests() {
    // ✅ FULLY CONCURRENT: Multiple workload requests in parallel
    let state = create_test_state_no_provider();
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = vec![];

    for _ in 0..20 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let request = create_test_request();
            execute_workload(State(state_clone), Json(request))
                .await
                .is_err()
        }));
    }

    let mut errors = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            errors += 1;
        }
    }

    assert_eq!(errors, 20, "All 20 requests should fail without provider");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_workload_requests_different_primals() {
    // ✅ FULLY CONCURRENT: Concurrent requests from different primals
    let state = create_test_state_no_provider();
    let barrier = Arc::new(Barrier::new(40));
    let mut tasks = vec![];

    let primals = vec!["songbird", "squirrel", "beardog", "nestgate"];

    for i in 0..40 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);
        let primal = primals[i % primals.len()].to_string();

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let mut request = create_test_request();
            request.from_primal = primal;
            execute_workload(State(state_clone), Json(request))
                .await
                .is_err()
        }));
    }

    let mut errors = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            errors += 1;
        }
    }

    assert_eq!(errors, 40, "All 40 requests should fail without provider");
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stress_100_concurrent_workload_requests() {
    // ✅ STRESS TEST: 100 concurrent workload requests
    let state = create_test_state_no_provider();
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for _ in 0..100 {
        let state_clone = state.clone();
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let request = create_test_request();
            execute_workload(State(state_clone), Json(request))
                .await
                .is_err()
        }));
    }

    let mut errors = 0;
    for task in tasks {
        if task.await.expect("Task should complete") {
            errors += 1;
        }
    }

    assert_eq!(errors, 100, "All 100 requests should fail gracefully");
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_with_empty_primal() {
    // ✅ FULLY CONCURRENT: Workload with empty primal name
    let state = create_test_state_no_provider();
    let mut request = create_test_request();
    request.from_primal = String::new();

    let result = execute_workload(State(state), Json(request)).await;
    assert!(result.is_err(), "Should fail without provider");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_with_empty_capability() {
    // ✅ FULLY CONCURRENT: Workload with empty capability
    let state = create_test_state_no_provider();
    let mut request = create_test_request();
    request.required_capability = String::new();

    let result = execute_workload(State(state), Json(request)).await;
    assert!(result.is_err(), "Should fail without provider");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_with_high_priority() {
    // ✅ FULLY CONCURRENT: Workload with high priority
    let state = create_test_state_no_provider();
    let mut request = create_test_request();
    request.priority = "high".to_string();

    let result = execute_workload(State(state), Json(request)).await;
    assert!(
        result.is_err(),
        "Should fail without provider regardless of priority"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_with_low_priority() {
    // ✅ FULLY CONCURRENT: Workload with low priority
    let state = create_test_state_no_provider();
    let mut request = create_test_request();
    request.priority = "low".to_string();

    let result = execute_workload(State(state), Json(request)).await;
    assert!(
        result.is_err(),
        "Should fail without provider regardless of priority"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_with_long_timeout() {
    // ✅ FULLY CONCURRENT: Workload with long timeout
    let state = create_test_state_no_provider();
    let mut request = create_test_request();
    request.timeout_seconds = Some(3600); // 1 hour

    let result = execute_workload(State(state), Json(request)).await;
    assert!(result.is_err(), "Should fail without provider");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_with_short_timeout() {
    // ✅ FULLY CONCURRENT: Workload with short timeout
    let state = create_test_state_no_provider();
    let mut request = create_test_request();
    request.timeout_seconds = Some(1);

    let result = execute_workload(State(state), Json(request)).await;
    assert!(result.is_err(), "Should fail without provider");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_with_complex_environment() {
    // ✅ FULLY CONCURRENT: Workload with complex environment variables
    let state = create_test_state_no_provider();
    let mut request = create_test_request();
    request
        .environment
        .insert("KEY1".to_string(), "value1".to_string());
    request
        .environment
        .insert("KEY2".to_string(), "value2".to_string());
    request
        .environment
        .insert("KEY3".to_string(), "value3".to_string());

    let result = execute_workload(State(state), Json(request)).await;
    assert!(result.is_err(), "Should fail without provider");
}

// ============================================================================
// VALIDATION TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_request_consistency() {
    // ✅ FULLY CONCURRENT: Same request should always fail the same way
    let state = create_test_state_no_provider();
    let request = create_test_request();

    let result1 = execute_workload(State(state.clone()), Json(request.clone())).await;
    let result2 = execute_workload(State(state.clone()), Json(request.clone())).await;
    let result3 = execute_workload(State(state), Json(request)).await;

    assert!(
        result1.is_err() && result2.is_err() && result3.is_err(),
        "All attempts should fail consistently"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_workload_performance() {
    // ✅ FULLY CONCURRENT: Workload rejection should be fast
    let state = create_test_state_no_provider();
    let request = create_test_request();

    let start = std::time::Instant::now();
    let _result = execute_workload(State(state), Json(request)).await;
    let duration = start.elapsed();

    assert!(
        duration.as_millis() < 10,
        "Workload rejection should be instant (<10ms)"
    );
}
