// ToadStool - Universal Compute Platform
// Tests for performance testing utilities

use std::sync::Arc;
use std::time::Duration;
use toadstool_testing::performance::*;

#[test]
fn test_performance_test_config_default() {
    let config = PerformanceTestConfig::default();

    assert_eq!(config.test_name, "unnamed_benchmark");
    assert_eq!(config.warm_up_iterations, 10);
    assert_eq!(config.measurement_iterations, 100);
    assert_eq!(config.concurrent_threads, 1);
    assert!(config.memory_profiling);
    assert!(config.cpu_profiling);
    assert!(config.custom_metrics.is_empty());
}

#[test]
fn test_performance_test_config_custom() {
    let config = PerformanceTestConfig {
        test_name: "custom_test".to_string(),
        warm_up_iterations: 5,
        measurement_iterations: 50,
        concurrent_threads: 4,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: vec!["metric1".to_string(), "metric2".to_string()],
    };

    assert_eq!(config.test_name, "custom_test");
    assert_eq!(config.warm_up_iterations, 5);
    assert_eq!(config.measurement_iterations, 50);
    assert_eq!(config.concurrent_threads, 4);
    assert!(!config.memory_profiling);
    assert!(!config.cpu_profiling);
    assert_eq!(config.custom_metrics.len(), 2);
}

#[test]
fn test_resource_usage_metrics_default() {
    let metrics = ResourceUsageMetrics::default();

    assert_eq!(metrics.peak_memory_mb, 0);
    assert_eq!(metrics.average_memory_mb, 0);
    assert_eq!(metrics.peak_cpu_percent, 0.0);
    assert_eq!(metrics.average_cpu_percent, 0.0);
    assert_eq!(metrics.disk_io_mb, 0);
    assert_eq!(metrics.network_io_mb, 0);
    assert_eq!(metrics.context_switches, 0);
}

