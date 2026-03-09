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
    pub test_name: String,
    pub warm_up_iterations: u32,
    pub measurement_iterations: u32,
    pub concurrent_threads: u32,
    pub memory_profiling: bool,
    pub cpu_profiling: bool,
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
    pub p50: Duration,
    pub p90: Duration,
    pub p95: Duration,
    pub p99: Duration,
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
    pub operations_per_second: f64,
    pub bytes_per_second: Option<u64>,
    pub requests_per_second: Option<f64>,
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
    pub peak_memory_mb: u32,
    pub average_memory_mb: u32,
    pub peak_cpu_percent: f32,
    pub average_cpu_percent: f32,
    pub disk_io_mb: u64,
    pub network_io_mb: u64,
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
