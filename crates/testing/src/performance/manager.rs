// SPDX-License-Identifier: AGPL-3.0-or-later
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

//! Performance test manager and execution logic

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, Instant};
use toadstool::ToadStoolError;
use toadstool::ToadStoolResult as Result;

use super::context::BenchmarkContext;
use super::reporting::PerformanceReport;
use super::types::*;

/// Performance test manager
pub struct PerformanceTestManager {
    config: PerformanceTestConfig,
    results: Arc<RwLock<Vec<BenchmarkResult>>>,
    active_benchmarks: Arc<RwLock<HashMap<String, BenchmarkContext>>>,
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
            let mut active = self
                .active_benchmarks
                .write()
                .unwrap_or_else(|e| e.into_inner());
            active.insert(test_name.clone(), context);
        }

        // Warm-up phase
        for _ in 0..self.config.warm_up_iterations {
            let _ = test_fn().await;
        }

        // Measurement phase — uses tokio::time::Instant so tests can use
        // start_paused = true + tokio::time::advance() for deterministic timing.
        let mut iteration_times = Vec::new();

        for i in 0..self.config.measurement_iterations {
            let start = tokio::time::Instant::now();

            match test_fn().await {
                Ok(()) => {
                    let duration = start.elapsed();
                    iteration_times.push(duration);

                    // Sample resource usage periodically
                    if i % 10 == 0 && self.config.memory_profiling {
                        // Get mutable reference to context for resource sampling
                        if let Some(ctx) = self
                            .active_benchmarks
                            .write()
                            .unwrap_or_else(|e| e.into_inner())
                            .get_mut(&test_name)
                        {
                            ctx.resource_monitor.sample_resources();
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(iteration = i, error = %e, "benchmark iteration failed");
                }
            }
        }

        // Calculate results (extract data and drop lock before await)
        let custom_metrics = self
            .active_benchmarks
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .get(&test_name)
            .ok_or_else(|| {
                ToadStoolError::runtime(format!(
                    "Benchmark context not found for test: {test_name}"
                ))
            })?
            .custom_metrics
            .clone();
        let result = self
            .calculate_benchmark_result(test_name.clone(), iteration_times, custom_metrics)
            .await;

        // Remove from active benchmarks (now safe to acquire write lock)
        {
            let mut active = self
                .active_benchmarks
                .write()
                .unwrap_or_else(|e| e.into_inner());
            active.remove(&test_name);
        }

        // Store result
        {
            let mut results = self.results.write().unwrap_or_else(|e| e.into_inner());
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
        let results = self
            .results
            .read()
            .unwrap_or_else(|e| e.into_inner())
            .clone();

        PerformanceReport {
            total_benchmarks: results.len(),
            results,
        }
    }

    async fn calculate_benchmark_result(
        &self,
        test_name: String,
        mut iteration_times: Vec<Duration>,
        custom_metrics: HashMap<String, Vec<f64>>,
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
            custom_metrics: custom_metrics
                .iter()
                .map(|(k, v)| (k.clone(), v.iter().sum::<f64>() / v.len() as f64))
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::{LoadTestConfig, PerformanceTestConfig};

    #[tokio::test]
    async fn test_benchmark_basic() {
        let config = PerformanceTestConfig {
            test_name: "quick_bench".to_string(),
            warm_up_iterations: 1,
            measurement_iterations: 3,
            memory_profiling: false,
            ..Default::default()
        };
        let manager = PerformanceTestManager::new(config);
        let result = manager
            .benchmark(|| async { Ok(()) })
            .await
            .expect("benchmark should succeed");
        assert_eq!(result.test_name, "quick_bench");
        assert!(result.iterations >= 1);
    }

    #[tokio::test]
    async fn test_load_test_short_duration() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);
        let load_config = LoadTestConfig {
            test_name: "short_load".to_string(),
            concurrent_users: 2,
            test_duration: std::time::Duration::from_millis(50),
            think_time: std::time::Duration::ZERO,
            ramp_up_duration: std::time::Duration::ZERO,
            target_rps: None,
        };
        let result = manager
            .load_test(load_config, || async { Ok(()) })
            .await
            .expect("load test should succeed");
        assert_eq!(result.test_name, "short_load");
        assert_eq!(result.concurrent_users, 2);
    }

    #[test]
    fn test_compare_results_improvement() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);
        let mut baseline = BenchmarkResult::default("b");
        baseline.average_duration = std::time::Duration::from_millis(100);
        let mut current = BenchmarkResult::default("c");
        current.average_duration = std::time::Duration::from_millis(80);
        let comparison = manager.compare_results(&baseline, &current);
        assert!(comparison.improvement_percent > 0.0);
        assert!(!comparison.regression_detected);
    }

    #[test]
    fn test_compare_results_regression() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);
        let mut baseline = BenchmarkResult::default("b");
        baseline.average_duration = std::time::Duration::from_millis(100);
        let mut current = BenchmarkResult::default("c");
        current.average_duration = std::time::Duration::from_millis(120);
        let comparison = manager.compare_results(&baseline, &current);
        assert!(comparison.improvement_percent < 0.0);
        assert!(comparison.regression_detected);
    }

    #[tokio::test]
    async fn test_generate_report() {
        let config = PerformanceTestConfig::default();
        let manager = PerformanceTestManager::new(config);
        let report = manager.generate_report().await;
        assert_eq!(report.total_benchmarks, 0);
    }
}
