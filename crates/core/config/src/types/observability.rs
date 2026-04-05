// SPDX-License-Identifier: AGPL-3.0-or-later
//! Observability configuration
//!
//! This module contains configuration types for observability including:
//! - Logging (levels, formats, rotation)
//! - Metrics (Prometheus, collection intervals)
//! - Database persistence
//! - Distributed caching

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::app;

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "bool fields map directly to hardware flags"
)]
pub struct LoggingConfig {
    /// Log level (trace, debug, info, warn, error)
    pub level: String,

    /// Log format (json, pretty, compact)
    pub format: String,

    /// Log to file in addition to stdout
    pub log_to_file: bool,

    /// Log file path
    pub log_file: String,

    /// Enable log file rotation
    pub log_rotation: bool,

    /// Maximum log file size in bytes
    pub max_log_size: u64,

    /// Maximum number of rotated log files
    pub max_log_files: u32,

    /// Enable colored output
    pub enable_colors: bool,

    /// Enable timestamps in logs
    pub enable_timestamps: bool,

    /// Enable thread IDs in logs
    pub enable_thread_ids: bool,

    /// Enable module paths in logs
    pub enable_module_paths: bool,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: app::DEFAULT_LOG_LEVEL.to_string(),
            format: "pretty".to_string(),
            log_to_file: false,
            log_file: "toadstool.log".to_string(),
            log_rotation: true,
            max_log_size: app::DEFAULT_MAX_LOG_SIZE,
            max_log_files: app::DEFAULT_MAX_LOG_FILES,
            enable_colors: true,
            enable_timestamps: true,
            enable_thread_ids: false,
            enable_module_paths: false,
        }
    }
}

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "bool fields map directly to hardware flags"
)]
pub struct MetricsConfig {
    /// Enable metrics collection
    pub enabled: bool,

    /// Metrics endpoint URL
    pub endpoint: String,

    /// Metrics format (prometheus, json, statsd)
    pub format: String,

    /// Collection interval
    pub collection_interval: Duration,

    /// Retention period for metrics
    pub retention_period: Duration,

    /// Enable histogram metrics
    pub enable_histograms: bool,

    /// Enable counter metrics
    pub enable_counters: bool,

    /// Enable gauge metrics
    pub enable_gauges: bool,
}

impl Default for MetricsConfig {
    fn default() -> Self {
        let config = crate::env_config::EnvironmentConfig::from_env();
        Self {
            enabled: true,
            endpoint: format!(
                "http://{}:{}/metrics",
                config.network.bind_address, config.network.metrics_port
            ),
            format: "prometheus".to_string(),
            collection_interval: Duration::from_secs(app::DEFAULT_METRICS_INTERVAL_SECS),
            retention_period: Duration::from_secs(3600 * 24 * 7), // 7 days
            enable_histograms: true,
            enable_counters: true,
            enable_gauges: true,
        }
    }
}

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database connection URL
    pub url: String,

    /// Database type (postgres, mysql, sqlite)
    pub database_type: String,

    /// Maximum connection pool size
    pub max_connections: u32,

    /// Connection timeout
    pub connection_timeout: Duration,

    /// Query timeout
    pub query_timeout: Duration,

    /// Enable automatic migrations
    pub enable_migrations: bool,

    /// Migration directory path
    pub migration_dir: String,
}

/// Backend cache configuration
///
/// Configuration for distributed caching systems (Redis, Memcached).
/// This is distinct from simple in-memory caching.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCacheConfig {
    /// Cache backend type (redis, memcached, memory)
    pub cache_type: String,

    /// Cache backend URL (for distributed caches)
    pub url: Option<String>,

    /// Maximum cache size in bytes
    pub max_size: u64,

    /// Time-to-live for cached entries
    pub ttl: Duration,

    /// Enable compression of cached data
    pub enable_compression: bool,

    /// Compression algorithm (gzip, lz4, zstd)
    pub compression_algorithm: String,
}

impl Default for BackendCacheConfig {
    fn default() -> Self {
        Self {
            cache_type: "memory".to_string(),
            url: None,
            max_size: app::DEFAULT_CACHE_MAX_SIZE,
            ttl: Duration::from_secs(app::DEFAULT_CACHE_TTL_SECS),
            enable_compression: false,
            compression_algorithm: "gzip".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_logging_config() {
        let config = LoggingConfig::default();
        assert_eq!(config.level, app::DEFAULT_LOG_LEVEL);
        assert!(config.enable_colors);
        assert!(config.log_rotation);
    }

    #[test]
    fn test_default_metrics_config() {
        let config = MetricsConfig::default();
        assert!(config.enabled);
        assert_eq!(config.format, "prometheus");
        assert!(config.enable_histograms);
    }

    #[test]
    fn test_default_cache_config() {
        let config = BackendCacheConfig::default();
        assert_eq!(config.cache_type, "memory");
        assert!(config.max_size > 0);
    }
}
