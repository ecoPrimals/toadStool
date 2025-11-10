//! Comprehensive tests for monitoring system

use chrono::Utc;
use std::collections::HashMap;
use std::time::Duration;
use toadstool_cli::monitoring::*;
use uuid::Uuid;

// ============================================================================
// MonitoringTarget Tests
// ============================================================================

#[test]
fn test_monitoring_target_biome() {
    let target = MonitoringTarget::Biome("test-biome".to_string());

    if let MonitoringTarget::Biome(name) = target {
        assert_eq!(name, "test-biome");
    } else {
        panic!("Expected Biome target");
    }
}

#[test]
fn test_monitoring_target_service() {
    let target = MonitoringTarget::Service("biome1".to_string(), "service1".to_string());

    if let MonitoringTarget::Service(biome, service) = target {
        assert_eq!(biome, "biome1");
        assert_eq!(service, "service1");
    } else {
        panic!("Expected Service target");
    }
}

#[test]
fn test_monitoring_target_system() {
    let target = MonitoringTarget::System;
    assert!(matches!(target, MonitoringTarget::System));
}

#[test]
fn test_monitoring_target_platform() {
    let target = MonitoringTarget::Platform("kubernetes".to_string());

    if let MonitoringTarget::Platform(platform) = target {
        assert_eq!(platform, "kubernetes");
    }
}

#[test]
fn test_monitoring_target_federation() {
    let target = MonitoringTarget::Federation;
    assert!(matches!(target, MonitoringTarget::Federation));
}

#[test]
fn test_monitoring_target_serialization() {
    let targets = vec![
        MonitoringTarget::System,
        MonitoringTarget::Federation,
        MonitoringTarget::Biome("test".to_string()),
    ];

    for target in targets {
        let json = serde_json::to_string(&target).expect("Failed to serialize");
        let _deserialized: MonitoringTarget =
            serde_json::from_str(&json).expect("Failed to deserialize");
    }
}

// ============================================================================
// SessionStatus Tests
// ============================================================================

#[test]
fn test_session_status_active() {
    let status = SessionStatus::Active;
    assert!(matches!(status, SessionStatus::Active));
}

#[test]
fn test_session_status_paused() {
    let status = SessionStatus::Paused;
    assert!(matches!(status, SessionStatus::Paused));
}

#[test]
fn test_session_status_stopped() {
    let status = SessionStatus::Stopped;
    assert!(matches!(status, SessionStatus::Stopped));
}

#[test]
fn test_session_status_error() {
    let status = SessionStatus::Error("Connection failed".to_string());

    if let SessionStatus::Error(msg) = status {
        assert_eq!(msg, "Connection failed");
    } else {
        panic!("Expected Error status");
    }
}

#[test]
fn test_session_status_clone() {
    let status = SessionStatus::Active;
    let cloned = status.clone();
    assert!(matches!(cloned, SessionStatus::Active));
}

// ============================================================================
// MonitoringSession Tests
// ============================================================================

#[test]
fn test_monitoring_session_creation() {
    let session = MonitoringSession {
        id: Uuid::new_v4(),
        target: MonitoringTarget::System,
        started: Utc::now(),
        interval: Duration::from_secs(60),
        metrics: vec!["cpu".to_string(), "memory".to_string()],
        status: SessionStatus::Active,
        last_update: Utc::now(),
    };

    assert_eq!(session.metrics.len(), 2);
    assert!(matches!(session.status, SessionStatus::Active));
}

#[test]
fn test_monitoring_session_biome_target() {
    let session = MonitoringSession {
        id: Uuid::new_v4(),
        target: MonitoringTarget::Biome("production".to_string()),
        started: Utc::now(),
        interval: Duration::from_secs(30),
        metrics: vec!["requests".to_string()],
        status: SessionStatus::Active,
        last_update: Utc::now(),
    };

    assert!(matches!(session.target, MonitoringTarget::Biome(_)));
}

#[test]
fn test_monitoring_session_clone() {
    let session = MonitoringSession {
        id: Uuid::new_v4(),
        target: MonitoringTarget::System,
        started: Utc::now(),
        interval: Duration::from_secs(60),
        metrics: vec![],
        status: SessionStatus::Active,
        last_update: Utc::now(),
    };

    let cloned = session.clone();
    assert_eq!(session.id, cloned.id);
}

// ============================================================================
// MetricValue Tests
// ============================================================================

#[test]
fn test_metric_value_counter() {
    let value = MetricValue::Counter(100);

    if let MetricValue::Counter(count) = value {
        assert_eq!(count, 100);
    } else {
        panic!("Expected Counter value");
    }
}

