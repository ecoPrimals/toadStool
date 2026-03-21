// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive tests for CLI monitoring system

use std::collections::HashMap;
use toadstool_cli::monitoring::*;
use tokio::time::Duration;

// ============================================================================
// MonitoringTarget Tests
// ============================================================================

#[test]
fn test_monitoring_target_biome() {
    let target = MonitoringTarget::Biome("test-biome".to_string());

    match target {
        MonitoringTarget::Biome(name) => assert_eq!(name, "test-biome"),
        _ => panic!("Expected Biome variant"),
    }
}

#[test]
fn test_monitoring_target_service() {
    let target = MonitoringTarget::Service("my-biome".to_string(), "my-service".to_string());

    match target {
        MonitoringTarget::Service(biome, service) => {
            assert_eq!(biome, "my-biome");
            assert_eq!(service, "my-service");
        }
        _ => panic!("Expected Service variant"),
    }
}

#[test]
fn test_monitoring_target_system() {
    let target = MonitoringTarget::System;
    assert!(matches!(target, MonitoringTarget::System));
}

#[test]
fn test_monitoring_target_platform() {
    let target = MonitoringTarget::Platform("linux-x86_64".to_string());

    match target {
        MonitoringTarget::Platform(platform) => assert_eq!(platform, "linux-x86_64"),
        _ => panic!("Expected Platform variant"),
    }
}

#[test]
fn test_monitoring_target_federation() {
    let target = MonitoringTarget::Federation;
    assert!(matches!(target, MonitoringTarget::Federation));
}

#[test]
fn test_monitoring_target_serialization() {
    let target = MonitoringTarget::Biome("test".to_string());
    let json = serde_json::to_string(&target).unwrap();
    let deserialized: MonitoringTarget = serde_json::from_str(&json).unwrap();

    match deserialized {
        MonitoringTarget::Biome(name) => assert_eq!(name, "test"),
        _ => panic!("Deserialization failed"),
    }
}

// ============================================================================
// MetricValue Tests
// ============================================================================

#[test]
fn test_metric_value_counter() {
    let value = MetricValue::Counter(12345);
    match value {
        MetricValue::Counter(v) => assert_eq!(v, 12345),
        _ => panic!("Expected Counter variant"),
    }
}

#[test]
fn test_metric_value_gauge() {
    let value = MetricValue::Gauge(98.6);
    match value {
        MetricValue::Gauge(v) => assert!((v - 98.6).abs() < f64::EPSILON),
        _ => panic!("Expected Gauge variant"),
    }
}

#[test]
fn test_metric_value_histogram() {
    let values = vec![1.0, 2.5, 3.7, 4.2];
    let value = MetricValue::Histogram(values.clone());

    match value {
        MetricValue::Histogram(v) => assert_eq!(v, values),
        _ => panic!("Expected Histogram variant"),
    }
}

#[test]
fn test_metric_value_text() {
    let value = MetricValue::Text("system healthy".to_string());
    match value {
        MetricValue::Text(s) => assert_eq!(s, "system healthy"),
        _ => panic!("Expected Text variant"),
    }
}

#[test]
fn test_metric_value_serialization() {
    let value = MetricValue::Counter(999);
    let json = serde_json::to_string(&value).unwrap();
    let deserialized: MetricValue = serde_json::from_str(&json).unwrap();

    match deserialized {
        MetricValue::Counter(v) => assert_eq!(v, 999),
        _ => panic!("Deserialization failed"),
    }
}

// ============================================================================
// Metric Tests
// ============================================================================

