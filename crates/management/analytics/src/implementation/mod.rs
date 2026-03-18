// SPDX-License-Identifier: AGPL-3.0-or-later
//! Intelligent Analytics Engine Implementation
//!
//! PURE RUST: In-memory analytics with statistical analysis and prediction.
//! No external database dependencies -- all data stored in-memory.

mod alerts;
mod buffer;
mod dashboards;
mod statistics;

#[cfg(test)]
mod tests;

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::{RwLock, broadcast};
use tracing::{debug, error, info};
use uuid::Uuid;

use toadstool::error::{ToadStoolError, ToadStoolResult};

use crate::config::{AnalyticsConfig, WebhookConfig};
use crate::engine::AnalyticsEngine;
use crate::types::{Alert, AnalyticsDataPoint, Dashboard, TrendAnalysis, TrendDirection};

/// In-memory analytics engine with statistical analysis and prediction.
pub struct IntelligentAnalyticsEngine {
    config: AnalyticsConfig,
    data_buffer: Arc<RwLock<VecDeque<AnalyticsDataPoint>>>,
    alert_sender: broadcast::Sender<Alert>,
    dashboards: Arc<RwLock<HashMap<Uuid, Dashboard>>>,
}

impl IntelligentAnalyticsEngine {
    /// Create a new analytics engine with the given config.
    #[expect(
        clippy::unused_async,
        reason = "API designed for async; future persistence will require await"
    )]
    pub async fn new(config: AnalyticsConfig) -> ToadStoolResult<Self> {
        info!("Initializing intelligent analytics engine (in-memory)");

        let (alert_sender, _) = broadcast::channel(1000);

        let engine = Self {
            config,
            data_buffer: Arc::new(RwLock::new(VecDeque::with_capacity(
                buffer::MAX_BUFFER_SIZE,
            ))),
            alert_sender,
            dashboards: Arc::new(RwLock::new(HashMap::new())),
        };

        info!("Intelligent analytics engine initialized successfully");
        Ok(engine)
    }

    /// Start background processing (collection and alert evaluation).
    #[expect(
        clippy::unused_async,
        reason = "spawned tasks use await; outer fn must stay async for API consistency"
    )]
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

    async fn process_buffered_data(&self) -> ToadStoolResult<()> {
        let buf = self.data_buffer.read().await;
        let len = buf.len();
        drop(buf);
        debug!("Processing buffered data: {} points in memory", len);
        Ok(())
    }
}

impl AnalyticsEngine for IntelligentAnalyticsEngine {
    async fn collect_data_point(&self, data_point: AnalyticsDataPoint) -> ToadStoolResult<()> {
        debug!(
            "Collecting analytics data point: {}",
            data_point.metric_name
        );

        let mut buf = self.data_buffer.write().await;
        buf.push_back(data_point);

        if buf.len() > buffer::MAX_BUFFER_SIZE {
            buf.pop_front();
        }
        drop(buf);

        Ok(())
    }

    #[allow(clippy::significant_drop_tightening)] // matching holds refs from buf
    async fn analyze_trends(
        &self,
        metric_name: &str,
        hours_back: u32,
    ) -> ToadStoolResult<TrendAnalysis> {
        debug!("Analyzing trends for metric: {}", metric_name);

        let cutoff_time = SystemTime::now() - Duration::from_secs(u64::from(hours_back) * 3600);
        let buf = self.data_buffer.read().await;
        let matching = buffer::query_data_points(&buf, metric_name, cutoff_time);

        if matching.is_empty() {
            return Err(ToadStoolError::not_found(format!(
                "No data found for metric: {metric_name}"
            )));
        }

        let values: Vec<f64> = matching.iter().map(|dp| dp.value).collect();
        let timestamps: Vec<SystemTime> = matching.iter().map(|dp| dp.timestamp).collect();

        let stats = statistics::perform_statistical_analysis(&values);

        let trend = if stats.correlation_coefficient > 0.7 {
            TrendDirection::Increasing {
                slope: stats.correlation_coefficient,
            }
        } else if stats.correlation_coefficient < -0.7 {
            TrendDirection::Decreasing {
                slope: stats.correlation_coefficient.abs(),
            }
        } else if stats.mean.abs() > f64::EPSILON && stats.std_deviation / stats.mean < 0.1 {
            TrendDirection::Stable {
                variation: stats.std_deviation,
            }
        } else {
            TrendDirection::Irregular
        };

        let predictions = statistics::generate_predictions(&values, 24);

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
            statistics: stats,
            confidence,
            predictions,
        })
    }

    #[allow(clippy::significant_drop_tightening)] // matching holds refs from buf
    async fn predict_values(
        &self,
        metric_name: &str,
        hours_ahead: u32,
    ) -> ToadStoolResult<Vec<crate::types::PredictionPoint>> {
        debug!("Predicting values for metric: {}", metric_name);

        let cutoff_time = SystemTime::now() - Duration::from_secs(168 * 3600);
        let buf = self.data_buffer.read().await;
        let matching = buffer::query_data_points(&buf, metric_name, cutoff_time);

        if matching.is_empty() {
            return Err(ToadStoolError::not_found(format!(
                "No data found for metric: {metric_name}"
            )));
        }

        let values: Vec<f64> = matching.iter().map(|dp| dp.value).collect();
        Ok(statistics::generate_predictions(&values, hours_ahead))
    }

    #[allow(clippy::significant_drop_tightening)] // recent_points holds refs from buf
    async fn evaluate_alerts(&self) -> ToadStoolResult<Vec<Alert>> {
        debug!("Evaluating alert conditions");

        let recent_time = SystemTime::now() - Duration::from_secs(5 * 60);
        let buf = self.data_buffer.read().await;
        let recent_points: Vec<&AnalyticsDataPoint> = buf
            .iter()
            .filter(|dp| dp.timestamp >= recent_time)
            .collect();
        let triggered_alerts = alerts::compute_triggered_alerts(&recent_points, &self.config);

        for alert in &triggered_alerts {
            let _ = self.alert_sender.send(alert.clone());
        }

        Ok(triggered_alerts)
    }

    async fn create_dashboard(&self, dashboard: Dashboard) -> ToadStoolResult<Uuid> {
        debug!("Creating dashboard: {}", dashboard.name);

        let dashboard_id = dashboard.id;
        self.dashboards
            .write()
            .await
            .insert(dashboard_id, dashboard);

        Ok(dashboard_id)
    }

    #[allow(clippy::significant_drop_tightening)] // dashboard and buffer refs needed for build
    async fn get_dashboard_data(&self, dashboard_id: Uuid) -> ToadStoolResult<serde_json::Value> {
        debug!("Getting dashboard data for: {}", dashboard_id);

        let dashboards = self.dashboards.read().await;
        let dashboard = dashboards.get(&dashboard_id).ok_or_else(|| {
            ToadStoolError::not_found(format!("Dashboard not found: {dashboard_id}"))
        })?;
        let buffer = self.data_buffer.read().await;
        dashboards::build_dashboard_json(dashboard, &buffer)
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
    #[expect(
        clippy::unused_async,
        reason = "trait method; will await HTTP when Songbird integration added"
    )]
    async fn export_to_webhook(&self, webhook: &WebhookConfig) -> ToadStoolResult<()> {
        tracing::info!(
            "Webhook export to {} -- use Songbird for external HTTP",
            webhook.url
        );
        Ok(())
    }
}
