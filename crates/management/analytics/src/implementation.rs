// SPDX-License-Identifier: AGPL-3.0-or-later
//! Intelligent Analytics Engine Implementation
//!
//! PURE RUST: In-memory analytics with statistical analysis and prediction.
//! No external database dependencies -- all data stored in-memory.
//!
//! ## Evolution Path
//!
//! - Current: In-memory `VecDeque` buffer with statistical analysis
//! - Future: Pure Rust embedded database (redb, sled) for persistence
//! - Never: sqlx/ring/C-dependency databases

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use statrs::statistics::Statistics;
use tokio::sync::{broadcast, RwLock};
use tracing::{debug, error, info};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::config::{AnalyticsConfig, WebhookConfig};
use crate::engine::AnalyticsEngine;
use crate::types::{
    Alert, AlertCondition, AlertSeverity, AlertStatus, AnalyticsDataPoint, Dashboard,
    PredictionPoint, TrendAnalysis, TrendDirection, TrendStatistics,
};
use crate::utils::{calculate_median, calculate_percentile};

/// Maximum data points to retain in memory
const MAX_BUFFER_SIZE: usize = 10_000;

/// Intelligent analytics engine implementation
///
/// PURE RUST: In-memory analytics only -- zero C dependencies.
/// Future: Can add pure Rust persistence (redb, sled) when needed.
pub struct IntelligentAnalyticsEngine {
    config: AnalyticsConfig,
    data_buffer: Arc<RwLock<VecDeque<AnalyticsDataPoint>>>,
    alert_sender: broadcast::Sender<Alert>,
    dashboards: Arc<RwLock<HashMap<Uuid, Dashboard>>>,
}

impl IntelligentAnalyticsEngine {
    /// Create a new intelligent analytics engine (pure Rust, in-memory)
    pub async fn new(config: AnalyticsConfig) -> ToadStoolResult<Self> {
        info!("Initializing intelligent analytics engine (in-memory)");

        let (alert_sender, _) = broadcast::channel(1000);

        let engine = Self {
            config,
            data_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(MAX_BUFFER_SIZE))),
            alert_sender,
            dashboards: Arc::new(RwLock::new(HashMap::new())),
        };

        info!("Intelligent analytics engine initialized successfully");
        Ok(engine)
    }

    /// Start background analytics processing
    pub async fn start_background_processing(self: Arc<Self>) -> ToadStoolResult<()> {
        info!("Starting background analytics processing");

        let collection_engine = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(
                collection_engine.config.collection_interval_secs,
            ));

            loop {
                interval.tick().await;
                if let Err(e) = collection_engine.process_buffered_data().await {
                    error!("Error processing buffered data: {:?}", e);
                }
            }
        });

        let alert_engine = self;
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60));

            loop {
                interval.tick().await;
                if let Err(e) = alert_engine.evaluate_alerts().await {
                    error!("Error evaluating alerts: {:?}", e);
                }
            }
        });

        Ok(())
    }

    /// Process buffered analytics data (in-memory compaction)
    async fn process_buffered_data(&self) -> ToadStoolResult<()> {
        let buffer = self.data_buffer.read().await;
        debug!(
            "Processing buffered data: {} points in memory",
            buffer.len()
        );
        // In-memory: data is already in buffer, nothing to persist.
        // Future evolution: flush to redb/sled here.
        Ok(())
    }

    /// Query data points by metric name and time range from in-memory buffer
    fn query_data_points<'a>(
        buffer: &'a VecDeque<AnalyticsDataPoint>,
        metric_name: &str,
        since: SystemTime,
    ) -> Vec<&'a AnalyticsDataPoint> {
        buffer
            .iter()
            .filter(|dp| dp.metric_name == metric_name && dp.timestamp >= since)
            .collect()
    }

    /// Perform statistical analysis on time series data
    fn perform_statistical_analysis(data: &[f64]) -> TrendStatistics {
        let mean = data.mean();
        let median = calculate_median(data);
        let std_deviation = data.std_dev();
        let min = data.min();
        let max = data.max();
        let percentile_95 = calculate_percentile(data, 0.95);

        let correlation_coefficient = if data.len() > 1 {
            let x: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
            let n = data.len() as f64;
            let x_mean: f64 = x.iter().sum::<f64>() / n;
            let y_mean: f64 = data.iter().sum::<f64>() / n;

            let numerator: f64 = x
                .iter()
                .zip(data.iter())
                .map(|(xi, yi)| (xi - x_mean) * (yi - y_mean))
                .sum();

            let x_variance: f64 = x.iter().map(|xi| (xi - x_mean).powi(2)).sum();
            let y_variance: f64 = data.iter().map(|yi| (yi - y_mean).powi(2)).sum();

            if x_variance > 0.0 && y_variance > 0.0 {
                numerator / (x_variance * y_variance).sqrt()
            } else {
                0.0
            }
        } else {
            0.0
        };

        TrendStatistics {
            mean,
            median,
            std_deviation,
            min,
            max,
            percentile_95,
            correlation_coefficient,
        }
    }

    /// Generate predictions using linear regression
    fn generate_predictions(data: &[f64], hours_ahead: u32) -> Vec<PredictionPoint> {
        if data.len() < 2 {
            return Vec::new();
        }

        let x: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
        let n = data.len() as f64;
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = data.iter().sum();
        let sum_xy: f64 = x.iter().zip(data.iter()).map(|(xi, yi)| xi * yi).sum();
        let sum_x2: f64 = x.iter().map(|xi| xi * xi).sum();

        let denominator = n * sum_x2 - sum_x * sum_x;
        if denominator.abs() < f64::EPSILON {
            return Vec::new();
        }

        let slope = (n * sum_xy - sum_x * sum_y) / denominator;
        let intercept = (sum_y - slope * sum_x) / n;

        let current_time = SystemTime::now();

        (1..=hours_ahead)
            .map(|i| {
                let future_x = data.len() as f64 + f64::from(i);
                let predicted_value = slope * future_x + intercept;

                let std_error = (data
                    .iter()
                    .map(|yi| yi - predicted_value)
                    .map(|diff| diff * diff)
                    .sum::<f64>()
                    / n)
                    .sqrt();
                let confidence_interval = (
                    predicted_value - 1.96 * std_error,
                    predicted_value + 1.96 * std_error,
                );

                PredictionPoint {
                    timestamp: current_time + Duration::from_secs((i as u64) * 3600),
                    predicted_value,
                    confidence_interval,
                    prediction_method: "linear_regression".to_string(),
                }
            })
            .collect()
    }
}

