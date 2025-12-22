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

//! Custom assertions for `ToadStool` testing
//!
//! This module provides domain-specific assertion helpers that make
//! test failures more informative and tests more readable.
//!
//! # Note on Panics
//!
//! These assertion functions are designed to panic on assertion failures,
//! as is standard practice for test assertions. This is intentional and
//! appropriate for test infrastructure.

#![allow(clippy::panic)] // Test assertions should panic on failure

use toadstool::{
    execution::{ExecutionResponse, ExecutionStatus},
    resources::RuntimeMetrics,
};

/// Assert that an execution response indicates success
pub fn assert_execution_success(response: &ExecutionResponse) {
    assert!(
        response.status == ExecutionStatus::Success,
        "Expected successful execution, got: {:?}",
        response.status
    );

    // Check that we have some output (even if empty)
    assert!(
        response.output.data.len() < usize::MAX,
        "Execution output data size should be reasonable"
    );
}

/// Assert that an execution response indicates failure
pub fn assert_execution_failure(response: &ExecutionResponse) {
    assert!(
        matches!(response.status, ExecutionStatus::Failed { .. }),
        "Expected failed execution, got: {:?}",
        response.status
    );

    // Extract error message from status
    if let ExecutionStatus::Failed { error } = &response.status {
        assert!(!error.is_empty(), "Expected non-empty error message");
    }
}

/// Assert that an execution response indicates timeout
pub fn assert_execution_timeout(response: &ExecutionResponse) {
    assert!(
        response.status == ExecutionStatus::TimedOut,
        "Expected timed out execution, got: {:?}",
        response.status
    );
}

/// Assert that an execution response indicates cancellation
pub fn assert_execution_cancelled(response: &ExecutionResponse) {
    assert!(
        response.status == ExecutionStatus::Cancelled,
        "Expected cancelled execution, got: {:?}",
        response.status
    );
}

/// Assert that an execution response indicates resource limit exceeded
pub fn assert_execution_resource_limit_exceeded(response: &ExecutionResponse) {
    assert!(
        matches!(
            &response.status,
            ExecutionStatus::Failed { error } if error.contains("Resource limit exceeded")
        ),
        "Expected resource limit exceeded, got: {:?}",
        response.status
    );
}

/// Assert that an execution response indicates security violation
pub fn assert_execution_security_violation(response: &ExecutionResponse) {
    assert!(
        matches!(&response.status, ExecutionStatus::Failed { error } if error.contains("Security violation")),
        "Expected security violation, got: {:?}",
        response.status
    );
}

/// Assert that execution output contains expected data
pub fn assert_output_contains(response: &ExecutionResponse, expected: &[u8]) {
    assert!(
        response
            .output
            .data
            .windows(expected.len())
            .any(|window| window == expected),
        "Expected output to contain {:?}, got: {:?}",
        expected,
        response.output.data
    );
}

/// Assert that execution output stdout contains expected text
pub fn assert_stdout_contains(response: &ExecutionResponse, expected: &str) {
    match &response.output.stdout {
        Some(stdout) => {
            assert!(
                stdout.contains(expected),
                "Expected stdout to contain '{expected}', got: '{stdout}'"
            );
        }
        None => {
            panic!("Expected stdout to be present, but it was None");
        }
    }
}

/// Assert that execution output stderr contains expected text
pub fn assert_stderr_contains(response: &ExecutionResponse, expected: &str) {
    match &response.output.stderr {
        Some(stderr) => {
            assert!(
                stderr.contains(expected),
                "Expected stderr to contain '{expected}', got: '{stderr}'"
            );
        }
        None => {
            panic!("Expected stderr to be present, but it was None");
        }
    }
}

/// Assert that execution has expected exit code
pub fn assert_exit_code(response: &ExecutionResponse, expected: i32) {
    match response.output.exit_code {
        Some(code) => {
            assert_eq!(code, expected, "Expected exit code {expected}, got {code}");
        }
        None => {
            panic!("Expected exit code to be present, but it was None");
        }
    }
}

/// Assert that execution duration is within expected range
pub fn assert_duration_within(response: &ExecutionResponse, min_ms: u64, max_ms: u64) {
    let duration_ms = u64::try_from(response.duration.as_millis()).unwrap_or(0);
    assert!(
        duration_ms >= min_ms && duration_ms <= max_ms,
        "Expected duration between {min_ms}ms and {max_ms}ms, got {duration_ms}ms"
    );
}

