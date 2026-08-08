// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Security monitoring for ToadStool.
//!
//! Tracks security-relevant events (auth failures, policy denials, anomalous
//! resource usage) in a bounded in-process ring buffer. No external dependency
//! required — all telemetry stays local unless an exporter is explicitly wired.

use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::RwLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Maximum events held in the ring buffer before oldest are dropped.
const DEFAULT_RING_CAPACITY: usize = 1_000;

/// Severity level for a security event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Severity {
    /// Informational; no action required.
    Info,
    /// Warning; may warrant investigation.
    Warning,
    /// Critical; requires immediate attention.
    Critical,
}

/// A single monitored security event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    /// Unix timestamp (milliseconds).
    pub timestamp_ms: u64,
    /// Severity level.
    pub severity: Severity,
    /// Event category for filtering.
    pub category: EventCategory,
    /// Human-readable event description.
    pub message: String,
    /// Optional workload or request identifier.
    pub correlation_id: Option<String>,
}

impl SecurityEvent {
    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )] // u128 ms since epoch fits u64 for centuries
    fn new(severity: Severity, category: EventCategory, message: impl Into<String>) -> Self {
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;
        Self {
            timestamp_ms,
            severity,
            category,
            message: message.into(),
            correlation_id: None,
        }
    }

    /// Attach a correlation ID for request tracing.
    #[must_use]
    pub fn with_correlation(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
}

/// Coarse category for quick filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventCategory {
    /// Authentication or authorization failure.
    AuthFailure,
    /// Access denied by policy.
    PolicyDenial,
    /// Unusual resource usage (CPU, memory).
    ResourceAnomaly,
    /// Unusual network activity.
    NetworkAnomaly,
    /// Data or integrity violation.
    IntegrityViolation,
    /// General operational notice.
    Operational,
}

/// Snapshot of system resource usage, sampled periodically for anomaly detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    /// Unix timestamp (milliseconds) when sampled.
    pub timestamp_ms: u64,
    /// CPU utilization (0–100%).
    pub cpu_usage_percent: f32,
    /// Memory used in bytes.
    pub memory_used_bytes: u64,
    /// Total memory in bytes.
    pub memory_total_bytes: u64,
}

/// In-process security monitor — zero external dependencies.
///
/// Emit events via the `record_*` methods. The internal ring buffer holds the
/// last `capacity` events; older events are silently dropped. Attach a
/// `SecurityEventExporter` if you need to forward events elsewhere.
pub struct SecurityMonitor {
    capacity: usize,
    events: Arc<RwLock<VecDeque<SecurityEvent>>>,
    resource_history: Arc<RwLock<VecDeque<ResourceSnapshot>>>,
}