#[test]
fn test_metric_creation() {
    let mut labels = HashMap::new();
    labels.insert("host".to_string(), "localhost".to_string());
    labels.insert("env".to_string(), "production".to_string());

    let metric = Metric {
        name: "cpu_usage".to_string(),
        value: MetricValue::Gauge(75.5),
        labels: labels.clone(),
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(metric.name, "cpu_usage");
    assert_eq!(metric.labels.len(), 2);
    assert_eq!(metric.labels.get("host").unwrap(), "localhost");
}

#[test]
fn test_metric_no_labels() {
    let metric = Metric {
        name: "request_count".to_string(),
        value: MetricValue::Counter(100),
        labels: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(metric.name, "request_count");
    assert!(metric.labels.is_empty());
}

#[test]
fn test_metric_serialization() {
    let metric = Metric {
        name: "memory_bytes".to_string(),
        value: MetricValue::Counter(1024),
        labels: HashMap::new(),
        timestamp: std::time::SystemTime::now(),
    };

    let json = serde_json::to_string(&metric).unwrap();
    let deserialized: Metric = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.name, "memory_bytes");
}

// ============================================================================
// MetricBatch Tests
// ============================================================================

#[test]
fn test_metric_batch_empty() {
    let batch = MetricBatch {
        timestamp: std::time::SystemTime::now(),
        source: "test-collector".to_string(),
        metrics: vec![],
    };

    assert_eq!(batch.source, "test-collector");
    assert!(batch.metrics.is_empty());
}

#[test]
fn test_metric_batch_with_metrics() {
    let metrics = vec![
        Metric {
            name: "cpu".to_string(),
            value: MetricValue::Gauge(50.0),
            labels: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        },
        Metric {
            name: "memory".to_string(),
            value: MetricValue::Counter(2048),
            labels: HashMap::new(),
            timestamp: std::time::SystemTime::now(),
        },
    ];

    let batch = MetricBatch {
        timestamp: std::time::SystemTime::now(),
        source: "system-monitor".to_string(),
        metrics: metrics.clone(),
    };

    assert_eq!(batch.metrics.len(), 2);
    assert_eq!(batch.source, "system-monitor");
}

#[test]
fn test_metric_batch_serialization() {
    let batch = MetricBatch {
        timestamp: std::time::SystemTime::now(),
        source: "test".to_string(),
        metrics: vec![],
    };

    let json = serde_json::to_string(&batch).unwrap();
    let deserialized: MetricBatch = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.source, "test");
}

// ============================================================================
// ComparisonOperator Tests
// ============================================================================

#[test]
fn test_comparison_operator_greater_than() {
    let op = ComparisonOperator::GreaterThan;
    assert!(matches!(op, ComparisonOperator::GreaterThan));
}

#[test]
fn test_comparison_operator_less_than() {
    let op = ComparisonOperator::LessThan;
    assert!(matches!(op, ComparisonOperator::LessThan));
}

#[test]
fn test_comparison_operator_equal() {
    let op = ComparisonOperator::Equal;
    assert!(matches!(op, ComparisonOperator::Equal));
}

#[test]
fn test_comparison_operator_not_equal() {
    let op = ComparisonOperator::NotEqual;
    assert!(matches!(op, ComparisonOperator::NotEqual));
}

#[test]
fn test_comparison_operator_greater_equal() {
    let op = ComparisonOperator::GreaterThanOrEqual;
    assert!(matches!(op, ComparisonOperator::GreaterThanOrEqual));
}

#[test]
fn test_comparison_operator_less_equal() {
    let op = ComparisonOperator::LessThanOrEqual;
    assert!(matches!(op, ComparisonOperator::LessThanOrEqual));
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

// LogicalOperator only has And and Or, no Not variant

// ============================================================================
// AlertSeverity Tests
// ============================================================================

#[test]
fn test_alert_severity_info() {
    let severity = AlertSeverity::Info;
    assert!(matches!(severity, AlertSeverity::Info));
}

#[test]
fn test_alert_severity_warning() {
    let severity = AlertSeverity::Warning;
    assert!(matches!(severity, AlertSeverity::Warning));
}

#[test]
fn test_alert_severity_emergency() {
    let severity = AlertSeverity::Emergency;
    assert!(matches!(severity, AlertSeverity::Emergency));
}

#[test]
fn test_alert_severity_critical() {
    let severity = AlertSeverity::Critical;
    assert!(matches!(severity, AlertSeverity::Critical));
}

#[test]
fn test_alert_severity_serialization() {
    let severity = AlertSeverity::Critical;
    let json = serde_json::to_string(&severity).unwrap();
    let deserialized: AlertSeverity = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, AlertSeverity::Critical));
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
        duration: Duration::from_secs(60),
    };

    match condition {
        AlertCondition::Threshold {
            metric,
            operator,
            value,
            duration,
        } => {
            assert_eq!(metric, "cpu_usage");
            assert!(matches!(operator, ComparisonOperator::GreaterThan));
            assert!((value - 80.0).abs() < f64::EPSILON);
            assert_eq!(duration, Duration::from_secs(60));
        }
        _ => panic!("Expected Threshold variant"),
    }
}

