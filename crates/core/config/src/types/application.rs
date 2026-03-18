// SPDX-License-Identifier: AGPL-3.0-or-later
//! Application lifecycle configuration
//!
//! This module contains configuration types for application-level settings including:
//! - Application identity (name, version, environment)
//! - Directory paths (data, cache, logs, temp)
//! - Thread pool settings
//! - Queue and batch processing
//! - Graceful shutdown

use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::app;

/// Application configuration
///
/// Controls application-level settings including identity, directories,
/// threading, and lifecycle management.
///
/// # Example
/// ```
/// use toadstool_config::types::ApplicationConfig;
///
/// let config = ApplicationConfig::default();
/// assert_eq!(config.name, "toadstool");
/// assert_eq!(config.environment, "development");
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// Application name
    pub name: String,

    /// Application version
    pub version: String,

    /// Environment (development, staging, production)
    pub environment: String,

    /// Data directory for persistent application data
    pub data_dir: String,

    /// Cache directory for temporary cached data
    pub cache_dir: String,

    /// Logs directory for application logs
    pub logs_dir: String,

    /// Temporary directory for ephemeral data
    pub temp_dir: String,

    /// Worker thread count for thread pool
    pub worker_threads: usize,

    /// Queue size for work items
    pub queue_size: usize,

    /// Batch processing size
    pub batch_size: usize,

    /// Graceful shutdown timeout
    pub shutdown_timeout: Duration,
}

impl Default for ApplicationConfig {
    fn default() -> Self {
        Self {
            name: app::DEFAULT_APP_NAME.to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            environment: app::DEFAULT_ENVIRONMENT.to_string(),
            data_dir: app::DEFAULT_DATA_DIR.to_string(),
            cache_dir: app::DEFAULT_CACHE_DIR.to_string(),
            logs_dir: app::DEFAULT_LOGS_DIR.to_string(),
            temp_dir: app::DEFAULT_TEMP_DIR.to_string(),
            worker_threads: app::DEFAULT_WORKER_THREADS,
            queue_size: app::DEFAULT_QUEUE_SIZE,
            batch_size: app::DEFAULT_BATCH_SIZE,
            shutdown_timeout: Duration::from_secs(app::DEFAULT_SHUTDOWN_TIMEOUT_SECS),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_application_config() {
        let config = ApplicationConfig::default();
        assert_eq!(config.name, app::DEFAULT_APP_NAME);
        assert_eq!(config.environment, app::DEFAULT_ENVIRONMENT);
        assert!(config.worker_threads > 0);
        assert!(config.queue_size > 0);
    }

    #[test]
    fn test_application_config_serialization() {
        let config = ApplicationConfig::default();
        let serialized = toml::to_string(&config).unwrap();
        let deserialized: ApplicationConfig = toml::from_str(&serialized).unwrap();
        assert_eq!(config.name, deserialized.name);
        assert_eq!(config.version, deserialized.version);
    }
}
