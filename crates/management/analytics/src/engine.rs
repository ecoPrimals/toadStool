//! Analytics engine trait definition

use toadstool::error::ToadStoolResult;
use uuid::Uuid;

use crate::types::Dashboard;
use crate::types::{Alert, AnalyticsDataPoint, PredictionPoint, TrendAnalysis};

/// Main analytics engine trait
pub trait AnalyticsEngine: Send + Sync {
    /// Collect and store analytics data point
    async fn collect_data_point(&self, data_point: AnalyticsDataPoint) -> ToadStoolResult<()>;

    /// Perform trend analysis on metric data
    async fn analyze_trends(
        &self,
        metric_name: &str,
        hours_back: u32,
    ) -> ToadStoolResult<TrendAnalysis>;

    /// Generate predictions for future values
    async fn predict_values(
        &self,
        metric_name: &str,
        hours_ahead: u32,
    ) -> ToadStoolResult<Vec<PredictionPoint>>;

    /// Evaluate alert conditions
    async fn evaluate_alerts(&self) -> ToadStoolResult<Vec<Alert>>;

    /// Create custom dashboard
    async fn create_dashboard(&self, dashboard: Dashboard) -> ToadStoolResult<Uuid>;

    /// Get dashboard data
    async fn get_dashboard_data(&self, dashboard_id: Uuid) -> ToadStoolResult<serde_json::Value>;

    /// Export metrics to external systems
    async fn export_metrics(&self) -> ToadStoolResult<()>;
}
