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
    /// Start of analysis period
    #[serde(with = "toadstool_common::system_time_serde")]
    pub start_time: SystemTime,
    /// End of analysis period
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
    /// Increasing trend.
    Increasing {
        /// Slope of the trend.
        slope: f64,
    },
    /// Decreasing trend.
    Decreasing {
        /// Slope magnitude.
        slope: f64,
    },
    /// Stable trend.
    Stable {
        /// Variation around mean.
        variation: f64,
    },
    /// Cyclical pattern.
    Cyclical {
        /// Period in hours.
        period_hours: f64,
    },
    /// Irregular/no clear pattern.
    Irregular,
}

/// Statistical measures for trend analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrendStatistics {
    /// Mean value.
    pub mean: f64,
    /// Median value.
    pub median: f64,
    /// Standard deviation.
    pub std_deviation: f64,
    /// Minimum value.
    pub min: f64,
    /// Maximum value.
    pub max: f64,
    /// 95th percentile.
    pub percentile_95: f64,
    /// Correlation coefficient.
    pub correlation_coefficient: f64,
}

/// Prediction point for forecasting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictionPoint {
    /// Timestamp of prediction.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: SystemTime,
    /// Predicted value.
    pub predicted_value: f64,
    /// Confidence interval (low, high).
    pub confidence_interval: (f64, f64),
    /// Method used for prediction.
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
    /// Threshold comparison.
    Threshold {
        /// Comparison operator (e.g. "gt", "lt").
        operator: String,
        /// Threshold value.
        value: f64,
    },
    /// Rate of change over window.
    RateOfChange {
        /// Window size in minutes.
        window_minutes: u32,
        /// Rate threshold.
        threshold: f64,
    },
    /// Anomaly detection.
    Anomaly {
        /// Sensitivity level.
        sensitivity: f64,
    },
    /// Complex expression.
    Complex {
        /// Expression string.
        expression: String,
    },
}

/// Alert severity levels
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational.
    Info,
    /// Warning level.
    Warning,
    /// Critical level.
    Critical,
    /// Emergency level.
    Emergency,
}

/// Alert status enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertStatus {
    /// Alert is active.
    Active,
    /// Alert is suppressed.
    Suppressed,
    /// Alert is resolved.
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
    /// Panel identifier.
    pub id: String,
    /// Panel title.
    pub title: String,
    /// Panel visualization type.
    pub panel_type: PanelType,
    /// Metrics displayed.
    pub metrics: Vec<String>,
    /// Time range for data.
    pub time_range: TimeRange,
    /// Layout position.
    pub position: PanelPosition,
}

/// Panel type enumeration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PanelType {
    /// Line chart.
    LineChart,
    /// Bar chart.
    BarChart,
    /// Gauge.
    Gauge,
    /// Table.
    Table,
    /// Heatmap.
    Heatmap,
    /// Custom component.
    Custom {
        /// Component identifier.
        component: String,
    },
}

/// Time range for panel data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start time.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub from: SystemTime,
    /// End time.
    #[serde(with = "toadstool_common::system_time_serde")]
    pub to: SystemTime,
    /// Refresh interval in seconds.
    pub refresh_interval_secs: u64,
}

/// Panel position in dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PanelPosition {
    /// X coordinate.
    pub x: u32,
    /// Y coordinate.
    pub y: u32,
    /// Panel width.
    pub width: u32,
    /// Panel height.
    pub height: u32,
}

/// Dashboard layout configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardLayout {
    /// Grid cell size.
    pub grid_size: u32,
    /// Auto-arrange panels.
    pub auto_arrange: bool,
    /// Responsive layout.
    pub responsive: bool,
}

/// Dashboard access permissions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardPermissions {
    /// Viewer user IDs.
    pub viewers: Vec<String>,
    /// Editor user IDs.
    pub editors: Vec<String>,
    /// Admin user IDs.
    pub admins: Vec<String>,
}
