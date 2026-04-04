// SPDX-License-Identifier: AGPL-3.0-only
#![expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Comprehensive tests for analytics types

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool_management_analytics::*;
use uuid::Uuid;

// ==================== Config Types Tests ====================

#[test]
fn test_analytics_config_default() {
    let config = AnalyticsConfig::default();

    assert!(config.enable_realtime);
    assert_eq!(config.retention_days, 90);
    assert_eq!(config.prediction_window_hours, 24);
    assert_eq!(config.collection_interval_secs, 60);
    assert_eq!(config.alert_thresholds.cpu_threshold, 80.0);
    assert_eq!(config.alert_thresholds.memory_threshold, 85.0);
}

#[test]
fn test_analytics_config_custom() {
    let config = AnalyticsConfig {
        enable_realtime: false,
        retention_days: 90,
        prediction_window_hours: 48,
        collection_interval_secs: 30,
        alert_thresholds: AlertThresholds {
            cpu_threshold: 90.0,
            memory_threshold: 95.0,
            error_rate_threshold: 10.0,
            response_time_threshold: 2000,
        },
        external_integrations: ExternalIntegrations { webhooks: vec![] },
    };

    assert!(!config.enable_realtime);
    assert_eq!(config.retention_days, 90);
    assert_eq!(config.alert_thresholds.cpu_threshold, 90.0);
}

#[test]
fn test_alert_thresholds_creation() {
    let thresholds = AlertThresholds {
        cpu_threshold: 85.0,
        memory_threshold: 90.0,
        error_rate_threshold: 5.0,
        response_time_threshold: 1500,
    };

    assert_eq!(thresholds.cpu_threshold, 85.0);
    assert_eq!(thresholds.response_time_threshold, 1500);
}

#[test]
fn test_webhook_config_creation() {
    let mut headers = HashMap::new();
    headers.insert("Authorization".to_string(), "Bearer token".to_string());

    let webhook = WebhookConfig {
        name: "slack-webhook".to_string(),
        url: "https://hooks.slack.com/test".to_string(),
        event_types: vec!["alert".to_string(), "warning".to_string()],
        headers: headers.clone(),
    };

    assert_eq!(webhook.name, "slack-webhook");
    assert_eq!(webhook.event_types.len(), 2);
    assert_eq!(webhook.headers.len(), 1);
}

#[test]
fn test_external_integrations_empty() {
    let integrations = ExternalIntegrations { webhooks: vec![] };

    assert!(integrations.webhooks.is_empty());
}

#[test]
fn test_external_integrations_with_webhooks() {
    let webhook = WebhookConfig {
        name: "test".to_string(),
        url: "https://example.com".to_string(),
        event_types: vec!["all".to_string()],
        headers: HashMap::new(),
    };

    let integrations = ExternalIntegrations {
        webhooks: vec![webhook],
    };

    assert_eq!(integrations.webhooks.len(), 1);
}

// ==================== Data Point Tests ====================

#[test]
fn test_analytics_data_point_creation() {
    let mut tags = HashMap::new();
    tags.insert("environment".to_string(), "production".to_string());

    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "cpu_usage".to_string(),
        value: 75.5,
        runtime_type: None,
        execution_id: Some("exec-123".to_string()),
        tags: tags.clone(),
    };

    assert_eq!(data_point.metric_name, "cpu_usage");
    assert_eq!(data_point.value, 75.5);
    assert_eq!(data_point.tags.len(), 1);
}

#[test]
fn test_analytics_data_point_no_tags() {
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "memory_usage".to_string(),
        value: 60.0,
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };

    assert!(data_point.tags.is_empty());
    assert!(data_point.execution_id.is_none());
}

#[test]
fn test_analytics_data_point_serialization() {
    let data_point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        timestamp: SystemTime::now(),
        metric_name: "test".to_string(),
        value: 42.0,
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };

    let serialized = serde_json::to_string(&data_point).expect("Failed to serialize");
    let deserialized: AnalyticsDataPoint =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(deserialized.metric_name, data_point.metric_name);
    assert_eq!(deserialized.value, data_point.value);
}

// ==================== Trend Analysis Tests ====================

#[test]
fn test_trend_direction_increasing() {
    let trend = TrendDirection::Increasing { slope: 0.8 };

    let debug_str = format!("{trend:?}");
    assert!(debug_str.contains("Increasing"));
}

