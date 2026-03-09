// SPDX-License-Identifier: AGPL-3.0-only

//! Circuit breaker and health monitoring configuration types.

use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use toadstool_common::config_bases::HttpHealthCheckConfig;

/// Circuit breaker configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CircuitBreakerConfig {
    /// Enable circuit breaker
    pub enabled: bool,
    /// Failure threshold
    pub failure_threshold: u32,
    /// Success threshold
    pub success_threshold: u32,
    /// Timeout duration
    pub timeout: Duration,
    /// Half-open timeout
    pub half_open_timeout: Duration,
    /// Reset timeout
    pub reset_timeout: Duration,
}

/// Health monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthMonitoringConfig {
    /// Enable health monitoring
    pub enabled: bool,
    /// Monitoring interval
    pub interval: Duration,
    /// Health check endpoints
    pub endpoints: Vec<HealthEndpoint>,
    /// Alerting configuration
    pub alerting: AlertingConfig,
    /// Metrics collection
    pub metrics: MetricsConfig,
}

/// Health endpoint configuration
///
/// Composes HTTP health check configuration with an endpoint name and URL.
/// Uses base `HttpHealthCheckConfig` for consistent health checking across the system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthEndpoint {
    /// Endpoint name
    pub name: String,
    /// Endpoint URL
    pub url: String,
    /// HTTP health check configuration (includes timeout, retries, status checks)
    #[serde(flatten)]
    pub health_check: HttpHealthCheckConfig,
}

/// Alerting configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertingConfig {
    /// Enable alerting
    pub enabled: bool,
    /// Alert channels
    pub channels: Vec<AlertChannel>,
    /// Alert rules
    pub rules: Vec<AlertRule>,
}

/// Alert channel
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertChannel {
    /// Channel name
    pub name: String,
    /// Channel type (email, slack, webhook)
    pub channel_type: String,
    /// Channel configuration
    pub config: HashMap<String, serde_json::Value>,
}

/// Alert rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertRule {
    /// Rule name
    pub name: String,
    /// Rule condition
    pub condition: String,
    /// Severity level
    pub severity: String,
    /// Target channels
    pub channels: Vec<String>,
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics
    pub enabled: bool,
    /// Metrics endpoint
    pub endpoint: String,
    /// Collection interval
    pub interval: Duration,
    /// Metrics exporters
    pub exporters: Vec<MetricsExporter>,
}

/// Metrics exporter
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsExporter {
    /// Exporter type (prometheus, influx, datadog)
    pub exporter_type: String,
    /// Exporter configuration
    pub config: HashMap<String, serde_json::Value>,
    /// Enabled
    pub enabled: bool,
}
