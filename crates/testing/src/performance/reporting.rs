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

//! Performance test reporting and output formatting

use super::types::BenchmarkResult;

/// Performance test report
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// Total number of benchmarks in the report
    pub total_benchmarks: usize,
    /// Individual benchmark results
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
