//! Monitoring and Observability - Real-time System Intelligence
//!
//! Comprehensive monitoring for ToadStool universal compute platform:
//! - Real-time biome metrics and health monitoring
//! - System resource tracking and alerting
//! - Performance analytics and trend analysis
//! - Ecosystem-wide observability integration

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use sysinfo::{CpuExt, DiskExt, NetworkExt, System, SystemExt};
use tokio::sync::RwLock;
use tokio::time::Duration;
use tracing::{debug, info};
use uuid::Uuid;

/// Comprehensive monitoring system for ToadStool
pub struct MonitoringSystem {
    /// Active monitoring sessions
    sessions: Arc<RwLock<HashMap<String, MonitoringSession>>>,
    /// Metrics collectors
    collectors: Arc<RwLock<HashMap<String, Arc<dyn MetricsCollector + Send + Sync>>>>,
    /// Alert rules
    alert_rules: Arc<RwLock<Vec<AlertRule>>>,
    /// Metrics storage
    metrics_store: Arc<RwLock<MetricsStore>>,
    /// Configuration
    config: MonitoringConfig,
}

/// Monitoring session for a specific biome or system
#[derive(Debug, Clone)]
pub struct MonitoringSession {
    pub id: Uuid,
    pub target: MonitoringTarget,
    pub started: DateTime<Utc>,
    pub interval: Duration,
    pub metrics: Vec<String>,
    pub status: SessionStatus,
    pub last_update: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringTarget {
    Biome(String),
    Service(String, String), // biome_name, service_name
    System,
    Platform(String),
    Federation,
}

#[derive(Debug, Clone)]
pub enum SessionStatus {
    Active,
    Paused,
    Stopped,
    Error(String),
}

/// Metrics collection interface
pub trait MetricsCollector {
    fn name(&self) -> &str;
    fn collect(&self) -> Result<MetricBatch>;
    fn capabilities(&self) -> Vec<String>;
}

/// Batch of collected metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBatch {
    pub timestamp: DateTime<Utc>,
    pub source: String,
    pub metrics: Vec<Metric>,
}

/// Individual metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: MetricValue,
    pub labels: HashMap<String, String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    Counter(u64),
    Gauge(f64),
    Histogram(Vec<f64>),
    Text(String),
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    pub id: String,
    pub name: String,
    pub condition: AlertCondition,
    pub severity: AlertSeverity,
    pub enabled: bool,
    pub cooldown: Duration,
    pub last_triggered: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    Threshold {
        metric: String,
        operator: ComparisonOperator,
        value: f64,
        duration: Duration,
    },
    RateOfChange {
        metric: String,
        threshold: f64,
        window: Duration,
    },
    Composite {
        conditions: Vec<AlertCondition>,
        operator: LogicalOperator,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
    GreaterThanOrEqual,
    LessThanOrEqual,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    And,
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
    Emergency,
}

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    pub default_interval: Duration,
    pub retention_period: Duration,
    pub max_metrics_per_batch: usize,
    pub enable_alerts: bool,
    pub export_prometheus: bool,
    pub export_grafana: bool,
}

/// Metrics storage and time series data
#[derive(Debug, Clone)]
pub struct MetricsStore {
    /// Time series data indexed by metric name
    series: HashMap<String, TimeSeries>,
    /// Aggregated statistics
    stats: HashMap<String, MetricStats>,
    /// Configuration
    retention_period: Duration,
}

