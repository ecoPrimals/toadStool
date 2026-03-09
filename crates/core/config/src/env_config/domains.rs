// SPDX-License-Identifier: AGPL-3.0-only
//! Resource, monitoring and security environment configurations.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::loader::EnvConfigLoader;

/// Resource limits loaded from environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEnvConfig {
    pub max_cpu_percent: f64,
    pub max_memory_bytes: u64,
    pub max_storage_bytes: u64,
    pub max_network_mbps: f64,
    pub max_gpu_percent: f64,
    pub max_concurrent_executions: u32,
    pub worker_threads: u32,
    pub queue_size: u32,
    pub batch_size: u32,
}

impl ResourceEnvConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let loader = EnvConfigLoader::new();
        Self {
            max_cpu_percent: loader.get_f64("MAX_CPU_PERCENT", 90.0),
            max_memory_bytes: loader.get_u64("MAX_MEMORY_BYTES", 8 * 1024 * 1024 * 1024),
            max_storage_bytes: loader.get_u64("MAX_STORAGE_BYTES", 100 * 1024 * 1024 * 1024),
            max_network_mbps: loader.get_f64("MAX_NETWORK_MBPS", 1000.0),
            max_gpu_percent: loader.get_f64("MAX_GPU_PERCENT", 95.0),
            max_concurrent_executions: loader.get_u32("MAX_CONCURRENT_EXECUTIONS", 100),
            worker_threads: loader.get_u32(
                "WORKER_THREADS",
                std::thread::available_parallelism()
                    .map(|n| u32::try_from(n.get()).unwrap_or(4))
                    .unwrap_or(4),
            ),
            queue_size: loader.get_u32("QUEUE_SIZE", 10000),
            batch_size: loader.get_u32("BATCH_SIZE", 1000),
        }
    }
}

/// Monitoring and observability configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct MonitoringEnvConfig {
    pub metrics_enabled: bool,
    pub metrics_interval_secs: u64,
    pub metrics_retention_days: u64,
    pub health_checks_enabled: bool,
    pub health_check_interval_secs: u64,
    pub logging_enabled: bool,
    pub log_level: String,
    pub log_dir: PathBuf,
    pub alerts_enabled: bool,
    pub cpu_alert_threshold: f64,
    pub memory_alert_threshold: f64,
    pub storage_alert_threshold: f64,
}

impl MonitoringEnvConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let loader = EnvConfigLoader::new();
        Self {
            metrics_enabled: loader.get_bool("METRICS_ENABLED", true),
            metrics_interval_secs: loader.get_u64("METRICS_INTERVAL_SECS", 10),
            metrics_retention_days: loader.get_u64("METRICS_RETENTION_DAYS", 7),
            health_checks_enabled: loader.get_bool("HEALTH_CHECKS_ENABLED", true),
            health_check_interval_secs: loader.get_u64("HEALTH_CHECK_INTERVAL_SECS", 30),
            logging_enabled: loader.get_bool("LOGGING_ENABLED", true),
            log_level: loader.get_string("LOG_LEVEL", "info"),
            log_dir: loader.get_path("LOG_DIR", "./logs"),
            alerts_enabled: loader.get_bool("ALERTS_ENABLED", false),
            cpu_alert_threshold: loader.get_f64("CPU_ALERT_THRESHOLD", 85.0),
            memory_alert_threshold: loader.get_f64("MEMORY_ALERT_THRESHOLD", 90.0),
            storage_alert_threshold: loader.get_f64("STORAGE_ALERT_THRESHOLD", 95.0),
        }
    }
}

/// Security and auth configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SecurityEnvConfig {
    pub auth_enabled: bool,
    pub auth_token_expiry_secs: u64,
    pub sandboxing_enabled: bool,
    pub isolation_level: String,
    pub encryption_enabled: bool,
    pub encryption_key_path: PathBuf,
    pub rate_limiting_enabled: bool,
    pub rate_limit_rps: u32,
    pub rate_limit_burst: u32,
    pub cors_enabled: bool,
    pub cors_allowed_origins: Vec<String>,
}

impl SecurityEnvConfig {
    #[must_use]
    pub fn from_env() -> Self {
        let loader = EnvConfigLoader::new();
        let cors_origins = loader
            .get_string("CORS_ALLOWED_ORIGINS", "*")
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();

        Self {
            auth_enabled: loader.get_bool("AUTH_ENABLED", false),
            auth_token_expiry_secs: loader.get_u64("AUTH_TOKEN_EXPIRY_SECS", 3600),
            sandboxing_enabled: loader.get_bool("SANDBOXING_ENABLED", true),
            isolation_level: loader.get_string("ISOLATION_LEVEL", "Standard"),
            encryption_enabled: loader.get_bool("ENCRYPTION_ENABLED", false),
            encryption_key_path: loader.get_path("ENCRYPTION_KEY_PATH", "./keys/encryption.key"),
            rate_limiting_enabled: loader.get_bool("RATE_LIMITING_ENABLED", false),
            rate_limit_rps: loader.get_u32("RATE_LIMIT_RPS", 100),
            rate_limit_burst: loader.get_u32("RATE_LIMIT_BURST", 1000),
            cors_enabled: loader.get_bool("CORS_ENABLED", true),
            cors_allowed_origins: cors_origins,
        }
    }
}