#[test]
fn test_trend_direction_decreasing() {
    let trend = TrendDirection::Decreasing { slope: 0.5 };

    let debug_str = format!("{trend:?}");
    assert!(debug_str.contains("Decreasing"));
}

#[test]
fn test_trend_direction_stable() {
    let trend = TrendDirection::Stable { variation: 0.05 };

    let debug_str = format!("{trend:?}");
    assert!(debug_str.contains("Stable"));
}

#[test]
fn test_trend_direction_cyclical() {
    let trend = TrendDirection::Cyclical { period_hours: 24.0 };

    let debug_str = format!("{trend:?}");
    assert!(debug_str.contains("Cyclical"));
}

#[test]
fn test_trend_direction_irregular() {
    let trend = TrendDirection::Irregular;

    let debug_str = format!("{trend:?}");
    assert!(debug_str.contains("Irregular"));
}

#[test]
fn test_trend_statistics_creation() {
    let stats = TrendStatistics {
        mean: 75.0,
        median: 74.0,
        std_deviation: 10.0,
        min: 50.0,
        max: 95.0,
        percentile_95: 92.0,
        correlation_coefficient: 0.85,
    };

    assert_eq!(stats.mean, 75.0);
    assert_eq!(stats.median, 74.0);
    assert_eq!(stats.percentile_95, 92.0);
}

#[test]
fn test_prediction_point_creation() {
    let prediction = PredictionPoint {
        timestamp: SystemTime::now(),
        predicted_value: 80.0,
        confidence_interval: (70.0, 90.0),
        prediction_method: "linear_regression".to_string(),
    };

    assert_eq!(prediction.predicted_value, 80.0);
    assert_eq!(prediction.confidence_interval.0, 70.0);
    assert_eq!(prediction.confidence_interval.1, 90.0);
}

#[test]
fn test_trend_analysis_creation() {
    let trend = TrendAnalysis {
        metric_name: "cpu_usage".to_string(),
        start_time: SystemTime::now(),
        end_time: SystemTime::now(),
        trend: TrendDirection::Increasing { slope: 0.5 },
        statistics: TrendStatistics {
            mean: 70.0,
            median: 68.0,
            std_deviation: 8.0,
            min: 50.0,
            max: 90.0,
            percentile_95: 88.0,
            correlation_coefficient: 0.75,
        },
        confidence: 0.9,
        predictions: vec![],
    };

    assert_eq!(trend.metric_name, "cpu_usage");
    assert_eq!(trend.confidence, 0.9);
}

// ==================== Alert Tests ====================

#[test]
fn test_alert_severity_levels() {
    let info = AlertSeverity::Info;
    let warning = AlertSeverity::Warning;
    let critical = AlertSeverity::Critical;
    let emergency = AlertSeverity::Emergency;

    let debug_info = format!("{info:?}");
    let debug_warning = format!("{warning:?}");
    let debug_critical = format!("{critical:?}");
    let debug_emergency = format!("{emergency:?}");

    assert!(debug_info.contains("Info"));
    assert!(debug_warning.contains("Warning"));
    assert!(debug_critical.contains("Critical"));
    assert!(debug_emergency.contains("Emergency"));
}

#[test]
fn test_alert_status_variants() {
    let active = AlertStatus::Active;
    let suppressed = AlertStatus::Suppressed;
    let resolved = AlertStatus::Resolved;

    let debug_active = format!("{active:?}");
    let debug_suppressed = format!("{suppressed:?}");
    let debug_resolved = format!("{resolved:?}");

    assert!(debug_active.contains("Active"));
    assert!(debug_suppressed.contains("Suppressed"));
    assert!(debug_resolved.contains("Resolved"));
}

#[test]
fn test_alert_condition_threshold() {
    let condition = AlertCondition::Threshold {
        operator: ">".to_string(),
        value: 80.0,
    };

    let debug_str = format!("{condition:?}");
    assert!(debug_str.contains("Threshold"));
}

#[test]
fn test_alert_condition_rate_of_change() {
    let condition = AlertCondition::RateOfChange {
        window_minutes: 5,
        threshold: 20.0,
    };

    let debug_str = format!("{condition:?}");
    assert!(debug_str.contains("RateOfChange"));
}

#[test]
fn test_alert_condition_anomaly() {
    let condition = AlertCondition::Anomaly { sensitivity: 0.95 };

    let debug_str = format!("{condition:?}");
    assert!(debug_str.contains("Anomaly"));
}

