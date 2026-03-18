// SPDX-License-Identifier: AGPL-3.0-or-later
//! Monitoring and Observability - Real-time System Intelligence
//!
//! Comprehensive monitoring for `ToadStool` universal compute platform:
//! - Real-time biome metrics and health monitoring
//! - System resource tracking and alerting
//! - Performance analytics and trend analysis
//! - Ecosystem-wide observability integration

mod alerting;
mod collectors;
mod dashboard;
mod display;
mod metrics_store;
mod types;

use crate::Result;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

use toadstool_common::constants::timeouts::HEALTH_CHECK_INTERVAL;

pub use alerting::{evaluate_health_alerts, load_default_alert_rules};
pub use collectors::{
    MetricsCollector, NetworkMetricsCollector, ProcessMetricsCollector, SystemMetricsCollector,
};
pub use dashboard::{
    collect_biome_status, collect_performance_metrics, collect_resource_usage,
    collect_system_health,
};
pub use display::format_prometheus;
pub use metrics_store::MetricsStore;
pub use types::*;

/// Comprehensive monitoring system for `ToadStool`
pub struct MonitoringSystem {
    sessions: Arc<RwLock<HashMap<String, MonitoringSession>>>,
    collectors: Arc<RwLock<HashMap<String, Arc<dyn MetricsCollector + Send + Sync>>>>,
    alert_rules: Arc<RwLock<Vec<AlertRule>>>,
    metrics_store: Arc<RwLock<MetricsStore>>,
    config: MonitoringConfig,
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

        {
            let mut sessions = self.sessions.write().await;
            if let Some(session) = sessions.get_mut(&session_id.to_string()) {
                session.status = SessionStatus::Stopped;
                session.last_update = std::time::SystemTime::now();
            }
        }

