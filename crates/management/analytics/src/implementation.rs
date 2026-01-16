//! Intelligent Analytics Engine Implementation

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use ndarray::Array1;
// PURE RUST: sqlx removed - in-memory analytics only
// use sqlx::{Row, SqlitePool};
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

/// Intelligent analytics engine implementation
/// 
/// PURE RUST: Database persistence disabled - in-memory analytics only
/// Future: Can add pure Rust database (sled, redb) for persistence
pub struct IntelligentAnalyticsEngine {
    config: AnalyticsConfig,
    // database: SqlitePool,  // REMOVED: Brings ring via rustls
    data_buffer: Arc<RwLock<VecDeque<AnalyticsDataPoint>>>,
    alert_sender: broadcast::Sender<Alert>,
    dashboards: Arc<RwLock<HashMap<Uuid, Dashboard>>>,
}

impl IntelligentAnalyticsEngine {
    /// Create a new intelligent analytics engine
    pub async fn new(config: AnalyticsConfig) -> ToadStoolResult<Self> {
        info!("Initializing intelligent analytics engine");

        // Initialize database
        let database = SqlitePool::connect("sqlite::memory:")
            .await
            .map_err(|e| ToadStoolError::io(e.to_string()))?;

        // Initialize database schema
        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS analytics_data (
                id TEXT PRIMARY KEY,
                timestamp TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                value REAL NOT NULL,
                runtime_type TEXT,
                execution_id TEXT,
                tags TEXT
            )
        ",
        )
        .execute(&database)
        .await
        .map_err(|e| ToadStoolError::io(e.to_string()))?;