#[test]
fn test_metric_value_gauge() {
    let value = MetricValue::Gauge(75.5);

    if let MetricValue::Gauge(gauge) = value {
        assert_eq!(gauge, 75.5);
    } else {
        panic!("Expected Gauge value");
    }
}

#[test]
fn test_metric_value_histogram() {
    let value = MetricValue::Histogram(vec![1.0, 2.0, 3.0]);

    if let MetricValue::Histogram(hist) = value {
        assert_eq!(hist.len(), 3);
    } else {
        panic!("Expected Histogram value");
    }
}

#[test]
fn test_metric_value_text() {
    let value = MetricValue::Text("healthy".to_string());

    if let MetricValue::Text(text) = value {
        assert_eq!(text, "healthy");
    } else {
        panic!("Expected Text value");
    }
}

#[test]
fn test_metric_value_serialization() {
    let values = vec![
        MetricValue::Counter(42),
        MetricValue::Gauge(3.15), // Not PI, just a test value
        MetricValue::Text("test".to_string()),
    ];

    for value in values {
        let json = serde_json::to_string(&value).expect("Failed to serialize");
        let _deserialized: MetricValue =
            serde_json::from_str(&json).expect("Failed to deserialize");
    }
}

// ============================================================================
// Metric Tests
// ============================================================================

#[test]
fn test_metric_creation() {
    let metric = Metric {
        name: "cpu_usage".to_string(),
        value: MetricValue::Gauge(45.2),
        labels: HashMap::new(),
        timestamp: Utc::now(),
    };

    assert_eq!(metric.name, "cpu_usage");
}

#[test]
fn test_metric_with_labels() {
    let mut labels = HashMap::new();
    labels.insert("host".to_string(), "server1".to_string());
    labels.insert("region".to_string(), "us-west".to_string());

    let metric = Metric {
        name: "requests".to_string(),
        value: MetricValue::Counter(1000),
        labels,
        timestamp: Utc::now(),
    };

    assert_eq!(metric.labels.len(), 2);
    assert_eq!(metric.labels.get("host").unwrap(), "server1");
}

#[test]
fn test_metric_serialization() {
    let metric = Metric {
        name: "test_metric".to_string(),
        value: MetricValue::Counter(123),
        labels: HashMap::new(),
        timestamp: Utc::now(),
    };

    let json = serde_json::to_string(&metric).expect("Failed to serialize");
    let deserialized: Metric = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(metric.name, deserialized.name);
}

// ============================================================================
// MetricBatch Tests
// ============================================================================

#[test]
fn test_metric_batch_creation() {
    let batch = MetricBatch {
        timestamp: Utc::now(),
        source: "monitoring-agent".to_string(),
        metrics: vec![],
    };

    assert_eq!(batch.source, "monitoring-agent");
    assert_eq!(batch.metrics.len(), 0);
}

#[test]
fn test_metric_batch_with_metrics() {
    let metric1 = Metric {
        name: "cpu".to_string(),
        value: MetricValue::Gauge(50.0),
        labels: HashMap::new(),
        timestamp: Utc::now(),
    };

    let metric2 = Metric {
        name: "memory".to_string(),
        value: MetricValue::Gauge(70.0),
        labels: HashMap::new(),
        timestamp: Utc::now(),
    };

    let batch = MetricBatch {
        timestamp: Utc::now(),
        source: "system".to_string(),
        metrics: vec![metric1, metric2],
    };

    assert_eq!(batch.metrics.len(), 2);
}

#[test]
fn test_metric_batch_serialization() {
    let batch = MetricBatch {
        timestamp: Utc::now(),
        source: "test".to_string(),
        metrics: vec![],
    };

    let json = serde_json::to_string(&batch).expect("Failed to serialize");
    let deserialized: MetricBatch = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(batch.source, deserialized.source);
}

// ============================================================================
// ComparisonOperator Tests
// ============================================================================

#[test]
fn test_comparison_operator_variants() {
    let operators = vec![
        ComparisonOperator::GreaterThan,
        ComparisonOperator::LessThan,
        ComparisonOperator::Equal,
        ComparisonOperator::NotEqual,
        ComparisonOperator::GreaterThanOrEqual,
        ComparisonOperator::LessThanOrEqual,
    ];

    for op in operators {
        let _debug = format!("{:?}", op);
    }
}

#[test]
fn test_comparison_operator_serialization() {
    let op = ComparisonOperator::GreaterThan;
    let json = serde_json::to_string(&op).expect("Failed to serialize");
    let _deserialized: ComparisonOperator =
        serde_json::from_str(&json).expect("Failed to deserialize");
}

// ============================================================================
// LogicalOperator Tests
// ============================================================================

#[test]
fn test_logical_operator_and() {
    let op = LogicalOperator::And;
    assert!(matches!(op, LogicalOperator::And));
}

