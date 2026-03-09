// SPDX-License-Identifier: AGPL-3.0-only
//! Analytics data types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::SystemTime;
use toadstool::execution::RuntimeType;
use uuid::Uuid;

/// Analytics data point for time series analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsDataPoint {
    /// Unique identifier
    pub id: Uuid,
    /// Timestamp of the data point
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
    /// Metric name
    pub metric_name: String,
    /// Metric value
    pub value: f64,
    /// Associated runtime type
    pub runtime_type: Option<RuntimeType>,
    /// Execution context
    pub execution_id: Option<String>,
    /// Tags for grouping and filtering
    pub tags: HashMap<String, String>,
}

/// Performance trend analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendAnalysis {
    /// Metric being analyzed
    pub metric_name: String,
    /// Time period of analysis
    #[serde(with = "toadstool_common::system_time_serde")]
    pub start_time: SystemTime,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub end_time: SystemTime,
    /// Trend direction and strength
    pub trend: TrendDirection,
    /// Statistical measures
    pub statistics: TrendStatistics,
    /// Confidence level of the analysis
    pub confidence: f64,
    /// Predictions for future values
    pub predictions: Vec<PredictionPoint>,
}

/// Trend direction enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrendDirection {
    Increasing { slope: f64 },
    Decreasing { slope: f64 },
    Stable { variation: f64 },
    Cyclical { period_hours: f64 },
    Irregular,
}

/// Statistical measures for trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendStatistics {
    pub mean: f64,
    pub median: f64,
    pub std_deviation: f64,
    pub min: f64,
    pub max: f64,
    pub percentile_95: f64,
    pub correlation_coefficient: f64,
}

/// Prediction point for forecasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionPoint {
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
    pub predicted_value: f64,
    pub confidence_interval: (f64, f64),
    pub prediction_method: String,
}

/// Alert definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    /// Alert identifier
    pub id: Uuid,
    /// Alert name
    pub name: String,
    /// Metric being monitored
    pub metric_name: String,
    /// Alert condition
    pub condition: AlertCondition,
    /// Alert severity
    pub severity: AlertSeverity,
    /// When the alert was created
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: SystemTime,
    /// When the alert was last triggered
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub last_triggered: Option<SystemTime>,
    /// Alert status
    pub status: AlertStatus,
    /// Recipients for notifications
    pub recipients: Vec<String>,
}

/// Alert condition types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    Threshold { operator: String, value: f64 },
    RateOfChange { window_minutes: u32, threshold: f64 },
    Anomaly { sensitivity: f64 },
    Complex { expression: String },
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Alert status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertStatus {
    Active,
    Suppressed,
    Resolved,
}

/// Dashboard definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dashboard {
    /// Dashboard identifier
    pub id: Uuid,
    /// Dashboard name
    pub name: String,
    /// Dashboard description
    pub description: String,
    /// Panels in the dashboard
    pub panels: Vec<DashboardPanel>,
    /// Dashboard layout configuration
    pub layout: DashboardLayout,
    /// Access permissions
    pub permissions: DashboardPermissions,
}

/// Dashboard panel configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPanel {
    pub id: String,
    pub title: String,
    pub panel_type: PanelType,
    pub metrics: Vec<String>,
    pub time_range: TimeRange,
    pub position: PanelPosition,
}

/// Panel type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelType {
    LineChart,
    BarChart,
    Gauge,
    Table,
    Heatmap,
    Custom { component: String },
}

/// Time range for panel data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    #[serde(with = "toadstool_common::system_time_serde")]
    pub from: SystemTime,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub to: SystemTime,
    pub refresh_interval_secs: u64,
}

/// Panel position in dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelPosition {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Dashboard layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    pub grid_size: u32,
    pub auto_arrange: bool,
    pub responsive: bool,
}

/// Dashboard access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPermissions {
    pub viewers: Vec<String>,
    pub editors: Vec<String>,
    pub admins: Vec<String>,
}
