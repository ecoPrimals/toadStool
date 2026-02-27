#![deny(unsafe_code)]

//! `ToadStool` Advanced Analytics Engine
//!
//! This module provides sophisticated analytics capabilities including:
//! - Real-time metrics aggregation and analysis
//! - Performance trend analysis with machine learning
//! - Predictive resource forecasting
//! - Custom dashboards and alerting systems
//! - Integration with external monitoring systems
//!
//! ## Architecture
//!
//! The analytics module is organized into focused sub-modules:
//! - `config`: Configuration types and defaults
//! - `types`: Core data types for analytics, alerts, and dashboards
//! - `engine`: Analytics engine trait definition
//! - `utils`: Helper functions for statistical calculations
//! - `implementation`: Main engine implementation (re-exported as IntelligentAnalyticsEngine)

mod config;
mod engine;
mod implementation;
mod types;
mod utils;

// Re-export public API
pub use config::{AlertThresholds, AnalyticsConfig, ExternalIntegrations, WebhookConfig};
pub use engine::AnalyticsEngine;
pub use implementation::IntelligentAnalyticsEngine;
pub use types::{
    Alert, AlertCondition, AlertSeverity, AlertStatus, AnalyticsDataPoint, Dashboard,
    DashboardLayout, DashboardPanel, DashboardPermissions, PanelPosition, PanelType,
    PredictionPoint, TimeRange, TrendAnalysis, TrendDirection, TrendStatistics,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::time::SystemTime;
    use uuid::Uuid;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_analytics_engine_creation() {
        let config = AnalyticsConfig::default();
        let engine = IntelligentAnalyticsEngine::new(config).await;
        assert!(engine.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_data_point_collection() {
        let config = AnalyticsConfig::default();
        let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

        let data_point = AnalyticsDataPoint {
            id: Uuid::new_v4(),
            timestamp: SystemTime::now(),
            metric_name: "test_metric".to_string(),
            value: 42.0,
            runtime_type: None,
            execution_id: None,
            tags: HashMap::new(),
        };

        let result = engine.collect_data_point(data_point).await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_dashboard_creation() {
        let config = AnalyticsConfig::default();
        let engine = IntelligentAnalyticsEngine::new(config).await.unwrap();

        let dashboard = Dashboard {
            id: Uuid::new_v4(),
            name: "Test Dashboard".to_string(),
            description: "Test dashboard description".to_string(),
            panels: vec![],
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
}
