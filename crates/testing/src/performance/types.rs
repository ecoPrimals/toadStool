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

//! Performance testing data types and configurations

use std::collections::HashMap;
use std::time::Duration;

/// Performance test configuration
#[derive(Debug, Clone)]
pub struct PerformanceTestConfig {
    /// Name of the benchmark
    pub test_name: String,
    /// Iterations to run before measurement (warm-up)
    pub warm_up_iterations: u32,
    /// Iterations used for timing measurement
    pub measurement_iterations: u32,
    /// Number of threads for concurrent benchmarks
    pub concurrent_threads: u32,
    /// Whether to profile memory usage
    pub memory_profiling: bool,
    /// Whether to profile CPU usage
    pub cpu_profiling: bool,
    /// Additional metric names to collect
    pub custom_metrics: Vec<String>,
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

/// Performance benchmark result
#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    /// Name of the benchmark
    pub test_name: String,
    /// Number of iterations completed
    pub iterations: u32,
    /// Total time across all iterations
    pub total_duration: Duration,
    /// Mean duration per iteration
    pub average_duration: Duration,
    /// Fastest iteration
    pub min_duration: Duration,
    /// Slowest iteration
    pub max_duration: Duration,
    /// P50, P90, P95, P99 latency percentiles
    pub percentiles: PercentileMetrics,
    /// Throughput metrics (ops/sec, etc.)
    pub throughput: ThroughputMetrics,
    /// Memory, CPU, I/O usage during benchmark
    pub resource_usage: ResourceUsageMetrics,
    /// Additional custom metrics
    pub custom_metrics: HashMap<String, f64>,
}

impl BenchmarkResult {
    /// Create a default benchmark result
    #[must_use]
    pub fn default(test_name: impl Into<String>) -> Self {
        Self {
            test_name: test_name.into(),
            iterations: 0,
            total_duration: Duration::ZERO,
            average_duration: Duration::ZERO,
            min_duration: Duration::ZERO,
            max_duration: Duration::ZERO,
            percentiles: PercentileMetrics::default(),
            throughput: ThroughputMetrics::default(),
            resource_usage: ResourceUsageMetrics::default(),
            custom_metrics: HashMap::new(),
        }
    }
}

/// Percentile performance metrics
#[derive(Debug, Clone)]
pub struct PercentileMetrics {
    /// 50th percentile (median) latency
    pub p50: Duration,
    /// 90th percentile latency
    pub p90: Duration,
    /// 95th percentile latency
    pub p95: Duration,
    /// 99th percentile latency
    pub p99: Duration,
    /// 99.9th percentile latency
    pub p99_9: Duration,
}

impl Default for PercentileMetrics {
    fn default() -> Self {
        Self {
            p50: Duration::ZERO,
            p90: Duration::ZERO,
            p95: Duration::ZERO,
            p99: Duration::ZERO,
            p99_9: Duration::ZERO,
        }
    }
}

/// Throughput metrics
#[derive(Debug, Clone)]
pub struct ThroughputMetrics {
    /// Operations completed per second
    pub operations_per_second: f64,
    /// Bytes processed per second (if applicable)
    pub bytes_per_second: Option<u64>,
    /// Requests per second (for load tests)
    pub requests_per_second: Option<f64>,
    /// Number of concurrent operations
    pub concurrent_operations: u32,
}

impl Default for ThroughputMetrics {
    fn default() -> Self {
        Self {
            operations_per_second: 0.0,
            bytes_per_second: None,
            requests_per_second: None,
            concurrent_operations: 0,
        }
    }
}

/// Resource usage metrics during performance tests
#[derive(Debug, Clone)]
pub struct ResourceUsageMetrics {
    /// Peak memory usage in megabytes
    pub peak_memory_mb: u32,
    /// Average memory usage in megabytes
    pub average_memory_mb: u32,
    /// Peak CPU utilization percentage
    pub peak_cpu_percent: f32,
    /// Average CPU utilization percentage
    pub average_cpu_percent: f32,
    /// Disk I/O in megabytes
    pub disk_io_mb: u64,
    /// Network I/O in megabytes
    pub network_io_mb: u64,
    /// Number of context switches
    pub context_switches: u64,
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

/// Performance comparison between benchmark results
#[derive(Debug, Clone)]
pub struct PerformanceComparison {
    /// Baseline benchmark for comparison
    pub baseline: BenchmarkResult,
    /// Current run being compared
    pub current: BenchmarkResult,
    /// Percentage improvement (negative = regression)
    pub improvement_percent: f64,
    /// Whether a performance regression was detected
    pub regression_detected: bool,
    /// Whether the change is statistically significant
    pub significant_change: bool,
    /// Human-readable comparison summary
    pub summary: String,
}

/// Load testing configuration
#[derive(Debug, Clone)]
pub struct LoadTestConfig {
    /// Name of the load test
    pub test_name: String,
    /// Simulated concurrent users
    pub concurrent_users: u32,
    /// Time to ramp up to full load
    pub ramp_up_duration: Duration,
    /// Total test duration
    pub test_duration: Duration,
    /// Target requests per second (if specified)
    pub target_rps: Option<f64>,
    /// Simulated think time between requests
    pub think_time: Duration,
}

/// Load test result
#[derive(Debug, Clone)]
pub struct LoadTestResult {
    /// Name of the load test
    pub test_name: String,
    /// Total requests issued
    pub total_requests: u64,
    /// Requests that succeeded
    pub successful_requests: u64,
    /// Requests that failed
    pub failed_requests: u64,
    /// Mean response time
    pub average_response_time: Duration,
    /// Fraction of requests that failed (0.0–1.0)
    pub error_rate: f64,
    /// Requests per second achieved
    pub throughput: f64,
    /// Concurrent users during test
    pub concurrent_users: u32,
    /// Resource usage during the load test
    pub resource_usage: ResourceUsageMetrics,
}
