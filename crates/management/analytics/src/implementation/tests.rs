// SPDX-License-Identifier: AGPL-3.0-or-later

use std::collections::HashMap;
use std::time::SystemTime;
use uuid::Uuid;

use super::*;
use crate::config::AnalyticsConfig;
use crate::types::{
    AnalyticsDataPoint, Dashboard, DashboardLayout, DashboardPanel, DashboardPermissions,
    PanelPosition, PanelType, TimeRange,
};

#[tokio::test]
async fn test_engine_new() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await;
    assert!(engine.is_ok());
}

#[tokio::test]
async fn test_collect_data_point() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        metric_name: "test".to_string(),
        value: 42.0,
        timestamp: SystemTime::now(),
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };
    let result = engine.collect_data_point(point).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_analyze_trends_no_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let result = engine.analyze_trends("nonexistent_metric", 24).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_create_dashboard() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let now = SystemTime::now();
    let dashboard = Dashboard {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        description: "Test dashboard".to_string(),
        panels: vec![DashboardPanel {
            id: "p1".to_string(),
            title: "Panel".to_string(),
            panel_type: PanelType::LineChart,
            metrics: vec!["cpu".to_string()],
            time_range: TimeRange {
                from: now,
                to: now,
                refresh_interval_secs: 60,
            },
            position: PanelPosition {
                x: 0,
                y: 0,
                width: 4,
                height: 2,
            },
        }],
        layout: DashboardLayout {
            grid_size: 12,
            auto_arrange: true,
            responsive: true,
        },
        permissions: DashboardPermissions {
            viewers: vec![],
            editors: vec![],
            admins: vec![],
        },
    };
    let result = engine.create_dashboard(dashboard).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_export_metrics() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let result = engine.export_metrics().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_analyze_trends_with_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let now = SystemTime::now();
    for i in 0..60 {
        let point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            metric_name: "cpu_usage".to_string(),
            value: f64::from(i).mul_add(0.5, 50.0),
            timestamp: now,
            runtime_type: None,
            execution_id: None,
            tags: HashMap::new(),
        };
        engine.collect_data_point(point).await.unwrap();
    }
    let result = engine.analyze_trends("cpu_usage", 24).await;
    assert!(result.is_ok());
    let analysis = result.unwrap();
    assert_eq!(analysis.metric_name, "cpu_usage");
    assert!(analysis.confidence > 0.5);
}

#[tokio::test]
async fn test_predict_values_with_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let now = SystemTime::now();
    for i in 0..100 {
        let point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            metric_name: "memory_usage".to_string(),
            value: f64::from(i).mul_add(0.1, 60.0),
            timestamp: now,
            runtime_type: None,
            execution_id: None,
            tags: HashMap::new(),
        };
        engine.collect_data_point(point).await.unwrap();
    }
    let result = engine.predict_values("memory_usage", 12).await;
    assert!(result.is_ok());
    let predictions = result.unwrap();
    assert!(!predictions.is_empty());
    assert!(predictions.len() <= 12);
}

#[tokio::test]
async fn test_predict_values_no_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let result = engine.predict_values("nonexistent", 24).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_evaluate_alerts_triggers_cpu() {
    let mut config = AnalyticsConfig::default();
    config.alert_thresholds.cpu_threshold = 80.0;
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        metric_name: "cpu_usage".to_string(),
        value: 95.0,
        timestamp: SystemTime::now(),
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };
    engine.collect_data_point(point).await.unwrap();
    let alerts = engine.evaluate_alerts().await.unwrap();
    assert!(!alerts.is_empty());
    assert!(alerts.iter().any(|a| a.metric_name.contains("cpu")));
}

#[tokio::test]
async fn test_evaluate_alerts_triggers_memory() {
    let mut config = AnalyticsConfig::default();
    config.alert_thresholds.memory_threshold = 85.0;
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let point = AnalyticsDataPoint {
        id: Uuid::new_v4(),
        metric_name: "memory_usage".to_string(),
        value: 95.0,
        timestamp: SystemTime::now(),
        runtime_type: None,
        execution_id: None,
        tags: HashMap::new(),
    };
    engine.collect_data_point(point).await.unwrap();
    let alerts = engine.evaluate_alerts().await.unwrap();
    assert!(!alerts.is_empty());
    assert!(alerts.iter().any(|a| a.metric_name.contains("memory")));
}

#[tokio::test]
async fn test_get_dashboard_data() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let now = SystemTime::now();
    let dashboard = Dashboard {
        id: Uuid::new_v4(),
        name: "Test".to_string(),
        description: "Test".to_string(),
        panels: vec![DashboardPanel {
            id: "p1".to_string(),
            title: "Panel".to_string(),
            panel_type: PanelType::LineChart,
            metrics: vec!["cpu".to_string()],
            time_range: TimeRange {
                from: now,
                to: now,
                refresh_interval_secs: 60,
            },
            position: PanelPosition {
                x: 0,
                y: 0,
                width: 4,
                height: 2,
            },
        }],
        layout: DashboardLayout {
            grid_size: 12,
            auto_arrange: true,
            responsive: true,
        },
        permissions: DashboardPermissions {
            viewers: vec![],
            editors: vec![],
            admins: vec![],
        },
    };
    let id = engine.create_dashboard(dashboard).await.unwrap();
    let result = engine.get_dashboard_data(id).await;
    assert!(result.is_ok());
    let data = result.unwrap();
    assert!(data.get("dashboard").is_some());
    assert!(data.get("data").is_some());
}

#[tokio::test]
async fn test_get_dashboard_data_not_found() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let result = engine.get_dashboard_data(Uuid::new_v4()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_buffer_eviction_when_full() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let now = SystemTime::now();
    for i in 0..10_005 {
        let point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            metric_name: "evict_test".to_string(),
            value: f64::from(i),
            timestamp: now,
            runtime_type: None,
            execution_id: None,
            tags: HashMap::new(),
        };
        engine.collect_data_point(point).await.unwrap();
    }
    let result = engine.analyze_trends("evict_test", 24).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_analyze_trends_stable_variation() {
    let config = AnalyticsConfig::default();
    let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();
    let now = SystemTime::now();
    for _ in 0..60 {
        let point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            metric_name: "stable_metric".to_string(),
            value: 50.0,
            timestamp: now,
            runtime_type: None,
            execution_id: None,
            tags: HashMap::new(),
        };
        engine.collect_data_point(point).await.unwrap();
    }
    let result = engine.analyze_trends("stable_metric", 24).await;
    assert!(result.is_ok());
}