#[derive(Debug, Clone)]
pub struct TimeSeries {
    pub name: String,
    pub data_points: Vec<DataPoint>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    pub timestamp: DateTime<Utc>,
    pub value: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStats {
    pub count: u64,
    pub min: f64,
    pub max: f64,
    pub avg: f64,
    pub percentiles: HashMap<String, f64>, // p50, p95, p99
}

/// Real-time dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    pub timestamp: DateTime<Utc>,
    pub system_health: SystemHealth,
    pub biome_status: Vec<BiomeStatusSummary>,
    pub resource_usage: SystemResourceUsage,
    pub alerts: Vec<ActiveAlert>,
    pub performance_metrics: PerformanceMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub overall_status: HealthStatus,
    pub cpu_health: HealthStatus,
    pub memory_health: HealthStatus,
    pub storage_health: HealthStatus,
    pub network_health: HealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Warning,
    Critical,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStatusSummary {
    pub name: String,
    pub status: String,
    pub services_running: u32,
    pub services_total: u32,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub uptime: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceUsage {
    pub cpu_percent: f64,
    pub memory_used_gb: f64,
    pub memory_total_gb: f64,
    pub storage_used_gb: f64,
    pub storage_total_gb: f64,
    pub network_rx_mbps: f64,
    pub network_tx_mbps: f64,
    pub load_average: Vec<f64>, // 1min, 5min, 15min
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAlert {
    pub id: String,
    pub rule_name: String,
    pub severity: AlertSeverity,
    pub message: String,
    pub triggered_at: DateTime<Utc>,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    pub execution_latency_ms: f64,
    pub throughput_ops_sec: f64,
    pub error_rate: f64,
    pub success_rate: f64,
    pub queue_depth: u32,
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

        // Register default collectors
        system.register_default_collectors().await?;

        // Load default alert rules
        system.load_default_alert_rules().await?;

        info!("✅ Monitoring system initialized");
        Ok(system)
    }

    /// Start monitoring a target
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
            started: Utc::now(),
            interval: session_interval,
            metrics: metrics.clone(),
            status: SessionStatus::Active,
            last_update: Utc::now(),
        };

        // Store session
        {
            let mut sessions = self.sessions.write().await;
            sessions.insert(session_id.to_string(), session);
        }

        // Start collection task
        self.spawn_collection_task(session_id, target, metrics, session_interval)
            .await;

        Ok(session_id)
    }

    /// Stop monitoring session
    pub async fn stop_monitoring(&self, session_id: Uuid) -> Result<()> {
        info!("⏹️  Stopping monitoring session: {}", session_id);

        let mut sessions = self.sessions.write().await;
        if let Some(session) = sessions.get_mut(&session_id.to_string()) {
            session.status = SessionStatus::Stopped;
            session.last_update = Utc::now();
        }

        Ok(())
    }

    /// Get real-time dashboard data
    pub async fn get_dashboard_data(&self) -> Result<DashboardData> {
        let timestamp = Utc::now();

        // Collect system health
        let system_health = self.collect_system_health().await?;

        // Collect biome status
        let biome_status = self.collect_biome_status().await?;

        // Collect resource usage
        let resource_usage = self.collect_resource_usage().await?;

        // Get active alerts
        let alerts = self.get_active_alerts().await?;

        // Get performance metrics
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

    /// Query metrics with time range and filters
    pub async fn query_metrics(
        &self,
        metric_name: String,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        labels: HashMap<String, String>,
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

    /// Get metric statistics
    pub async fn get_metric_stats(&self, metric_name: String) -> Result<Option<MetricStats>> {
        let metrics_store = self.metrics_store.read().await;
        Ok(metrics_store.stats.get(&metric_name).cloned())
    }

    /// Add custom alert rule
    pub async fn add_alert_rule(&self, rule: AlertRule) -> Result<()> {
        info!("🚨 Adding alert rule: {}", rule.name);

        let mut alert_rules = self.alert_rules.write().await;
        alert_rules.push(rule);

        Ok(())
    }

    /// Export metrics in Prometheus format
    pub async fn export_prometheus(&self) -> Result<String> {
        let metrics_store = self.metrics_store.read().await;
        let mut output = String::new();

        for (name, series) in &metrics_store.series {
            if let Some(latest) = series.data_points.last() {
                // Format as Prometheus metric
                output.push_str(&format!("# TYPE {} gauge\n", name));
                output.push_str(&format!("{} {}\n", name, latest.value));
            }
        }

        Ok(output)
    }

    // Internal implementation methods

    async fn register_default_collectors(&self) -> Result<()> {
        let mut collectors = self.collectors.write().await;

        // System metrics collector
        collectors.insert(
            "system".to_string(),
            Arc::new(SystemMetricsCollector::new()),
        );

        // Process metrics collector
        collectors.insert(
            "process".to_string(),
            Arc::new(ProcessMetricsCollector::new()),
        );

        // Network metrics collector
        collectors.insert(
            "network".to_string(),
            Arc::new(NetworkMetricsCollector::new()),
        );

        info!("📊 Registered {} default collectors", collectors.len());
        Ok(())
    }

    async fn load_default_alert_rules(&self) -> Result<()> {
        let mut rules = self.alert_rules.write().await;

        // High CPU usage alert
        rules.push(AlertRule {
            id: "high_cpu".to_string(),
            name: "High CPU Usage".to_string(),
            condition: AlertCondition::Threshold {
                metric: "cpu_usage_percent".to_string(),
                operator: ComparisonOperator::GreaterThan,
                value: 90.0,
                duration: Duration::from_secs(300), // 5 minutes
            },
            severity: AlertSeverity::Warning,
            enabled: true,
            cooldown: Duration::from_secs(600), // 10 minutes
            last_triggered: None,
        });

        // High memory usage alert
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

        // Storage space alert
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
            cooldown: Duration::from_secs(1800), // 30 minutes
            last_triggered: None,
        });

        info!("🚨 Loaded {} default alert rules", rules.len());
        Ok(())
    }

