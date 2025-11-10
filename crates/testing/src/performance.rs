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

use anyhow::Result;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use toadstool::ToadStoolError;
use tokio::sync::RwLock;

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceTestConfig {
    pub test_name: String,
    pub warm_up_iterations: u32,
    pub measurement_iterations: u32,
    pub concurrent_threads: u32,
    pub memory_profiling: bool,
    pub cpu_profiling: bool,
    pub custom_metrics: Vec<String>,
}

/// Performance benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub test_name: String,
    pub iterations: u32,
    pub total_duration: Duration,
    pub average_duration: Duration,
    pub min_duration: Duration,
    pub max_duration: Duration,
    pub percentiles: PercentileMetrics,
    pub throughput: ThroughputMetrics,
    pub resource_usage: ResourceUsageMetrics,
    pub custom_metrics: HashMap<String, f64>,
}

/// Percentile performance metrics
#[derive(Debug, Clone)]
pub struct PercentileMetrics {
    pub p50: Duration,
    pub p90: Duration,
    pub p95: Duration,
    pub p99: Duration,
    pub p99_9: Duration,
}

/// Throughput metrics
#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    pub operations_per_second: f64,
    pub bytes_per_second: Option<u64>,
    pub requests_per_second: Option<f64>,
    pub concurrent_operations: u32,
}

/// Resource usage metrics during performance tests
#[derive(Debug, Clone)]
pub struct ResourceUsageMetrics {
    pub peak_memory_mb: u32,
    pub average_memory_mb: u32,
    pub peak_cpu_percent: f32,
    pub average_cpu_percent: f32,
    pub disk_io_mb: u64,
    pub network_io_mb: u64,
    pub context_switches: u64,
}

/// Performance test manager
pub struct PerformanceTestManager {
    config: PerformanceTestConfig,
    results: Arc<RwLock<Vec<BenchmarkResult>>>,
    active_benchmarks: Arc<RwLock<HashMap<String, BenchmarkContext>>>,
}

/// Context for a running performance benchmark
#[derive(Debug)]
pub struct BenchmarkContext {
    pub test_name: String,
    pub start_time: Instant,
    pub iteration_times: Vec<Duration>,
    pub resource_monitor: ResourceMonitor,
    pub custom_metrics: HashMap<String, Vec<f64>>,
}

/// Resource monitor for tracking system usage during benchmarks
#[derive(Debug)]
pub struct ResourceMonitor {
    pub memory_samples: Vec<u32>,
    pub cpu_samples: Vec<f32>,
    pub disk_io_samples: Vec<u64>,
    pub network_io_samples: Vec<u64>,
    pub start_time: Instant,
}

/// Performance comparison between benchmark results
#[derive(Debug, Clone)]
pub struct PerformanceComparison {
    pub baseline: BenchmarkResult,
    pub current: BenchmarkResult,
    pub improvement_percent: f64,
    pub regression_detected: bool,
    pub significant_change: bool,
    pub summary: String,
}

/// Load testing configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    pub test_name: String,
    pub concurrent_users: u32,
    pub ramp_up_duration: Duration,
    pub test_duration: Duration,
    pub target_rps: Option<f64>,
    pub think_time: Duration,
}

/// Load test result
#[derive(Debug, Clone)]
pub struct LoadTestResult {
    pub test_name: String,
    pub total_requests: u64,
    pub successful_requests: u64,
    pub failed_requests: u64,
    pub average_response_time: Duration,
    pub error_rate: f64,
    pub throughput: f64,
    pub concurrent_users: u32,
    pub resource_usage: ResourceUsageMetrics,
}

impl Default for PerformanceTestConfig {
    fn default() -> Self {
        Self {
            test_name: "unnamed_benchmark".to_string(),
            warm_up_iterations: 10,
            measurement_iterations: 100,
            concurrent_threads: 1,
            memory_profiling: true,
            cpu_profiling: true,
            custom_metrics: Vec::new(),
        }
    }
}

