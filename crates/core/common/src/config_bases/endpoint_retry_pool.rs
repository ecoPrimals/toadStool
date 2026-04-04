// SPDX-License-Identifier: AGPL-3.0-only

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Base backend endpoint configuration
///
/// Represents a network endpoint (address, port) that can be used
/// for load balancers, discovery backends, policy engines, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendEndpoint {
    /// Endpoint name or identifier
    pub name: String,

    /// Network address (hostname or IP)
    pub address: String,

    /// Network port
    pub port: u16,

    /// Whether this endpoint is enabled
    #[serde(default = "crate::config_bases::serde_defaults::default_true")]
    pub enabled: bool,
}

impl BackendEndpoint {
    /// Create a new backend endpoint
    #[must_use]
    pub fn new(name: impl Into<String>, address: impl Into<String>, port: u16) -> Self {
        Self {
            name: name.into(),
            address: address.into(),
            port,
            enabled: true,
        }
    }

    /// Get the full URL for this endpoint
    #[must_use]
    pub fn url(&self, scheme: &str) -> String {
        format!("{}://{}:{}", scheme, self.address, self.port)
    }
}

/// Retry configuration with exponential backoff
///
/// Provides a common retry strategy that can be used across
/// different service integrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryConfig {
    /// Maximum number of retries
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,

    /// Initial/base delay between retries
    #[serde(default = "default_base_delay", with = "humantime_serde")]
    pub base_delay: Duration,

    /// Maximum delay between retries
    #[serde(default = "default_max_delay", with = "humantime_serde")]
    pub max_delay: Duration,

    /// Backoff multiplier (e.g., 2.0 for exponential)
    #[serde(default = "default_backoff_multiplier")]
    pub backoff_multiplier: f64,

    /// Jitter percentage (0-100) to add randomness
    #[serde(default = "default_jitter_percent")]
    pub jitter_percent: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: default_max_retries(),
            base_delay: default_base_delay(),
            max_delay: default_max_delay(),
            backoff_multiplier: default_backoff_multiplier(),
            jitter_percent: default_jitter_percent(),
        }
    }
}

const fn default_max_retries() -> u32 {
    3
}

const fn default_base_delay() -> Duration {
    Duration::from_millis(100)
}

const fn default_max_delay() -> Duration {
    Duration::from_secs(30)
}

const fn default_backoff_multiplier() -> f64 {
    2.0
}

const fn default_jitter_percent() -> f64 {
    10.0
}

/// Connection pooling configuration
///
/// Provides standard connection pool settings for HTTP clients and services.
/// This is a common pattern across networking layers and service integrations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionPoolConfig {
    /// Enable connection pooling
    #[serde(default = "default_pool_enabled")]
    pub enabled: bool,

    /// Maximum connections per host
    #[serde(default = "default_max_connections_per_host")]
    pub max_connections_per_host: u32,

    /// Maximum idle connections
    #[serde(default = "default_max_idle_connections")]
    pub max_idle_connections: u32,

    /// Idle connection timeout
    #[serde(default = "default_idle_connection_timeout", with = "humantime_serde")]
    pub idle_timeout: Duration,

    /// Connection lifetime
    #[serde(default = "default_connection_lifetime", with = "humantime_serde")]
    pub connection_lifetime: Duration,
}

impl Default for ConnectionPoolConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_connections_per_host: default_max_connections_per_host(),
            max_idle_connections: default_max_idle_connections(),
            idle_timeout: default_idle_connection_timeout(),
            connection_lifetime: default_connection_lifetime(),
        }
    }
}

const fn default_pool_enabled() -> bool {
    true
}

const fn default_max_connections_per_host() -> u32 {
    100
}

const fn default_max_idle_connections() -> u32 {
    10
}

const fn default_idle_connection_timeout() -> Duration {
    Duration::from_secs(300)
}

const fn default_connection_lifetime() -> Duration {
    Duration::from_secs(3600)
}
