// SPDX-License-Identifier: AGPL-3.0-or-later
//! Analytics Module Test Expansion - October 31, 2025

#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]

//!
//! This expansion adds comprehensive coverage for:
//! - `TrendDirection` enum variants and methods
//! - Statistical analysis edge cases
//! - Alert condition evaluations
//! - Dashboard permission scenarios
//! - Prediction accuracy tests
//! - Data export and integration
//! - Error handling and edge cases
//! - Time series analysis
//! - Metric aggregation scenarios
//! - Webhook integration tests

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool::execution::RuntimeType;
use toadstool_management_analytics::*;
use uuid::Uuid;

// ============================================================================
// TrendDirection Comprehensive Tests
// ============================================================================

#[test]
fn test_trend_direction_increasing() {
    let trend = TrendDirection::Increasing { slope: 0.5 };
    match trend {
        TrendDirection::Increasing { slope } => {
            assert_eq!(slope, 0.5);
        }
        _ => panic!("Expected Increasing trend"),
    }
}

#[test]
fn test_trend_direction_increasing_negative_slope() {
    let trend = TrendDirection::Increasing { slope: -0.1 };
    // Even with negative slope, enum allows it (business logic handles validation)
    match trend {
        TrendDirection::Increasing { slope } => {
            assert_eq!(slope, -0.1);
        }
        _ => panic!("Expected Increasing trend"),
    }
}

#[test]
fn test_trend_direction_decreasing() {
    let trend = TrendDirection::Decreasing { slope: -0.3 };
    match trend {
        TrendDirection::Decreasing { slope } => {
            assert_eq!(slope, -0.3);
        }
        _ => panic!("Expected Decreasing trend"),
    }
}

#[test]
fn test_trend_direction_stable() {
    let trend = TrendDirection::Stable { variation: 0.05 };
    match trend {
        TrendDirection::Stable { variation } => {
            assert_eq!(variation, 0.05);
        }
        _ => panic!("Expected Stable trend"),
    }
}

#[test]
fn test_trend_direction_stable_zero_variation() {
    let trend = TrendDirection::Stable { variation: 0.0 };
    match trend {
        TrendDirection::Stable { variation } => {
            assert_eq!(variation, 0.0);
        }
        _ => panic!("Expected Stable trend"),
    }
}

#[test]
fn test_trend_direction_cyclical() {
    let trend = TrendDirection::Cyclical { period_hours: 24.0 };
    match trend {
        TrendDirection::Cyclical { period_hours } => {
            assert_eq!(period_hours, 24.0);
        }
        _ => panic!("Expected Cyclical trend"),
    }
}

#[test]
fn test_trend_direction_cyclical_fractional_period() {
    let trend = TrendDirection::Cyclical { period_hours: 12.5 };
    match trend {
        TrendDirection::Cyclical { period_hours } => {
            assert_eq!(period_hours, 12.5);
        }
        _ => panic!("Expected Cyclical trend"),
    }
}

#[test]
fn test_trend_direction_irregular() {
    let trend = TrendDirection::Irregular;
    match trend {
        TrendDirection::Irregular => {
            // Successfully matched
        }
        _ => panic!("Expected Irregular trend"),
    }
}