#[test]
fn test_alert_condition_rate_of_change() {
    let condition = AlertCondition::RateOfChange {
        metric: "requests".to_string(),
        threshold: 1000.0,
        window: Duration::from_secs(60),
    };

    match condition {
        AlertCondition::RateOfChange {
            metric,
            threshold,
            window,
        } => {
            assert_eq!(metric, "requests");
            assert!((threshold - 1000.0).abs() < f64::EPSILON);
            assert_eq!(window, Duration::from_secs(60));
        }
        _ => panic!("Expected RateOfChange variant"),
    }
}

#[test]
fn test_alert_condition_composite() {
    let condition = AlertCondition::Composite {
        conditions: vec![AlertCondition::Threshold {
            metric: "cpu".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: 80.0,
            duration: Duration::from_secs(30),
        }],
        operator: LogicalOperator::And,
    };

    match condition {
        AlertCondition::Composite {
            conditions,
            operator,
        } => {
            assert_eq!(conditions.len(), 1);
            assert!(matches!(operator, LogicalOperator::And));
        }
        _ => panic!("Expected Composite variant"),
    }
}

// ============================================================================
// AlertRule Tests
// ============================================================================

#[test]
fn test_alert_rule_creation() {
    let rule = AlertRule {
        id: "alert-1".to_string(),
        name: "High CPU".to_string(),
        condition: AlertCondition::Threshold {
            metric: "cpu".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: 90.0,
            duration: Duration::from_secs(60),
        },
        severity: AlertSeverity::Warning,
        enabled: true,
        cooldown: Duration::from_secs(300),
        last_triggered: None,
    };

    assert_eq!(rule.id, "alert-1");
    assert_eq!(rule.name, "High CPU");
    assert!(rule.enabled);
    assert!(rule.last_triggered.is_none());
}

#[test]
fn test_alert_rule_disabled() {
    let rule = AlertRule {
        id: "alert-2".to_string(),
        name: "Disabled Alert".to_string(),
        condition: AlertCondition::Threshold {
            metric: "test".to_string(),
            operator: ComparisonOperator::Equal,
            value: 0.0,
            duration: Duration::from_secs(30),
        },
        severity: AlertSeverity::Info,
        enabled: false,
        cooldown: Duration::from_secs(60),
        last_triggered: None,
    };

    assert!(!rule.enabled);
}

#[test]
fn test_alert_rule_serialization() {
    let rule = AlertRule {
        id: "test-alert".to_string(),
        name: "Test".to_string(),
        condition: AlertCondition::Threshold {
            metric: "metric".to_string(),
            operator: ComparisonOperator::GreaterThan,
            value: 50.0,
            duration: Duration::from_secs(30),
        },
        severity: AlertSeverity::Warning,
        enabled: true,
        cooldown: Duration::from_secs(60),
        last_triggered: None,
    };

    let json = serde_json::to_string(&rule).unwrap();
    let deserialized: AlertRule = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.id, "test-alert");
    assert_eq!(deserialized.name, "Test");
}

// ============================================================================
// HealthStatus Tests
// ============================================================================

#[test]
fn test_health_status_healthy() {
    let status = HealthStatus::Healthy;
    assert!(matches!(status, HealthStatus::Healthy));
}

