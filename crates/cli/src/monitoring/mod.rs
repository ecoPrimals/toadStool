//! Monitoring and Observability - Real-time System Intelligence
//!
//! Comprehensive monitoring for `ToadStool` universal compute platform:
//! - Real-time biome metrics and health monitoring
//! - System resource tracking and alerting
//! - Performance analytics and trend analysis
//! - Ecosystem-wide observability integration

mod collectors;
mod display;
mod types;

use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool_common::constants::timeouts::HEALTH_CHECK_INTERVAL;

pub use collectors::{
    MetricsCollector, NetworkMetricsCollector, ProcessMetricsCollector, SystemMetricsCollector,
};
pub use display::format_prometheus;
pub use types::*;

/// Comprehensive monitoring system for `ToadStool`
pub struct MonitoringSystem {
    sessions: Arc<RwLock<HashMap<String, MonitoringSession>>>,
    collectors: Arc<RwLock<HashMap<String, Arc<dyn MetricsCollector + Send + Sync>>>>,
    alert_rules: Arc<RwLock<Vec<AlertRule>>>,
    metrics_store: Arc<RwLock<MetricsStore>>,
    config: MonitoringConfig,
}

/// Metrics storage and time series data
pub struct MetricsStore {
    series: HashMap<String, TimeSeries>,
    stats: HashMap<String, MetricStats>,
    retention_period: Duration,
}

impl MonitoringSystem {
    pub async fn new(config: MonitoringConfig) -> Result<Self> {
        info!("📊 Initializing ToadStool Monitoring System");

        let system = Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            collectors: Arc::new(RwLock::new(HashMap::new())),
            alert_rules: Arc::new(RwLock::new(Vec::new())),
            metrics_store: Arc::new(RwLock::new(MetricsStore::new(config.retention_period))),
            config,
        };

        system.register_default_collectors().await?;
        system.load_default_alert_rules().await?;