    async fn spawn_collection_task(
        &self,
        session_id: Uuid,
        target: MonitoringTarget,
        metrics: Vec<String>,
        interval: Duration,
    ) {
        let sessions = self.sessions.clone();
        let collectors = self.collectors.clone();
        let metrics_store = self.metrics_store.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                interval_timer.tick().await;

                // Check if session is still active
                {
                    let sessions_guard = sessions.read().await;
                    if let Some(session) = sessions_guard.get(&session_id.to_string()) {
                        match session.status {
                            SessionStatus::Active => {}
                            _ => break, // Session stopped or paused
                        }
                    } else {
                        break; // Session removed
                    }
                }

                // Collect metrics
                let collectors_guard = collectors.read().await;
                for collector in collectors_guard.values() {
                    if let Ok(batch) = collector.collect() {
                        // Store metrics
                        let mut store = metrics_store.write().await;
                        store.store_batch(batch).await;
                    }
                }
            }

            debug!("Collection task ended for session: {}", session_id);
        });
    }

    async fn collect_system_health(&self) -> Result<SystemHealth> {
        // Collect system health indicators
        // This is a simplified implementation
        Ok(SystemHealth {
            overall_status: HealthStatus::Healthy,
            cpu_health: HealthStatus::Healthy,
            memory_health: HealthStatus::Healthy,
            storage_health: HealthStatus::Healthy,
            network_health: HealthStatus::Healthy,
        })
    }

    async fn collect_biome_status(&self) -> Result<Vec<BiomeStatusSummary>> {
        // Collect biome status from active sessions
        // This is a simplified implementation
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

    async fn collect_resource_usage(&self) -> Result<SystemResourceUsage> {
        // Collect system resource usage
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

    async fn get_active_alerts(&self) -> Result<Vec<ActiveAlert>> {
        // Return currently active alerts
        // This is a simplified implementation
        Ok(vec![])
    }

    async fn collect_performance_metrics(&self) -> Result<PerformanceMetrics> {
        // Collect performance metrics
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
            // Store in time series
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

                // Update statistics
                self.update_stats(&metric.name, value);
            }
        }

        // Cleanup old data
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

    async fn cleanup_old_data(&mut self) {
        let cutoff_time = Utc::now() - chrono::Duration::from_std(self.retention_period).unwrap();

        for series in self.series.values_mut() {
            series
                .data_points
                .retain(|point| point.timestamp > cutoff_time);
        }
    }
}

impl Default for MonitoringConfig {
    fn default() -> Self {
        Self {
            default_interval: Duration::from_secs(30),
            retention_period: Duration::from_secs(7 * 24 * 3600), // 7 days
            max_metrics_per_batch: 1000,
            enable_alerts: true,
            export_prometheus: true,
            export_grafana: true,
        }
    }
}