#[test]
fn test_alert_condition_complex() {
    let condition = AlertCondition::Complex {
        expression: "cpu > 80 AND memory > 90".to_string(),
    };

    let debug_str = format!("{condition:?}");
    assert!(debug_str.contains("Complex"));
}

#[test]
fn test_alert_creation() {
    let alert = Alert {
        id: Uuid::new_v4(),
        name: "High CPU Usage".to_string(),
        metric_name: "cpu_usage".to_string(),
        condition: AlertCondition::Threshold {
            operator: ">".to_string(),
            value: 80.0,
        },
        severity: AlertSeverity::Warning,
        created_at: SystemTime::now(),
        last_triggered: None,
        status: AlertStatus::Active,
        recipients: vec!["admin@example.com".to_string()],
    };

    assert_eq!(alert.name, "High CPU Usage");
    assert_eq!(alert.recipients.len(), 1);
    assert!(alert.last_triggered.is_none());
}

#[test]
fn test_alert_with_multiple_recipients() {
    let alert = Alert {
        id: Uuid::new_v4(),
        name: "Critical Alert".to_string(),
        metric_name: "error_rate".to_string(),
        condition: AlertCondition::Threshold {
            operator: ">".to_string(),
            value: 5.0,
        },
        severity: AlertSeverity::Critical,
        created_at: SystemTime::now(),
        last_triggered: Some(SystemTime::now()),
        status: AlertStatus::Active,
        recipients: vec![
            "admin@example.com".to_string(),
            "oncall@example.com".to_string(),
            "team@example.com".to_string(),
        ],
    };

    assert_eq!(alert.recipients.len(), 3);
    assert!(alert.last_triggered.is_some());
}

// ==================== Dashboard Tests ====================

#[test]
fn test_panel_type_variants() {
    let line = PanelType::LineChart;
    let bar = PanelType::BarChart;
    let gauge = PanelType::Gauge;
    let table = PanelType::Table;
    let heatmap = PanelType::Heatmap;
    let custom = PanelType::Custom {
        component: "CustomViz".to_string(),
    };

    let debug_line = format!("{line:?}");
    let debug_bar = format!("{bar:?}");
    let debug_gauge = format!("{gauge:?}");
    let debug_table = format!("{table:?}");
    let debug_heatmap = format!("{heatmap:?}");
    let debug_custom = format!("{custom:?}");

    assert!(debug_line.contains("LineChart"));
    assert!(debug_bar.contains("BarChart"));
    assert!(debug_gauge.contains("Gauge"));
    assert!(debug_table.contains("Table"));
    assert!(debug_heatmap.contains("Heatmap"));
    assert!(debug_custom.contains("Custom"));
}

#[test]
fn test_time_range_creation() {
    let now = SystemTime::now();
    let time_range = TimeRange {
        from: now - Duration::from_secs(24 * 3600),
        to: now,
        refresh_interval_secs: 30,
    };

    assert_eq!(time_range.refresh_interval_secs, 30);
}

#[test]
fn test_panel_position_creation() {
    let position = PanelPosition {
        x: 0,
        y: 0,
        width: 6,
        height: 4,
    };

    assert_eq!(position.x, 0);
    assert_eq!(position.width, 6);
    assert_eq!(position.height, 4);
}

#[test]
fn test_dashboard_panel_creation() {
    let now = SystemTime::now();
    let panel = DashboardPanel {
        id: "panel-1".to_string(),
        title: "CPU Usage".to_string(),
        panel_type: PanelType::LineChart,
        metrics: vec!["cpu_usage".to_string(), "cpu_load".to_string()],
        time_range: TimeRange {
            from: now - Duration::from_secs(3600),
            to: now,
            refresh_interval_secs: 10,
        },
        position: PanelPosition {
            x: 0,
            y: 0,
            width: 6,
            height: 3,
        },
    };

    assert_eq!(panel.id, "panel-1");
    assert_eq!(panel.metrics.len(), 2);
}

