// SPDX-License-Identifier: AGPL-3.0-or-later
//! Metrics collection for WASM runtime
//!
//! Tracks execution metrics, cache performance, and resource usage.

use std::sync::atomic::{AtomicU64, Ordering};

/// Metrics collector for WASM runtime
#[derive(Debug)]
pub struct MetricsCollector {
    /// Total executions
    total_executions: AtomicU64,

    /// Successful executions
    successful_executions: AtomicU64,

    /// Failed executions
    failed_executions: AtomicU64,

    /// Total execution time (microseconds)
    total_execution_time_us: AtomicU64,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub const fn new() -> Self {
        Self {
            total_executions: AtomicU64::new(0),
            successful_executions: AtomicU64::new(0),
            failed_executions: AtomicU64::new(0),
            total_execution_time_us: AtomicU64::new(0),
        }
    }

    /// Record successful execution
    pub fn record_success(&self, execution_time_us: u64) {
        self.total_executions.fetch_add(1, Ordering::Relaxed);
        self.successful_executions.fetch_add(1, Ordering::Relaxed);
        self.total_execution_time_us
            .fetch_add(execution_time_us, Ordering::Relaxed);
    }

    /// Record failed execution
    pub fn record_failure(&self) {
        self.total_executions.fetch_add(1, Ordering::Relaxed);
        self.failed_executions.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total executions
    pub fn total_executions(&self) -> u64 {
        self.total_executions.load(Ordering::Relaxed)
    }

    /// Get successful executions
    pub fn successful_executions(&self) -> u64 {
        self.successful_executions.load(Ordering::Relaxed)
    }

    /// Get failed executions
    pub fn failed_executions(&self) -> u64 {
        self.failed_executions.load(Ordering::Relaxed)
    }

    /// Get average execution time in microseconds
    pub fn average_execution_time_us(&self) -> u64 {
        let total = self.total_executions.load(Ordering::Relaxed);
        self.total_execution_time_us
            .load(Ordering::Relaxed)
            .checked_div(total)
            .unwrap_or(0)
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_collector_starts_at_zero() {
        let m = MetricsCollector::new();
        assert_eq!(m.total_executions(), 0);
        assert_eq!(m.successful_executions(), 0);
        assert_eq!(m.failed_executions(), 0);
        assert_eq!(m.average_execution_time_us(), 0);
    }

    #[test]
    fn default_matches_new() {
        let m = MetricsCollector::default();
        assert_eq!(m.total_executions(), 0);
    }

    #[test]
    fn record_success_increments_counters() {
        let m = MetricsCollector::new();
        m.record_success(100);
        assert_eq!(m.total_executions(), 1);
        assert_eq!(m.successful_executions(), 1);
        assert_eq!(m.failed_executions(), 0);
        assert_eq!(m.average_execution_time_us(), 100);
    }

    #[test]
    fn record_failure_increments_counters() {
        let m = MetricsCollector::new();
        m.record_failure();
        assert_eq!(m.total_executions(), 1);
        assert_eq!(m.successful_executions(), 0);
        assert_eq!(m.failed_executions(), 1);
    }

    #[test]
    fn average_computed_over_total_executions() {
        let m = MetricsCollector::new();
        m.record_success(200);
        m.record_success(400);
        m.record_failure();
        assert_eq!(m.total_executions(), 3);
        assert_eq!(m.successful_executions(), 2);
        assert_eq!(m.failed_executions(), 1);
        assert_eq!(m.average_execution_time_us(), 200);
    }

    #[test]
    fn average_with_zero_time_succeeds() {
        let m = MetricsCollector::new();
        m.record_success(0);
        assert_eq!(m.average_execution_time_us(), 0);
    }

    #[test]
    fn multiple_failures_do_not_affect_time() {
        let m = MetricsCollector::new();
        m.record_failure();
        m.record_failure();
        m.record_failure();
        assert_eq!(m.total_executions(), 3);
        assert_eq!(m.average_execution_time_us(), 0);
    }
}
