// SPDX-License-Identifier: AGPL-3.0-only
//! Monitoring types - structs and enums for metrics, alerts, and dashboards

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::time::Duration;
use uuid::Uuid;

/// Monitoring session for a specific biome or system
#[derive(Debug, Clone)]
pub struct MonitoringSession {
    pub id: Uuid,
    pub target: MonitoringTarget,
    pub started: std::time::SystemTime,
    pub interval: Duration,
    pub metrics: Vec<String>,
    pub status: SessionStatus,
    pub last_update: std::time::SystemTime,
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

/// Batch of collected metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricBatch {
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
    pub source: String,
    pub metrics: Vec<Metric>,
}

/// Individual metric data point
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: MetricValue,
    pub labels: HashMap<String, String>,
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
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
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "toadstool_common::system_time_serde::opt"
    )]
    pub last_triggered: Option<std::time::SystemTime>,
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

#[derive(Debug, Clone)]
pub struct TimeSeries {
    pub name: String,
    pub data_points: Vec<DataPoint>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataPoint {
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,
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
    #[serde(with = "toadstool_common::system_time_serde")]
    pub triggered_at: std::time::SystemTime,
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
