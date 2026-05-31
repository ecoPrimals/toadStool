// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::interned_strings::socket_env;

/// Base cache configuration
///
/// Provides standard caching parameters that can be used across different
/// caching implementations (DNS, HTTP, data caching, etc.).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable caching
    #[serde(default = "default_cache_enabled")]
    pub enabled: bool,

    /// Cache entry TTL (time-to-live)
    #[serde(default = "default_cache_ttl", with = "humantime_serde")]
    pub ttl: Duration,

    /// Maximum number of cache entries
    #[serde(default = "default_max_cache_entries")]
    pub max_entries: u32,

    /// Negative cache TTL (for failed lookups)
    #[serde(default = "default_negative_cache_ttl", with = "humantime_serde")]
    pub negative_ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            ttl: default_cache_ttl(),
            max_entries: default_max_cache_entries(),
            negative_ttl: default_negative_cache_ttl(),
        }
    }
}

const fn default_cache_enabled() -> bool {
    true
}

const fn default_cache_ttl() -> Duration {
    Duration::from_secs(300) // 5 minutes
}

const fn default_max_cache_entries() -> u32 {
    1000
}

const fn default_negative_cache_ttl() -> Duration {
    Duration::from_secs(60) // 1 minute
}

/// Telemetry configuration for metrics, tracing, and logging
///
/// This provides a standard set of observability settings that can be used
/// across different services for consistent monitoring and debugging.
///
/// # Example
///
/// ```rust
/// use toadstool_common::config_bases::TelemetryConfig;
///
/// let config = TelemetryConfig {
///     metrics_enabled: true,
///     tracing_enabled: true,
///     access_logs: true,
///     metrics_port: 9090,
///     tracing_endpoint: Some("http://jaeger:14268".to_string()),
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryConfig {
    /// Enable metrics collection
    pub metrics_enabled: bool,

    /// Enable distributed tracing
    pub tracing_enabled: bool,

    /// Enable access logs
    pub access_logs: bool,

    /// Metrics port
    pub metrics_port: u16,

    /// Tracing export endpoint (e.g., Jaeger)
    pub tracing_endpoint: Option<String>,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        // Sovereignty principle: all data collection is opt-in, never opt-out.
        // Operators explicitly enable telemetry via config or TOADSTOOL_TELEMETRY=1.
        let opt_in = std::env::var(socket_env::TOADSTOOL_TELEMETRY)
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        Self {
            metrics_enabled: opt_in,
            tracing_enabled: opt_in,
            access_logs: opt_in,
            metrics_port: default_metrics_port(),
            tracing_endpoint: None,
        }
    }
}

const fn default_metrics_port() -> u16 {
    9090
}