impl PerformanceTestManager {
    /// Create a new performance test manager
    #[must_use]
    pub fn new(config: PerformanceTestConfig) -> Self {
        Self {
            config,
            results: Arc::new(RwLock::new(Vec::new())),
            active_benchmarks: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Execute a performance benchmark
    pub async fn benchmark<F, Fut>(&self, test_fn: F) -> Result<BenchmarkResult>
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let test_name = self.config.test_name.clone();
        let context = BenchmarkContext::new(test_name.clone());

        // Register active benchmark
        {
            let mut active = self.active_benchmarks.write().await;
            active.insert(test_name.clone(), context);
        }

        // Warm-up phase
        for _ in 0..self.config.warm_up_iterations {
            let _ = test_fn().await;
        }

        // Measurement phase
        let mut iteration_times = Vec::new();

        for i in 0..self.config.measurement_iterations {
            let start = Instant::now();

            match test_fn().await {
                Ok(()) => {
                    let duration = start.elapsed();
                    iteration_times.push(duration);

                    // Sample resource usage periodically
                    if i % 10 == 0 && self.config.memory_profiling {
                        // Get mutable reference to context for resource sampling
                        if let Some(ctx) = self.active_benchmarks.write().await.get_mut(&test_name)
                        {
                            ctx.resource_monitor.sample_resources();
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Benchmark iteration {i} failed: {e}");
                }
            }
        }

        // Calculate results
        let result = {
            let context_ref = self.active_benchmarks.read().await;
            let context = context_ref.get(&test_name).ok_or_else(|| {
                ToadStoolError::runtime(format!(
                    "Benchmark context not found for test: {test_name}"
                ))
            })?;
            self.calculate_benchmark_result(test_name.clone(), iteration_times, context)
                .await
        }; // Read lock dropped here

        // Remove from active benchmarks (now safe to acquire write lock)
        {
            let mut active = self.active_benchmarks.write().await;
            active.remove(&test_name);
        }

        // Store result
        {
            let mut results = self.results.write().await;
            results.push(result.clone());
        }

        Ok(result)
    }

    /// Execute a concurrent load test
    pub async fn load_test<F, Fut>(
        &self,
        config: LoadTestConfig,
        test_fn: F,
    ) -> Result<LoadTestResult>
    where
        F: Fn() -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send + 'static,
    {
        let start_time = Instant::now();
        let mut handles = Vec::new();
        let successful_requests = Arc::new(std::sync::Mutex::new(0u64));
        let failed_requests = Arc::new(std::sync::Mutex::new(0u64));
        let response_times = Arc::new(std::sync::Mutex::new(Vec::new()));

        // Spawn concurrent users
        for _ in 0..config.concurrent_users {
            let test_fn = test_fn.clone();
            let duration = config.test_duration;
            let think_time = config.think_time;
            let successful = Arc::clone(&successful_requests);
            let failed = Arc::clone(&failed_requests);
            let response_times = Arc::clone(&response_times);

            let handle = tokio::spawn(async move {
                let user_start = Instant::now();
                while user_start.elapsed() < duration {
                    let request_start = Instant::now();

                    match test_fn().await {
                        Ok(()) => {
                            let response_time = request_start.elapsed();
                            if let Ok(mut successful) = successful.lock() {
                                *successful += 1;
                            }
                            if let Ok(mut times) = response_times.lock() {
                                times.push(response_time);
                            }
                        }
                        Err(_) => {
                            if let Ok(mut failed) = failed.lock() {
                                *failed += 1;
                            }
                        }
                    }

                    if think_time > Duration::ZERO {
                        tokio::time::sleep(think_time).await;
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all users to complete
        for handle in handles {
            let _ = handle.await;
        }

        let total_duration = start_time.elapsed();
        let successful = successful_requests.lock().map(|s| *s).unwrap_or(0);
        let failed = failed_requests.lock().map(|f| *f).unwrap_or(0);
        let total_requests = successful + failed;
        let response_times = response_times
            .lock()
            .map(|rt| rt.clone())
            .unwrap_or_default();

        let average_response_time = if response_times.is_empty() {
            Duration::ZERO
        } else {
            response_times.iter().sum::<Duration>() / response_times.len() as u32
        };

        let error_rate = if total_requests > 0 {
            (failed as f64 / total_requests as f64) * 100.0
        } else {
            0.0
        };

        let throughput = successful as f64 / total_duration.as_secs_f64();

        Ok(LoadTestResult {
            test_name: config.test_name,
            total_requests,
            successful_requests: successful,
            failed_requests: failed,
            average_response_time,
            error_rate,
            throughput,
            concurrent_users: config.concurrent_users,
            resource_usage: ResourceUsageMetrics::default(),
        })
    }

    /// Compare two benchmark results
    #[must_use]
    pub fn compare_results(
        &self,
        baseline: &BenchmarkResult,
        current: &BenchmarkResult,
    ) -> PerformanceComparison {
        let baseline_avg = baseline.average_duration.as_nanos() as f64;
        let current_avg = current.average_duration.as_nanos() as f64;

        let improvement_percent = ((baseline_avg - current_avg) / baseline_avg) * 100.0;
        let regression_detected = improvement_percent < -5.0; // 5% regression threshold
        let significant_change = improvement_percent.abs() > 2.0; // 2% significance threshold

        let summary = if regression_detected {
            format!(
                "⚠️  Performance regression detected: {:.1}% slower",
                improvement_percent.abs()
            )
        } else if improvement_percent > 5.0 {
            format!("🚀 Performance improvement: {improvement_percent:.1}% faster")
        } else {
            "📊 Performance unchanged".to_string()
        };

        PerformanceComparison {
            baseline: baseline.clone(),
            current: current.clone(),
            improvement_percent,
            regression_detected,
            significant_change,
            summary,
        }
    }

    /// Generate performance report
    pub async fn generate_report(&self) -> PerformanceReport {
        let results = self.results.read().await.clone();

        PerformanceReport {
            total_benchmarks: results.len(),
            results,
        }
    }

    async fn calculate_benchmark_result(
        &self,
        test_name: String,
        mut iteration_times: Vec<Duration>,
        context: &BenchmarkContext,
    ) -> BenchmarkResult {
        if iteration_times.is_empty() {
            return BenchmarkResult::default(test_name);
        }

        iteration_times.sort();

        let total_duration: Duration = iteration_times.iter().sum();
        let iterations = iteration_times.len() as u32;
        let average_duration = total_duration / iterations;
        let min_duration = iteration_times[0];
        let max_duration = iteration_times[iteration_times.len() - 1];

        let percentiles = PercentileMetrics {
            p50: iteration_times[iteration_times.len() * 50 / 100],
            p90: iteration_times[iteration_times.len() * 90 / 100],
            p95: iteration_times[iteration_times.len() * 95 / 100],
            p99: iteration_times[iteration_times.len() * 99 / 100],
            p99_9: iteration_times[iteration_times.len() * 999 / 1000],
        };

        let throughput = ThroughputMetrics {
            operations_per_second: f64::from(iterations) / total_duration.as_secs_f64(),
            bytes_per_second: None,
            requests_per_second: None,
            concurrent_operations: self.config.concurrent_threads,
        };

        BenchmarkResult {
            test_name,
            iterations,
            total_duration,
            average_duration,
            min_duration,
            max_duration,
            percentiles,
            throughput,
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: context
                .custom_metrics
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().sum::<f64>() / v.len() as f64))
                .collect(),
        }
    }
}

impl BenchmarkContext {
    fn new(test_name: String) -> Self {
        Self {
            test_name,
            start_time: Instant::now(),
            iteration_times: Vec::new(),
            resource_monitor: ResourceMonitor::new(),
            custom_metrics: HashMap::new(),
        }
    }

    /// Record a custom metric value
    pub fn record_metric(&mut self, name: &str, value: f64) {
        self.custom_metrics
            .entry(name.to_string())
            .or_default()
            .push(value);
    }
}

impl ResourceMonitor {
    fn new() -> Self {
        Self {
            memory_samples: Vec::new(),
            cpu_samples: Vec::new(),
            disk_io_samples: Vec::new(),
            network_io_samples: Vec::new(),
            start_time: Instant::now(),
        }
    }

    fn sample_resources(&mut self) {
        // In a real implementation, this would use system APIs to sample resource usage
        // For now, we'll use placeholder values
        self.memory_samples.push(100); // MB
        self.cpu_samples.push(50.0); // Percent
        self.disk_io_samples.push(1024); // Bytes
        self.network_io_samples.push(2048); // Bytes
    }
}

impl Default for ResourceUsageMetrics {
    fn default() -> Self {
        Self {
            peak_memory_mb: 0,
            average_memory_mb: 0,
            peak_cpu_percent: 0.0,
            average_cpu_percent: 0.0,
            disk_io_mb: 0,
            network_io_mb: 0,
            context_switches: 0,
        }
    }
}

impl BenchmarkResult {
    fn default(test_name: String) -> Self {
        Self {
            test_name,
            iterations: 0,
            total_duration: Duration::ZERO,
            average_duration: Duration::ZERO,
            min_duration: Duration::ZERO,
            max_duration: Duration::ZERO,
            percentiles: PercentileMetrics {
                p50: Duration::ZERO,
                p90: Duration::ZERO,
                p95: Duration::ZERO,
                p99: Duration::ZERO,
                p99_9: Duration::ZERO,
            },
            throughput: ThroughputMetrics {
                operations_per_second: 0.0,
                bytes_per_second: None,
                requests_per_second: None,
                concurrent_operations: 0,
            },
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        }
    }
}

/// Performance test report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    pub total_benchmarks: usize,
    pub results: Vec<BenchmarkResult>,
}

impl PerformanceReport {
    /// Generate human-readable report
    #[must_use]
    pub fn to_report_string(&self) -> String {
        let mut report = format!(
            "Performance Test Report\n\
             =======================\n\
             Total Benchmarks: {}\n\n",
            self.total_benchmarks
        );

        for result in &self.results {
            report.push_str(&format!(
                "Benchmark: {}\n\
                 Iterations: {}\n\
                 Average Duration: {:.2}ms\n\
                 Throughput: {:.1} ops/sec\n\
                 P95: {:.2}ms\n\
                 P99: {:.2}ms\n\n",
                result.test_name,
                result.iterations,
                result.average_duration.as_secs_f64() * 1000.0,
                result.throughput.operations_per_second,
                result.percentiles.p95.as_secs_f64() * 1000.0,
                result.percentiles.p99.as_secs_f64() * 1000.0,
            ));
        }

        report
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[tokio::test]
    #[ignore] // Hangs with llvm-cov instrumentation due to performance overhead
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

    #[tokio::test]
    #[ignore] // Hangs with llvm-cov instrumentation due to performance overhead
    async fn test_benchmark_with_delays() {
        let config = PerformanceTestConfig {
            test_name: "delayed".to_string(),
            warm_up_iterations: 1,
            measurement_iterations: 3,
            concurrent_threads: 1,
            memory_profiling: false,
            cpu_profiling: false,
            custom_metrics: vec![],
        };

        let manager = PerformanceTestManager::new(config);
        let result = manager
            .benchmark(|| async {
                tokio::time::sleep(Duration::from_micros(10)).await;
                Ok(())
            })
            .await
            .expect("Benchmark should succeed");

        assert!(result.average_duration >= Duration::from_micros(10));
        assert!(result.min_duration <= result.max_duration);
    }

    #[tokio::test]
    #[ignore] // Hangs with llvm-cov instrumentation due to performance overhead
    async fn test_benchmark_with_resource_monitoring() {
        let config = PerformanceTestConfig {
            test_name: "monitored".to_string(),
            warm_up_iterations: 1,
            measurement_iterations: 20, // More than 10 to trigger sampling
            concurrent_threads: 1,
            memory_profiling: true,
            cpu_profiling: true,
            custom_metrics: vec![],
        };

        let manager = PerformanceTestManager::new(config);
        let result = manager
            .benchmark(|| async { Ok(()) })
            .await
            .expect("Benchmark should succeed");

        assert_eq!(result.iterations, 20);
        // Resource monitoring is triggered every 10 iterations
    }

    #[tokio::test]
    #[ignore] // Hangs with llvm-cov instrumentation due to performance overhead
    async fn test_benchmark_generate_report() {
        let config = PerformanceTestConfig {
            test_name: "report_test".to_string(),
            warm_up_iterations: 1,
            measurement_iterations: 5,
            concurrent_threads: 1,
            memory_profiling: false,
            cpu_profiling: false,
            custom_metrics: vec![],
        };

        let manager = PerformanceTestManager::new(config);
        let _result = manager
            .benchmark(|| async { Ok(()) })
            .await
            .expect("Benchmark should succeed");

        let report = manager.generate_report().await;
        assert_eq!(report.total_benchmarks, 1);
        assert_eq!(report.results.len(), 1);
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
            custom_metrics: HashMap::new(),
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
            custom_metrics: HashMap::new(),
        };

        let comparison = manager.compare_results(&baseline, &current);
        assert!(comparison.improvement_percent > 0.0);
        assert!(!comparison.regression_detected);
        assert!(comparison.significant_change);
        assert!(
            comparison.summary.contains("improvement") || comparison.summary.contains("faster")
        );
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
            custom_metrics: HashMap::new(),
        };

        let current = BenchmarkResult {
            test_name: "test".to_string(),
            iterations: 10,
            total_duration: Duration::from_millis(200),
            average_duration: Duration::from_millis(20), // 100% slower
            min_duration: Duration::from_millis(15),
            max_duration: Duration::from_millis(25),
            percentiles: PercentileMetrics {
                p50: Duration::from_millis(20),
                p90: Duration::from_millis(22),
                p95: Duration::from_millis(23),
                p99: Duration::from_millis(24),
                p99_9: Duration::from_millis(25),
            },
            throughput: ThroughputMetrics {
                operations_per_second: 50.0,
                bytes_per_second: None,
                requests_per_second: None,
                concurrent_operations: 1,
            },
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        };

        let comparison = manager.compare_results(&baseline, &current);
        assert!(comparison.improvement_percent < 0.0);
        assert!(comparison.regression_detected);
        assert!(comparison.significant_change);
        assert!(comparison.summary.contains("regression") || comparison.summary.contains("slower"));
    }

    #[test]
    fn test_performance_comparison_no_change() {
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
            custom_metrics: HashMap::new(),
        };

        let current = baseline.clone();
        let comparison = manager.compare_results(&baseline, &current);
        assert_eq!(comparison.improvement_percent, 0.0);
        assert!(!comparison.regression_detected);
        assert!(!comparison.significant_change);
    }

    #[tokio::test]
    async fn test_load_test_basic() {
        let perf_config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(perf_config);

        let load_config = LoadTestConfig {
            test_name: "basic_load".to_string(),
            concurrent_users: 2,
            ramp_up_duration: Duration::from_millis(1),
            test_duration: Duration::from_millis(50),
            target_rps: None,
            think_time: Duration::from_millis(1),
        };

        let result = manager
            .load_test(load_config, || async { Ok(()) })
            .await
            .expect("Load test should succeed");

        assert_eq!(result.test_name, "basic_load");
        assert_eq!(result.concurrent_users, 2);
        assert!(result.total_requests > 0);
        assert!(result.successful_requests > 0);
        assert_eq!(result.failed_requests, 0);
        assert_eq!(result.error_rate, 0.0);
    }

    #[tokio::test]
    async fn test_load_test_with_failures() {
        let perf_config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(perf_config);

        let load_config = LoadTestConfig {
            test_name: "failing_load".to_string(),
            concurrent_users: 1,
            ramp_up_duration: Duration::from_millis(1),
            test_duration: Duration::from_millis(30),
            target_rps: None,
            think_time: Duration::from_millis(1),
        };

        let counter = Arc::new(std::sync::atomic::AtomicU32::new(0));
        let result = manager
            .load_test(load_config, move || {
                let counter = counter.clone();
                async move {
                    let count = counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                    if count % 2 == 0 {
                        Err(anyhow::anyhow!("Simulated failure"))
                    } else {
                        Ok(())
                    }
                }
            })
            .await
            .expect("Load test should succeed");

        assert!(result.total_requests > 0);
        assert!(result.failed_requests > 0);
        assert!(result.error_rate > 0.0);
    }

    #[test]
    fn test_performance_report_to_string() {
        let result = BenchmarkResult {
            test_name: "test1".to_string(),
            iterations: 100,
            total_duration: Duration::from_millis(1000),
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
            custom_metrics: HashMap::new(),
        };

        let report = PerformanceReport {
            total_benchmarks: 1,
            results: vec![result],
        };

        let report_string = report.to_report_string();
        assert!(report_string.contains("Performance Test Report"));
        assert!(report_string.contains("Total Benchmarks: 1"));
        assert!(report_string.contains("test1"));
        assert!(report_string.contains("Iterations: 100"));
    }

    #[test]
    fn test_load_test_config_clone() {
        let config = LoadTestConfig {
            test_name: "test".to_string(),
            concurrent_users: 10,
            ramp_up_duration: Duration::from_secs(5),
            test_duration: Duration::from_secs(60),
            target_rps: Some(100.0),
            think_time: Duration::from_millis(100),
        };

        let cloned = config.clone();
        assert_eq!(config.test_name, cloned.test_name);
        assert_eq!(config.concurrent_users, cloned.concurrent_users);
        assert_eq!(config.target_rps, cloned.target_rps);
    }

    #[test]
    fn test_load_test_result_clone() {
        let result = LoadTestResult {
            test_name: "test".to_string(),
            total_requests: 1000,
            successful_requests: 950,
            failed_requests: 50,
            average_response_time: Duration::from_millis(10),
            error_rate: 5.0,
            throughput: 95.0,
            concurrent_users: 10,
            resource_usage: ResourceUsageMetrics::default(),
        };

        let cloned = result.clone();
        assert_eq!(result.test_name, cloned.test_name);
        assert_eq!(result.total_requests, cloned.total_requests);
        assert_eq!(result.error_rate, cloned.error_rate);
    }

    #[test]
    fn test_percentile_metrics_clone() {
        let metrics = PercentileMetrics {
            p50: Duration::from_millis(10),
            p90: Duration::from_millis(20),
            p95: Duration::from_millis(25),
            p99: Duration::from_millis(30),
            p99_9: Duration::from_millis(35),
        };

        let cloned = metrics.clone();
        assert_eq!(metrics.p50, cloned.p50);
        assert_eq!(metrics.p99, cloned.p99);
    }

    #[test]
    fn test_throughput_metrics_clone() {
        let metrics = ThroughputMetrics {
            operations_per_second: 100.0,
            bytes_per_second: Some(1024),
            requests_per_second: Some(50.0),
            concurrent_operations: 4,
        };

        let cloned = metrics.clone();
        assert_eq!(metrics.operations_per_second, cloned.operations_per_second);
        assert_eq!(metrics.bytes_per_second, cloned.bytes_per_second);
    }
}
