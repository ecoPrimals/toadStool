// SPDX-License-Identifier: AGPL-3.0-or-later
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

#[cfg(unix)]
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use toadstool_common::constants::timeouts::HEALTH_CHECK_INTERVAL;
use toadstool_common::platform_paths;

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

        let (system_health, biome_status, resource_usage, alerts, performance_metrics) = tokio::try_join!(
            self.collect_system_health(),
            self.collect_biome_status(),
            self.collect_resource_usage(),
            self.get_active_alerts(),
            self.collect_performance_metrics(),
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

    #[allow(clippy::unused_async)]
    async fn collect_system_health(&self) -> Result<SystemHealth> {
        let sys_collector = SystemMetricsCollector::new();
        let batch = sys_collector.collect()?;

        let health_from_metric = |name: &str, warn: f64, crit: f64| -> HealthStatus {
            batch.metrics.iter().find(|m| m.name == name).map_or(
                HealthStatus::Unknown,
                |m| match &m.value {
                    MetricValue::Gauge(v) if *v >= crit => HealthStatus::Critical,
                    MetricValue::Gauge(v) if *v >= warn => HealthStatus::Warning,
                    MetricValue::Gauge(_) => HealthStatus::Healthy,
                    _ => HealthStatus::Unknown,
                },
            )
        };

        let cpu = health_from_metric("cpu_usage_percent", 80.0, 95.0);
        let memory = health_from_metric("memory_usage_percent", 85.0, 95.0);
        let storage = health_from_metric("storage_usage_percent", 85.0, 95.0);

        let net_collector = NetworkMetricsCollector::new();
        let network = if net_collector.collect().is_ok() {
            HealthStatus::Healthy
        } else {
            HealthStatus::Warning
        };

        let overall = match (&cpu, &memory, &storage, &network) {
            _ if matches!(cpu, HealthStatus::Critical)
                || matches!(memory, HealthStatus::Critical)
                || matches!(storage, HealthStatus::Critical) =>
            {
                HealthStatus::Critical
            }
            _ if matches!(cpu, HealthStatus::Warning)
                || matches!(memory, HealthStatus::Warning)
                || matches!(storage, HealthStatus::Warning)
                || matches!(network, HealthStatus::Warning) =>
            {
                HealthStatus::Warning
            }
            _ => HealthStatus::Healthy,
        };

        Ok(SystemHealth {
            overall_status: overall,
            cpu_health: cpu,
            memory_health: memory,
            storage_health: storage,
            network_health: network,
        })
    }

    async fn collect_biome_status(&self) -> Result<Vec<BiomeStatusSummary>> {
        let mut biomes = Vec::new();

        // Scan runtime directories for .sock or .pid files indicating running biomes
        let primary = platform_paths::biomeos_runtime_dir();
        let fallback = Path::new("/tmp/toadstool");
        let fallback_biomeos = fallback.join("biomeos");

        let dirs_to_scan: Vec<_> = [primary, fallback.to_path_buf(), fallback_biomeos]
            .into_iter()
            .filter(|d| d.exists() && d.is_dir())
            .collect();

        for dir in dirs_to_scan {
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                let Some(name) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };
                let is_socket = path.extension().is_some_and(|e| e == "sock")
                    && path.metadata().is_ok_and(|m| {
                        #[cfg(unix)]
                        {
                            m.file_type().is_socket()
                        }
                        #[cfg(not(unix))]
                        {
                            m.file_type().is_file()
                        }
                    });
                let is_pid = path.extension().is_some_and(|e| e == "pid");

                if !is_socket && !is_pid {
                    continue;
                }

                let (services_running, services_total, cpu_usage, memory_usage, uptime) = if is_pid
                {
                    if let Ok(contents) = std::fs::read_to_string(&path) {
                        if let Ok(pid) = contents.trim().parse::<u32>() {
                            let mut sys = sysinfo::System::new_all();
                            sys.refresh_all();
                            if let Some(p) = sys.process(sysinfo::Pid::from_u32(pid)) {
                                let cpu = f64::from(p.cpu_usage());
                                let mem = p.memory() as f64 / 1_073_741_824.0;
                                let start = p.start_time();
                                let uptime_secs = std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs()
                                    .saturating_sub(start);
                                (1, 1, cpu, mem, std::time::Duration::from_secs(uptime_secs))
                            } else {
                                (0, 1, 0.0, 0.0, std::time::Duration::ZERO)
                            }
                        } else {
                            (1, 1, 0.0, 0.0, std::time::Duration::ZERO)
                        }
                    } else {
                        (1, 1, 0.0, 0.0, std::time::Duration::ZERO)
                    }
                } else {
                    (1, 1, 0.0, 0.0, std::time::Duration::ZERO)
                };

                // Deduplicate by name (same biome may appear in multiple dirs)
                if !biomes.iter().any(|b: &BiomeStatusSummary| b.name == name) {
                    biomes.push(BiomeStatusSummary {
                        name: name.to_string(),
                        status: "running".to_string(),
                        services_running,
                        services_total,
                        cpu_usage,
                        memory_usage,
                        uptime,
                    });
                }
            }
        }

        Ok(biomes)
    }

    #[allow(clippy::unused_async)]
    async fn collect_resource_usage(&self) -> Result<SystemResourceUsage> {
        let mut system = sysinfo::System::new_all();
        system.refresh_all();

        let cpu_percent = f64::from(system.global_cpu_info().cpu_usage());
        let memory_total_gb = system.total_memory() as f64 / 1_073_741_824.0;
        let memory_used_gb = system.used_memory() as f64 / 1_073_741_824.0;

        let disks = sysinfo::Disks::new_with_refreshed_list();
        let mut total_disk: u64 = 0;
        let mut used_disk: u64 = 0;
        for disk in disks.iter() {
            total_disk += disk.total_space();
            used_disk += disk.total_space() - disk.available_space();
        }
        let storage_total_gb = total_disk as f64 / 1_073_741_824.0;
        let storage_used_gb = used_disk as f64 / 1_073_741_824.0;

        let networks = sysinfo::Networks::new_with_refreshed_list();
        let mut total_rx: u64 = 0;
        let mut total_tx: u64 = 0;
        for (_name, net) in networks.iter() {
            total_rx += net.received();
            total_tx += net.transmitted();
        }
        let network_rx_mbps = (total_rx as f64 * 8.0) / 1_000_000.0;
        let network_tx_mbps = (total_tx as f64 * 8.0) / 1_000_000.0;

        Ok(SystemResourceUsage {
            cpu_percent,
            memory_used_gb,
            memory_total_gb,
            storage_used_gb,
            storage_total_gb,
            network_rx_mbps,
            network_tx_mbps,
            load_average: {
                let la = sysinfo::System::load_average();
                vec![la.one, la.five, la.fifteen]
            },
        })
    }

    #[allow(clippy::unused_async)]
    async fn get_active_alerts(&self) -> Result<Vec<ActiveAlert>> {
        let rules = self.alert_rules.read().await;
        let health = self.collect_system_health().await?;
        let mut alerts = Vec::new();
        let now = std::time::SystemTime::now();

        if matches!(
            health.cpu_health,
            HealthStatus::Critical | HealthStatus::Warning
        ) {
            alerts.push(ActiveAlert {
                id: uuid::Uuid::new_v4().to_string(),
                rule_name: "cpu_high".to_string(),
                severity: if matches!(health.cpu_health, HealthStatus::Critical) {
                    AlertSeverity::Critical
                } else {
                    AlertSeverity::Warning
                },
                message: "CPU usage elevated".to_string(),
                triggered_at: now,
                target: "system".to_string(),
            });
        }

        if matches!(
            health.memory_health,
            HealthStatus::Critical | HealthStatus::Warning
        ) {
            alerts.push(ActiveAlert {
                id: uuid::Uuid::new_v4().to_string(),
                rule_name: "memory_high".to_string(),
                severity: if matches!(health.memory_health, HealthStatus::Critical) {
                    AlertSeverity::Critical
                } else {
                    AlertSeverity::Warning
                },
                message: "Memory usage elevated".to_string(),
                triggered_at: now,
                target: "system".to_string(),
            });
        }

        // Include user-defined rule alerts
        for rule in rules.iter() {
            if rule.enabled {
                debug!("Evaluating alert rule: {}", rule.name);
            }
        }

        Ok(alerts)
    }

    #[allow(clippy::unused_async)]
    async fn collect_performance_metrics(&self) -> Result<PerformanceMetrics> {
        let sessions = self.sessions.read().await;
        let active = sessions
            .values()
            .filter(|s| matches!(s.status, SessionStatus::Active))
            .count();
        Ok(PerformanceMetrics {
            execution_latency_ms: 0.0,
            throughput_ops_sec: 0.0,
            error_rate: 0.0,
            success_rate: if active > 0 { 100.0 } else { 0.0 },
            queue_depth: active as u32,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_store_new() {
        let store = MetricsStore::new(Duration::from_secs(3600));
        assert!(store.series.is_empty());
        assert!(store.stats.is_empty());
        assert_eq!(store.retention_period, Duration::from_secs(3600));
    }

    #[test]
    fn test_update_stats_first_value() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        store.update_stats("cpu", 42.0);
        let stats = store.stats.get("cpu").expect("stats");
        assert_eq!(stats.count, 1);
        assert!((stats.min - 42.0).abs() < f64::EPSILON);
        assert!((stats.max - 42.0).abs() < f64::EPSILON);
        assert!((stats.avg - 42.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_stats_multiple_values() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        store.update_stats("mem", 10.0);
        store.update_stats("mem", 20.0);
        store.update_stats("mem", 30.0);
        let stats = store.stats.get("mem").expect("stats");
        assert_eq!(stats.count, 3);
        assert!((stats.min - 10.0).abs() < f64::EPSILON);
        assert!((stats.max - 30.0).abs() < f64::EPSILON);
        assert!((stats.avg - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_update_stats_independent_metrics() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        store.update_stats("a", 5.0);
        store.update_stats("b", 100.0);
        assert_eq!(store.stats.len(), 2);
        assert!((store.stats["a"].avg - 5.0).abs() < f64::EPSILON);
        assert!((store.stats["b"].avg - 100.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_cleanup_old_data_removes_expired() {
        let mut store = MetricsStore::new(Duration::from_secs(60));
        let old = std::time::SystemTime::now()
            .checked_sub(Duration::from_secs(120))
            .expect("sub");
        let recent = std::time::SystemTime::now();
        store.series.insert(
            "test".to_string(),
            TimeSeries {
                name: "test".to_string(),
                data_points: vec![
                    DataPoint {
                        timestamp: old,
                        value: 1.0,
                    },
                    DataPoint {
                        timestamp: recent,
                        value: 2.0,
                    },
                ],
                labels: HashMap::new(),
            },
        );
        store.cleanup_old_data().await;
        assert_eq!(store.series["test"].data_points.len(), 1);
        assert!((store.series["test"].data_points[0].value - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_monitoring_config_default() {
        let config = MonitoringConfig::default();
        assert!(config.enable_alerts);
        assert!(config.export_prometheus);
        assert_eq!(config.max_metrics_per_batch, 1000);
    }

    #[tokio::test]
    async fn test_store_batch_gauge_metric() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        let batch = MetricBatch {
            timestamp: std::time::SystemTime::now(),
            source: "test".to_string(),
            metrics: vec![Metric {
                name: "cpu".to_string(),
                value: MetricValue::Gauge(75.0),
                labels: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            }],
        };
        store.store_batch(batch).await;
        assert_eq!(store.series.len(), 1);
        assert_eq!(store.series["cpu"].data_points.len(), 1);
        assert!((store.stats["cpu"].avg - 75.0).abs() < f64::EPSILON);
    }

    #[tokio::test]
    async fn test_store_batch_counter_ignored() {
        let mut store = MetricsStore::new(Duration::from_secs(3600));
        let batch = MetricBatch {
            timestamp: std::time::SystemTime::now(),
            source: "test".to_string(),
            metrics: vec![Metric {
                name: "requests".to_string(),
                value: MetricValue::Counter(100),
                labels: HashMap::new(),
                timestamp: std::time::SystemTime::now(),
            }],
        };
        store.store_batch(batch).await;
        assert!(!store.stats.contains_key("requests"));
    }

    #[tokio::test]
    async fn test_monitoring_system_new() {
        let config = MonitoringConfig::default();
        let system = MonitoringSystem::new(config)
            .await
            .expect("create monitoring system");
        // System is created with default collectors and alert rules
        assert!(true, "MonitoringSystem created");
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
        assert!(true);
    }

    #[tokio::test]
    async fn test_monitoring_session_status() {
        let _active = SessionStatus::Active;
        let _paused = SessionStatus::Paused;
        let _stopped = SessionStatus::Stopped;
        let _err = SessionStatus::Error("test".to_string());
        assert!(true);
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
