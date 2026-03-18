// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource, monitoring and security environment configurations.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::loader::EnvConfigLoader;

/// Resource limits loaded from environment variables.
///
/// Controls CPU, memory, storage, network, and execution concurrency limits.
/// Valid values: percentages 0–100, bytes/u64 for memory/storage, Mbps for network.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEnvConfig {
    /// Maximum CPU usage as percentage (0–100). Env: `MAX_CPU_PERCENT`.
    pub max_cpu_percent: f64,
    /// Maximum memory in bytes. Env: `MAX_MEMORY_BYTES`.
    pub max_memory_bytes: u64,
    /// Maximum storage in bytes. Env: `MAX_STORAGE_BYTES`.
    pub max_storage_bytes: u64,
    /// Maximum network throughput in Mbps. Env: `MAX_NETWORK_MBPS`.
    pub max_network_mbps: f64,
    /// Maximum GPU utilization as percentage (0–100). Env: `MAX_GPU_PERCENT`.
    pub max_gpu_percent: f64,
    /// Maximum concurrent task executions. Env: `MAX_CONCURRENT_EXECUTIONS`.
    pub max_concurrent_executions: u32,
    /// Number of worker threads. Env: `WORKER_THREADS`.
    pub worker_threads: u32,
    /// Task queue capacity. Env: `QUEUE_SIZE`.
    pub queue_size: u32,
    /// Batch size for bulk operations. Env: `BATCH_SIZE`.
    pub batch_size: u32,
}

impl ResourceEnvConfig {
    /// Load resource limits from environment variables (MAX_CPU_PERCENT, etc.).
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
///
/// Controls metrics collection, health checks, logging, and alert thresholds.
/// Valid log levels: trace, debug, info, warn, error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct MonitoringEnvConfig {
    /// Enable metrics collection. Env: `METRICS_ENABLED`.
    pub metrics_enabled: bool,
    /// Metrics collection interval in seconds. Env: `METRICS_INTERVAL_SECS`.
    pub metrics_interval_secs: u64,
    /// How long to retain metrics data in days. Env: `METRICS_RETENTION_DAYS`.
    pub metrics_retention_days: u64,
    /// Enable periodic health checks. Env: `HEALTH_CHECKS_ENABLED`.
    pub health_checks_enabled: bool,
    /// Health check interval in seconds. Env: `HEALTH_CHECK_INTERVAL_SECS`.
    pub health_check_interval_secs: u64,
    /// Enable structured logging. Env: `LOGGING_ENABLED`.
    pub logging_enabled: bool,
    /// Log verbosity level (trace|debug|info|warn|error). Env: `LOG_LEVEL`.
    pub log_level: String,
    /// Directory for log files. Env: `LOG_DIR`.
    pub log_dir: PathBuf,
    /// Enable resource usage alerts. Env: `ALERTS_ENABLED`.
    pub alerts_enabled: bool,
    /// CPU usage percentage that triggers alert (0–100). Env: `CPU_ALERT_THRESHOLD`.
    pub cpu_alert_threshold: f64,
    /// Memory usage percentage that triggers alert (0–100). Env: `MEMORY_ALERT_THRESHOLD`.
    pub memory_alert_threshold: f64,
    /// Storage usage percentage that triggers alert (0–100). Env: `STORAGE_ALERT_THRESHOLD`.
    pub storage_alert_threshold: f64,
}

impl MonitoringEnvConfig {
    /// Load monitoring config from environment variables (METRICS_ENABLED, LOG_LEVEL, etc.).
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
///
/// Controls authentication, sandboxing, encryption, rate limiting, and CORS.
/// Isolation levels: Standard, Strict, Minimal.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct SecurityEnvConfig {
    /// Enable authentication. Env: `AUTH_ENABLED`.
    pub auth_enabled: bool,
    /// Auth token lifetime in seconds. Env: `AUTH_TOKEN_EXPIRY_SECS`.
    pub auth_token_expiry_secs: u64,
    /// Enable execution sandboxing. Env: `SANDBOXING_ENABLED`.
    pub sandboxing_enabled: bool,
    /// Sandbox isolation level. Env: `ISOLATION_LEVEL`.
    pub isolation_level: String,
    /// Enable data encryption at rest. Env: `ENCRYPTION_ENABLED`.
    pub encryption_enabled: bool,
    /// Path to encryption key file. Env: `ENCRYPTION_KEY_PATH`.
    pub encryption_key_path: PathBuf,
    /// Enable rate limiting. Env: `RATE_LIMITING_ENABLED`.
    pub rate_limiting_enabled: bool,
    /// Max requests per second. Env: `RATE_LIMIT_RPS`.
    pub rate_limit_rps: u32,
    /// Burst capacity for rate limiter. Env: `RATE_LIMIT_BURST`.
    pub rate_limit_burst: u32,
    /// Enable CORS. Env: `CORS_ENABLED`.
    pub cors_enabled: bool,
    /// Allowed CORS origins (comma-separated). Env: `CORS_ALLOWED_ORIGINS`.
    pub cors_allowed_origins: Vec<String>,
}

impl SecurityEnvConfig {
    /// Load security config from environment variables (AUTH_ENABLED, CORS_ALLOWED_ORIGINS, etc.).
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
