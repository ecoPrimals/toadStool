// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
// SPDX-License-Identifier: AGPL-3.0

//! Comprehensive tests for PerformanceTestManager
//!
//! This test file expands coverage of the performance testing infrastructure,
//! focusing on previously untested code paths.

use std::time::Duration;
use toadstool_testing::performance::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_benchmark_with_memory_profiling() {
    let config = PerformanceTestConfig {
        test_name: "memory_profiling_test".to_string(),
        warm_up_iterations: 2,
        measurement_iterations: 10,
        concurrent_threads: 1,
        memory_profiling: true, // Enable memory profiling
        cpu_profiling: false,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    let result = manager
        .benchmark(|| async {
            // Allocate some memory to test profiling
            let _data: Vec<u8> = vec![0; 1024];
            tokio::task::yield_now().await;
            Ok(())
        })
        .await;

    assert!(result.is_ok());
    let benchmark_result = result.unwrap();
    assert_eq!(benchmark_result.test_name, "memory_profiling_test");
    assert_eq!(benchmark_result.iterations, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_benchmark_with_cpu_profiling() {
    let config = PerformanceTestConfig {
        test_name: "cpu_profiling_test".to_string(),
        warm_up_iterations: 2,
        measurement_iterations: 5,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: true, // Enable CPU profiling
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    let result = manager
        .benchmark(|| async {
            // Do some CPU work
            let mut sum = 0u64;
            for i in 0..1000 {
                sum = sum.wrapping_add(i);
            }
            assert!(sum > 0);
            Ok(())
        })
        .await;

    assert!(result.is_ok());
    let benchmark_result = result.unwrap();
    assert_eq!(benchmark_result.iterations, 5);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_benchmark_with_custom_metrics() {
    let config = PerformanceTestConfig {
        test_name: "custom_metrics_test".to_string(),
        warm_up_iterations: 1,
        measurement_iterations: 3,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: vec!["throughput".to_string(), "latency".to_string()],
    };

    let manager = PerformanceTestManager::new(config);

    let result = manager
        .benchmark(|| async {
            // Simulate work with custom metrics
            tokio::time::sleep(Duration::from_micros(100)).await;
            Ok(())
        })
        .await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_benchmark_error_handling() {
    let config = PerformanceTestConfig {
        test_name: "error_test".to_string(),
        warm_up_iterations: 1,
        measurement_iterations: 5,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    let iteration = Arc::new(AtomicUsize::new(0));
    let result = manager
        .benchmark(|| {
            let iter_clone = Arc::clone(&iteration);
            async move {
                let current = iter_clone.fetch_add(1, Ordering::SeqCst);
                // Fail on specific iterations to test error handling
                if current == 3 {
                    Err(anyhow::anyhow!("Test error"))
                } else {
                    Ok(())
                }
            }
        })
        .await;

    // Should still complete despite errors
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_benchmark_with_warm_up() {
    let config = PerformanceTestConfig {
        test_name: "warmup_test".to_string(),
        warm_up_iterations: 10, // Significant warm-up
        measurement_iterations: 5,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;
    let call_count = Arc::new(AtomicUsize::new(0));
    let result = manager
        .benchmark(|| {
            let count_clone = Arc::clone(&call_count);
            async move {
                count_clone.fetch_add(1, Ordering::SeqCst);
                tokio::task::yield_now().await;
                Ok(())
            }
        })
        .await;

    assert!(result.is_ok());
    // Warm-up + measurement iterations
    assert_eq!(call_count.load(Ordering::SeqCst), 15);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_benchmark_duration_accuracy() {
    let config = PerformanceTestConfig {
        test_name: "duration_test".to_string(),
        warm_up_iterations: 0,
        measurement_iterations: 3,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    let sleep_duration = Duration::from_millis(10);
    let result = manager
        .benchmark(|| async move {
            tokio::time::sleep(sleep_duration).await;
            Ok(())
        })
        .await;

    assert!(result.is_ok());
    let benchmark_result = result.unwrap();

    // Total duration should be at least 3 * 10ms = 30ms
    assert!(benchmark_result.total_duration >= Duration::from_millis(30));

    // Average should be around 10ms (with some tolerance)
    let avg_millis = benchmark_result.average_duration.as_millis();
    assert!(
        (8..=15).contains(&avg_millis),
        "Average was {}ms",
        avg_millis
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_percentile_metrics() {
    let config = PerformanceTestConfig {
        test_name: "percentile_test".to_string(),
        warm_up_iterations: 0,
        measurement_iterations: 100, // Need many samples for percentiles
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    let counter = Arc::new(AtomicU64::new(0));

    let result = manager
        .benchmark(|| {
            let c = Arc::clone(&counter);
            async move {
                // Variable duration to test percentiles
                let n = c.fetch_add(1, Ordering::Relaxed);
                let duration = Duration::from_micros(10 + (n % 990));
                tokio::time::sleep(duration).await;
                Ok(())
            }
        })
        .await;

    assert!(result.is_ok());
    let benchmark_result = result.unwrap();

    // Percentiles should be calculated
    assert!(benchmark_result.percentiles.p50 > Duration::ZERO);
    assert!(benchmark_result.percentiles.p90 > Duration::ZERO);
    assert!(benchmark_result.percentiles.p95 > Duration::ZERO);
    assert!(benchmark_result.percentiles.p99 > Duration::ZERO);

    // p99 should be >= p95 >= p90 >= p50
    assert!(benchmark_result.percentiles.p99 >= benchmark_result.percentiles.p95);
    assert!(benchmark_result.percentiles.p95 >= benchmark_result.percentiles.p90);
    assert!(benchmark_result.percentiles.p90 >= benchmark_result.percentiles.p50);
}

#[test]
fn test_resource_usage_metrics_default() {
    let metrics = ResourceUsageMetrics::default();
    assert_eq!(metrics.peak_memory_mb, 0);
    assert_eq!(metrics.average_memory_mb, 0);
    assert_eq!(metrics.peak_cpu_percent, 0.0);
    assert_eq!(metrics.average_cpu_percent, 0.0);
    assert_eq!(metrics.context_switches, 0);
}

#[test]
fn test_throughput_metrics() {
    let metrics = ThroughputMetrics {
        operations_per_second: 1000.0,
        bytes_per_second: Some(1_000_000),
        requests_per_second: Some(500.0),
        concurrent_operations: 4,
    };

    assert_eq!(metrics.operations_per_second, 1000.0);
    assert_eq!(metrics.bytes_per_second, Some(1_000_000));
    assert_eq!(metrics.requests_per_second, Some(500.0));
}

#[test]
fn test_percentile_metrics_ordering() {
    let p50 = Duration::from_millis(10);
    let p90 = Duration::from_millis(15);
    let p95 = Duration::from_millis(20);
    let p99 = Duration::from_millis(30);
    let p99_9 = Duration::from_millis(35);

    let metrics = PercentileMetrics {
        p50,
        p90,
        p95,
        p99,
        p99_9,
    };

    // Verify ordering
    assert!(metrics.p50 <= metrics.p90);
    assert!(metrics.p90 <= metrics.p95);
    assert!(metrics.p95 <= metrics.p99);
    assert!(metrics.p99 <= metrics.p99_9);
}

#[test]
fn test_performance_comparison() {
    let baseline = BenchmarkResult::default("baseline".to_string());
    let mut comparison = BenchmarkResult::default("comparison".to_string());
    comparison.average_duration = Duration::from_millis(20);

    // PerformanceComparison would compare these results
    // (Implementation tests would go here if PerformanceComparison is exposed)
    assert_ne!(baseline.average_duration, comparison.average_duration);
}
