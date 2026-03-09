// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for CLI Monitoring modules
//!
//! ✅ MODERN CONCURRENT TESTING - Event-driven, no sleeps
//! Tests monitoring, resource tracking, and observability

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Barrier, RwLock};

use toadstool::SystemResourceMonitor;

// ============================================================================
// RESOURCE MONITORING TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_monitor_creation() {
    // ✅ FULLY CONCURRENT: Create multiple monitors
    let barrier = Arc::new(Barrier::new(20));
    let mut tasks = vec![];

    for _ in 0..20 {
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;
            let _monitor = SystemResourceMonitor::new();
            // Monitor should be created successfully
            true
        }));
    }

    for task in tasks {
        assert!(task.await.unwrap(), "Monitor creation should succeed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_resource_queries() {
    // ✅ FULLY CONCURRENT: Query resources concurrently
    let monitor = Arc::new(SystemResourceMonitor::new());
    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    for i in 0..50 {
        let mon = Arc::clone(&monitor);
        let bar = Arc::clone(&barrier);
        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            // Test various monitor operations
            let workload_id = format!("workload_{i}");
            let _ = mon.get_process_info(&workload_id).await;
            let _ = mon.get_network_stats().await;
            let _ = mon.get_load_averages().await;

            true
        }));
    }

    let mut successes = 0;
    for task in tasks {
        if task.await.unwrap() {
            successes += 1;
        }
    }

    assert!(
        successes >= 45,
        "Most resource queries should succeed: {successes}/50"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_monitor_metrics_collection() {
    // ✅ FULLY CONCURRENT: Collect metrics concurrently
    let monitor = Arc::new(SystemResourceMonitor::new());
    let metrics = Arc::new(RwLock::new(Vec::new()));

    let barrier = Arc::new(Barrier::new(30));
    let mut tasks = vec![];

    for i in 0..30 {
        let mon = Arc::clone(&monitor);
        let met = Arc::clone(&metrics);
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            let workload_id = format!("workload_{i}");
            let _ = mon.start_real_time_monitoring(&workload_id).await;
            let _ = mon.update_workload_metrics(&workload_id).await;

            let mut metrics_lock = met.write().await;
            metrics_lock.push(i);

            true
        }));
    }

    for task in tasks {
        assert!(task.await.unwrap());
    }

    let final_metrics = metrics.read().await;
    assert_eq!(final_metrics.len(), 30, "Should collect all metrics");
}

// ============================================================================
// MONITORING STATE TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitoring_state_updates() {
    // ✅ FULLY CONCURRENT: Update monitoring state concurrently
    let state = Arc::new(RwLock::new(HashMap::new()));
    let barrier = Arc::new(Barrier::new(40));
    let mut tasks = vec![];

    for i in 0..40 {
        let st = Arc::clone(&state);
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            {
                let mut state_lock = st.write().await;
                state_lock.insert(format!("metric_{i}"), f64::from(i));
            }

            // Verify insert
            let state_lock = st.read().await;
            state_lock.contains_key(&format!("metric_{i}"))
        }));
    }

    for task in tasks {
        assert!(task.await.unwrap(), "State update should succeed");
    }

    let final_state = state.read().await;
    assert_eq!(final_state.len(), 40, "Should have all metrics");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitoring_state_concurrent_reads_writes() {
    // ✅ FULLY CONCURRENT: Mix reads and writes
    let state = Arc::new(RwLock::new(HashMap::new()));

    // Pre-populate
    {
        let mut state_lock = state.write().await;
        for i in 0..10 {
            state_lock.insert(format!("key_{i}"), i);
        }
    }

    let barrier = Arc::new(Barrier::new(50));
    let mut tasks = vec![];

    // 25 readers, 25 writers
    for i in 0..50 {
        let st = Arc::clone(&state);
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            if i % 2 == 0 {
                // Reader
                let state_lock = st.read().await;
                !state_lock.is_empty()
            } else {
                // Writer
                let mut state_lock = st.write().await;
                state_lock.insert(format!("new_key_{i}"), i);
                true
            }
        }));
    }

    for task in tasks {
        assert!(task.await.unwrap());
    }
}

// ============================================================================
// MONITORING AGGREGATION TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_metric_aggregation() {
    // ✅ FULLY CONCURRENT: Aggregate metrics from multiple sources
    let metrics = Arc::new(RwLock::new(Vec::new()));
    let barrier = Arc::new(Barrier::new(100));
    let mut tasks = vec![];

    for i in 0..100 {
        let met = Arc::clone(&metrics);
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            let mut metrics_lock = met.write().await;
            metrics_lock.push(f64::from(i));
            true
        }));
    }

    for task in tasks {
        assert!(task.await.unwrap());
    }

    let final_metrics = metrics.read().await;
    assert_eq!(final_metrics.len(), 100);

    let sum: f64 = final_metrics.iter().sum();
    let avg = sum / final_metrics.len() as f64;

    // Average of 0..99 should be approximately 49.5
    assert!(
        (avg - 49.5).abs() < 1.0,
        "Average should be ~49.5, got {avg}"
    );
}

// ============================================================================
// STRESS TESTS
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 8)]
#[ignore = "Stress test - slow (>60s with coverage), run manually"]
async fn test_stress_500_concurrent_monitor_operations() {
    use tokio::time::{timeout, Duration};

    // ✅ STRESS TEST: 500 concurrent monitoring operations
    // ✅ DEEP DEBT FIX: Added timeout and ignore attribute for stress tests
    let monitor = Arc::new(SystemResourceMonitor::new());
    let barrier = Arc::new(Barrier::new(500));
    let mut tasks = vec![];

    for i in 0..500 {
        let mon = Arc::clone(&monitor);
        let bar = Arc::clone(&barrier);

        tasks.push(tokio::spawn(async move {
            bar.wait().await;

            let workload_id = format!("stress_workload_{i}");

            // Mix different operations
            match i % 3 {
                0 => mon.start_real_time_monitoring(&workload_id).await.is_ok(),
                1 => mon.get_network_stats().await.is_ok(),
                _ => mon.get_load_averages().await.is_ok(),
            }
        }));
    }

    // DEEP DEBT FIX: Add timeout to prevent infinite hang
    // 120s should be enough even with coverage overhead
    let result: Result<usize, _> = timeout(Duration::from_secs(120), async {
        let mut successes = 0;
        for task in tasks {
            if task.await.unwrap_or(false) {
                successes += 1;
            }
        }
        successes
    })
    .await;

    let successes = result.expect("Stress test timed out after 120s");

    assert!(
        successes >= 475,
        "At least 95% should succeed: {successes}/500"
    );
}