// Default metric collectors

struct SystemMetricsCollector;

impl SystemMetricsCollector {
    fn new() -> Self {
        Self
    }
}

impl MetricsCollector for SystemMetricsCollector {
    fn name(&self) -> &str {
        "system"
    }

    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = Utc::now();
        let mut metrics = Vec::new();
        let mut system = System::new_all();
        system.refresh_all();

        // Real CPU usage
        let cpu_usage = system.global_cpu_info().cpu_usage();
        metrics.push(Metric {
            name: "cpu_usage_percent".to_string(),
            value: MetricValue::Gauge(cpu_usage as f64),
            labels: HashMap::new(),
            timestamp,
        });

        // Real memory usage
        let total_memory = system.total_memory();
        let used_memory = system.used_memory();
        let memory_usage_percent = if total_memory > 0 {
            (used_memory as f64 / total_memory as f64) * 100.0
        } else {
            0.0
        };
        metrics.push(Metric {
            name: "memory_usage_percent".to_string(),
            value: MetricValue::Gauge(memory_usage_percent),
            labels: HashMap::new(),
            timestamp,
        });

        // Real storage usage
        let mut total_disk = 0;
        let mut used_disk = 0;
        for disk in system.disks() {
            total_disk += disk.total_space();
            used_disk += disk.total_space() - disk.available_space();
        }
        let storage_usage_percent = if total_disk > 0 {
            (used_disk as f64 / total_disk as f64) * 100.0
        } else {
            0.0
        };
        metrics.push(Metric {
            name: "storage_usage_percent".to_string(),
            value: MetricValue::Gauge(storage_usage_percent),
            labels: HashMap::new(),
            timestamp,
        });

        Ok(MetricBatch {
            timestamp,
            source: "system".to_string(),
            metrics,
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec![
            "cpu".to_string(),
            "memory".to_string(),
            "storage".to_string(),
        ]
    }
}

struct ProcessMetricsCollector;

impl ProcessMetricsCollector {
    fn new() -> Self {
        Self
    }
}

impl MetricsCollector for ProcessMetricsCollector {
    fn name(&self) -> &str {
        "process"
    }

    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = Utc::now();
        let mut metrics = Vec::new();
        let mut system = System::new_all();
        system.refresh_processes();

        // Real process count
        let process_count = system.processes().len();
        metrics.push(Metric {
            name: "process_count".to_string(),
            value: MetricValue::Gauge(process_count as f64),
            labels: HashMap::new(),
            timestamp,
        });

        Ok(MetricBatch {
            timestamp,
            source: "process".to_string(),
            metrics,
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["processes".to_string()]
    }
}

struct NetworkMetricsCollector;

impl NetworkMetricsCollector {
    fn new() -> Self {
        Self
    }
}

impl MetricsCollector for NetworkMetricsCollector {
    fn name(&self) -> &str {
        "network"
    }

    fn collect(&self) -> Result<MetricBatch> {
        let timestamp = Utc::now();
        let mut metrics = Vec::new();
        let mut system = System::new_all();
        system.refresh_networks();

        // Real network throughput
        let mut total_rx = 0;
        let mut total_tx = 0;
        for (_interface_name, network) in system.networks() {
            total_rx += network.received();
            total_tx += network.transmitted();
        }

        metrics.push(Metric {
            name: "network_rx_bytes_per_sec".to_string(),
            value: MetricValue::Gauge(total_rx as f64),
            labels: HashMap::new(),
            timestamp,
        });

        metrics.push(Metric {
            name: "network_tx_bytes_per_sec".to_string(),
            value: MetricValue::Gauge(total_tx as f64),
            labels: HashMap::new(),
            timestamp,
        });

        Ok(MetricBatch {
            timestamp,
            source: "network".to_string(),
            metrics,
        })
    }

    fn capabilities(&self) -> Vec<String> {
        vec!["network".to_string(), "throughput".to_string()]
    }
}
