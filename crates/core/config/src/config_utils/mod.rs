//! Configuration Utilities
//!
//! This module provides utility functions to replace hardcoded values with
//! environment-aware configuration throughout the `ToadStool` codebase.

use std::collections::HashMap;
use std::env;
use std::time::Duration;

use crate::env_config::{EnvConfigLoader, NetworkEnvConfig};
use crate::network;

/// Global configuration utilities for replacing hardcoded values
pub struct ConfigUtils;

impl ConfigUtils {
    /// Get Songbird port from environment or default
    ///
    /// # ⚠️ Legacy Pattern - Prefer Capability-Based Discovery
    ///
    /// **Modern Pattern**:
    /// ```ignore
    /// // Self-knowledge: ToadStool knows only its own config
    /// let my_port = config.network.toadstool_port;
    ///
    /// // Runtime discovery: Find coordination services by capability
    /// let discovery = RuntimeDiscovery::new();
    /// let coord_services = discovery
    ///     .discover_capability(&Capability::Coordination)
    ///     .await?;
    /// ```
    ///
    /// This method remains for backwards compatibility and fallback scenarios only.
    #[deprecated(
        since = "0.2.0",
        note = "Use capability-based discovery (RuntimeDiscovery::discover_capability) instead of hardcoded primal endpoints"
    )]
    #[must_use]
    #[allow(deprecated)] // Using legacy fallback constant during migration
    pub fn get_songbird_port() -> u16 {
        // ✅ DEEP SOLUTION: No prefix for other primals - respects self-knowledge principle
        // Use constant default, not cached config value (avoids double-loading issue)
        let loader = EnvConfigLoader::with_prefix(""); // Check SONGBIRD_PORT, not TOADSTOOL_SONGBIRD_PORT
        loader.get_u16(
            "SONGBIRD_PORT",
            crate::defaults::network::COORDINATION_FALLBACK_PORT,
        )
    }

    /// Get `BearDog` port from environment or default
    ///
    /// # ⚠️ Legacy Pattern - Prefer Capability-Based Discovery
    ///
    /// **Modern Pattern**: Use `RuntimeDiscovery::discover_capability(&Capability::Crypto)`
    /// instead of hardcoded BearDog endpoints. Each primal has self-knowledge only.
    ///
    /// # Self-Knowledge Principle
    ///
    /// This function violates self-knowledge by checking OTHER primal's environment.
    /// Uses non-prefixed env var: `BEARDOG_PORT` (not `TOADSTOOL_BEARDOG_PORT`)
    #[deprecated(
        since = "0.2.0",
        note = "Use capability-based discovery for crypto services instead of hardcoded endpoints"
    )]
    #[must_use]
    #[allow(deprecated)] // Using legacy fallback constant during migration
    pub fn get_beardog_port() -> u16 {
        // ✅ DEEP SOLUTION: No prefix for other primals - they manage their own env vars
        // Use constant default, not cached config value (avoids double-loading issue)
        let loader = EnvConfigLoader::with_prefix(""); // No prefix - check raw BEARDOG_PORT
        loader.get_u16(
            "BEARDOG_PORT",
            crate::defaults::network::SECURITY_FALLBACK_PORT,
        )
    }

    /// Get `NestGate` port from environment or default
    ///
    /// # ⚠️ Legacy Pattern - Prefer Capability-Based Discovery
    ///
    /// **Modern Pattern**: Use `RuntimeDiscovery::discover_capability(&Capability::Storage)`
    /// instead of hardcoded NestGate endpoints.
    #[deprecated(
        since = "0.2.0",
        note = "Use capability-based discovery for storage services instead of hardcoded endpoints"
    )]
    #[must_use]
    #[allow(deprecated)] // Using legacy fallback constant during migration
    pub fn get_nestgate_port() -> u16 {
        // ✅ DEEP SOLUTION: No prefix for other primals - respects self-knowledge principle
        // Use constant default, not cached config value (avoids double-loading issue)
        let loader = EnvConfigLoader::with_prefix(""); // Check NESTGATE_PORT, not TOADSTOOL_NESTGATE_PORT
        loader.get_u16(
            "NESTGATE_PORT",
            crate::defaults::network::STORAGE_FALLBACK_PORT,
        )
    }

    /// Get Squirrel port from environment or default
    ///
    /// # ⚠️ Legacy Pattern - Prefer Capability-Based Discovery
    ///
    /// **Modern Pattern**: Use `RuntimeDiscovery::discover_capability(&Capability::AI)`
    /// instead of hardcoded Squirrel endpoints.
    #[deprecated(
        since = "0.2.0",
        note = "Use capability-based discovery for AI services instead of hardcoded endpoints"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated field during migration
    #[allow(deprecated)] // Using legacy fallback constant during migration
    pub fn get_squirrel_port() -> u16 {
        // ✅ DEEP SOLUTION: No prefix for other primals - respects self-knowledge principle
        // Use constant default, not cached config value
        let loader = EnvConfigLoader::with_prefix(""); // Check SQUIRREL_PORT, not TOADSTOOL_SQUIRREL_PORT
        loader.get_u16("SQUIRREL_PORT", crate::defaults::network::AI_FALLBACK_PORT)
    }

    /// Get `ToadStool` port from environment or default
    #[must_use]
    pub fn get_toadstool_port() -> u16 {
        // ✅ SELF-KNOWLEDGE: ToadStool knows its own port
        // Use empty prefix and full env var name "TOADSTOOL_PORT"
        let loader = EnvConfigLoader::with_prefix(""); // Check TOADSTOOL_PORT directly
        loader.get_u16("TOADSTOOL_PORT", crate::defaults::network::API_PORT)
    }

    /// Get federation port from environment or default
    #[must_use]
    pub fn get_federation_port() -> u16 {
        let config = crate::env_config::EnvironmentConfig::from_env();
        let loader = EnvConfigLoader::new();
        loader.get_u16("FEDERATION_PORT", config.network.federation_port)
    }

    /// Get metrics port from environment or default
    #[must_use]
    pub fn get_metrics_port() -> u16 {
        let config = crate::env_config::EnvironmentConfig::from_env();
        let loader = EnvConfigLoader::new();
        loader.get_u16("METRICS_PORT", config.network.metrics_port)
    }

    /// Get health check port from environment or default
    #[must_use]
    pub fn get_health_port() -> u16 {
        let config = crate::env_config::EnvironmentConfig::from_env();
        let loader = EnvConfigLoader::new();
        loader.get_u16("HEALTH_PORT", config.network.health_port)
    }

    /// Get events port from environment or default (JSON-RPC event streaming; replaces deprecated WebSocket)
    #[must_use]
    pub fn get_events_port() -> u16 {
        let config = crate::env_config::EnvironmentConfig::from_env();
        let loader = EnvConfigLoader::new();
        loader.get_u16("EVENTS_PORT", config.network.events_port)
    }

    /// Get bind address from environment or default
    #[must_use]
    pub fn get_bind_address() -> String {
        // ✅ SELF-KNOWLEDGE: ToadStool knows its own bind address
        // Use constant default, not cached config value
        let loader = EnvConfigLoader::with_prefix(""); // Check BIND_ADDRESS directly
        loader.get_string("BIND_ADDRESS", "127.0.0.1")
    }

    /// Get external hostname from environment or default
    #[must_use]
    pub fn get_external_hostname() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("EXTERNAL_HOSTNAME", "localhost")
    }

    /// Get Songbird endpoint from environment or default
    ///
    /// # ⚠️ Deprecated - Use Capability-Based Discovery
    ///
    /// **Legacy fallback only**. Modern code should use runtime discovery:
    /// ```ignore
    /// let discovery = RuntimeDiscovery::new();
    /// let services = discovery.discover_capability(&Capability::Coordination).await?;
    /// let endpoint = services.first().map(|s| &s.endpoint);
    /// ```
    #[deprecated(
        since = "0.2.0",
        note = "Hardcoded endpoints violate self-knowledge principle. Use RuntimeDiscovery for capability-based service location."
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated method during migration
    pub fn get_songbird_endpoint() -> String {
        // ✅ SELF-KNOWLEDGE: Build endpoint from discovered port
        format!(
            "http://{}:{}",
            Self::get_bind_address(),
            Self::get_songbird_port()
        )
    }

    /// Get `BearDog` endpoint from environment or default
    ///
    /// # ⚠️ Deprecated - Use Capability-Based Discovery
    ///
    /// Prefer `RuntimeDiscovery::discover_capability(&Capability::Crypto)` for dynamic service location.
    #[deprecated(
        since = "0.2.0",
        note = "Use capability-based discovery instead of hardcoded crypto service endpoints"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated method during migration
    pub fn get_beardog_endpoint() -> String {
        // ✅ SELF-KNOWLEDGE: Build endpoint from discovered port
        format!(
            "http://{}:{}",
            Self::get_bind_address(),
            Self::get_beardog_port()
        )
    }

    /// Get `NestGate` endpoint from environment or default
    ///
    /// # ⚠️ Deprecated - Use Capability-Based Discovery
    ///
    /// Prefer `RuntimeDiscovery::discover_capability(&Capability::Storage)` for dynamic service location.
    #[deprecated(
        since = "0.2.0",
        note = "Use capability-based discovery instead of hardcoded storage service endpoints"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated method during migration
    pub fn get_nestgate_endpoint() -> String {
        // ✅ SELF-KNOWLEDGE: Build endpoint from discovered port
        format!(
            "http://{}:{}",
            Self::get_bind_address(),
            Self::get_nestgate_port()
        )
    }

    /// Get Squirrel endpoint from environment or default
    ///
    /// # ⚠️ Deprecated - Use Capability-Based Discovery
    ///
    /// Prefer `RuntimeDiscovery::discover_capability(&Capability::AI)` for dynamic service location.
    #[deprecated(
        since = "0.2.0",
        note = "Use capability-based discovery instead of hardcoded AI service endpoints"
    )]
    #[must_use]
    #[allow(deprecated)] // Using deprecated method during migration
    pub fn get_squirrel_endpoint() -> String {
        // ✅ SELF-KNOWLEDGE: Build endpoint from discovered port
        format!(
            "http://{}:{}",
            Self::get_bind_address(),
            Self::get_squirrel_port()
        )
    }

    /// Get `ToadStool` endpoint from environment or default
    #[must_use]
    pub fn get_toadstool_endpoint() -> String {
        let net_config = NetworkEnvConfig::from_env();
        net_config.toadstool_endpoint()
    }

    /// Get request timeout from environment or default
    #[must_use]
    pub fn get_request_timeout() -> Duration {
        let loader = EnvConfigLoader::new();
        loader.get_duration(
            "REQUEST_TIMEOUT_SECS",
            Duration::from_secs(network::DEFAULT_REQUEST_TIMEOUT_SECS),
        )
    }

    /// Get connection timeout from environment or default
    #[must_use]
    pub fn get_connection_timeout() -> Duration {
        let loader = EnvConfigLoader::new();
        loader.get_duration(
            "CONNECTION_TIMEOUT_SECS",
            Duration::from_secs(network::DEFAULT_CONNECTION_TIMEOUT_SECS),
        )
    }

    /// Get max retries from environment or default
    #[must_use]
    pub fn get_max_retries() -> u32 {
        let loader = EnvConfigLoader::new();
        loader.get_u32("MAX_RETRIES", network::DEFAULT_MAX_RETRIES)
    }

    /// Get max connections per host from environment or default
    #[must_use]
    pub fn get_max_connections_per_host() -> u32 {
        let loader = EnvConfigLoader::new();
        loader.get_u32(
            "MAX_CONNECTIONS_PER_HOST",
            network::DEFAULT_MAX_CONNECTIONS_PER_HOST,
        )
    }

    /// Get keepalive interval from environment or default
    #[must_use]
    pub fn get_keepalive_interval() -> Duration {
        let loader = EnvConfigLoader::new();
        loader.get_duration(
            "KEEPALIVE_INTERVAL_SECS",
            Duration::from_secs(network::DEFAULT_KEEPALIVE_INTERVAL_SECS),
        )
    }

    /// Get worker threads from environment or default
    #[must_use]
    pub fn get_worker_threads() -> u32 {
        let loader = EnvConfigLoader::new();
        #[allow(clippy::cast_possible_truncation)]
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
        loader.get_duration("EXECUTION_TIMEOUT_SECS", Duration::from_secs(300))
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
        loader.get_duration("METRICS_INTERVAL_SECS", Duration::from_secs(10))
    }

    /// Get health check interval from environment or default
    #[must_use]
    pub fn get_health_check_interval() -> Duration {
        let loader = EnvConfigLoader::new();
        loader.get_duration("HEALTH_CHECK_INTERVAL_SECS", Duration::from_secs(30))
    }

    /// Get log level from environment or default
    #[must_use]
    pub fn get_log_level() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("LOG_LEVEL", "info")
    }

    /// Get data directory from environment or default
    #[must_use]
    pub fn get_data_dir() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("DATA_DIR", "./data")
    }

    /// Get cache directory from environment or default
    #[must_use]
    pub fn get_cache_dir() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("CACHE_DIR", "./cache")
    }

    /// Get temp directory from environment or default
    #[must_use]
    pub fn get_temp_dir() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("TEMP_DIR", "./tmp")
    }

    /// Get log directory from environment or default
    #[must_use]
    pub fn get_log_dir() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("LOG_DIR", "./logs")
    }

    /// Get environment name from environment or default
    #[must_use]
    pub fn get_environment() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("ENV", "development")
    }

    /// Get debug mode from environment or default
    #[must_use]
    pub fn get_debug_mode() -> bool {
        let loader = EnvConfigLoader::new();
        loader.get_bool("DEBUG", false)
    }

    /// Get verbose mode from environment or default
    #[must_use]
    pub fn get_verbose_mode() -> bool {
        let loader = EnvConfigLoader::new();
        loader.get_bool("VERBOSE", false)
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

    /// Get all service ports as a map
    ///
    /// Returns only self-knowledge ports (ToadStool's own). Ports for external
    /// primals (songbird, beardog, nestgate, squirrel) are discovered at runtime.
    #[must_use]
    pub fn get_service_ports() -> HashMap<String, u16> {
        use toadstool_common::constants::primal_identity::PRIMAL_NAME;

        let mut ports = HashMap::new();
        ports.insert(PRIMAL_NAME.to_string(), Self::get_toadstool_port());
        ports.insert("federation".to_string(), Self::get_federation_port());
        ports.insert("metrics".to_string(), Self::get_metrics_port());
        ports.insert("health".to_string(), Self::get_health_port());
        ports.insert("events".to_string(), Self::get_events_port());
        ports
    }

    /// Get all service endpoints as a map
    ///
    /// Returns only ToadStool's own endpoint. External primal endpoints
    /// (songbird, beardog, nestgate, squirrel) are discovered at runtime.
    #[must_use]
    pub fn get_service_endpoints() -> HashMap<String, String> {
        use toadstool_common::constants::primal_identity::PRIMAL_NAME;

        let mut endpoints = HashMap::new();
        endpoints.insert(PRIMAL_NAME.to_string(), Self::get_toadstool_endpoint());
        endpoints
    }

    /// Get container port range from environment or default
    #[must_use]
    pub fn get_container_port_range() -> (u16, u16) {
        let loader = EnvConfigLoader::new();
        let start = loader.get_u16(
            "CONTAINER_PORT_START",
            crate::defaults::ports::CONTAINER_START,
        );
        let end = loader.get_u16("CONTAINER_PORT_END", crate::defaults::ports::CONTAINER_END);
        (start, end)
    }

    /// Get port allocation range from environment or default
    #[must_use]
    pub fn get_port_allocation_range() -> (u16, u16) {
        let loader = EnvConfigLoader::new();
        let start = loader.get_u16("PORT_RANGE_START", crate::defaults::ports::RANGE_START);
        let end = loader.get_u16("PORT_RANGE_END", crate::defaults::ports::RANGE_END);
        (start, end)
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
            "CACHE_URL",
            &format!("redis://localhost:{}", crate::defaults::storage::REDIS_PORT),
        )
    }

    /// Get message broker URL from environment or default
    #[must_use]
    pub fn get_message_broker_url() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("MESSAGE_BROKER_URL", "amqp://localhost:5672")
    }

    /// Get distributed storage URL from environment or default
    #[must_use]
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
            "MONITORING_ENDPOINT",
            &format!(
                "http://localhost:{}",
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

    /// Get encryption key path from environment or default
    #[must_use]
    pub fn get_encryption_key_path() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("ENCRYPTION_KEY_PATH", "./keys/encryption.key")
    }

    /// Get TLS cert path from environment or default
    #[must_use]
    pub fn get_tls_cert_path() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("TLS_CERT_PATH", "./certs/tls.crt")
    }

    /// Get TLS key path from environment or default
    #[must_use]
    pub fn get_tls_key_path() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("TLS_KEY_PATH", "./certs/tls.key")
    }

    /// Get CA cert path from environment or default
    #[must_use]
    pub fn get_ca_cert_path() -> String {
        let loader = EnvConfigLoader::new();
        loader.get_string("CA_CERT_PATH", "./certs/ca.crt")
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
        loader.get_string("FEDERATION_TRUST_DOMAIN", "localhost")
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

    /// Get all environment variables with TOADSTOOL prefix
    #[must_use]
    pub fn get_all_toadstool_env_vars() -> HashMap<String, String> {
        env::vars()
            .filter(|(key, _)| key.starts_with("TOADSTOOL_"))
            .collect()
    }

    /// Print all current configuration values (for debugging)
    #[cfg(debug_assertions)]
    pub fn print_current_config() {
        println!("=== ToadStool Configuration ===");
        println!("Environment: {}", Self::get_environment());
        println!("Debug: {}", Self::get_debug_mode());
        println!("Verbose: {}", Self::get_verbose_mode());
        println!();

        println!("=== Network Configuration ===");
        println!("Bind Address: {}", Self::get_bind_address());
        println!("External Hostname: {}", Self::get_external_hostname());
        println!("TLS Enabled: {}", Self::get_tls_enabled());
        println!();

        println!("=== Service Ports ===");
        for (service, port) in Self::get_service_ports() {
            println!("{service}: {port}");
        }
        println!();

        println!("=== Service Endpoints ===");
        for (service, endpoint) in Self::get_service_endpoints() {
            println!("{service}: {endpoint}");
        }
        println!();

        println!("=== Resource Limits ===");
        println!("Max CPU: {}%", Self::get_max_cpu_usage());
        println!("Max Memory: {} bytes", Self::get_max_memory_usage());
        println!("Max Storage: {} bytes", Self::get_max_storage_usage());
        println!("Worker Threads: {}", Self::get_worker_threads());
        println!(
            "Max Concurrent Executions: {}",
            Self::get_max_concurrent_executions()
        );
        println!();

        println!("=== Timeouts ===");
        println!("Request Timeout: {:?}", Self::get_request_timeout());
        println!("Connection Timeout: {:?}", Self::get_connection_timeout());
        println!("Execution Timeout: {:?}", Self::get_execution_timeout());
        println!();

        println!("=== Directories ===");
        println!("Data Dir: {}", Self::get_data_dir());
        println!("Cache Dir: {}", Self::get_cache_dir());
        println!("Temp Dir: {}", Self::get_temp_dir());
        println!("Log Dir: {}", Self::get_log_dir());
        println!();

        println!("=== Security ===");
        println!("Auth Enabled: {}", Self::get_auth_enabled());
        println!("Sandboxing Enabled: {}", Self::get_sandboxing_enabled());
        println!("Encryption Key Path: {}", Self::get_encryption_key_path());
        println!();

        println!("=== Monitoring ===");
        println!("Metrics Enabled: {}", Self::get_metrics_enabled());
        println!(
            "Health Checks Enabled: {}",
            Self::get_health_checks_enabled()
        );
        println!("Metrics Interval: {:?}", Self::get_metrics_interval());
        println!(
            "Health Check Interval: {:?}",
            Self::get_health_check_interval()
        );
        println!();

        println!("=== Logging ===");
        println!("Log Level: {}", Self::get_log_level());
        println!("Log Dir: {}", Self::get_log_dir());
        println!();
    }
}

#[cfg(test)]
#[allow(deprecated)]
mod tests;