impl SecurityMonitor {
    /// Create a monitor with default ring buffer capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_RING_CAPACITY)
    }

    /// Create a monitor with the given ring buffer capacity.
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            capacity,
            events: Arc::new(RwLock::new(VecDeque::with_capacity(capacity))),
            resource_history: Arc::new(RwLock::new(VecDeque::with_capacity(64))),
        }
    }

    // ── Recording ─────────────────────────────────────────────────────────────

    /// Record an authentication failure.
    pub async fn record_auth_failure(&self, message: impl Into<String>) {
        let event = SecurityEvent::new(Severity::Warning, EventCategory::AuthFailure, message);
        warn!(category = "auth_failure", "{}", &event.message);
        self.push_event(event).await;
    }

    /// Record a policy denial.
    pub async fn record_policy_denial(&self, message: impl Into<String>) {
        let event = SecurityEvent::new(Severity::Warning, EventCategory::PolicyDenial, message);
        warn!(category = "policy_denial", "{}", &event.message);
        self.push_event(event).await;
    }

    /// Record a critical integrity violation.
    pub async fn record_integrity_violation(&self, message: impl Into<String>) {
        let event = SecurityEvent::new(
            Severity::Critical,
            EventCategory::IntegrityViolation,
            message,
        );
        tracing::error!(category = "integrity_violation", "{}", &event.message);
        self.push_event(event).await;
    }

    /// Record a generic operational notice.
    pub async fn record_operational(&self, message: impl Into<String>) {
        let event = SecurityEvent::new(Severity::Info, EventCategory::Operational, message);
        info!(category = "operational", "{}", &event.message);
        self.push_event(event).await;
    }

    /// Record a resource anomaly (e.g. unexpected CPU spike).
    pub async fn record_resource_anomaly(&self, message: impl Into<String>) {
        let event = SecurityEvent::new(Severity::Warning, EventCategory::ResourceAnomaly, message);
        warn!(category = "resource_anomaly", "{}", &event.message);
        self.push_event(event).await;
    }

    // ── Querying ──────────────────────────────────────────────────────────────

    /// Return a snapshot of all buffered events (newest last).
    pub async fn events(&self) -> Vec<SecurityEvent> {
        self.events.read().expect("lock poisoned").iter().cloned().collect()
    }

    /// Return events at or above the given severity.
    pub async fn events_above(&self, min: Severity) -> Vec<SecurityEvent> {
        self.events
            .read()
            .expect("lock poisoned")
            .iter()
            .filter(|e| e.severity >= min)
            .cloned()
            .collect()
    }

    /// Count events in the buffer by category.
    pub async fn count_by_category(&self, category: EventCategory) -> usize {
        self.events
            .read()
            .expect("lock poisoned")
            .iter()
            .filter(|e| e.category == category)
            .count()
    }

    // ── Resource sampling ─────────────────────────────────────────────────────

    /// Sample current system resources and store them in the history buffer.
    /// Detects anomalies (CPU > 90 %, memory > 95 %) and raises events.
    #[expect(
        clippy::cast_precision_loss,
        clippy::cast_possible_truncation,
        reason = "precision loss and truncation acceptable for metrics"
    )] // numeric conversions for metrics
    pub async fn sample_resources(&self) {
        let cpu = toadstool_sysmon::cpu_usage(
            toadstool_common::constants::timeouts::CPU_USAGE_SAMPLE_WINDOW,
        )
        .unwrap_or(0.0);
        let (mem_used, mem_total) =
            toadstool_sysmon::memory_info().map_or((0, 1), |m| (m.used, m.total));
        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or(Duration::ZERO)
            .as_millis() as u64;

        let snapshot = ResourceSnapshot {
            timestamp_ms,
            cpu_usage_percent: cpu,
            memory_used_bytes: mem_used,
            memory_total_bytes: mem_total,
        };

        let mut history = self.resource_history.write().expect("lock poisoned");
        if history.len() >= 64 {
            history.pop_front();
        }
        history.push_back(snapshot);
        drop(history);

        if mem_total > 0 {
            let mem_pct = mem_used as f64 / mem_total as f64 * 100.0;
            if mem_pct > 95.0 {
                self.record_resource_anomaly(format!(
                    "Memory usage critical: {mem_pct:.1}% ({mem_used} / {mem_total} bytes)"
                ))
                .await;
            }
        }
        if cpu > 90.0 {
            self.record_resource_anomaly(format!("CPU usage critical: {cpu:.1}%"))
                .await;
        }
    }

    // ── Internal ──────────────────────────────────────────────────────────────

    async fn push_event(&self, event: SecurityEvent) {
        let mut buf = self.events.write().expect("lock poisoned");
        if buf.len() >= self.capacity {
            buf.pop_front();
        }
        buf.push_back(event);
    }
}

impl Default for SecurityMonitor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_recording_and_query() {
        let monitor = SecurityMonitor::new();
        monitor.record_auth_failure("bad token").await;
        monitor.record_policy_denial("denied write").await;
        monitor.record_operational("node started").await;

        let all = monitor.events().await;
        assert_eq!(all.len(), 3);

        let warnings = monitor.events_above(Severity::Warning).await;
        assert_eq!(warnings.len(), 2);

        let auth_count = monitor.count_by_category(EventCategory::AuthFailure).await;
        assert_eq!(auth_count, 1);
    }

    #[tokio::test]
    async fn test_ring_buffer_drops_oldest() {
        let monitor = SecurityMonitor::with_capacity(3);
        for i in 0..5 {
            monitor.record_operational(format!("event {i}")).await;
        }
        let events = monitor.events().await;
        assert_eq!(events.len(), 3);
        assert!(events[0].message.contains("event 2"));
    }

    #[test]
    fn test_severity_ordering() {
        assert!(Severity::Critical > Severity::Warning);
        assert!(Severity::Warning > Severity::Info);
    }
}
