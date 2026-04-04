// SPDX-License-Identifier: AGPL-3.0-only
// ToadStool - Universal Compute Platform
// Copyright (C) 2025 ToadStool Development Team
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

//! Performance testing utilities
//!
//! This module provides comprehensive performance testing infrastructure including:
//! - Benchmark execution and measurement
//! - Resource usage monitoring
//! - Load testing capabilities
//! - Performance comparison and reporting
//!
//! # Architecture
//!
//! The module is organized into focused submodules:
//! - `types`: Core data structures and configuration
//! - `context`: Runtime benchmark context and resource monitoring
//! - `manager`: Test execution logic and benchmark orchestration
//! - `reporting`: Result formatting and report generation
//!
//! # Example
//!
//! ```rust,ignore
//! use toadstool_testing::performance::{PerformanceTestConfig, PerformanceTestManager};
//!
//! let config = PerformanceTestConfig {
//!     test_name: "my_benchmark".to_string(),
//!     warm_up_iterations: 10,
//!     measurement_iterations: 100,
//!     ..Default::default()
//! };
//!
//! let manager = PerformanceTestManager::new(config);
//! let result = manager.benchmark(|| async {
//!     // Your test code here
//!     Ok(())
//! }).await?;
//! ```

mod context;
mod manager;
mod reporting;
mod types;

