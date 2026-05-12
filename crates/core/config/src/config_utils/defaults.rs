// SPDX-License-Identifier: AGPL-3.0-or-later
//! Default values and fallback configuration
//!
//! Resource limits, storage URLs, security settings, and feature flags.

use std::time::Duration;
use tracing::{debug, info};

use crate::env_config::EnvConfigLoader;

use super::network;
use super::paths;

const DEFAULT_EXECUTION_TIMEOUT_SECS: u64 = 300;
const DEFAULT_METRICS_INTERVAL_SECS: u64 = 10;
const DEFAULT_HEALTH_CHECK_INTERVAL_SECS: u64 = 30;

/// Get worker threads from environment or default
#[must_use]
pub fn get_worker_threads() -> u32 {
    let loader = EnvConfigLoader::new();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "truncation acceptable for this conversion"
    )]
    loader.get_u32(
        "WORKER_THREADS",
        std::thread::available_parallelism()
            .map(|n| n.get() as u32)
            .unwrap_or(4),
    )
}

/// Get max concurrent executions from environment or default
#[must_use]
pub fn get_max_concurrent_executions() -> u32 {
    let loader = EnvConfigLoader::new();
    loader.get_u32("MAX_CONCURRENT_EXECUTIONS", 100)
}

/// Get execution timeout from environment or default
#[must_use]
pub fn get_execution_timeout() -> Duration {
    let loader = EnvConfigLoader::new();
    loader.get_duration("EXECUTION_TIMEOUT_SECS", Duration::from_secs(DEFAULT_EXECUTION_TIMEOUT_SECS))
}

/// Get max CPU usage from environment or default
#[must_use]
pub fn get_max_cpu_usage() -> f64 {
    let loader = EnvConfigLoader::new();
    loader.get_f64("MAX_CPU_PERCENT", 90.0)
}

/// Get max memory usage from environment or default
#[must_use]
pub fn get_max_memory_usage() -> u64 {
    let loader = EnvConfigLoader::new();
    loader.get_u64("MAX_MEMORY_BYTES", 8 * 1024 * 1024 * 1024) // 8GB
}

/// Get max storage usage from environment or default
#[must_use]
pub fn get_max_storage_usage() -> u64 {
    let loader = EnvConfigLoader::new();
    loader.get_u64("MAX_STORAGE_BYTES", 100 * 1024 * 1024 * 1024) // 100GB
}

/// Get metrics collection interval from environment or default
#[must_use]
pub fn get_metrics_interval() -> Duration {
    let loader = EnvConfigLoader::new();
    loader.get_duration("METRICS_INTERVAL_SECS", Duration::from_secs(DEFAULT_METRICS_INTERVAL_SECS))
}

/// Get health check interval from environment or default
#[must_use]
pub fn get_health_check_interval() -> Duration {
    let loader = EnvConfigLoader::new();
    loader.get_duration("HEALTH_CHECK_INTERVAL_SECS", Duration::from_secs(DEFAULT_HEALTH_CHECK_INTERVAL_SECS))
}

/// Get log level from environment or default
#[must_use]
pub fn get_log_level() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("LOG_LEVEL", "info")
}

/// Get TLS enabled from environment or default
#[must_use]
pub fn get_tls_enabled() -> bool {
    let loader = EnvConfigLoader::new();
    loader.get_bool("TLS_ENABLED", false)
}

/// Get auth enabled from environment or default
#[must_use]
pub fn get_auth_enabled() -> bool {
    let loader = EnvConfigLoader::new();
    loader.get_bool("AUTH_ENABLED", false)
}

/// Get sandboxing enabled from environment or default
#[must_use]
pub fn get_sandboxing_enabled() -> bool {
    let loader = EnvConfigLoader::new();
    loader.get_bool("SANDBOXING_ENABLED", true)
}

/// Get metrics enabled from environment or default
#[must_use]
pub fn get_metrics_enabled() -> bool {
    let loader = EnvConfigLoader::new();
    loader.get_bool("METRICS_ENABLED", true)
}

/// Get health checks enabled from environment or default
#[must_use]
pub fn get_health_checks_enabled() -> bool {
    let loader = EnvConfigLoader::new();
    loader.get_bool("HEALTH_CHECKS_ENABLED", true)
}

/// Get database URL from environment or default
#[must_use]
pub fn get_database_url() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("DATABASE_URL", "sqlite://./data/toadstool.db")
}

/// Get cache URL from environment or default
#[must_use]
pub fn get_cache_url() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string(
        "REDIS_URL",
        &format!(
            "redis://{}:{}",
            crate::defaults::network::LOCALHOST,
            crate::defaults::storage::REDIS_PORT
        ),
    )
}