#[test]
fn test_health_status_warning() {
    let status = HealthStatus::Warning;
    assert!(matches!(status, HealthStatus::Warning));
}

#[test]
fn test_health_status_critical() {
    let status = HealthStatus::Critical;
    assert!(matches!(status, HealthStatus::Critical));
}

#[test]
fn test_health_status_unknown() {
    let status = HealthStatus::Unknown;
    assert!(matches!(status, HealthStatus::Unknown));
}

#[test]
fn test_health_status_serialization() {
    let status = HealthStatus::Warning;
    let json = serde_json::to_string(&status).unwrap();
    let deserialized: HealthStatus = serde_json::from_str(&json).unwrap();
    assert!(matches!(deserialized, HealthStatus::Warning));
}

// ============================================================================
// DataPoint Tests
// ============================================================================

#[test]
fn test_data_point_creation() {
    let dp = DataPoint {
        timestamp: std::time::SystemTime::now(),
        value: 42.0,
    };

    assert!((dp.value - 42.0).abs() < f64::EPSILON);
}

#[test]
fn test_data_point_zero() {
    let dp = DataPoint {
        timestamp: std::time::SystemTime::now(),
        value: 0.0,
    };

    assert!((dp.value - 0.0).abs() < f64::EPSILON);
}

#[test]
fn test_data_point_negative() {
    let dp = DataPoint {
        timestamp: std::time::SystemTime::now(),
        value: -15.5,
    };

    assert!((dp.value + 15.5).abs() < f64::EPSILON);
}

#[test]
fn test_data_point_serialization() {
    let dp = DataPoint {
        timestamp: std::time::SystemTime::now(),
        value: 100.0,
    };

    let json = serde_json::to_string(&dp).unwrap();
    let deserialized: DataPoint = serde_json::from_str(&json).unwrap();

    assert!((deserialized.value - 100.0).abs() < f64::EPSILON);
}

// ============================================================================
// TimeSeries Tests
// ============================================================================

#[test]
fn test_time_series_empty() {
    let ts = TimeSeries {
        name: "cpu_usage".to_string(),
        data_points: vec![],
        labels: HashMap::new(),
    };

    assert_eq!(ts.name, "cpu_usage");
    assert!(ts.data_points.is_empty());
}

#[test]
fn test_time_series_with_data() {
    let data_points = vec![
        DataPoint {
            timestamp: std::time::SystemTime::now(),
            value: 10.0,
        },
        DataPoint {
            timestamp: std::time::SystemTime::now(),
            value: 20.0,
        },
        DataPoint {
            timestamp: std::time::SystemTime::now(),
            value: 30.0,
        },
    ];

    let ts = TimeSeries {
        name: "memory_usage".to_string(),
        data_points: data_points.clone(),
        labels: HashMap::new(),
    };

    assert_eq!(ts.data_points.len(), 3);
    assert_eq!(ts.name, "memory_usage");
}

// TimeSeries is not Serialize/Deserialize, skip serialization test

// ============================================================================
// MonitoringConfig Tests
// ============================================================================

#[test]
fn test_monitoring_config_default() {
    let config = MonitoringConfig {
        default_interval: Duration::from_secs(60),
        retention_period: Duration::from_secs(30 * 24 * 3600),
        max_metrics_per_batch: 1000,
        enable_alerts: true,
        export_prometheus: true,
        export_grafana: false,
    };

    assert!(config.enable_alerts);
    assert_eq!(config.default_interval, Duration::from_secs(60));
    assert_eq!(config.max_metrics_per_batch, 1000);
}

#[test]
fn test_monitoring_config_disabled() {
    let config = MonitoringConfig {
        default_interval: Duration::from_secs(300),
        retention_period: Duration::from_secs(7 * 24 * 3600),
        max_metrics_per_batch: 500,
        enable_alerts: false,
        export_prometheus: false,
        export_grafana: false,
    };

    assert!(!config.enable_alerts);
    assert!(!config.export_prometheus);
}

// MonitoringConfig is not Serialize/Deserialize, skip serialization test