#[test]
fn test_dashboard_layout_creation() {
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
fn test_dashboard_permissions_creation() {
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
fn test_dashboard_permissions_empty() {
    let permissions = DashboardPermissions {
        viewers: vec![],
        editors: vec![],
        admins: vec![],
    };

    assert!(permissions.viewers.is_empty());
    assert!(permissions.editors.is_empty());
    assert!(permissions.admins.is_empty());
}

#[test]
fn test_dashboard_creation() {
    let dashboard = Dashboard {
        id: Uuid::new_v4(),
        name: "System Overview".to_string(),
        description: "Main system dashboard".to_string(),
        panels: vec![],
        layout: DashboardLayout {
            grid_size: 12,
            auto_arrange: false,
            responsive: true,
        },
        permissions: DashboardPermissions {
            viewers: vec![],
            editors: vec![],
            admins: vec!["admin".to_string()],
        },
    };

    assert_eq!(dashboard.name, "System Overview");
    assert!(dashboard.panels.is_empty());
}

#[test]
fn test_dashboard_with_multiple_panels() {
    let now = SystemTime::now();
    let panel1 = DashboardPanel {
        id: "panel-1".to_string(),
        title: "CPU".to_string(),
        panel_type: PanelType::LineChart,
        metrics: vec!["cpu".to_string()],
        time_range: TimeRange {
            from: now - Duration::from_secs(3600),
            to: now,
            refresh_interval_secs: 10,
        },
        position: PanelPosition {
            x: 0,
            y: 0,
            width: 6,
            height: 3,
        },
    };

    let panel2 = DashboardPanel {
        id: "panel-2".to_string(),
        title: "Memory".to_string(),
        panel_type: PanelType::LineChart,
        metrics: vec!["memory".to_string()],
        time_range: TimeRange {
            from: now - Duration::from_secs(3600),
            to: now,
            refresh_interval_secs: 10,
        },
        position: PanelPosition {
            x: 6,
            y: 0,
            width: 6,
            height: 3,
        },
    };

    let dashboard = Dashboard {
        id: Uuid::new_v4(),
        name: "Multi-Panel Dashboard".to_string(),
        description: "Dashboard with multiple panels".to_string(),
        panels: vec![panel1, panel2],
        layout: DashboardLayout {
            grid_size: 12,
            auto_arrange: false,
            responsive: true,
        },
        permissions: DashboardPermissions {
            viewers: vec![],
            editors: vec![],
            admins: vec![],
        },
    };

    assert_eq!(dashboard.panels.len(), 2);
}

// ==================== Serialization Tests ====================

#[test]
fn test_alert_severity_serialization() {
    let severity = AlertSeverity::Critical;
    let serialized = serde_json::to_string(&severity).expect("Failed to serialize");
    let deserialized: AlertSeverity =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    let debug1 = format!("{severity:?}");
    let debug2 = format!("{deserialized:?}");
    assert_eq!(debug1, debug2);
}

#[test]
fn test_alert_status_serialization() {
    let status = AlertStatus::Active;
    let serialized = serde_json::to_string(&status).expect("Failed to serialize");
    let deserialized: AlertStatus =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    let debug1 = format!("{status:?}");
    let debug2 = format!("{deserialized:?}");
    assert_eq!(debug1, debug2);
}

#[test]
fn test_panel_type_serialization() {
    let panel_type = PanelType::Gauge;
    let serialized = serde_json::to_string(&panel_type).expect("Failed to serialize");
    let deserialized: PanelType = serde_json::from_str(&serialized).expect("Failed to deserialize");

    let debug1 = format!("{panel_type:?}");
    let debug2 = format!("{deserialized:?}");
    assert_eq!(debug1, debug2);
}

#[test]
fn test_trend_direction_serialization() {
    let trend = TrendDirection::Increasing { slope: 0.8 };
    let serialized = serde_json::to_string(&trend).expect("Failed to serialize");
    let _deserialized: TrendDirection =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
}

#[test]
fn test_prediction_high_confidence() {
    let prediction = PredictionPoint {
        timestamp: SystemTime::now(),
        predicted_value: 85.0,
        confidence_interval: (84.0, 86.0),
        prediction_method: "arima".to_string(),
    };

    let interval_width = prediction.confidence_interval.1 - prediction.confidence_interval.0;
    assert!(interval_width < 5.0); // Tight confidence interval
}

#[test]
fn test_prediction_low_confidence() {
    let prediction = PredictionPoint {
        timestamp: SystemTime::now(),
        predicted_value: 75.0,
        confidence_interval: (60.0, 90.0),
        prediction_method: "naive".to_string(),
    };

    let interval_width = prediction.confidence_interval.1 - prediction.confidence_interval.0;
    assert!(interval_width > 20.0); // Wide confidence interval
}
