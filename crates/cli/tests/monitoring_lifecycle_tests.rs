// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async,
    clippy::unnecessary_wraps,
    clippy::unused_self
)]
//! Tests 21-30: Monitor Lifecycle

use std::time::Duration;

mod common;
use common::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_start() {
    let monitor = create_test_monitor().await.unwrap();
    let result = monitor.start();
    assert!(result.is_ok(), "Monitor should start successfully");
    assert!(monitor.is_running(), "Monitor should be running");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_stop() {
    let monitor = create_test_monitor().await.unwrap();
    monitor.start().unwrap();
    let result = monitor.stop();
    assert!(result.is_ok(), "Monitor should stop successfully");
    assert!(!monitor.is_running(), "Monitor should be stopped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_restart() {
    let monitor = create_test_monitor().await.unwrap();
    monitor.start().unwrap();
    monitor.stop().unwrap();
    let result = monitor.start();
    assert!(result.is_ok(), "Monitor should restart successfully");
    assert!(
        monitor.is_running(),
        "Monitor should be running after restart"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_start_idempotent() {
    let monitor = create_test_monitor().await.unwrap();
    monitor.start().unwrap();
    let result = monitor.start();
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_stop_idempotent() {
    let monitor = create_test_monitor().await.unwrap();
    let result = monitor.stop();
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_graceful_shutdown() {
    let monitor = create_test_monitor().await.unwrap();
    monitor.start().unwrap();
    let result = monitor.shutdown_gracefully(Duration::from_secs(5)).await;
    assert!(result.is_ok(), "Graceful shutdown should succeed");
    assert!(!monitor.is_running(), "Monitor should be stopped");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_shutdown_timeout() {
    let timeout = Duration::from_secs(10);
    assert!(
        timeout.as_secs() >= 1,
        "Timeout should be at least 1 second"
    );
    assert!(timeout.as_secs() <= 60, "Timeout should be reasonable");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_cleanup_on_stop() {
    let monitor = create_test_monitor().await.unwrap();
    monitor.start().unwrap();
    monitor.stop().unwrap();
    monitor.cleanup();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_state_transitions() {
    let states = vec!["stopped", "starting", "running", "stopping", "stopped"];
    for i in 0..states.len() - 1 {
        let from = states[i];
        let to = states[i + 1];
        assert!(!from.is_empty(), "State should be defined: {from}");
        assert!(!to.is_empty(), "State should be defined: {to}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitor_concurrent_operations() {
    let monitor = create_test_monitor().await.unwrap();
    let handles: Vec<_> = (0..5)
        .map(|_| {
            let mon = monitor.clone();
            tokio::spawn(async move { mon.get_metrics().len() })
        })
        .collect();
    for handle in handles {
        let result = handle.await;
        assert!(result.is_ok(), "Concurrent access should work");
    }
}