impl AnalyticsEngine for IntelligentAnalyticsEngine {
    async fn collect_data_point(&self, data_point: AnalyticsDataPoint) -> ToadStoolResult<()> {
        debug!(
            "Collecting analytics data point: {}",
            data_point.metric_name
        );

        let mut buffer = self.data_buffer.write().await;
        buffer.push_back(data_point);

        // Evict oldest when buffer is full
        if buffer.len() > MAX_BUFFER_SIZE {
            buffer.pop_front();
        }

        Ok(())
    }

    async fn analyze_trends(
        &self,
        metric_name: &str,
        hours_back: u32,
    ) -> ToadStoolResult<TrendAnalysis> {
        debug!("Analyzing trends for metric: {}", metric_name);

        let cutoff_time = SystemTime::now() - Duration::from_secs((hours_back as u64) * 3600);
        let buffer = self.data_buffer.read().await;
        let matching = Self::query_data_points(&buffer, metric_name, cutoff_time);

        if matching.is_empty() {
            return Err(ToadStoolError::not_found(format!(
                "No data found for metric: {metric_name}"
            )));
        }

        let values: Vec<f64> = matching.iter().map(|dp| dp.value).collect();
        let timestamps: Vec<SystemTime> = matching.iter().map(|dp| dp.timestamp).collect();

        let statistics = Self::perform_statistical_analysis(&values);

        let trend = if statistics.correlation_coefficient > 0.7 {
            TrendDirection::Increasing {
                slope: statistics.correlation_coefficient,
            }
        } else if statistics.correlation_coefficient < -0.7 {
            TrendDirection::Decreasing {
                slope: statistics.correlation_coefficient.abs(),
            }
        } else if statistics.mean.abs() > f64::EPSILON
            && statistics.std_deviation / statistics.mean < 0.1
        {
            TrendDirection::Stable {
                variation: statistics.std_deviation,
            }
        } else {
            TrendDirection::Irregular
        };

        let predictions = Self::generate_predictions(&values, 24);

        let confidence = if values.len() > 100 {
            0.9
        } else if values.len() > 50 {
            0.8
        } else {
            0.6
        };

        Ok(TrendAnalysis {
            metric_name: metric_name.to_string(),
            start_time: timestamps.first().copied().unwrap_or_else(SystemTime::now),
            end_time: timestamps.last().copied().unwrap_or_else(SystemTime::now),
            trend,
            statistics,
            confidence,
            predictions,
        })
    }

    async fn predict_values(
        &self,
        metric_name: &str,
        hours_ahead: u32,
    ) -> ToadStoolResult<Vec<PredictionPoint>> {
        debug!("Predicting values for metric: {}", metric_name);

        let cutoff_time = SystemTime::now() - Duration::from_secs(168 * 3600); // Last week
        let buffer = self.data_buffer.read().await;
        let matching = Self::query_data_points(&buffer, metric_name, cutoff_time);

        if matching.is_empty() {
            return Err(ToadStoolError::not_found(format!(
                "No data found for metric: {metric_name}"
            )));
        }

        let values: Vec<f64> = matching.iter().map(|dp| dp.value).collect();
        Ok(Self::generate_predictions(&values, hours_ahead))
    }