        info!("✅ Monitoring system initialized");
        Ok(system)
    }

    pub async fn start_monitoring(
        &self,
        target: MonitoringTarget,
        metrics: Vec<String>,
        interval: Option<Duration>,
    ) -> Result<Uuid> {
        let session_id = Uuid::new_v4();
        let session_interval = interval.unwrap_or(self.config.default_interval);

        info!(
            "📈 Starting monitoring session: {} for {:?}",
            session_id, target
        );

        let session = MonitoringSession {
            id: session_id,
            target: target.clone(),
            started: std::time::SystemTime::now(),
            interval: session_interval,
            metrics: metrics.clone(),
            status: SessionStatus::Active,
            last_update: std::time::SystemTime::now(),
        };

        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.to_string(), session);
        }

        self.spawn_collection_task(session_id, target, metrics, session_interval)
            .await;

        Ok(session_id)
    }

    pub async fn stop_monitoring(&self, session_id: Uuid) -> Result<()> {
        info!("⏹️  Stopping monitoring session: {}", session_id);

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id.to_string()) {
            session.status = SessionStatus::Stopped;
            session.last_update = std::time::SystemTime::now();
        }

        Ok(())
    }

    pub async fn get_dashboard_data(&self) -> Result<DashboardData> {
        let timestamp = std::time::SystemTime::now();

        let system_health = self.collect_system_health().await?;
        let biome_status = self.collect_biome_status().await?;
        let resource_usage = self.collect_resource_usage().await?;
        let alerts = self.get_active_alerts().await?;
        let performance_metrics = self.collect_performance_metrics().await?;

        Ok(DashboardData {
            timestamp,
            system_health,
            biome_status,
            resource_usage,
            alerts,
            performance_metrics,
        })
    }

    pub async fn query_metrics(
        &self,
        metric_name: String,
        start_time: std::time::SystemTime,
        end_time: std::time::SystemTime,
        _labels: HashMap<String, String>,
    ) -> Result<Vec<DataPoint>> {
        let metrics_store = self.metrics_store.read().await;

        if let Some(series) = metrics_store.series.get(&metric_name) {
            let filtered_points: Vec<DataPoint> = series
                .data_points
                .iter()
                .filter(|point| point.timestamp >= start_time && point.timestamp <= end_time)
                .cloned()
                .collect();

            Ok(filtered_points)
        } else {
            Ok(Vec::new())
        }
    }

    pub async fn get_metric_stats(&self, metric_name: &str) -> Result<Option<MetricStats>> {
        let metrics_store = self.metrics_store.read().await;
        Ok(metrics_store.stats.get(metric_name).cloned())
    }

    pub async fn add_alert_rule(&self, rule: AlertRule) -> Result<()> {
        info!("🚨 Adding alert rule: {}", rule.name);

        let mut alert_rules = self.alert_rules.write().await;
        alert_rules.push(rule);

        Ok(())
    }

    pub async fn export_prometheus(&self) -> Result<String> {
        let metrics_store = self.metrics_store.read().await;
        Ok(display::format_prometheus(&metrics_store.series))
    }

    async fn register_default_collectors(&self) -> Result<()> {
        let mut collectors = self.collectors.write().await;

        collectors.insert(
            "system".to_string(),
            Arc::new(SystemMetricsCollector::new()),
        );
        collectors.insert(
            "process".to_string(),
            Arc::new(ProcessMetricsCollector::new()),
        );
        collectors.insert(
            "network".to_string(),
            Arc::new(NetworkMetricsCollector::new()),
        );

        info!("📊 Registered {} default collectors", collectors.len());
        Ok(())
    }

    async fn load_default_alert_rules(&self) -> Result<()> {
        let mut rules = self.alert_rules.write().await;

        rules.push(AlertRule {
            id: "high_cpu".to_string(),
            name: "High CPU Usage".to_string(),
            condition: AlertCondition::Threshold {
                metric: "cpu_usage_percent".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 90.0,
                duration: Duration::from_secs(300),
            },
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown: Duration::from_secs(600),
            last_triggered: None,
        });

        rules.push(AlertRule {
            id: "high_memory".to_string(),
            name: "High Memory Usage".to_string(),
            condition: AlertCondition::Threshold {
                metric: "memory_usage_percent".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 85.0,
                duration: Duration::from_secs(300),
            },
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown: Duration::from_secs(600),
            last_triggered: None,
        });

        rules.push(AlertRule {
            id: "low_storage".to_string(),
            name: "Low Storage Space".to_string(),
            condition: AlertCondition::Threshold {
                metric: "storage_usage_percent".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 95.0,
                duration: Duration::from_secs(60),
            },
            severity: AlertSeverity::Critical,
            enabled: true,
            cooldown: Duration::from_secs(1800),
            last_triggered: None,
        });

        info!("🚨 Loaded {} default alert rules", rules.len());
        Ok(())
    }

    #[allow(clippy::unused_async)] // Spawns background task; async for API consistency
    async fn spawn_collection_task(
        &self,
        session_id: Uuid,
        _target: MonitoringTarget,
        _metrics: Vec<String>,
        interval: Duration,
    ) {
        let sessions = Arc::clone(&self.sessions);
        let collectors = Arc::clone(&self.collectors);
        let metrics_store = Arc::clone(&self.metrics_store);

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                {
                    let sessions_guard = sessions.read().await;
                    if let Some(session) = sessions_guard.get(&session_id.to_string()) {
                        match session.status {
                            SessionStatus::Active => {}
                            _ => break,
                        }
                    } else {
                        break;
                    }
                }

                let collectors_guard = collectors.read().await;
                for collector in collectors_guard.values() {
                    if let Ok(batch) = collector.collect() {
                        let mut store = metrics_store.write().await;
                        store.store_batch(batch).await;
                    }
                }
            }

            debug!("Collection task ended for session: {}", session_id);
        });
    }

    #[allow(clippy::unused_async)] // Collector trait; placeholder impl
    async fn collect_system_health(&self) -> Result<SystemHealth> {
        Ok(SystemHealth {
            overall_status: HealthStatus::Healthy,
            cpu_health: HealthStatus::Healthy,
            memory_health: HealthStatus::Healthy,
            storage_health: HealthStatus::Healthy,
            network_health: HealthStatus::Healthy,
        })
    }

    #[allow(clippy::unused_async)] // Collector trait; placeholder impl
    async fn collect_biome_status(&self) -> Result<Vec<BiomeStatusSummary>> {
        Ok(vec![BiomeStatusSummary {
            name: "example-biome".to_string(),
            status: "running".to_string(),
            services_running: 3,
            services_total: 3,
            cpu_usage: 45.2,
            memory_usage: 62.8,
            uptime: Duration::from_secs(3600),
        }])
    }

    #[allow(clippy::unused_async)] // Collector trait; placeholder impl
    async fn collect_resource_usage(&self) -> Result<SystemResourceUsage> {
        Ok(SystemResourceUsage {
            cpu_percent: 45.2,
            memory_used_gb: 8.5,
            memory_total_gb: 16.0,
            storage_used_gb: 125.0,
            storage_total_gb: 500.0,
            network_rx_mbps: 12.5,
            network_tx_mbps: 8.3,
            load_average: vec![1.2, 1.5, 1.8],
        })
    }

    #[allow(clippy::unused_async)] // Collector trait; placeholder impl
    async fn get_active_alerts(&self) -> Result<Vec<ActiveAlert>> {
        Ok(vec![])
    }

    #[allow(clippy::unused_async)] // Collector trait; placeholder impl
    async fn collect_performance_metrics(&self) -> Result<PerformanceMetrics> {
        Ok(PerformanceMetrics {
            execution_latency_ms: 125.5,
            throughput_ops_sec: 1250.0,
            error_rate: 0.02,
            success_rate: 99.98,
            queue_depth: 5,
        })
    }
}