#[test]
fn test_performance_test_manager_creation() {
    let config = PerformanceTestConfig {
        test_name: "test_manager".to_string(),
        warm_up_iterations: 5,
        measurement_iterations: 10,
        concurrent_threads: 1,
        memory_profiling: true,
        cpu_profiling: true,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);
    // Manager should be created successfully (if we get here, test passes)
    drop(manager);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_simple_benchmark() {
    let config = PerformanceTestConfig {
        test_name: "simple_test".to_string(),
        warm_up_iterations: 2,
        measurement_iterations: 5,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    let result = manager
        .benchmark(|| async {
            // Simple no-op test function
            tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
            Ok(())
        })
        .await;

    assert!(result.is_ok());
    let benchmark_result = result.unwrap();
    assert_eq!(benchmark_result.test_name, "simple_test");
    assert_eq!(benchmark_result.iterations, 5);
    assert!(benchmark_result.total_duration > Duration::ZERO);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_benchmark_with_failures() {
    let config = PerformanceTestConfig {
        test_name: "failing_test".to_string(),
        warm_up_iterations: 1,
        measurement_iterations: 3,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    let call_count = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let result = manager
        .benchmark(|| {
            let call_count = Arc::clone(&call_count);
            async move {
                let count = call_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count % 2 == 0 {
                    Err(anyhow::anyhow!("Simulated failure"))
                } else {
                    Ok(())
                }
            }
        })
        .await;

    // Should still return a result even with some failures
    assert!(result.is_ok());
}

#[test]
fn test_benchmark_result_creation() {
    let result = BenchmarkResult {
        test_name: "test".to_string(),
        iterations: 10,
        total_duration: Duration::from_millis(100),
        average_duration: Duration::from_millis(10),
        min_duration: Duration::from_millis(5),
        max_duration: Duration::from_millis(20),
        percentiles: PercentileMetrics {
            p50: Duration::from_millis(10),
            p90: Duration::from_millis(15),
            p95: Duration::from_millis(18),
            p99: Duration::from_millis(19),
            p99_9: Duration::from_millis(20),
        },
        throughput: ThroughputMetrics {
            operations_per_second: 100.0,
            bytes_per_second: Some(1024),
            requests_per_second: Some(50.0),
            concurrent_operations: 1,
        },
        resource_usage: ResourceUsageMetrics::default(),
        custom_metrics: std::collections::HashMap::new(),
    };

    assert_eq!(result.test_name, "test");
    assert_eq!(result.iterations, 10);
    assert_eq!(result.throughput.operations_per_second, 100.0);
}

#[test]
fn test_performance_comparison_improvement() {
    let config = PerformanceTestConfig::default();
    let manager = PerformanceTestManager::new(config);

    let baseline = BenchmarkResult {
        test_name: "test".to_string(),
        iterations: 10,
        total_duration: Duration::from_millis(100),
        average_duration: Duration::from_millis(10),
        min_duration: Duration::from_millis(8),
        max_duration: Duration::from_millis(12),
        percentiles: PercentileMetrics {
            p50: Duration::from_millis(10),
            p90: Duration::from_millis(11),
            p95: Duration::from_millis(11),
            p99: Duration::from_millis(12),
            p99_9: Duration::from_millis(12),
        },
        throughput: ThroughputMetrics {
            operations_per_second: 100.0,
            bytes_per_second: None,
            requests_per_second: None,
            concurrent_operations: 1,
        },
        resource_usage: ResourceUsageMetrics::default(),
        custom_metrics: std::collections::HashMap::new(),
    };

    let current = BenchmarkResult {
        test_name: "test".to_string(),
        iterations: 10,
        total_duration: Duration::from_millis(80),
        average_duration: Duration::from_millis(8), // 20% faster
        min_duration: Duration::from_millis(6),
        max_duration: Duration::from_millis(10),
        percentiles: PercentileMetrics {
            p50: Duration::from_millis(8),
            p90: Duration::from_millis(9),
            p95: Duration::from_millis(9),
            p99: Duration::from_millis(10),
            p99_9: Duration::from_millis(10),
        },
        throughput: ThroughputMetrics {
            operations_per_second: 125.0,
            bytes_per_second: None,
            requests_per_second: None,
            concurrent_operations: 1,
        },
        resource_usage: ResourceUsageMetrics::default(),
        custom_metrics: std::collections::HashMap::new(),
    };

    let comparison = manager.compare_results(&baseline, &current);

    assert!(comparison.improvement_percent > 0.0);
    assert!(!comparison.regression_detected);
    assert!(comparison.significant_change);
    assert!(comparison.summary.contains("improvement") || comparison.summary.contains("faster"));
}

#[test]
fn test_performance_comparison_regression() {
    let config = PerformanceTestConfig::default();
    let manager = PerformanceTestManager::new(config);

    let baseline = BenchmarkResult {
        test_name: "test".to_string(),
        iterations: 10,
        total_duration: Duration::from_millis(100),
        average_duration: Duration::from_millis(10),
        min_duration: Duration::from_millis(8),
        max_duration: Duration::from_millis(12),
        percentiles: PercentileMetrics {
            p50: Duration::from_millis(10),
            p90: Duration::from_millis(11),
            p95: Duration::from_millis(11),
            p99: Duration::from_millis(12),
            p99_9: Duration::from_millis(12),
        },
        throughput: ThroughputMetrics {
            operations_per_second: 100.0,
            bytes_per_second: None,
            requests_per_second: None,
            concurrent_operations: 1,
        },
        resource_usage: ResourceUsageMetrics::default(),
        custom_metrics: std::collections::HashMap::new(),
    };

    let current = BenchmarkResult {
        test_name: "test".to_string(),
        iterations: 10,
        total_duration: Duration::from_millis(120),
        average_duration: Duration::from_millis(12), // 20% slower
        min_duration: Duration::from_millis(10),
        max_duration: Duration::from_millis(14),
        percentiles: PercentileMetrics {
            p50: Duration::from_millis(12),
            p90: Duration::from_millis(13),
            p95: Duration::from_millis(13),
            p99: Duration::from_millis(14),
            p99_9: Duration::from_millis(14),
        },
        throughput: ThroughputMetrics {
            operations_per_second: 83.3,
            bytes_per_second: None,
            requests_per_second: None,
            concurrent_operations: 1,
        },
        resource_usage: ResourceUsageMetrics::default(),
        custom_metrics: std::collections::HashMap::new(),
    };

    let comparison = manager.compare_results(&baseline, &current);

    assert!(comparison.improvement_percent < 0.0);
    assert!(comparison.regression_detected);
    assert!(comparison.significant_change);
    assert!(comparison.summary.contains("regression") || comparison.summary.contains("slower"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_test_basic() {
    let config = PerformanceTestConfig::default();
    let manager = PerformanceTestManager::new(config);

    let load_config = LoadTestConfig {
        test_name: "basic_load".to_string(),
        concurrent_users: 2,
        ramp_up_duration: Duration::from_millis(10),
        test_duration: Duration::from_millis(50),
        target_rps: None,
        think_time: Duration::from_millis(1),
    };

    let result = manager
        .load_test(load_config, || async {
            tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
            Ok(())
        })
        .await;

    assert!(result.is_ok());
    let load_result = result.unwrap();
    assert_eq!(load_result.test_name, "basic_load");
    assert!(load_result.total_requests > 0);
    assert!(load_result.successful_requests > 0);
    assert_eq!(load_result.failed_requests, 0);
    assert!(load_result.throughput > 0.0);
    assert_eq!(load_result.error_rate, 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_test_with_failures() {
    let config = PerformanceTestConfig::default();
    let manager = PerformanceTestManager::new(config);

    let load_config = LoadTestConfig {
        test_name: "failing_load".to_string(),
        concurrent_users: 2,
        ramp_up_duration: Duration::from_millis(10),
        test_duration: Duration::from_millis(50),
        target_rps: None,
        think_time: Duration::from_millis(1),
    };

    let counter = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));

    let result = manager
        .load_test(load_config, move || {
            let counter = Arc::clone(&counter);
            async move {
                let count = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                if count % 3 == 0 {
                    Err(anyhow::anyhow!("Simulated failure"))
                } else {
                    Ok(())
                }
            }
        })
        .await;

    assert!(result.is_ok());
    let load_result = result.unwrap();
    assert!(load_result.total_requests > 0);
    assert!(load_result.failed_requests > 0);
    assert!(load_result.error_rate > 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_report_generation() {
    let config = PerformanceTestConfig {
        test_name: "report_test".to_string(),
        warm_up_iterations: 1,
        measurement_iterations: 3,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: Vec::new(),
    };

    let manager = PerformanceTestManager::new(config);

    let _ = manager
        .benchmark(|| async {
            tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
            Ok(())
        })
        .await;

    let report = manager.generate_report().await;

    assert_eq!(report.total_benchmarks, 1);
    assert_eq!(report.results.len(), 1);
}

#[test]
fn test_performance_report_string_generation() {
    let report = PerformanceReport {
        total_benchmarks: 2,
        results: vec![
            BenchmarkResult {
                test_name: "test1".to_string(),
                iterations: 100,
                total_duration: Duration::from_millis(100),
                average_duration: Duration::from_millis(1),
                min_duration: Duration::from_micros(800),
                max_duration: Duration::from_millis(2),
                percentiles: PercentileMetrics {
                    p50: Duration::from_millis(1),
                    p90: Duration::from_millis(1),
                    p95: Duration::from_millis(2),
                    p99: Duration::from_millis(2),
                    p99_9: Duration::from_millis(2),
                },
                throughput: ThroughputMetrics {
                    operations_per_second: 1000.0,
                    bytes_per_second: None,
                    requests_per_second: None,
                    concurrent_operations: 1,
                },
                resource_usage: ResourceUsageMetrics::default(),
                custom_metrics: std::collections::HashMap::new(),
            },
            BenchmarkResult {
                test_name: "test2".to_string(),
                iterations: 50,
                total_duration: Duration::from_millis(50),
                average_duration: Duration::from_millis(1),
                min_duration: Duration::from_micros(900),
                max_duration: Duration::from_millis(2),
                percentiles: PercentileMetrics {
                    p50: Duration::from_millis(1),
                    p90: Duration::from_millis(1),
                    p95: Duration::from_millis(2),
                    p99: Duration::from_millis(2),
                    p99_9: Duration::from_millis(2),
                },
                throughput: ThroughputMetrics {
                    operations_per_second: 1000.0,
                    bytes_per_second: None,
                    requests_per_second: None,
                    concurrent_operations: 1,
                },
                resource_usage: ResourceUsageMetrics::default(),
                custom_metrics: std::collections::HashMap::new(),
            },
        ],
    };

    let report_string = report.to_report_string();

    assert!(report_string.contains("Performance Test Report"));
    assert!(report_string.contains("Total Benchmarks: 2"));
    assert!(report_string.contains("test1"));
    assert!(report_string.contains("test2"));
    assert!(report_string.contains("Iterations:"));
    assert!(report_string.contains("Average Duration:"));
    assert!(report_string.contains("Throughput:"));
}

#[test]
fn test_throughput_metrics_creation() {
    let metrics = ThroughputMetrics {
        operations_per_second: 1000.0,
        bytes_per_second: Some(1024 * 1024),
        requests_per_second: Some(500.0),
        concurrent_operations: 4,
    };

    assert_eq!(metrics.operations_per_second, 1000.0);
    assert_eq!(metrics.bytes_per_second, Some(1024 * 1024));
    assert_eq!(metrics.requests_per_second, Some(500.0));
    assert_eq!(metrics.concurrent_operations, 4);
}

#[test]
fn test_percentile_metrics_creation() {
    let metrics = PercentileMetrics {
        p50: Duration::from_millis(10),
        p90: Duration::from_millis(15),
        p95: Duration::from_millis(18),
        p99: Duration::from_millis(20),
        p99_9: Duration::from_millis(22),
    };

    assert_eq!(metrics.p50, Duration::from_millis(10));
    assert_eq!(metrics.p90, Duration::from_millis(15));
    assert_eq!(metrics.p95, Duration::from_millis(18));
    assert_eq!(metrics.p99, Duration::from_millis(20));
    assert_eq!(metrics.p99_9, Duration::from_millis(22));
}

#[test]
fn test_load_test_config_creation() {
    let config = LoadTestConfig {
        test_name: "load_test".to_string(),
        concurrent_users: 10,
        ramp_up_duration: Duration::from_secs(5),
        test_duration: Duration::from_secs(60),
        target_rps: Some(100.0),
        think_time: Duration::from_millis(500),
    };

    assert_eq!(config.test_name, "load_test");
    assert_eq!(config.concurrent_users, 10);
    assert_eq!(config.ramp_up_duration, Duration::from_secs(5));
    assert_eq!(config.test_duration, Duration::from_secs(60));
    assert_eq!(config.target_rps, Some(100.0));
    assert_eq!(config.think_time, Duration::from_millis(500));
}

#[test]
fn test_load_test_result_creation() {
    let result = LoadTestResult {
        test_name: "test".to_string(),
        total_requests: 1000,
        successful_requests: 950,
        failed_requests: 50,
        average_response_time: Duration::from_millis(10),
        error_rate: 5.0,
        throughput: 100.0,
        concurrent_users: 10,
        resource_usage: ResourceUsageMetrics::default(),
    };

    assert_eq!(result.test_name, "test");
    assert_eq!(result.total_requests, 1000);
    assert_eq!(result.successful_requests, 950);
    assert_eq!(result.failed_requests, 50);
    assert_eq!(result.error_rate, 5.0);
    assert_eq!(result.throughput, 100.0);
}

// Note: BenchmarkContext::new is private, so we test it indirectly through benchmark()
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_benchmark_with_custom_metrics() {
    let config = PerformanceTestConfig {
        test_name: "custom_metrics_test".to_string(),
        warm_up_iterations: 1,
        measurement_iterations: 3,
        concurrent_threads: 1,
        memory_profiling: false,
        cpu_profiling: false,
        custom_metrics: vec!["latency".to_string()],
    };

    let manager = PerformanceTestManager::new(config);

    let result = manager
        .benchmark(|| async {
            tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
            Ok(())
        })
        .await;

    assert!(result.is_ok());
    let benchmark_result = result.unwrap();
    // Custom metrics should be initialized even if not explicitly recorded
    assert_eq!(benchmark_result.test_name, "custom_metrics_test");
}