    async fn evaluate_alerts(&self) -> ToadStoolResult<Vec<Alert>> {
        debug!("Evaluating alert conditions");

        let recent_time = SystemTime::now() - Duration::from_secs(5 * 60);
        let buffer = self.data_buffer.read().await;

        let mut triggered_alerts = Vec::new();

        // Collect recent data points across all metrics
        let recent_points: Vec<&AnalyticsDataPoint> = buffer
            .iter()
            .filter(|dp| dp.timestamp >= recent_time)
            .collect();

        for dp in recent_points {
            // Check CPU threshold
            if dp.metric_name.contains("cpu")
                && dp.value > self.config.alert_thresholds.cpu_threshold
            {
                triggered_alerts.push(Alert {
                    id: Uuid::new_v4(),
                    name: format!("High CPU Usage: {}", dp.metric_name),
                    metric_name: dp.metric_name.clone(),
                    condition: AlertCondition::Threshold {
                        operator: ">".to_string(),
                        value: self.config.alert_thresholds.cpu_threshold,
                    },
                    severity: AlertSeverity::Warning,
                    created_at: SystemTime::now(),
                    last_triggered: Some(SystemTime::now()),
                    status: AlertStatus::Active,
                    recipients: vec!["admin@example.com".to_string()],
                });
            }

            // Check memory threshold
            if dp.metric_name.contains("memory")
                && dp.value > self.config.alert_thresholds.memory_threshold
            {
                triggered_alerts.push(Alert {
                    id: Uuid::new_v4(),
                    name: format!("High Memory Usage: {}", dp.metric_name),
                    metric_name: dp.metric_name.clone(),
                    condition: AlertCondition::Threshold {
                        operator: ">".to_string(),
                        value: self.config.alert_thresholds.memory_threshold,
                    },
                    severity: AlertSeverity::Critical,
                    created_at: SystemTime::now(),
                    last_triggered: Some(SystemTime::now()),
                    status: AlertStatus::Active,
                    recipients: vec!["admin@example.com".to_string()],
                });
            }
        }

        // Broadcast triggered alerts
        for alert in &triggered_alerts {
            let _ = self.alert_sender.send(alert.clone());
        }

        Ok(triggered_alerts)
    }

    async fn create_dashboard(&self, dashboard: Dashboard) -> ToadStoolResult<Uuid> {
        debug!("Creating dashboard: {}", dashboard.name);

        let dashboard_id = dashboard.id;
        let mut dashboards = self.dashboards.write().await;
        dashboards.insert(dashboard_id, dashboard);

        Ok(dashboard_id)
    }

    async fn get_dashboard_data(&self, dashboard_id: Uuid) -> ToadStoolResult<serde_json::Value> {
        debug!("Getting dashboard data for: {}", dashboard_id);

        let dashboards = self.dashboards.read().await;
        let dashboard = dashboards.get(&dashboard_id).ok_or_else(|| {
            ToadStoolError::not_found(format!("Dashboard not found: {dashboard_id}"))
        })?;

        let mut dashboard_data = serde_json::Map::new();
        dashboard_data.insert(
            "dashboard".to_string(),
            serde_json::to_value(dashboard).map_err(|e| {
                tracing::error!("Failed to serialize dashboard data: {}", e);
                ToadStoolError::runtime(format!("Dashboard serialization failed: {e}"))
            })?,
        );

        // Fetch data for each panel from in-memory buffer
        let buffer = self.data_buffer.read().await;
        let mut panel_data = serde_json::Map::new();

        for panel in &dashboard.panels {
            let mut metrics_data = Vec::new();

            for metric_name in &panel.metrics {
                let data_points: Vec<serde_json::Value> = buffer
                    .iter()
                    .filter(|dp| {
                        dp.metric_name == *metric_name
                            && dp.timestamp >= panel.time_range.from
                            && dp.timestamp <= panel.time_range.to
                    })
                    .map(|dp| {
                        serde_json::json!({
                            "timestamp": toadstool_common::system_time_serde::format_rfc3339(dp.timestamp),
                            "value": dp.value
                        })
                    })
                    .collect();

                metrics_data.push(serde_json::json!({
                    "metric_name": metric_name,
                    "data": data_points
                }));
            }

            panel_data.insert(panel.id.clone(), serde_json::json!(metrics_data));
        }

        dashboard_data.insert("data".to_string(), serde_json::Value::Object(panel_data));

        Ok(serde_json::Value::Object(dashboard_data))
    }

    async fn export_metrics(&self) -> ToadStoolResult<()> {
        debug!("Exporting metrics to external systems");

        for webhook in &self.config.external_integrations.webhooks {
            match self.export_to_webhook(webhook).await {
                Ok(()) => info!("Successfully exported metrics to webhook: {}", webhook.name),
                Err(e) => error!(
                    "Failed to export metrics to webhook {}: {:?}",
                    webhook.name, e
                ),
            }
        }

        Ok(())
    }
}

impl IntelligentAnalyticsEngine {
    /// Export metrics to a webhook endpoint
    ///
    /// **EVOLUTION**: Use Songbird for external HTTP when available.
    /// Currently a no-op that logs the export intent.
    async fn export_to_webhook(&self, webhook: &WebhookConfig) -> ToadStoolResult<()> {
        // PURE RUST: External HTTP disabled -- use Songbird for external comms
        tracing::info!(
            "Webhook export to {} -- use Songbird for external HTTP",
            webhook.url
        );
        Ok(())
    }
}