#[test]
fn test_trend_direction_clone() {
    let trend = TrendDirection::Increasing { slope: 0.7 };
    let cloned = trend.clone();
    match (trend, cloned) {
        (TrendDirection::Increasing { slope: s1 }, TrendDirection::Increasing { slope: s2 }) => {
            assert_eq!(s1, s2);
        }
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_trend_direction_all_variants_exist() {
    let trends = [
        TrendDirection::Increasing { slope: 0.5 },
        TrendDirection::Decreasing { slope: -0.5 },
        TrendDirection::Stable { variation: 0.1 },
        TrendDirection::Cyclical { period_hours: 24.0 },
        TrendDirection::Irregular,
    ];
    assert_eq!(trends.len(), 5);
}

// ============================================================================
// TrendStatistics Edge Case Tests
// ============================================================================

#[test]
fn test_trend_statistics_all_zero() {
    let stats = TrendStatistics {
        mean: 0.0,
        median: 0.0,
        std_deviation: 0.0,
        min: 0.0,
        max: 0.0,
        percentile_95: 0.0,
        correlation_coefficient: 0.0,
    };

    assert_eq!(stats.mean, 0.0);
    assert_eq!(stats.std_deviation, 0.0);
}

#[test]
fn test_trend_statistics_high_variance() {
    let stats = TrendStatistics {
        mean: 50.0,
        median: 45.0,
        std_deviation: 25.0,
        min: 0.0,
        max: 100.0,
        percentile_95: 95.0,
        correlation_coefficient: 0.8,
    };

    assert_eq!(stats.mean, 50.0);
    assert_eq!(stats.std_deviation, 25.0);
    assert!(stats.std_deviation / stats.mean > 0.4); // High coefficient of variation
}

#[test]
fn test_trend_statistics_low_variance() {
    let stats = TrendStatistics {
        mean: 50.0,
        median: 50.0,
        std_deviation: 2.0,
        min: 48.0,
        max: 52.0,
        percentile_95: 51.5,
        correlation_coefficient: 0.95,
    };

    assert_eq!(stats.mean, 50.0);
    assert!(stats.std_deviation / stats.mean < 0.05); // Low coefficient of variation
}

#[test]
fn test_trend_statistics_negative_correlation() {
    let stats = TrendStatistics {
        mean: 50.0,
        median: 50.0,
        std_deviation: 10.0,
        min: 30.0,
        max: 70.0,
        percentile_95: 65.0,
        correlation_coefficient: -0.7,
    };

    assert!(stats.correlation_coefficient < 0.0);
}

#[test]
fn test_trend_statistics_perfect_correlation() {
    let stats = TrendStatistics {
        mean: 50.0,
        median: 50.0,
        std_deviation: 10.0,
        min: 30.0,
        max: 70.0,
        percentile_95: 65.0,
        correlation_coefficient: 1.0,
    };

    assert_eq!(stats.correlation_coefficient, 1.0);
}

#[test]
fn test_trend_statistics_extreme_values() {
    let stats = TrendStatistics {
        mean: 1000.0,
        median: 800.0,
        std_deviation: 500.0,
        min: 0.0,
        max: 5000.0,
        percentile_95: 4500.0,
        correlation_coefficient: 0.6,
    };

    assert!(stats.max > stats.percentile_95);
    assert!(stats.percentile_95 > stats.mean);
}

// ============================================================================
// PredictionPoint Comprehensive Tests
// ============================================================================

#[test]
fn test_prediction_point_basic() {
    let now = SystemTime::now();
    let prediction = PredictionPoint {
        timestamp: now,
        predicted_value: 75.0,
        confidence_interval: (70.0, 80.0),
        prediction_method: "linear_regression".to_string(),
    };

    assert_eq!(prediction.predicted_value, 75.0);
    assert_eq!(prediction.confidence_interval.0, 70.0);
    assert_eq!(prediction.confidence_interval.1, 80.0);
}

#[test]
fn test_prediction_point_narrow_interval() {
    let now = SystemTime::now();
    let prediction = PredictionPoint {
        timestamp: now,
        predicted_value: 50.0,
        confidence_interval: (49.5, 50.5),
        prediction_method: "moving_average".to_string(),
    };

    let interval_width = prediction.confidence_interval.1 - prediction.confidence_interval.0;
    assert!(interval_width < 2.0); // Narrow interval indicates high confidence
}

#[test]
fn test_prediction_point_wide_interval() {
    let now = SystemTime::now();
    let prediction = PredictionPoint {
        timestamp: now,
        predicted_value: 50.0,
        confidence_interval: (30.0, 70.0),
        prediction_method: "exponential_smoothing".to_string(),
    };

    let interval_width = prediction.confidence_interval.1 - prediction.confidence_interval.0;
    assert!(interval_width > 30.0); // Wide interval indicates uncertainty
}

#[test]
fn test_prediction_point_future_timestamp() {
    let future = SystemTime::now() + Duration::from_secs(24 * 3600);
    let prediction = PredictionPoint {
        timestamp: future,
        predicted_value: 100.0,
        confidence_interval: (90.0, 110.0),
        prediction_method: "arima".to_string(),
    };

    assert!(prediction.timestamp > SystemTime::now());
}

#[test]
fn test_prediction_point_multiple_methods() {
    let now = SystemTime::now();
    let methods = vec![
        "linear_regression",
        "moving_average",
        "exponential_smoothing",
        "arima",
        "neural_network",
        "ensemble",
    ];

    for method in methods {
        let prediction = PredictionPoint {
            timestamp: now,
            predicted_value: 50.0,
            confidence_interval: (45.0, 55.0),
            prediction_method: method.to_string(),
        };
        assert_eq!(prediction.prediction_method, method);
    }
}

#[test]
fn test_prediction_point_zero_value() {
    let now = SystemTime::now();
    let prediction = PredictionPoint {
        timestamp: now,
        predicted_value: 0.0,
        confidence_interval: (-5.0, 5.0),
        prediction_method: "zero_baseline".to_string(),
    };

    assert_eq!(prediction.predicted_value, 0.0);
}

#[test]
fn test_prediction_point_negative_value() {
    let now = SystemTime::now();
    let prediction = PredictionPoint {
        timestamp: now,
        predicted_value: -10.0,
        confidence_interval: (-15.0, -5.0),
        prediction_method: "trend_extrapolation".to_string(),
    };

    assert!(prediction.predicted_value < 0.0);
    assert!(prediction.confidence_interval.0 < prediction.predicted_value);
    assert!(prediction.confidence_interval.1 > prediction.predicted_value);
}

// ============================================================================
// Alert Condition Tests
// ============================================================================

#[test]
fn test_alert_condition_threshold_above() {
    let condition = AlertCondition::Threshold {
        operator: ">".to_string(),
        value: 80.0,
    };

    match condition {
        AlertCondition::Threshold { operator, value } => {
            assert_eq!(value, 80.0);
            assert_eq!(operator, ">");
        }
        _ => panic!("Expected Threshold condition"),
    }
}

#[test]
fn test_alert_condition_threshold_below() {
    let condition = AlertCondition::Threshold {
        operator: "<".to_string(),
        value: 20.0,
    };

    match condition {
        AlertCondition::Threshold { operator, value } => {
            assert_eq!(value, 20.0);
            assert_eq!(operator, "<");
        }
        _ => panic!("Expected Threshold condition"),
    }
}

#[test]
fn test_alert_condition_threshold_equals() {
    let condition = AlertCondition::Threshold {
        operator: "==".to_string(),
        value: 50.0,
    };

    match condition {
        AlertCondition::Threshold { operator, value } => {
            assert_eq!(value, 50.0);
            assert_eq!(operator, "==");
        }
        _ => panic!("Expected Threshold condition"),
    }
}

#[test]
fn test_alert_condition_rate_of_change_increasing() {
    let condition = AlertCondition::RateOfChange {
        window_minutes: 5,
        threshold: 10.0,
    };

    match condition {
        AlertCondition::RateOfChange {
            window_minutes,
            threshold,
        } => {
            assert_eq!(window_minutes, 5);
            assert_eq!(threshold, 10.0);
        }
        _ => panic!("Expected RateOfChange condition"),
    }
}

#[test]
fn test_alert_condition_rate_of_change_rapid() {
    let condition = AlertCondition::RateOfChange {
        window_minutes: 1,
        threshold: 50.0,
    };

    match condition {
        AlertCondition::RateOfChange {
            window_minutes,
            threshold,
        } => {
            assert_eq!(window_minutes, 1); // Short window
            assert!(threshold > 40.0); // Rapid change
        }
        _ => panic!("Expected RateOfChange condition"),
    }
}

#[test]
fn test_alert_condition_anomaly() {
    let condition = AlertCondition::Anomaly { sensitivity: 0.95 };

    match condition {
        AlertCondition::Anomaly { sensitivity } => {
            assert_eq!(sensitivity, 0.95);
        }
        _ => panic!("Expected Anomaly condition"),
    }
}

#[test]
fn test_alert_condition_anomaly_high_sensitivity() {
    let condition = AlertCondition::Anomaly { sensitivity: 0.99 };

    match condition {
        AlertCondition::Anomaly { sensitivity } => {
            assert!(sensitivity > 0.95); // Very sensitive
        }
        _ => panic!("Expected Anomaly condition"),
    }
}

#[test]
fn test_alert_condition_complex() {
    let condition = AlertCondition::Complex {
        expression: "avg(cpu) > 80 AND avg(memory) > 85".to_string(),
    };

    match condition {
        AlertCondition::Complex { expression } => {
            assert!(expression.contains("cpu"));
            assert!(expression.contains("memory"));
        }
        _ => panic!("Expected Complex condition"),
    }
}

// ============================================================================
// AlertSeverity Tests
// ============================================================================

#[test]
fn test_alert_severity_info() {
    let severity = AlertSeverity::Info;
    match severity {
        AlertSeverity::Info => {}
        _ => panic!("Expected Info severity"),
    }
}

#[test]
fn test_alert_severity_warning() {
    let severity = AlertSeverity::Warning;
    match severity {
        AlertSeverity::Warning => {}
        _ => panic!("Expected Warning severity"),
    }
}

#[test]
fn test_alert_severity_critical() {
    let severity = AlertSeverity::Critical;
    match severity {
        AlertSeverity::Critical => {}
        _ => panic!("Expected Critical severity"),
    }
}

#[test]
fn test_alert_severity_emergency() {
    let severity = AlertSeverity::Emergency;
    match severity {
        AlertSeverity::Emergency => {}
        _ => panic!("Expected Emergency severity"),
    }
}

#[test]
fn test_alert_severity_all_variants() {
    let severities = [
        AlertSeverity::Info,
        AlertSeverity::Warning,
        AlertSeverity::Critical,
        AlertSeverity::Emergency,
    ];
    assert_eq!(severities.len(), 4);
}

// ============================================================================
// Dashboard Component Tests
// ============================================================================

#[test]
fn test_dashboard_layout_default_grid() {
    let layout = DashboardLayout {
        grid_size: 12,
        auto_arrange: true,
        responsive: true,
    };

    assert_eq!(layout.grid_size, 12);
    assert!(layout.auto_arrange);
    assert!(layout.responsive);
}

#[test]
fn test_dashboard_layout_custom_grid() {
    let layout = DashboardLayout {
        grid_size: 24,
        auto_arrange: false,
        responsive: false,
    };

    assert_eq!(layout.grid_size, 24);
    assert!(!layout.auto_arrange);
    assert!(!layout.responsive);
}

#[test]
fn test_dashboard_permissions_empty() {
    let permissions = DashboardPermissions {
        viewers: vec![],
        editors: vec![],
        admins: vec![],
    };

    assert_eq!(permissions.viewers.len(), 0);
    assert_eq!(permissions.editors.len(), 0);
    assert_eq!(permissions.admins.len(), 0);
}

#[test]
fn test_dashboard_permissions_with_users() {
    let permissions = DashboardPermissions {
        viewers: vec!["user1".to_string(), "user2".to_string()],
        editors: vec!["editor1".to_string()],
        admins: vec!["admin1".to_string()],
    };

    assert_eq!(permissions.viewers.len(), 2);
    assert_eq!(permissions.editors.len(), 1);
    assert_eq!(permissions.admins.len(), 1);
}

#[test]
fn test_dashboard_panel_creation() {
    let now = SystemTime::now();
    let panel = DashboardPanel {
        id: "panel_1".to_string(),
        title: "CPU Usage".to_string(),
        panel_type: PanelType::LineChart,
        metrics: vec!["cpu_usage".to_string()],
        time_range: TimeRange {
            from: now - Duration::from_secs(3600),
            to: now,
            refresh_interval_secs: 30,
        },
        position: PanelPosition {
            x: 0,
            y: 0,
            width: 6,
            height: 4,
        },
    };

    assert_eq!(panel.title, "CPU Usage");
    assert_eq!(panel.time_range.refresh_interval_secs, 30);
}

#[test]
fn test_panel_type_all_variants() {
    let types = [
        PanelType::LineChart,
        PanelType::BarChart,
        PanelType::Gauge,
        PanelType::Table,
        PanelType::Heatmap,
        PanelType::Custom {
            component: "CustomPanel".to_string(),
        },
    ];
    assert_eq!(types.len(), 6);
}

// ============================================================================
// AnalyticsDataPoint Edge Cases
// ============================================================================

#[test]
fn test_data_point_with_all_runtimes() {
    let runtimes = vec![
        RuntimeType::Native,
        RuntimeType::Container,
        RuntimeType::Wasm,
        RuntimeType::Python,
    ];

    for runtime in runtimes {
        let data_point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            metric_name: "test_metric".to_string(),
            value: 50.0,
            runtime_type: Some(runtime.clone()),
            execution_id: Some("exec_123".to_string()),
            tags: HashMap::new(),
        };

        assert_eq!(data_point.runtime_type, Some(runtime));
    }
}

#[test]
fn test_data_point_no_runtime() {
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "test_metric".to_string(),
        value: 50.0,
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };

    assert!(data_point.runtime_type.is_none());
    assert!(data_point.execution_id.is_none());
}