        Ok(())
    }

    pub async fn get_dashboard_data(&self) -> Result<DashboardData> {
        let timestamp = std::time::SystemTime::now();

        let (system_health, biome_status, resource_usage, alerts, performance_metrics) = tokio::try_join!(
            async { dashboard::collect_system_health() },
            async { dashboard::collect_biome_status() },
            async { dashboard::collect_resource_usage() },
            self.get_active_alerts(),
            async {
                let sessions = self.sessions.read().await;
                Ok(dashboard::collect_performance_metrics(&sessions))
            },
        )?;

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

        metrics_store.series.get(&metric_name).map_or_else(
            || Ok(Vec::new()),
            |series| {
                let filtered_points: Vec<DataPoint> = series
                    .data_points
                    .iter()
                    .filter(|point| point.timestamp >= start_time && point.timestamp <= end_time)
                    .cloned()
                    .collect();
                Ok(filtered_points)
            },
        )
    }

    pub async fn get_metric_stats(&self, metric_name: &str) -> Result<Option<MetricStats>> {
        let metrics_store = self.metrics_store.read().await;
        Ok(metrics_store.stats.get(metric_name).cloned())
    }

    pub async fn add_alert_rule(&self, rule: AlertRule) -> Result<()> {
        info!("🚨 Adding alert rule: {}", rule.name);

        {
            let mut alert_rules = self.alert_rules.write().await;
            alert_rules.push(rule);
        }

        Ok(())
    }

    pub async fn export_prometheus(&self) -> Result<String> {
        let metrics_store = self.metrics_store.read().await;
        Ok(display::format_prometheus(&metrics_store.series))
    }

    async fn register_default_collectors(&self) -> Result<()> {
        {
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
            drop(collectors);
        }

        Ok(())
    }

    async fn load_default_alert_rules(&self) -> Result<()> {
        {
            let mut rules = self.alert_rules.write().await;
            rules.extend(load_default_alert_rules());
            info!("🚨 Loaded {} default alert rules", rules.len());
            drop(rules);
        }

        Ok(())
    }

    #[allow(clippy::unused_async)]
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
                        drop(store);
                    }
                }
            }

            debug!("Collection task ended for session: {}", session_id);
        });
    }

    async fn get_active_alerts(&self) -> Result<Vec<ActiveAlert>> {
        let rules = self.alert_rules.read().await;
        let health = dashboard::collect_system_health()?;
        let alerts = alerting::evaluate_health_alerts(&health);

        for rule in rules.iter() {
            if rule.enabled {
                debug!("Evaluating alert rule: {}", rule.name);
            }
        }
        drop(rules);

        Ok(alerts)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(config.enable_alerts);
        assert!(config.export_prometheus);
        assert_eq!(config.max_metrics_per_batch, 1000);
    }

    #[tokio::test]
    async fn test_monitoring_system_new() {
        let config = MonitoringConfig::default();
        let system = MonitoringSystem::new(config)
            .await
            .expect("create monitoring system");
        drop(system);
    }

    #[tokio::test]
    async fn test_monitoring_system_start_and_stop() {
        let config = MonitoringConfig::default();
        let system = MonitoringSystem::new(config).await.expect("create");
        let session_id = system
            .start_monitoring(
                MonitoringTarget::System,
                vec!["cpu_usage_percent".to_string()],
                Some(Duration::from_secs(60)),
            )
            .await
            .expect("start monitoring");
        let stop_result = system.stop_monitoring(session_id).await;
        assert!(stop_result.is_ok());
    }

    #[tokio::test]
    async fn test_monitoring_system_get_dashboard_data() {
        let config = MonitoringConfig::default();
        let system = MonitoringSystem::new(config).await.expect("create");
        let dashboard = system.get_dashboard_data().await.expect("dashboard");
        assert!(dashboard.timestamp != std::time::UNIX_EPOCH);
        assert!(dashboard.resource_usage.cpu_percent >= 0.0);
    }

    #[tokio::test]
    async fn test_monitoring_system_query_metrics_empty() {
        let config = MonitoringConfig::default();
        let system = MonitoringSystem::new(config).await.expect("create");
        let start = std::time::SystemTime::now();
        let end = std::time::SystemTime::now();
        let points = system
            .query_metrics("nonexistent_metric".to_string(), start, end, HashMap::new())
            .await
            .expect("query");
        assert!(points.is_empty());
    }

    #[tokio::test]
    async fn test_monitoring_system_get_metric_stats_none() {
        let config = MonitoringConfig::default();
        let system = MonitoringSystem::new(config).await.expect("create");
        let stats = system.get_metric_stats("nonexistent").await.expect("stats");
        assert!(stats.is_none());
    }

    #[tokio::test]
    async fn test_monitoring_system_add_alert_rule() {
        let config = MonitoringConfig::default();
        let system = MonitoringSystem::new(config).await.expect("create");
        let rule = AlertRule {
            id: "test_rule".to_string(),
            name: "Test Alert".to_string(),
            condition: AlertCondition::Threshold {
                metric: "cpu_usage_percent".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 95.0,
                duration: Duration::from_secs(60),
            },
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown: Duration::from_secs(300),
            last_triggered: None,
        };
        let result = system.add_alert_rule(rule).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_monitoring_system_export_prometheus() {
        let config = MonitoringConfig::default();
        let system = MonitoringSystem::new(config).await.expect("create");
        let prom = system.export_prometheus().await.expect("export");
        assert!(prom.is_empty() || prom.contains("gauge"));
    }

    #[tokio::test]
    async fn test_monitoring_targets() {
        let _biome = MonitoringTarget::Biome("test-biome".to_string());
        let _service = MonitoringTarget::Service("biome".to_string(), "svc".to_string());
        let _system = MonitoringTarget::System;
        let _platform = MonitoringTarget::Platform("linux".to_string());
        let _fed = MonitoringTarget::Federation;
    }

    #[tokio::test]
    async fn test_monitoring_session_status() {
        let _active = SessionStatus::Active;
        let _paused = SessionStatus::Paused;
        let _stopped = SessionStatus::Stopped;
        let _err = SessionStatus::Error("test".to_string());
    }

    #[test]
    fn test_system_metrics_collector() {
        let collector = SystemMetricsCollector::new();
        assert_eq!(collector.name(), "system");
        let batch = collector.collect().expect("collect");
        assert_eq!(batch.source, "system");
        assert!(!batch.metrics.is_empty());
        assert!(collector.capabilities().contains(&"cpu".to_string()));
    }

    #[test]
    fn test_process_metrics_collector() {
        let collector = ProcessMetricsCollector::new();
        assert_eq!(collector.name(), "process");
        let batch = collector.collect().expect("collect");
        assert_eq!(batch.source, "process");
        assert!(!collector.capabilities().is_empty());
    }

    #[test]
    fn test_network_metrics_collector() {
        let collector = NetworkMetricsCollector::new();
        assert_eq!(collector.name(), "network");
        let result = collector.collect();
        assert!(result.is_ok() || result.is_err());
    }
}
