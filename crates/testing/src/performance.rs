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
        let context_ref = self.active_benchmarks.read().await;
        let context = context_ref.get(&test_name).unwrap();
        let result = self
            .calculate_benchmark_result(test_name.clone(), iteration_times, context)
            .await;

        // Remove from active benchmarks
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
                            *successful.lock().unwrap() += 1;
                            response_times.lock().unwrap().push(response_time);
                        }
                        Err(_) => {
                            *failed.lock().unwrap() += 1;
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
        let successful = *successful_requests.lock().unwrap();
        let failed = *failed_requests.lock().unwrap();
        let total_requests = successful + failed;
        let response_times = response_times.lock().unwrap();

        let average_response_time = if !response_times.is_empty() {
            response_times.iter().sum::<Duration>() / response_times.len() as u32
        } else {
            Duration::ZERO
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
            operations_per_second: iterations as f64 / total_duration.as_secs_f64(),
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
    pub fn to_string(&self) -> String {
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