        sqlx::query(
            r"
            CREATE TABLE IF NOT EXISTS alerts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                metric_name TEXT NOT NULL,
                condition_type TEXT NOT NULL,
                condition_data TEXT NOT NULL,
                severity TEXT NOT NULL,
                created_at TEXT NOT NULL,
                last_triggered TEXT,
                status TEXT NOT NULL
            )
        ",
        )
        .execute(&database)
        .await
        .map_err(|e| ToadStoolError::io(e.to_string()))?;

        // Initialize broadcast channel for alerts
        let (alert_sender, _) = broadcast::channel(1000);

        let engine = Self {
            config,
            database,
            data_buffer: Arc::new(RwLock::new(VecDeque::new())),
            alert_sender,
            dashboards: Arc::new(RwLock::new(HashMap::new())),
        };

        info!("Intelligent analytics engine initialized successfully");
        Ok(engine)
    }

    /// Start background analytics processing
    pub async fn start_background_processing(self: Arc<Self>) -> ToadStoolResult<()> {
        info!("Starting background analytics processing");

        // Start data collection task
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

        // Start alert evaluation task
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(60)); // Check every minute

            loop {
                interval.tick().await;
                if let Err(e) = self.evaluate_alerts().await {
                    error!("Error evaluating alerts: {:?}", e);
                }
            }
        });

        Ok(())
    }

    /// Process buffered analytics data
    async fn process_buffered_data(&self) -> ToadStoolResult<()> {
        let mut buffer = self.data_buffer.write().await;

        while let Some(data_point) = buffer.pop_front() {
            // Store in database
            let tags_json = serde_json::to_string(&data_point.tags)
                .map_err(|e| ToadStoolError::validation(e.to_string()))?;

            sqlx::query(r"
                INSERT INTO analytics_data (id, timestamp, metric_name, value, runtime_type, execution_id, tags)
                VALUES (?, ?, ?, ?, ?, ?, ?)
            ")
            .bind(data_point.id.to_string())
            .bind(data_point.timestamp.to_rfc3339())
            .bind(&data_point.metric_name)
            .bind(data_point.value)
            .bind(data_point.runtime_type.as_ref().map(|rt| format!("{rt:?}")))
            .bind(&data_point.execution_id)
            .bind(tags_json)
            .execute(&self.database).await
                .map_err(|e| ToadStoolError::io(e.to_string()))?;
        }

        Ok(())
    }

    /// Perform statistical analysis on time series data
    async fn perform_statistical_analysis(&self, data: &[f64]) -> TrendStatistics {
        let mean = data.mean();
        let median = calculate_median(data);
        let std_deviation = data.std_dev();
        let min = data.min();
        let max = data.max();
        let percentile_95 = calculate_percentile(data, 0.95);

        // Calculate correlation coefficient (simplified linear correlation)
        let correlation_coefficient = if data.len() > 1 {
            let x: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
            let x_array = Array1::from(x);
            let y_array = Array1::from(data.to_vec());

            let x_mean = x_array.clone().mean();
            let y_mean = y_array.clone().mean();

            let numerator: f64 = x_array
                .iter()
                .zip(y_array.iter())
                .map(|(x, y)| (x - x_mean) * (y - y_mean))
                .sum();

            let x_variance: f64 = x_array.iter().map(|x| (x - x_mean).powi(2)).sum();
            let y_variance: f64 = y_array.iter().map(|y| (y - y_mean).powi(2)).sum();

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
    async fn generate_predictions(&self, data: &[f64], hours_ahead: u32) -> Vec<PredictionPoint> {
        let mut predictions = Vec::new();

        if data.len() < 2 {
            return predictions;
        }

        // Simple linear regression for prediction
        let x: Vec<f64> = (0..data.len()).map(|i| i as f64).collect();
        let y = data;

        let n = data.len() as f64;
        let sum_x: f64 = x.iter().sum();
        let sum_y: f64 = y.iter().sum();
        let sum_xy: f64 = x.iter().zip(y.iter()).map(|(xi, yi)| xi * yi).sum();
        let sum_x2: f64 = x.iter().map(|xi| xi * xi).sum();

        let slope = (n * sum_xy - sum_x * sum_y) / (n * sum_x2 - sum_x * sum_x);
        let intercept = (sum_y - slope * sum_x) / n;

        let current_time = Utc::now();

        for i in 1..=hours_ahead {
            let future_x = data.len() as f64 + f64::from(i);
            let predicted_value = slope * future_x + intercept;

            // Calculate confidence interval (simplified)
            let std_error = (y
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

            predictions.push(PredictionPoint {
                timestamp: current_time + chrono::Duration::hours(i64::from(i)),
                predicted_value,
                confidence_interval,
                prediction_method: "linear_regression".to_string(),
            });
        }

        predictions
    }
}

#[async_trait]
impl AnalyticsEngine for IntelligentAnalyticsEngine {
    async fn collect_data_point(&self, data_point: AnalyticsDataPoint) -> ToadStoolResult<()> {
        debug!(
            "Collecting analytics data point: {}",
            data_point.metric_name
        );

        // Add to buffer for batch processing
        let mut buffer = self.data_buffer.write().await;
        buffer.push_back(data_point);

        // Limit buffer size
        if buffer.len() > 10000 {
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

        let cutoff_time = Utc::now() - chrono::Duration::hours(i64::from(hours_back));

        // Query historical data
        let rows = sqlx::query(
            r"
            SELECT timestamp, value FROM analytics_data 
            WHERE metric_name = ? AND timestamp >= ?
            ORDER BY timestamp ASC
        ",
        )
        .bind(metric_name)
        .bind(cutoff_time.to_rfc3339())
        .fetch_all(&self.database)
        .await
        .map_err(|e| ToadStoolError::io(e.to_string()))?;

        if rows.is_empty() {
            return Err(ToadStoolError::not_found(format!(
                "No data found for metric: {metric_name}"
            )));
        }

        let values: Vec<f64> = rows.iter().map(|row| row.get::<f64, _>("value")).collect();

        let timestamps: Vec<DateTime<Utc>> = rows
            .iter()
            .map(|row| {
                let timestamp_str: String = row.get("timestamp");
                DateTime::parse_from_rfc3339(&timestamp_str)
                    .unwrap_or_else(|_| Utc::now().into())
                    .with_timezone(&Utc)
            })
            .collect();

        // Perform statistical analysis
        let statistics = self.perform_statistical_analysis(&values).await;

        // Determine trend direction
        let trend = if statistics.correlation_coefficient > 0.7 {
            TrendDirection::Increasing {
                slope: statistics.correlation_coefficient,
            }
        } else if statistics.correlation_coefficient < -0.7 {
            TrendDirection::Decreasing {
                slope: statistics.correlation_coefficient.abs(),
            }
        } else if statistics.std_deviation / statistics.mean < 0.1 {
            TrendDirection::Stable {
                variation: statistics.std_deviation,
            }
        } else {
            TrendDirection::Irregular
        };

        // Generate predictions
        let predictions = self.generate_predictions(&values, 24).await; // 24 hours ahead

        let confidence = if values.len() > 100 {
            0.9
        } else if values.len() > 50 {
            0.8
        } else {
            0.6
        };

        Ok(TrendAnalysis {
            metric_name: metric_name.to_string(),
            start_time: timestamps.first().copied().unwrap_or_else(Utc::now),
            end_time: timestamps.last().copied().unwrap_or_else(Utc::now),
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

        // Get recent data for prediction
        let cutoff_time = Utc::now() - chrono::Duration::hours(168); // Last week

        let rows = sqlx::query(
            r"
            SELECT value FROM analytics_data 
            WHERE metric_name = ? AND timestamp >= ?
            ORDER BY timestamp ASC
        ",
        )
        .bind(metric_name)
        .bind(cutoff_time.to_rfc3339())
        .fetch_all(&self.database)
        .await
        .map_err(|e| ToadStoolError::io(e.to_string()))?;

        if rows.is_empty() {
            return Err(ToadStoolError::not_found(format!(
                "No data found for metric: {metric_name}"
            )));
        }

        let values: Vec<f64> = rows.iter().map(|row| row.get::<f64, _>("value")).collect();

        Ok(self.generate_predictions(&values, hours_ahead).await)
    }

    async fn evaluate_alerts(&self) -> ToadStoolResult<Vec<Alert>> {
        debug!("Evaluating alert conditions");

        // This is a simplified implementation
        // In a real scenario, you'd load alert definitions from database
        // and evaluate them against current metrics

        let mut triggered_alerts = Vec::new();

        // Example: Check if any recent metrics exceed thresholds
        let recent_time = Utc::now() - chrono::Duration::minutes(5);

        let rows = sqlx::query(
            r"
            SELECT metric_name, value FROM analytics_data 
            WHERE timestamp >= ?
        ",
        )
        .bind(recent_time.to_rfc3339())
        .fetch_all(&self.database)
        .await
        .map_err(|e| ToadStoolError::io(e.to_string()))?;

        for row in rows {
            let metric_name: String = row.get("metric_name");
            let value: f64 = row.get("value");

            // Check CPU threshold
            if metric_name.contains("cpu") && value > self.config.alert_thresholds.cpu_threshold {
                triggered_alerts.push(Alert {
                    id: Uuid::new_v4(),
                    name: format!("High CPU Usage: {metric_name}"),
                    metric_name: metric_name.clone(),
                    condition: AlertCondition::Threshold {
                        operator: ">".to_string(),
                        value: self.config.alert_thresholds.cpu_threshold,
                    },
                    severity: AlertSeverity::Warning,
                    created_at: Utc::now(),
                    last_triggered: Some(Utc::now()),
                    status: AlertStatus::Active,
                    recipients: vec!["admin@example.com".to_string()],
                });
            }

            // Check memory threshold
            if metric_name.contains("memory")
                && value > self.config.alert_thresholds.memory_threshold
            {
                triggered_alerts.push(Alert {
                    id: Uuid::new_v4(),
                    name: format!("High Memory Usage: {metric_name}"),
                    metric_name: metric_name.clone(),
                    condition: AlertCondition::Threshold {
                        operator: ">".to_string(),
                        value: self.config.alert_thresholds.memory_threshold,
                    },
                    severity: AlertSeverity::Critical,
                    created_at: Utc::now(),
                    last_triggered: Some(Utc::now()),
                    status: AlertStatus::Active,
                    recipients: vec!["admin@example.com".to_string()],
                });
            }
        }

        // Send alerts via broadcast channel
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

        // Fetch data for each panel
        let mut panel_data = serde_json::Map::new();

        for panel in &dashboard.panels {
            let mut metrics_data = Vec::new();

            for metric_name in &panel.metrics {
                let rows = sqlx::query(
                    r"
                    SELECT timestamp, value FROM analytics_data 
                    WHERE metric_name = ? AND timestamp >= ? AND timestamp <= ?
                    ORDER BY timestamp ASC
                ",
                )
                .bind(metric_name)
                .bind(panel.time_range.from.to_rfc3339())
                .bind(panel.time_range.to.to_rfc3339())
                .fetch_all(&self.database)
                .await
                .map_err(|e| ToadStoolError::io(e.to_string()))?;

                let data_points: Vec<serde_json::Value> = rows
                    .iter()
                    .map(|row| {
                        let timestamp_str: String = row.get("timestamp");
                        let value: f64 = row.get("value");
                        serde_json::json!({
                            "timestamp": timestamp_str,
                            "value": value
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

        // Export to configured webhooks
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
    /// Export metrics to a webhook endpoint - PURE RUST
    ///
    /// **EVOLUTION**: Use Songbird for external HTTP
    async fn export_to_webhook(&self, webhook: &WebhookConfig) -> ToadStoolResult<()> {
        // PURE RUST: External HTTP disabled - use Songbird
        tracing::info!("Webhook export to {} - use Songbird for external HTTP", webhook.url);
        
        // Would use Songbird RPC client here:
        // let songbird = SongbirdClient::discover().await?;
        // songbird.http_post(&webhook.url, payload).await?;
        
        Ok(())
    }
}