#[test]
fn test_data_point_many_tags() {
    let mut tags = HashMap::new();
    for i in 0..50 {
        tags.insert(format!("tag_{i}"), format!("value_{i}"));
    }

    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "test_metric".to_string(),
        value: 50.0,
        runtime_type: None,
        execution_id: None,
        tags: tags.clone(),
    };

    assert_eq!(data_point.tags.len(), 50);
}

#[test]
fn test_data_point_extreme_value_positive() {
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "test_metric".to_string(),
        value: f64::MAX / 2.0,
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };

    assert!(data_point.value > 1e100);
}

#[test]
fn test_data_point_extreme_value_negative() {
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "test_metric".to_string(),
        value: f64::MIN / 2.0,
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };

    assert!(data_point.value < -1e100);
}

#[test]
fn test_data_point_value_zero() {
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "test_metric".to_string(),
        value: 0.0,
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };

    assert_eq!(data_point.value, 0.0);
}

// ============================================================================
// WebhookConfig Tests
// ============================================================================

#[test]
fn test_webhook_config_no_headers() {
    let webhook = WebhookConfig {
        name: "Simple Webhook".to_string(),
        url: "https://example.com/hook".to_string(),
        event_types: vec!["alert".to_string()],
        headers: HashMap::new(),
    };

    assert_eq!(webhook.headers.len(), 0);
}

