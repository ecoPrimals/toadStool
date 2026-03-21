// SPDX-License-Identifier: AGPL-3.0-only
//! Monitoring types - structs and enums for metrics, alerts, and dashboards

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Duration;
use uuid::Uuid;

/// Monitoring session for a specific biome or system
#[derive(Debug, Clone)]
pub struct MonitoringSession {
    /// Unique session ID
    pub id: Uuid,
    /// What is being monitored (biome, service, system, etc.)
    pub target: MonitoringTarget,
    /// When the session started
    pub started: std::time::SystemTime,
    /// Collection interval
    pub interval: Duration,
    /// Metric names to collect
    pub metrics: Vec<String>,
    /// Session state
    pub status: SessionStatus,
    /// Last metrics update time
    pub last_update: std::time::SystemTime,
}

/// Target of monitoring (biome, service, system, platform, federation)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MonitoringTarget {
    /// Monitor a biome by name
    Biome(String),
    /// Monitor a service (biome_name, service_name)
    Service(String, String),
    /// Monitor the host system
    System,
    /// Monitor a platform by name
    Platform(String),
    /// Monitor federation-wide metrics
    Federation,
}

/// State of a monitoring session
#[derive(Debug, Clone)]
pub enum SessionStatus {
    /// Actively collecting metrics
    Active,
    /// Paused (no collection)
    Paused,
    /// Session stopped
    Stopped,
    /// Error with message
    Error(String),
}

/// Batch of collected metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBatch {
    /// When the batch was collected
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    /// Source identifier (biome name, hostname, etc.)
    pub source: String,
    /// Individual metrics in the batch
    pub metrics: Vec<Metric>,
}

/// Individual metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    /// Metric name (e.g. cpu_usage, memory_bytes)
    pub name: String,
    /// Metric value (counter, gauge, histogram, text)
    pub value: MetricValue,
    /// Labels for dimensional queries
    pub labels: HashMap<String, String>,
    /// When the metric was sampled
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
}

/// Value type for a metric (counter, gauge, histogram, text)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MetricValue {
    /// Monotonically increasing counter
    Counter(u64),
    /// Instantaneous gauge value
    Gauge(f64),
    /// Distribution of values (for percentiles)
    Histogram(Vec<f64>),
    /// Free-form text
    Text(String),
}

/// Alert rule configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Unique rule ID
    pub id: String,
    /// Human-readable rule name
    pub name: String,
    /// Condition that triggers the alert
    pub condition: AlertCondition,
    /// Severity when triggered
    pub severity: AlertSeverity,
    /// Whether the rule is active
    pub enabled: bool,
    /// Minimum time between repeated alerts
    pub cooldown: Duration,
    /// Last time the alert fired
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    pub last_triggered: Option<std::time::SystemTime>,
}

/// Condition that triggers an alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    /// Metric crosses a threshold for a duration
    Threshold {
        /// Metric name
        metric: String,
        /// Comparison operator
        operator: ComparisonOperator,
        /// Threshold value
        value: f64,
        /// How long the condition must hold
        duration: Duration,
    },
    /// Rate of change exceeds threshold
    RateOfChange {
        /// Metric name
        metric: String,
        /// Rate threshold
        threshold: f64,
        /// Time window for rate calculation
        window: Duration,
    },
    /// Combine multiple conditions with AND/OR
    Composite {
        /// Sub-conditions
        conditions: Vec<Self>,
        /// Logical operator
        operator: LogicalOperator,
    },
}

/// Comparison operator for threshold alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ComparisonOperator {
    /// >
    GreaterThan,
    /// <
    LessThan,
    /// ==
    Equal,
    /// !=
    NotEqual,
    /// >=
    GreaterThanOrEqual,
    /// <=
    LessThanOrEqual,
}

/// Logical operator for composite alert conditions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    /// All conditions must be true
    And,
    /// Any condition can be true
    Or,
}

/// Severity level for alerts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertSeverity {
    /// Informational
    Info,
    /// Warning (attention needed)
    Warning,
    /// Critical (immediate action)
    Critical,
    /// Emergency (system-wide)
    Emergency,
}

