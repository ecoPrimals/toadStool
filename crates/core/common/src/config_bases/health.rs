// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Base health check configuration
///
/// Provides standard health checking parameters that can be used
/// across HTTP, TCP, gRPC, or other health check implementations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckConfig {
    /// Enable health checks
    #[serde(default = "crate::config_bases::serde_defaults::default_true")]
    pub enabled: bool,

    /// Interval between health checks
    #[serde(default = "default_health_check_interval", with = "humantime_serde")]
    pub interval: Duration,

    /// Timeout for each health check
    #[serde(default = "default_health_check_timeout", with = "humantime_serde")]
    pub timeout: Duration,

    /// Number of consecutive successful checks to mark as healthy
    #[serde(default = "default_healthy_threshold")]
    pub healthy_threshold: u32,

    /// Number of consecutive failed checks to mark as unhealthy
    #[serde(default = "default_unhealthy_threshold")]
    pub unhealthy_threshold: u32,

    /// Number of retries on failure before marking unhealthy
    #[serde(default = "default_retry_count")]
    pub retry_count: u32,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval: default_health_check_interval(),
            timeout: default_health_check_timeout(),
            healthy_threshold: default_healthy_threshold(),
            unhealthy_threshold: default_unhealthy_threshold(),
            retry_count: default_retry_count(),
        }
    }
}

const fn default_health_check_interval() -> Duration {
    Duration::from_secs(30)
}

const fn default_health_check_timeout() -> Duration {
    Duration::from_secs(10)
}

const fn default_healthy_threshold() -> u32 {
    2
}

const fn default_unhealthy_threshold() -> u32 {
    3
}

const fn default_retry_count() -> u32 {
    3
}

/// HTTP-specific health check configuration
///
/// Extends the base health check with HTTP-specific parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HttpHealthCheckConfig {
    /// Base health check configuration
    #[serde(flatten)]
    pub base: HealthCheckConfig,

    /// HTTP path to check
    #[serde(default = "default_health_path")]
    pub path: String,

    /// Expected HTTP status code (default: 200)
    #[serde(default = "default_http_status")]
    pub expected_status: u16,

    /// Optional HTTP method (default: GET)
    #[serde(default = "default_http_method")]
    pub method: String,
}

impl Default for HttpHealthCheckConfig {
    fn default() -> Self {
        Self {
            base: HealthCheckConfig::default(),
            path: default_health_path(),
            expected_status: default_http_status(),
            method: default_http_method(),
        }
    }
}

fn default_health_path() -> String {
    "/health".to_string()
}

const fn default_http_status() -> u16 {
    200
}

fn default_http_method() -> String {
    "GET".to_string()
}
