// SPDX-License-Identifier: AGPL-3.0-only
//! Background Task Management Tests
//!
//! Target: `crates/server/src/background.rs`
//! Tests background services and task spawning patterns used by the server.
//!
//! Note: background.rs uses `tokio::spawn` directly - no task manager API.
//! `perform_health_check` is tested in background.rs unit tests (pub(crate)).
//! Tests for task manager features (queue, priority, cancellation) are marked
//! #[ignore] until that API exists.

#![allow(clippy::assertions_on_constants)] // For #[ignore] placeholder tests

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};

use toadstool_server::config::{HealthCheckConfig, ServerConfig};
use toadstool_server::state::{ServerState, ServerStatistics};
use toadstool_testing::mocks::resource_monitors::MockResourceMonitor;

fn create_test_state(config: ServerConfig) -> ServerState {
    let (event_broadcaster, _) = broadcast::channel(100);
    ServerState {
        runtime_engines: Arc::new(RwLock::new(HashMap::new())),
        active_executions: Arc::new(RwLock::new(HashMap::new())),
        event_broadcaster,
        config,
        resource_monitor: Arc::new(MockResourceMonitor::new_successful()),
        stats: Arc::new(RwLock::new(ServerStatistics::default())),
        capability_provider: None,
    }
}

// ── Tests using actual background API ─────────────────────────────────────

/// Test 1: `ServerState` creation for background services
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_creation() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_resources: false,
            check_runtime_engines: false,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };
    let state = create_test_state(config);
    assert!(state.runtime_engines.read().await.is_empty());
}

/// Test 2: Spawn background task via `tokio::spawn` (mirrors background.rs pattern)
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_spawn_background_task() {
    let counter = Arc::new(RwLock::new(0u32));
    let c = Arc::clone(&counter);
    let handle = tokio::spawn(async move {
        *c.write().await += 1;
    });
    handle.await.unwrap();
    assert_eq!(*counter.read().await, 1);
}

/// Test 3: Multiple concurrent tasks complete successfully
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_background_tasks() {
    let counter = Arc::new(RwLock::new(0u32));
    let mut handles = Vec::new();
    for _ in 0..5 {
        let c = Arc::clone(&counter);
        handles.push(tokio::spawn(async move {
            *c.write().await += 1;
        }));
    }
    for h in handles {
        h.await.unwrap();
    }
    assert_eq!(*counter.read().await, 5);
}

/// Test 4: Task cancellation via abort
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cancel_background_task() {
    let handle = tokio::spawn(async {
        std::future::pending::<()>().await;
    });
    handle.abort();
    let result = handle.await;
    assert!(result.is_err()); // JoinError because aborted
}

/// Test 5: Timeout on slow task
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_timeout() {
    let result =
        tokio::time::timeout(Duration::from_millis(50), std::future::pending::<()>()).await;
    assert!(result.is_err());
}

/// Test 6: Panicking task does not crash runtime
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_panic_handling() {
    let handle = tokio::spawn(async {
        panic!("intentional test panic");
    });
    let result = handle.await;
    assert!(result.is_err());
    assert!(result.unwrap_err().is_panic());
}

/// Test 7: Error propagation from spawned task
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_task_error_result() {
    let handle = tokio::spawn(async move { Err::<(), String>("task error".to_string()) });
    let result: Result<(), String> = handle.await.unwrap();
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "task error");
}

/// Test 8: `start_background_services` does not panic
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_background_manager_shutdown() {
    let config = ServerConfig {
        health_check: HealthCheckConfig {
            check_resources: false,
            check_runtime_engines: false,
            ..HealthCheckConfig::default()
        },
        ..ServerConfig::default()
    };
    let state = create_test_state(config);
    let state_clone = state.clone();
    toadstool_server::background::start_background_services(state_clone).await;
    // Yield to let spawned tasks start — no sleep needed
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    // No panic = success
}
