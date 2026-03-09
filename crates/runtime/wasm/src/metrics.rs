// SPDX-License-Identifier: AGPL-3.0-only
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
    pub fn new() -> Self {
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
        if total == 0 {
            0
        } else {
            self.total_execution_time_us.load(Ordering::Relaxed) / total
        }
    }
}

impl Default for MetricsCollector {
    fn default() -> Self {
        Self::new()
    }
}