/// Monitoring configuration
#[derive(Debug, Clone)]
pub struct MonitoringConfig {
    /// Default collection interval
    pub default_interval: Duration,
    /// How long to retain metrics
    pub retention_period: Duration,
    /// Max metrics per batch (for batching)
    pub max_metrics_per_batch: usize,
    /// Whether alert rules are evaluated
    pub enable_alerts: bool,
    /// Export metrics to Prometheus
    pub export_prometheus: bool,
    /// Export dashboards to Grafana
    pub export_grafana: bool,
}

/// Time series of metric data points
#[derive(Debug, Clone)]
pub struct TimeSeries {
    /// Metric name
    pub name: String,
    /// Ordered (timestamp, value) pairs
    pub data_points: Vec<DataPoint>,
    /// Labels for the series
    pub labels: HashMap<String, String>,
}

/// Single (timestamp, value) point in a time series
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    /// When the value was sampled
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    /// Numeric value
    pub value: f64,
}

/// Statistical summary of a metric (min, max, avg, percentiles)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricStats {
    /// Number of samples
    pub count: u64,
    /// Minimum value
    pub min: f64,
    /// Maximum value
    pub max: f64,
    /// Average value
    pub avg: f64,
    /// Percentiles (e.g. p50, p95, p99)
    pub percentiles: HashMap<String, f64>,
}

/// Real-time dashboard data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardData {
    /// When the dashboard was generated
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    /// Overall system health by component
    pub system_health: SystemHealth,
    /// Status of each biome
    pub biome_status: Vec<BiomeStatusSummary>,
    /// System-wide resource usage
    pub resource_usage: SystemResourceUsage,
    /// Currently active alerts
    pub alerts: Vec<ActiveAlert>,
    /// Performance metrics (latency, throughput, etc.)
    pub performance_metrics: PerformanceMetrics,
}

/// Health status of system components
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    /// Overall aggregated status
    pub overall_status: HealthStatus,
    /// CPU health
    pub cpu_health: HealthStatus,
    /// Memory health
    pub memory_health: HealthStatus,
    /// Storage health
    pub storage_health: HealthStatus,
    /// Network health
    pub network_health: HealthStatus,
}

/// Health status level
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Component is healthy
    Healthy,
    /// Degraded or approaching limits
    Warning,
    /// Critical failure
    Critical,
    /// Status unknown
    Unknown,
}

/// Summary of a biome's status for the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStatusSummary {
    /// Biome name
    pub name: String,
    /// Current status (running, stopped, etc.)
    pub status: String,
    /// Number of services running
    pub services_running: u32,
    /// Total number of services
    pub services_total: u32,
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage percentage
    pub memory_usage: f64,
    /// How long the biome has been running
    pub uptime: Duration,
}

/// System-wide resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResourceUsage {
    /// CPU utilization percentage
    pub cpu_percent: f64,
    /// Memory used in GB
    pub memory_used_gb: f64,
    /// Total memory in GB
    pub memory_total_gb: f64,
    /// Storage used in GB
    pub storage_used_gb: f64,
    /// Total storage in GB
    pub storage_total_gb: f64,
    /// Network receive rate in Mbps
    pub network_rx_mbps: f64,
    /// Network transmit rate in Mbps
    pub network_tx_mbps: f64,
    /// Load average (1min, 5min, 15min)
    pub load_average: Vec<f64>,
}

/// Currently firing alert
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveAlert {
    /// Alert instance ID
    pub id: String,
    /// Name of the rule that fired
    pub rule_name: String,
    /// Severity level
    pub severity: AlertSeverity,
    /// Human-readable message
    pub message: String,
    /// When the alert was triggered
    #[serde(with = "toadstool_common::system_time_serde")]
    pub triggered_at: std::time::SystemTime,
    /// Target (biome, service, system)
    pub target: String,
}

/// Performance metrics for the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceMetrics {
    /// Average execution latency in milliseconds
    pub execution_latency_ms: f64,
    /// Throughput in operations per second
    pub throughput_ops_sec: f64,
    /// Error rate (0.0–1.0)
    pub error_rate: f64,
    /// Success rate (0.0–1.0)
    pub success_rate: f64,
    /// Current queue depth
    pub queue_depth: u32,
}