impl MetricsStore {
    fn new(retention_period: Duration) -> Self {
        Self {
            series: HashMap::new(),
            stats: HashMap::new(),
            retention_period,
        }
    }

    async fn store_batch(&mut self, batch: MetricBatch) {
        for metric in batch.metrics {
            let series = self
                .series
                .entry(metric.name.clone())
                .or_insert_with(|| TimeSeries {
                    name: metric.name.clone(),
                    data_points: Vec::new(),
                    labels: metric.labels.clone(),
                });

            if let MetricValue::Gauge(value) = metric.value {
                series.data_points.push(DataPoint {
                    timestamp: metric.timestamp,
                    value,
                });

                self.update_stats(&metric.name, value);
            }
        }

        self.cleanup_old_data().await;
    }

    fn update_stats(&mut self, metric_name: &str, value: f64) {
        let stats = self
            .stats
            .entry(metric_name.to_string())
            .or_insert_with(|| MetricStats {
                count: 0,
                min: f64::INFINITY,
                max: f64::NEG_INFINITY,
                avg: 0.0,
                percentiles: HashMap::new(),
            });

        stats.count += 1;
        stats.min = stats.min.min(value);
        stats.max = stats.max.max(value);
        stats.avg = (stats.avg * (stats.count - 1) as f64 + value) / stats.count as f64;
    }

    #[allow(clippy::unused_async)] // Sync retain; async for API consistency
    async fn cleanup_old_data(&mut self) {
        let now = std::time::SystemTime::now();
        if let Some(cutoff_time) = now.checked_sub(self.retention_period) {
            for series in self.series.values_mut() {
                series
                    .data_points
                    .retain(|point| point.timestamp > cutoff_time);
            }
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            default_interval: HEALTH_CHECK_INTERVAL,
            retention_period: Duration::from_secs(7 * 24 * 3600),
            max_metrics_per_batch: 1000,
            enable_alerts: true,
            export_prometheus: true,
            export_grafana: true,
        }
    }
}