/// Get message broker URL from environment or default
#[must_use]
pub fn get_message_broker_url() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string(
        "AMQP_URL",
        &format!(
            "amqp://{}:{}",
            crate::defaults::network::LOCALHOST,
            crate::defaults::storage::AMQP_PORT
        ),
    )
}

/// Get distributed storage URL from environment or default
#[must_use]
#[expect(
    deprecated,
    reason = "legacy storage URL helper; callers migrating to capability-based discovery"
)]
pub fn get_distributed_storage_url() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string(
        "DISTRIBUTED_STORAGE_URL",
        crate::defaults::storage::DISTRIBUTED_URL,
    )
}

/// Get monitoring endpoint from environment or default
#[must_use]
pub fn get_monitoring_endpoint() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string(
        "METRICS_URL",
        &format!(
            "http://{}:{}",
            crate::defaults::network::LOCALHOST,
            crate::defaults::network::METRICS_PORT
        ),
    )
}

/// Get alert webhook URL from environment or default
#[must_use]
pub fn get_alert_webhook_url() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("ALERT_WEBHOOK_URL", "")
}

/// Get JWT secret from environment or default
#[must_use]
pub fn get_jwt_secret() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("JWT_SECRET", "default-jwt-secret-change-in-production")
}

/// Get API key from environment or default
#[must_use]
pub fn get_api_key() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("API_KEY", "default-api-key-change-in-production")
}

/// Get webhook secret from environment or default
#[must_use]
pub fn get_webhook_secret() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string(
        "WEBHOOK_SECRET",
        "default-webhook-secret-change-in-production",
    )
}

/// Get federation trust domain from environment or default
#[must_use]
pub fn get_federation_trust_domain() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string(
        "FEDERATION_TRUST_DOMAIN",
        toadstool_common::constants::network::DEFAULT_HOSTNAME,
    )
}

/// Get cluster name from environment or default
#[must_use]
pub fn get_cluster_name() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("CLUSTER_NAME", "toadstool-cluster")
}

/// Get node name from environment or default
#[must_use]
pub fn get_node_name() -> String {
    let loader = EnvConfigLoader::new();
    loader.get_string("NODE_NAME", "toadstool-node-1")
}

/// Print all current configuration values (for debugging)
#[cfg(debug_assertions)]
pub fn print_current_config() {
    use super::environment;

    info!("=== ToadStool Configuration ===");
    debug!("Environment: {}", environment::get_environment());
    debug!("Debug: {}", environment::get_debug_mode());
    debug!("Verbose: {}", environment::get_verbose_mode());

    info!("=== Network Configuration ===");
    debug!("Bind Address: {}", network::get_bind_address());
    debug!("External Hostname: {}", network::get_external_hostname());
    debug!("TLS Enabled: {}", get_tls_enabled());

    info!("=== Service Ports ===");
    for (service, port) in network::get_service_ports() {
        debug!("{service}: {port}");
    }

    info!("=== Service Endpoints ===");
    for (service, endpoint) in network::get_service_endpoints() {
        debug!("{service}: {endpoint}");
    }

    info!("=== Resource Limits ===");
    debug!("Max CPU: {}%", get_max_cpu_usage());
    debug!("Max Memory: {} bytes", get_max_memory_usage());
    debug!("Max Storage: {} bytes", get_max_storage_usage());
    debug!("Worker Threads: {}", get_worker_threads());
    debug!(
        "Max Concurrent Executions: {}",
        get_max_concurrent_executions()
    );

    info!("=== Timeouts ===");
    debug!("Request Timeout: {:?}", network::get_request_timeout());
    debug!(
        "Connection Timeout: {:?}",
        network::get_connection_timeout()
    );
    debug!("Execution Timeout: {:?}", get_execution_timeout());

    info!("=== Directories ===");
    debug!("Data Dir: {}", paths::get_data_dir());
    debug!("Cache Dir: {}", paths::get_cache_dir());
    debug!("Temp Dir: {}", paths::get_temp_dir());
    debug!("Log Dir: {}", paths::get_log_dir());

    info!("=== Security ===");
    debug!("Auth Enabled: {}", get_auth_enabled());
    debug!("Sandboxing Enabled: {}", get_sandboxing_enabled());
    debug!("Encryption Key Path: {}", paths::get_encryption_key_path());

    info!("=== Monitoring ===");
    debug!("Metrics Enabled: {}", get_metrics_enabled());
    debug!("Health Checks Enabled: {}", get_health_checks_enabled());
    debug!("Metrics Interval: {:?}", get_metrics_interval());
    debug!("Health Check Interval: {:?}", get_health_check_interval());

    info!("=== Logging ===");
    debug!("Log Level: {}", get_log_level());
    debug!("Log Dir: {}", paths::get_log_dir());
}
