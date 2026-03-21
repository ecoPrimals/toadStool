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

//! Property-based testing data types and configurations

use std::collections::HashMap;
use std::time::Duration;

/// Type alias for custom test functions to reduce complexity
pub type CustomTestFunc = dyn Fn(&str) -> Vec<String>;

/// Property-based test configuration
#[derive(Debug, Clone)]
pub struct PropertyTestConfig {
    /// Name of the property test
    pub test_name: String,
    /// Number of random test cases to run
    pub test_cases: u32,
    /// Max shrink attempts when a failure is found
    pub shrink_attempts: u32,
    /// Timeout for the entire property test
    pub timeout: Duration,
    /// Whether to emit verbose output
    pub verbose: bool,
    /// Optional RNG seed for reproducibility
    pub seed: Option<u64>,
}

impl Default for PropertyTestConfig {
    fn default() -> Self {
        Self {
            test_name: "unnamed_property".to_string(),
            test_cases: 100,
            shrink_attempts: 100,
            timeout: Duration::from_secs(30),
            verbose: false,
            seed: None,
        }
    }
}

/// Property test result
#[derive(Debug, Clone)]
pub struct PropertyTestResult {
    /// Name of the property test
    pub test_name: String,
    /// Whether all test cases passed
    pub success: bool,
    /// Number of test cases executed
    pub test_cases_run: u32,
    /// Failures found (with shrunk inputs)
    pub failures: Vec<PropertyFailure>,
    /// Total duration of the test run
    pub duration: Duration,
    /// Input distribution and timing statistics
    pub statistics: TestStatistics,
}

impl PropertyTestResult {
    /// Generate a human-readable test report
    #[must_use]
    pub fn to_report_string(&self) -> String {
        let mut report = format!(
            "Property Test: {}\n\
             Status: {}\n\
             Test Cases: {}\n\
             Duration: {:.2}ms\n",
            self.test_name,
            if self.success {
                "✅ PASSED"
            } else {
                "❌ FAILED"
            },
            self.test_cases_run,
            self.duration.as_secs_f64() * 1000.0
        );

        if !self.failures.is_empty() {
            report.push_str("\nFailures:\n");
            for (i, failure) in self.failures.iter().enumerate() {
                report.push_str(&format!(
                    "  {}. Original: {}\n\
                     Shrunk: {}\n\
                     Error: {}\n\n",
                    i + 1,
                    failure.original_input,
                    failure.shrunk_input,
                    failure.error_message
                ));
            }
        }

        if !self.statistics.execution_times.is_empty() {
            report.push_str(&format!(
                "Average Execution Time: {:.2}ms\n",
                self.statistics.average_execution_time().as_secs_f64() * 1000.0
            ));
        }

        report
    }
}

/// Property test failure with shrinking information
#[derive(Debug, Clone)]
pub struct PropertyFailure {
    /// Original failing input (string representation)
    pub original_input: String,
    /// Smallest failing input after shrinking
    pub shrunk_input: String,
    /// Error message from the failure
    pub error_message: String,
    /// Number of shrink steps performed
    pub shrink_steps: u32,
}

/// Statistics from property testing
#[derive(Debug, Clone)]
pub struct TestStatistics {
    /// Distribution of input types/categories
    pub input_distribution: HashMap<String, u32>,
    /// Per-case execution times
    pub execution_times: Vec<Duration>,
    /// Coverage metrics (e.g. branch coverage)
    pub coverage_metrics: HashMap<String, f64>,
}

impl TestStatistics {
    /// Create new empty statistics
    #[must_use]
    pub fn new() -> Self {
        Self {
            input_distribution: HashMap::new(),
            execution_times: Vec::new(),
            coverage_metrics: HashMap::new(),
        }
    }

    /// Calculate average execution time
    #[must_use]
    pub fn average_execution_time(&self) -> Duration {
        if self.execution_times.is_empty() {
            return Duration::ZERO;
        }
        self.execution_times.iter().sum::<Duration>() / self.execution_times.len() as u32
    }

    /// Calculate maximum execution time
    #[must_use]
    pub fn max_execution_time(&self) -> Duration {
        self.execution_times
            .iter()
            .max()
            .copied()
            .unwrap_or(Duration::ZERO)
    }

    /// Calculate minimum execution time
    #[must_use]
    pub fn min_execution_time(&self) -> Duration {
        self.execution_times
            .iter()
            .min()
            .copied()
            .unwrap_or(Duration::ZERO)
    }
}

impl Default for TestStatistics {
    fn default() -> Self {
        Self::new()
    }
}

/// Shrinking strategies for minimizing failing inputs
pub enum ShrinkStrategy {
    /// No shrinking
    None,
    /// Linear search for smaller inputs
    Linear,
    /// Binary search for smaller inputs
    Binary,
    /// Recursive shrinking (e.g. for nested structures)
    Recursive,
    /// Custom shrinking function
    Custom(Box<CustomTestFunc>),
}

impl std::fmt::Debug for ShrinkStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => write!(f, "ShrinkStrategy::None"),
            Self::Linear => write!(f, "ShrinkStrategy::Linear"),
            Self::Binary => write!(f, "ShrinkStrategy::Binary"),
            Self::Recursive => write!(f, "ShrinkStrategy::Recursive"),
            Self::Custom(_) => write!(f, "ShrinkStrategy::Custom(<function>)"),
        }
    }
}