#[test]
fn test_logical_operator_or() {
    let op = LogicalOperator::Or;
    assert!(matches!(op, LogicalOperator::Or));
}

// LogicalOperator only has And/Or variants, no Not

// ============================================================================
// AlertSeverity Tests
// ============================================================================

#[test]
fn test_alert_severity_variants() {
    let severities = vec![
        AlertSeverity::Info,
        AlertSeverity::Warning,
        AlertSeverity::Critical,
    ];

    for severity in severities {
        let _debug = format!("{:?}", severity);
    }
}

#[test]
fn test_alert_severity_serialization() {
    let severity = AlertSeverity::Critical;
    let json = serde_json::to_string(&severity).expect("Failed to serialize");
    let _deserialized: AlertSeverity = serde_json::from_str(&json).expect("Failed to deserialize");
}

// ============================================================================
// AlertCondition Tests
// ============================================================================

#[test]
fn test_alert_condition_threshold() {
    let condition = AlertCondition::Threshold {
        metric: "cpu_usage".to_string(),
        operator: ComparisonOperator::GreaterThan,
        value: 80.0,
        duration: Duration::from_secs(300),
    };

    if let AlertCondition::Threshold { metric, value, .. } = condition {
        assert_eq!(metric, "cpu_usage");
        assert_eq!(value, 80.0);
    } else {
        panic!("Expected Threshold condition");
    }
}

#[test]
fn test_alert_condition_rate_of_change() {
    let condition = AlertCondition::RateOfChange {
        metric: "requests".to_string(),
        threshold: 100.0,
        window: Duration::from_secs(60),
    };

    if let AlertCondition::RateOfChange {
        metric, threshold, ..
    } = condition
    {
        assert_eq!(metric, "requests");
        assert_eq!(threshold, 100.0);
    }
}

#[test]
fn test_alert_condition_composite() {
    let condition1 = AlertCondition::Threshold {
        metric: "cpu".to_string(),
        operator: ComparisonOperator::GreaterThan,
        value: 80.0,
        duration: Duration::from_secs(60),
    };

    let condition2 = AlertCondition::Threshold {
        metric: "memory".to_string(),
        operator: ComparisonOperator::GreaterThan,
        value: 90.0,
        duration: Duration::from_secs(60),
    };

    let composite = AlertCondition::Composite {
        conditions: vec![condition1, condition2],
        operator: LogicalOperator::And,
    };

    if let AlertCondition::Composite { conditions, .. } = composite {
        assert_eq!(conditions.len(), 2);
    }
}

// ============================================================================
// AlertRule Tests
// ============================================================================

#[test]
fn test_alert_rule_creation() {
    let rule = AlertRule {
        id: "rule-1".to_string(),
        name: "High CPU Usage".to_string(),
        condition: AlertCondition::Threshold {
            metric: "cpu".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: 85.0,
            duration: Duration::from_secs(300),
        },
        severity: AlertSeverity::Warning,
        enabled: true,
        cooldown: Duration::from_secs(600),
        last_triggered: None,
    };

    assert_eq!(rule.name, "High CPU Usage");
    assert!(rule.enabled);
}

#[test]
fn test_alert_rule_disabled() {
    let rule = AlertRule {
        id: "rule-2".to_string(),
        name: "Test Rule".to_string(),
        condition: AlertCondition::Threshold {
            metric: "test".to_string(),
            operator: ComparisonOperator::Equal,
            value: 0.0,
            duration: Duration::from_secs(1),
        },
        severity: AlertSeverity::Info,
        enabled: false,
        cooldown: Duration::from_secs(0),
        last_triggered: None,
    };

    assert!(!rule.enabled);
}

#[test]
fn test_alert_rule_serialization() {
    let rule = AlertRule {
        id: "test".to_string(),
        name: "Test".to_string(),
        condition: AlertCondition::Threshold {
            metric: "m".to_string(),
            operator: ComparisonOperator::Equal,
            value: 1.0,
            duration: Duration::from_secs(1),
        },
        severity: AlertSeverity::Info,
        enabled: true,
        cooldown: Duration::from_secs(60),
        last_triggered: None,
    };

    let json = serde_json::to_string(&rule).expect("Failed to serialize");
    let deserialized: AlertRule = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(rule.id, deserialized.id);
    assert_eq!(rule.name, deserialized.name);
}

// ============================================================================
// HealthStatus Tests
// ============================================================================

#[test]
fn test_health_status_variants() {
    let statuses = vec![
        HealthStatus::Healthy,
        HealthStatus::Warning,
        HealthStatus::Critical,
        HealthStatus::Unknown,
    ];

    for status in statuses {
        let _debug = format!("{:?}", status);
    }
}