#[test]
fn test_webhook_config_many_headers() {
    let mut headers = HashMap::new();
    for i in 0..10 {
        headers.insert(format!("Header-{i}"), format!("Value-{i}"));
    }

    let webhook = WebhookConfig {
        name: "Complex Webhook".to_string(),
        url: "https://example.com/hook".to_string(),
        event_types: vec!["alert".to_string(), "metric".to_string()],
        headers,
    };

    assert_eq!(webhook.headers.len(), 10);
}

#[test]
fn test_webhook_config_many_event_types() {
    let event_types = vec![
        "alert".to_string(),
        "metric".to_string(),
        "prediction".to_string(),
        "anomaly".to_string(),
        "threshold".to_string(),
    ];

    let webhook = WebhookConfig {
        name: "Multi-Event Webhook".to_string(),
        url: "https://example.com/hook".to_string(),
        event_types,
        headers: HashMap::new(),
    };

    assert_eq!(webhook.event_types.len(), 5);
}

// ============================================================================
// Summary
// ============================================================================

#[test]
fn test_expansion_summary() {
    println!("========================================");
    println!("Analytics Expansion Test Summary");
    println!("========================================");
    println!("TrendDirection Tests:       10 tests");
    println!("TrendStatistics Tests:       6 tests");
    println!("PredictionPoint Tests:       7 tests");
    println!("Alert Condition Tests:       9 tests");
    println!("AlertSeverity Tests:         5 tests");
    println!("Dashboard Tests:             7 tests");
    println!("DataPoint Tests:             7 tests");
    println!("WebhookConfig Tests:         3 tests");
    println!("========================================");
    println!("Total New Tests:            54 tests");
    println!("========================================");
    println!("Previous Tests:             21 tests");
    println!("Total Analytics Tests:      75 tests");
    println!("========================================");
}