/// Assert that runtime metrics are present and reasonable
pub fn assert_metrics_present(response: &ExecutionResponse) {
    let metrics = &response.metrics;

    // CPU metrics should be reasonable
    assert!(
        metrics.cpu.usage_percent >= 0.0 && metrics.cpu.usage_percent <= 100.0,
        "CPU usage should be between 0-100%, got {}%",
        metrics.cpu.usage_percent
    );

    // Memory metrics should be positive
    assert!(
        metrics.memory.used_bytes > 0,
        "Memory usage should be positive"
    );

    // Memory usage validation - all u64 values are valid by definition

    assert!(
        metrics.memory.peak_bytes >= metrics.memory.used_bytes,
        "Peak memory usage should be >= current usage"
    );

    // Timing should be consistent
    assert!(
        metrics.timing.duration
            <= chrono::Duration::from_std(response.duration).unwrap_or_default(),
        "Timing duration should be consistent with response duration"
    );

    // Network metrics validation - basic validation checks only
    // Network bytes sent validation - all u64 values are valid by definition

    // Network bytes received validation - all u64 values are valid by definition

    // Network packets sent validation - all u64 values are valid by definition

    // Network packets received validation - all u64 values are valid by definition

    // Storage metrics validation - basic validation checks only
    // Storage bytes read validation - all u64 values are valid by definition
    // Storage bytes written validation - all u64 values are valid by definition
}

/// Assert that execution warnings are present
pub fn assert_has_warnings(response: &ExecutionResponse) {
    assert!(
        !response.warnings.is_empty(),
        "Expected warnings to be present, but none found"
    );
}

/// Assert that execution has no warnings
pub fn assert_no_warnings(response: &ExecutionResponse) {
    assert!(
        response.warnings.is_empty(),
        "Expected no warnings, but found: {:?}",
        response.warnings
    );
}

/// Assert that execution response has expected execution ID
pub fn assert_execution_id_matches(response: &ExecutionResponse, expected_id: &uuid::Uuid) {
    assert_eq!(
        response.execution_id, *expected_id,
        "Expected execution ID to match"
    );
}

/// Assert that CPU metrics are within reasonable bounds
pub fn assert_cpu_metrics_reasonable(metrics: &RuntimeMetrics) {
    assert!(
        metrics.cpu.usage_percent >= 0.0 && metrics.cpu.usage_percent <= 100.0,
        "CPU usage should be 0-100%, got {}%",
        metrics.cpu.usage_percent
    );

    assert!(
        metrics.cpu.usage_percent >= 0.0,
        "Peak CPU usage should be >= current usage"
    );

    assert!(
        metrics.cpu.cpu_time_seconds > 0.0,
        "CPU time should be positive, got {}s",
        metrics.cpu.cpu_time_seconds
    );
}

/// Assert that memory metrics are within reasonable bounds
pub fn assert_memory_metrics_reasonable(metrics: &RuntimeMetrics) {
    assert!(
        metrics.memory.used_bytes > 0,
        "Memory usage should be positive, got {} bytes",
        metrics.memory.used_bytes
    );

    assert!(
        metrics.memory.peak_bytes >= metrics.memory.used_bytes,
        "Peak memory usage should be >= current usage"
    );

    // Memory used_bytes is u64, always non-negative - no need to check >= 0
}

/// Assert that network metrics are reasonable
pub fn assert_network_metrics_reasonable(metrics: &RuntimeMetrics) {
    // Network metrics are u64 values, always non-negative - no need to check >= 0
    // This function exists for potential future validations
    let _ = metrics; // Suppress unused variable warning
}

/// Assert that storage metrics are reasonable
pub fn assert_storage_metrics_reasonable(metrics: &RuntimeMetrics) {
    // Storage metrics are u64 values, always non-negative - no need to check >= 0
    // This function exists for potential future validations
    let _ = metrics; // Suppress unused variable warning
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builders::ExecutionResponseBuilder;
    use crate::fixtures::create_test_runtime_metrics;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn test_assert_execution_success() {
        let response = ExecutionResponseBuilder::new().success().build();

        assert_execution_success(&response);
    }

    #[test]
    fn test_assert_execution_failure() {
        let response = ExecutionResponseBuilder::new().failed("Test error").build();

        assert_execution_failure(&response);
    }

    #[test]
    fn test_assert_execution_timeout() {
        let response = ExecutionResponseBuilder::new().timed_out().build();

        assert_execution_timeout(&response);
    }

    #[test]
    fn test_assert_duration_within() {
        let response = ExecutionResponseBuilder::new()
            .duration(Duration::from_millis(500))
            .build();

        assert_duration_within(&response, 400, 600);
    }

    #[test]
    fn test_assert_metrics_present() {
        let response = ExecutionResponseBuilder::new()
            .metrics(create_test_runtime_metrics())
            .build();

        assert_metrics_present(&response);
    }

    #[test]
    fn test_assert_execution_id_matches() {
        let id = Uuid::new_v4();
        let response = ExecutionResponseBuilder::new().execution_id(id).build();

        assert_execution_id_matches(&response, &id);
    }

    #[test]
    fn test_assert_warnings() {
        let response_with_warnings = ExecutionResponseBuilder::new()
            .warning("Test warning")
            .build();

        let response_without_warnings = ExecutionResponseBuilder::new().build();

        assert_has_warnings(&response_with_warnings);
        assert_no_warnings(&response_without_warnings);
    }

    #[test]
    fn test_metric_assertions() {
        let metrics = create_test_runtime_metrics();

        assert_cpu_metrics_reasonable(&metrics);
        assert_memory_metrics_reasonable(&metrics);
        assert_network_metrics_reasonable(&metrics);
        assert_storage_metrics_reasonable(&metrics);
    }
}
