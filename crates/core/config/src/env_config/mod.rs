//! Environment Variable Configuration System
//!
//! Provides comprehensive environment variable support to eliminate all hardcoded
//! values from the ToadStool codebase. Uses type-safe parsing with fallback defaults.
//!
//! # Zero-Copy
//!
//! `EnvConfigLoader` uses `Cow<'static, str>` for the prefix so the common
//! "TOADSTOOL" prefix never allocates.

mod domains;
mod loader;
mod network;

#[cfg(test)]
pub(crate) mod tests;

use std::env;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolConfig;

pub use domains::{MonitoringEnvConfig, ResourceEnvConfig, SecurityEnvConfig};
pub use loader::EnvConfigLoader;
pub use network::NetworkEnvConfig;

/// Comprehensive environment configuration — aggregates all domain configs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    pub network: NetworkEnvConfig,
    pub resources: ResourceEnvConfig,
    pub monitoring: MonitoringEnvConfig,
    pub security: SecurityEnvConfig,
    pub environment: String,
    pub debug: bool,
    pub verbose: bool,
    pub data_dir: PathBuf,
    pub cache_dir: PathBuf,
    pub temp_dir: PathBuf,
}

impl EnvironmentConfig {
    /// Load all environment configuration.
    #[must_use]
    pub fn from_env() -> Self {
        let loader = EnvConfigLoader::new();
        Self {
            network: NetworkEnvConfig::from_env(),
            resources: ResourceEnvConfig::from_env(),
            monitoring: MonitoringEnvConfig::from_env(),
            security: SecurityEnvConfig::from_env(),
            environment: loader.get_string("ENV", "development"),
            debug: loader.get_bool("DEBUG", false),
            verbose: loader.get_bool("VERBOSE", false),
            data_dir: loader.get_path("DATA_DIR", "./data"),
            cache_dir: loader.get_path("CACHE_DIR", "./cache"),
            temp_dir: loader.get_path("TEMP_DIR", "./tmp"),
        }
    }

    /// Apply environment configuration to an existing `ToadStoolConfig`.
    pub fn apply_to_config(&self, config: &mut ToadStoolConfig) {
        if let Ok(addr) = format!(
            "{}:{}",
            self.network.bind_address, self.network.toadstool_port
        )
        .parse()
        {
            config.network.bind_address = addr;
        }

        #[allow(deprecated)]
        {
            config.network.endpoints.songbird = self.network.songbird_endpoint();
            config.network.endpoints.beardog = self.network.beardog_endpoint();
            config.network.endpoints.nestgate = self.network.nestgate_endpoint();
            config.network.endpoints.squirrel = self.network.squirrel_endpoint();
        }

        config.network.connection.request_timeout =
            Duration::from_secs(self.network.request_timeout_secs);
        config.network.connection.connection_timeout =
            Duration::from_secs(self.network.connection_timeout_secs);
        config.network.connection.max_retries = self.network.max_retries;
        config.network.connection.max_connections_per_host = self.network.max_connections_per_host;

        config.runtime.resource_limits.max_cpu_usage = self.resources.max_cpu_percent;
        config.runtime.resource_limits.max_memory_usage = self.resources.max_memory_bytes as f64;
        config.app.worker_threads = self.resources.worker_threads as usize;
        config.app.queue_size = self.resources.queue_size as usize;
        config.app.batch_size = self.resources.batch_size as usize;

        config.logging.level = self.monitoring.log_level.clone();

        config.security.auth.enabled = self.security.auth_enabled;
        config.security.sandbox.enabled = self.security.sandboxing_enabled;

        config.app.environment = self.environment.clone();
        config.app.data_dir = self.data_dir.to_string_lossy().to_string();
        config.app.cache_dir = self.cache_dir.to_string_lossy().to_string();
        config.app.temp_dir = self.temp_dir.to_string_lossy().to_string();

        debug!("Applied environment configuration to ToadStool config");
    }
}

// ── Module-level helpers ──────────────────────────────────────────────────────

/// Get an environment variable with a custom prefix.
#[must_use]
pub fn get_env_with_prefix(prefix: &str, key: &str, default: &str) -> String {
    let env_key = format!("{prefix}_{key}");
    env::var(&env_key).unwrap_or_else(|_| default.to_string())
}

/// Get an environment variable as `bool`.
#[must_use]
pub fn get_env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(default)
}

/// Get an environment variable as any type that implements `FromStr`.
pub fn get_env_number<T: FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
        .unwrap_or(default)
}

/// Get an environment variable as a `Duration` (value in whole seconds).
#[must_use]
pub fn get_env_duration(key: &str, default: Duration) -> Duration {
    env::var(key)
        .and_then(|v| {
            v.parse::<u64>()
                .map(Duration::from_secs)
                .map_err(|_| env::VarError::NotPresent)
        })
        .unwrap_or(default)
}

/// Load the global environment configuration.
pub fn load_global_env_config() -> EnvironmentConfig {
    let config = EnvironmentConfig::from_env();
    debug!(
        "Loaded global environment configuration for {} environment",
        config.environment
    );
    config
}

/// Apply environment overrides to an existing `ToadStool` config.
pub fn apply_env_config(config: &mut ToadStoolConfig) {
    let env_config = EnvironmentConfig::from_env();
    env_config.apply_to_config(config);
    debug!("Applied environment configuration overrides");
}
