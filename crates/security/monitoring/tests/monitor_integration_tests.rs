//! Integration tests for SecurityMonitor
//!
//! Covers recording, querying, capacity bounds, concurrency, resource sampling,
//! and correlation ID tracking.

use std::sync::Arc;
use toadstool_security_monitoring::{EventCategory, SecurityMonitor, Severity};

// ── Basic event recording ────────────────────────────────────────────────────

#[tokio::test]
async fn test_empty_monitor_has_no_events() {
    let monitor = SecurityMonitor::new();
    assert!(monitor.events().await.is_empty());
}

#[tokio::test]
async fn test_record_auth_failure_appears_in_events() {
    let monitor = SecurityMonitor::new();
    monitor.record_auth_failure("invalid JWT").await;
    let events = monitor.events().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].category, EventCategory::AuthFailure);
    assert_eq!(events[0].severity, Severity::Warning);
    assert!(events[0].message.contains("invalid JWT"));
}

#[tokio::test]
async fn test_record_policy_denial_appears_in_events() {
    let monitor = SecurityMonitor::new();
    monitor
        .record_policy_denial("write denied on /etc/passwd")
        .await;
    let events = monitor.events().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].category, EventCategory::PolicyDenial);
    assert_eq!(events[0].severity, Severity::Warning);
}

#[tokio::test]
async fn test_record_integrity_violation_is_critical() {
    let monitor = SecurityMonitor::new();
    monitor
        .record_integrity_violation("checksum mismatch on kernel module")
        .await;
    let events = monitor.events().await;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].severity, Severity::Critical);
    assert_eq!(events[0].category, EventCategory::IntegrityViolation);
}

#[tokio::test]
async fn test_record_operational_is_info() {
    let monitor = SecurityMonitor::new();
    monitor.record_operational("daemon started").await;
    let events = monitor.events().await;
    assert_eq!(events[0].severity, Severity::Info);
    assert_eq!(events[0].category, EventCategory::Operational);
}

#[tokio::test]
async fn test_record_resource_anomaly_is_warning() {
    let monitor = SecurityMonitor::new();
    monitor.record_resource_anomaly("CPU 97%").await;
    let events = monitor.events().await;
    assert_eq!(events[0].severity, Severity::Warning);
    assert_eq!(events[0].category, EventCategory::ResourceAnomaly);
}

// ── Query filtering ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_events_above_critical_filters_below() {
    let monitor = SecurityMonitor::new();
    monitor.record_auth_failure("fail").await; // Warning
    monitor.record_operational("ok").await; // Info
    monitor.record_integrity_violation("bad").await; // Critical

    let critical = monitor.events_above(Severity::Critical).await;
    assert_eq!(critical.len(), 1);
    assert_eq!(critical[0].severity, Severity::Critical);
}

#[tokio::test]
async fn test_events_above_info_returns_all() {
    let monitor = SecurityMonitor::new();
    monitor.record_auth_failure("fail").await;
    monitor.record_operational("ok").await;
    monitor.record_integrity_violation("bad").await;

    let all = monitor.events_above(Severity::Info).await;
    assert_eq!(all.len(), 3);
}

#[tokio::test]
async fn test_count_by_category_correct() {
    let monitor = SecurityMonitor::new();
    monitor.record_auth_failure("fail 1").await;
    monitor.record_auth_failure("fail 2").await;
    monitor.record_policy_denial("denied").await;

    assert_eq!(
        monitor.count_by_category(EventCategory::AuthFailure).await,
        2
    );
    assert_eq!(
        monitor.count_by_category(EventCategory::PolicyDenial).await,
        1
    );
    assert_eq!(
        monitor.count_by_category(EventCategory::Operational).await,
        0
    );
}

// ── Ring buffer capacity ─────────────────────────────────────────────────────

#[tokio::test]
async fn test_ring_buffer_drops_oldest_at_capacity() {
    let monitor = SecurityMonitor::with_capacity(5);
    for i in 0..8_u32 {
        monitor.record_operational(format!("event {i}")).await;
    }
    let events = monitor.events().await;
    assert_eq!(events.len(), 5, "Should keep exactly capacity events");
    // Oldest (0-2) dropped; newest (3-7) retained
    assert!(
        events[0].message.contains("event 3"),
        "First retained: {:?}",
        events[0].message
    );
    assert!(events[4].message.contains("event 7"));
}

#[tokio::test]
async fn test_ring_buffer_exactly_at_capacity() {
    let monitor = SecurityMonitor::with_capacity(3);
    monitor.record_operational("a").await;
    monitor.record_operational("b").await;
    monitor.record_operational("c").await;
    assert_eq!(monitor.events().await.len(), 3);

    // One more pushes oldest out
    monitor.record_operational("d").await;
    let events = monitor.events().await;
    assert_eq!(events.len(), 3);
    assert!(!events.iter().any(|e| e.message == "a"));
}

// ── Correlation IDs ──────────────────────────────────────────────────────────

#[tokio::test]
async fn test_security_event_with_correlation() {
    let monitor = SecurityMonitor::new();
    monitor.record_auth_failure("token expired").await;
    let events = monitor.events().await;
    // correlation_id is None by default — the field is present and accessible
    assert!(events[0].correlation_id.is_none());
    assert!(events[0].timestamp_ms > 0);
}

#[tokio::test]
async fn test_event_timestamps_monotonic() {
    let monitor = SecurityMonitor::new();
    monitor.record_operational("first").await;
    monitor.record_operational("second").await;
    let events = monitor.events().await;
    assert!(
        events[0].timestamp_ms <= events[1].timestamp_ms,
        "Events should be recorded in non-decreasing timestamp order"
    );
}

// ── Concurrency ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_concurrent_writes_do_not_corrupt() {
    let monitor = Arc::new(SecurityMonitor::with_capacity(100));
    let mut handles = Vec::new();

    for i in 0..20_u32 {
        let m = Arc::clone(&monitor);
        handles.push(tokio::spawn(async move {
            m.record_auth_failure(format!("concurrent fail {i}")).await;
        }));
    }
    for h in handles {
        h.await.unwrap();
    }

    let events = monitor.events().await;
    assert_eq!(events.len(), 20);
    assert!(events
        .iter()
        .all(|e| e.category == EventCategory::AuthFailure));
}

// ── Resource sampling (smoke test) ───────────────────────────────────────────

#[tokio::test]
async fn test_sample_resources_does_not_panic() {
    let monitor = SecurityMonitor::new();
    // Must not panic or error — anomaly events are only emitted under load
    monitor.sample_resources().await;
}

// ── Default construction ─────────────────────────────────────────────────────

#[test]
fn test_default_is_new() {
    let _monitor: SecurityMonitor = Default::default();
}