// Re-export public API
pub use context::{BenchmarkContext, ResourceMonitor};
pub use manager::PerformanceTestManager;
pub use reporting::PerformanceReport;
pub use types::{
    BenchmarkResult, LoadTestConfig, LoadTestResult, PercentileMetrics, PerformanceComparison,
    PerformanceTestConfig, ResourceUsageMetrics, ThroughputMetrics,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::Duration;

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
    fn test_performance_test_config_clone() {
        let config = PerformanceTestConfig {
            test_name: "test".to_string(),
            warm_up_iterations: 5,
            measurement_iterations: 50,
            concurrent_threads: 2,
            memory_profiling: false,
            cpu_profiling: false,
            custom_metrics: vec!["metric1".to_string()],
        };
        let cloned = config.clone();
        assert_eq!(config.test_name, cloned.test_name);
        assert_eq!(config.warm_up_iterations, cloned.warm_up_iterations);
    }

    #[test]
    fn test_benchmark_result_default() {
        let result = BenchmarkResult::default("test".to_string());
        assert_eq!(result.test_name, "test");
        assert_eq!(result.iterations, 0);
        assert_eq!(result.total_duration, Duration::ZERO);
    }

    #[test]
    fn test_benchmark_result_clone() {
        let result = BenchmarkResult {
            test_name: "test".to_string(),
            iterations: 10,
            total_duration: Duration::from_millis(100),
            average_duration: Duration::from_millis(10),
            min_duration: Duration::from_millis(5),
            max_duration: Duration::from_millis(15),
            percentiles: PercentileMetrics {
                p50: Duration::from_millis(10),
                p90: Duration::from_millis(12),
                p95: Duration::from_millis(13),
                p99: Duration::from_millis(14),
                p99_9: Duration::from_millis(15),
            },
            throughput: ThroughputMetrics {
                operations_per_second: 100.0,
                bytes_per_second: Some(1024),
                requests_per_second: Some(50.0),
                concurrent_operations: 1,
            },
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        };
        let cloned = result.clone();
        assert_eq!(result.test_name, cloned.test_name);
        assert_eq!(result.iterations, cloned.iterations);
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact comparison intended in this context"
    )] // test values are exact literals
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
    fn test_resource_monitor_new() {
        let monitor = ResourceMonitor::new();
        assert!(monitor.memory_samples.is_empty());
        assert!(monitor.cpu_samples.is_empty());
        assert!(monitor.disk_io_samples.is_empty());
        assert!(monitor.network_io_samples.is_empty());
    }

    #[test]
    #[expect(
        clippy::float_cmp,
        reason = "exact comparison intended in this context"
    )] // test values are exact literals
    fn test_resource_monitor_sample() {
        let mut monitor = ResourceMonitor::new();
        monitor.sample_resources();
        assert_eq!(monitor.memory_samples.len(), 1);
        assert_eq!(monitor.cpu_samples.len(), 1);
        assert_eq!(monitor.disk_io_samples.len(), 1);
        assert_eq!(monitor.network_io_samples.len(), 1);
        // Check placeholder values
        assert_eq!(monitor.memory_samples[0], 100);
        assert_eq!(monitor.cpu_samples[0], 50.0);
        assert_eq!(monitor.disk_io_samples[0], 1024);
        assert_eq!(monitor.network_io_samples[0], 2048);
    }

    #[test]
    fn test_benchmark_context_new() {
        let context = BenchmarkContext::new("test".to_string());
        assert_eq!(context.test_name, "test");
        assert!(context.iteration_times.is_empty());
        assert!(context.custom_metrics.is_empty());
    }

    #[test]
    fn test_benchmark_context_record_metric() {
        let mut context = BenchmarkContext::new("test".to_string());
        context.record_metric("latency", 10.5);
        context.record_metric("latency", 12.3);
        context.record_metric("throughput", 100.0);

        assert_eq!(context.custom_metrics.len(), 2);
        assert_eq!(context.custom_metrics.get("latency").unwrap().len(), 2);
        assert_eq!(context.custom_metrics.get("throughput").unwrap().len(), 1);
    }

    #[test]
    fn test_performance_test_manager_new() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);
        // Just verify creation succeeds
        drop(manager);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[cfg_attr(
        not(feature = "slow-tests"),
        ignore = "Hangs with llvm-cov instrumentation due to performance overhead"
    )]
    async fn test_benchmark_simple() {
        let config = PerformanceTestConfig {
            test_name: "simple".to_string(),
            warm_up_iterations: 1,
            measurement_iterations: 5,
            concurrent_threads: 1,
            memory_profiling: false,
            cpu_profiling: false,
            custom_metrics: vec![],
        };

        let manager = PerformanceTestManager::new(config);
        let result = manager
            .benchmark(|| async { Ok(()) })
            .await
            .expect("Benchmark should succeed");

        assert_eq!(result.test_name, "simple");
        assert_eq!(result.iterations, 5);
        assert!(result.total_duration > Duration::ZERO);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[cfg_attr(
        not(feature = "slow-tests"),
        ignore = "Hangs with llvm-cov instrumentation due to performance overhead"
    )]
    async fn test_benchmark_with_resource_monitoring() {
        let config = PerformanceTestConfig {
            test_name: "with_monitoring".to_string(),
            warm_up_iterations: 1,
            measurement_iterations: 20,
            concurrent_threads: 1,
            memory_profiling: true,
            cpu_profiling: true,
            custom_metrics: vec![],
        };

        let manager = PerformanceTestManager::new(config);
        let result = manager
            .benchmark(|| async {
                let _ = (0..1000u64).fold(0u64, |a, b| a.wrapping_add(b));
                tokio::task::yield_now().await;
                Ok(())
            })
            .await
            .expect("Benchmark should succeed");

        assert_eq!(result.test_name, "with_monitoring");
        assert_eq!(result.iterations, 20);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    #[cfg_attr(
        not(feature = "slow-tests"),
        ignore = "Hangs with llvm-cov instrumentation due to performance overhead"
    )]
    async fn test_load_test_simple() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);

        let load_config = LoadTestConfig {
            test_name: "load_test".to_string(),
            concurrent_users: 2,
            ramp_up_duration: Duration::from_millis(10),
            test_duration: Duration::from_millis(100),
            target_rps: None,
            think_time: Duration::ZERO,
        };

        let result = manager
            .load_test(load_config, || async { Ok(()) })
            .await
            .expect("Load test should succeed");

        assert_eq!(result.test_name, "load_test");
        assert!(result.total_requests > 0);
        assert_eq!(result.concurrent_users, 2);
    }

    #[test]
    fn test_compare_results_improvement() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);

        let baseline = BenchmarkResult {
            test_name: "test".to_string(),
            iterations: 100,
            total_duration: Duration::from_secs(10),
            average_duration: Duration::from_millis(100),
            min_duration: Duration::from_millis(90),
            max_duration: Duration::from_millis(110),
            percentiles: PercentileMetrics::default(),
            throughput: ThroughputMetrics::default(),
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        };

        let current = BenchmarkResult {
            test_name: "test".to_string(),
            iterations: 100,
            total_duration: Duration::from_secs(8),
            average_duration: Duration::from_millis(80),
            min_duration: Duration::from_millis(70),
            max_duration: Duration::from_millis(90),
            percentiles: PercentileMetrics::default(),
            throughput: ThroughputMetrics::default(),
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        };

        let comparison = manager.compare_results(&baseline, &current);
        assert!(comparison.improvement_percent > 0.0);
        assert!(!comparison.regression_detected);
        assert!(comparison.significant_change);
    }

    #[test]
    fn test_compare_results_regression() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);

        let baseline = BenchmarkResult {
            test_name: "test".to_string(),
            iterations: 100,
            total_duration: Duration::from_secs(10),
            average_duration: Duration::from_millis(100),
            min_duration: Duration::from_millis(90),
            max_duration: Duration::from_millis(110),
            percentiles: PercentileMetrics::default(),
            throughput: ThroughputMetrics::default(),
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        };

        let current = BenchmarkResult {
            test_name: "test".to_string(),
            iterations: 100,
            total_duration: Duration::from_secs(12),
            average_duration: Duration::from_millis(120),
            min_duration: Duration::from_millis(110),
            max_duration: Duration::from_millis(130),
            percentiles: PercentileMetrics::default(),
            throughput: ThroughputMetrics::default(),
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        };

        let comparison = manager.compare_results(&baseline, &current);
        assert!(comparison.improvement_percent < 0.0);
        assert!(comparison.regression_detected);
        assert!(comparison.significant_change);
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_generate_report() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);

        let report = manager.generate_report().await;
        assert_eq!(report.total_benchmarks, 0);
        assert!(report.results.is_empty());
    }

    #[test]
    fn test_performance_report_to_string() {
        let result = BenchmarkResult {
            test_name: "test".to_string(),
            iterations: 100,
            total_duration: Duration::from_secs(10),
            average_duration: Duration::from_millis(100),
            min_duration: Duration::from_millis(90),
            max_duration: Duration::from_millis(110),
            percentiles: PercentileMetrics {
                p50: Duration::from_millis(100),
                p90: Duration::from_millis(105),
                p95: Duration::from_millis(108),
                p99: Duration::from_millis(110),
                p99_9: Duration::from_millis(110),
            },
            throughput: ThroughputMetrics {
                operations_per_second: 10.0,
                bytes_per_second: None,
                requests_per_second: None,
                concurrent_operations: 1,
            },
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        };

        let report = PerformanceReport {
            total_benchmarks: 1,
            results: vec![result],
        };

        let report_string = report.to_report_string();
        assert!(report_string.contains("Performance Test Report"));
        assert!(report_string.contains("Total Benchmarks: 1"));
        assert!(report_string.contains("Benchmark: test"));
        assert!(report_string.contains("Iterations: 100"));
    }
}
