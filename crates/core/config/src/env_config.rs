//! Environment Variable Configuration System
//!
//! This module provides comprehensive environment variable support to eliminate
//! all hardcoded values from the `ToadStool` codebase. It supports type-safe
//! environment variable parsing with fallback defaults.
//!
//! # Modern Rust Features
//!
//! - Zero-copy string handling with `Cow` for static prefixes
//! - Compile-time validation
//! - Type-safe environment parsing

use std::borrow::Cow;
use std::collections::HashMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::ToadStoolConfig;

/// Environment variable configuration loader with type-safe parsing
///
/// # Zero-Copy Optimization
///
/// The `prefix` field uses `Cow<'static, str>` to avoid allocations for the
/// common case of using the default "TOADSTOOL" prefix. This is a zero-cost
/// abstraction that only allocates when a custom prefix is provided.
#[derive(Debug, Clone)]
pub struct EnvConfigLoader {
    /// Environment prefix for `ToadStool` variables (zero-copy for defaults)
    prefix: Cow<'static, str>,
    /// Cache of loaded environment variables
    cache: HashMap<String, String>,
}

impl EnvConfigLoader {
    /// Create a new environment configuration loader
    ///
    /// Uses zero-copy for the default "TOADSTOOL" prefix - no heap allocation!
    #[must_use]
    pub fn new() -> Self {
        Self {
            prefix: Cow::Borrowed("TOADSTOOL"), // Zero allocation! 🚀
            cache: HashMap::new(),
        }
    }

    /// Create a new environment configuration loader with custom prefix
    ///
    /// Only allocates when using a non-static custom prefix.
    #[must_use]
    pub fn with_prefix(prefix: &str) -> Self {
        Self {
            prefix: Cow::Owned(prefix.to_string()), // Allocate only when custom
            cache: HashMap::new(),
        }
    }

    /// Load environment variables into cache
    pub fn load_cache(&mut self) {
        for (key, value) in env::vars() {
            if key.starts_with(self.prefix.as_ref()) {
                self.cache.insert(key, value);
            }
        }
        debug!("Loaded {} environment variables", self.cache.len());
    }

    /// Get environment variable as string with fallback
    #[must_use]
    pub fn get_string(&self, key: &str, default: &str) -> String {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key).unwrap_or_else(|_| default.to_string())
    }

    /// Get environment variable as boolean with fallback
    #[must_use]
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .ok()
            .and_then(|v| {
                let lower = v.to_lowercase();
                match lower.as_str() {
                    "true" | "1" | "yes" | "on" => Some(true),
                    "false" | "0" | "no" | "off" => Some(false),
                    _ => None, // Invalid values return None, so default is used
                }
            })
            .unwrap_or(default)
    }

    /// Get environment variable as u16 with fallback
    #[must_use]
    pub fn get_u16(&self, key: &str, default: u16) -> u16 {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as u32 with fallback
    #[must_use]
    pub fn get_u32(&self, key: &str, default: u32) -> u32 {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as u64 with fallback
    #[must_use]
    pub fn get_u64(&self, key: &str, default: u64) -> u64 {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as f64 with fallback
    #[must_use]
    pub fn get_f64(&self, key: &str, default: f64) -> f64 {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as Duration with fallback
    #[must_use]
    pub fn get_duration(&self, key: &str, default: Duration) -> Duration {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| {
                v.parse::<u64>()
                    .map(Duration::from_secs)
                    .map_err(|_| env::VarError::NotPresent)
            })
            .unwrap_or(default)
    }

    /// Get environment variable as `SocketAddr` with fallback
    #[must_use]
    pub fn get_socket_addr(&self, key: &str, default: SocketAddr) -> SocketAddr {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key)
            .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
            .unwrap_or(default)
    }

    /// Get environment variable as `PathBuf` with fallback
    #[must_use]
    pub fn get_path(&self, key: &str, default: &str) -> PathBuf {
        let env_key = format!("{}_{}", self.prefix, key);
        env::var(&env_key).map_or_else(|_| PathBuf::from(default), PathBuf::from)
    }

    /// Get all environment variables with a specific prefix
    #[must_use]
    pub fn get_prefixed(&self, prefix: &str) -> HashMap<String, String> {
        let full_prefix = format!("{}_{}", self.prefix, prefix);
        env::vars()
            .filter(|(key, _)| key.starts_with(&full_prefix))
            .collect()
    }
}

impl Default for EnvConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration for network-related environment variables
///
/// # Self-Knowledge Architecture
///
/// Following the principle that "ToadStool knows only itself," this configuration
/// contains only self-related network settings. Other primals are discovered at
/// runtime through capability-based discovery.
///
/// ## Self-Knowledge (Valid):
/// - `toadstool_*` - Our own ports and settings
/// - `bind_address` - Where we listen
/// - `external_hostname` - How we identify ourselves
///
/// ## Legacy Fields (Deprecated):
/// - `songbird_port`, `beardog_port`, etc. - Use RuntimeDiscovery instead
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkEnvConfig {
    // ========================================================================
    // Self-Knowledge: ToadStool's Own Configuration
    // ========================================================================
    /// `ToadStool` API port (self-knowledge: our own port)
    pub toadstool_port: u16,
    /// Federation port (self-knowledge: our federation capability)
    pub federation_port: u16,
    /// Metrics port (self-knowledge: our observability)
    pub metrics_port: u16,
    /// Health check port (self-knowledge: our health endpoint)
    pub health_port: u16,
    /// WebSocket port (self-knowledge: our realtime capability)
    pub websocket_port: u16,
    /// Bind address (self-knowledge: where we listen)
    pub bind_address: String,
    /// External hostname (self-knowledge: our identity)
    pub external_hostname: String,

    // ========================================================================
    // Connection Behavior (applies to outbound connections)
    // ========================================================================
    /// Enable TLS for outbound connections
    pub tls_enabled: bool,
    /// Connection timeout for outbound connections
    pub connection_timeout_secs: u64,
    /// Request timeout for outbound requests
    pub request_timeout_secs: u64,
    /// Max retries for failed requests
    pub max_retries: u32,
    /// Max connections per remote host
    pub max_connections_per_host: u32,

    // ========================================================================
    // Legacy Fields (Deprecated - Use RuntimeDiscovery)
    // ========================================================================
    /// ⚠️ DEPRECATED: Songbird service port
    ///
    /// **Modern approach**: Use `RuntimeDiscovery::discover_capability(&Capability::Coordination)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery for capability-based service discovery"
    )]
    pub songbird_port: u16,

    /// ⚠️ DEPRECATED: `BearDog` service port
    ///
    /// **Modern approach**: Use `RuntimeDiscovery::discover_capability(&Capability::Authentication)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery for capability-based service discovery"
    )]
    pub beardog_port: u16,

    /// ⚠️ DEPRECATED: `NestGate` service port
    ///
    /// **Modern approach**: Use `RuntimeDiscovery::discover_capability(&Capability::Storage)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery for capability-based service discovery"
    )]
    pub nestgate_port: u16,

    /// ⚠️ DEPRECATED: Squirrel MCP service port
    ///
    /// **Modern approach**: Use `RuntimeDiscovery::discover_capability(&Capability::MCP)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery for capability-based service discovery"
    )]
    pub squirrel_port: u16,
}

impl NetworkEnvConfig {
    /// Load network configuration from environment variables
    #[must_use]
    #[allow(deprecated)] // Using deprecated constants during migration to capability-based discovery
    pub fn from_env() -> Self {
        let loader = EnvConfigLoader::new();

        Self {
            songbird_port: loader.get_u16("SONGBIRD_PORT", crate::defaults::network::SONGBIRD_PORT),
            beardog_port: loader.get_u16("BEARDOG_PORT", crate::defaults::network::BEARDOG_PORT),
            nestgate_port: loader.get_u16("NESTGATE_PORT", crate::defaults::network::NESTGATE_PORT),
            squirrel_port: loader.get_u16("SQUIRREL_PORT", 8083),
            toadstool_port: loader.get_u16("TOADSTOOL_PORT", 8084),
            federation_port: loader.get_u16("FEDERATION_PORT", 7777),
            metrics_port: loader.get_u16("METRICS_PORT", 9090),
            health_port: loader.get_u16("HEALTH_PORT", 8085),
            websocket_port: loader.get_u16("WEBSOCKET_PORT", 8086),
            bind_address: loader.get_string("BIND_ADDRESS", "127.0.0.1"),
            external_hostname: loader.get_string("EXTERNAL_HOSTNAME", "localhost"),
            tls_enabled: loader.get_bool("TLS_ENABLED", false),
            connection_timeout_secs: loader.get_u64("CONNECTION_TIMEOUT_SECS", 10),
            request_timeout_secs: loader.get_u64("REQUEST_TIMEOUT_SECS", 30),
            max_retries: loader.get_u32("MAX_RETRIES", 3),
            max_connections_per_host: loader.get_u32("MAX_CONNECTIONS_PER_HOST", 100),
        }
    }

    // ========================================================================
    // Self-Knowledge Methods (Valid - Our Own Endpoints)
    // ========================================================================

    /// Get `ToadStool` endpoint (self-knowledge: our own endpoint)
    #[must_use]
    pub fn toadstool_endpoint(&self) -> String {
        format!("http://{}:{}", self.external_hostname, self.toadstool_port)
    }

    /// Get our federation endpoint (self-knowledge: our federation capability)
    #[must_use]
    pub fn federation_endpoint(&self) -> String {
        format!("http://{}:{}", self.external_hostname, self.federation_port)
    }

    /// Get our metrics endpoint (self-knowledge: our observability)
    #[must_use]
    pub fn metrics_endpoint(&self) -> String {
        format!("http://{}:{}", self.external_hostname, self.metrics_port)
    }

    /// Get our health check endpoint (self-knowledge: our health)
    #[must_use]
    pub fn health_endpoint(&self) -> String {
        format!("http://{}:{}", self.external_hostname, self.health_port)
    }

    // ========================================================================
    // Legacy Methods (Deprecated - Use RuntimeDiscovery)
    // ========================================================================

    /// ⚠️ DEPRECATED: Get Songbird endpoint
    ///
    /// **Use instead**:
    /// ```rust,ignore
    /// let discovery = RuntimeDiscovery::new(client);
    /// let services = discovery.discover_capability(&Capability::Coordination).await?;
    /// let endpoint = &services[0].endpoint;
    /// ```
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::Coordination)"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn songbird_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address, self.songbird_port)
    }

    /// ⚠️ DEPRECATED: Get `BearDog` endpoint
    ///
    /// **Use instead**: `RuntimeDiscovery::discover_capability(&Capability::Authentication)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::Authentication)"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn beardog_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address, self.beardog_port)
    }

    /// ⚠️ DEPRECATED: Get `NestGate` endpoint
    ///
    /// **Use instead**: `RuntimeDiscovery::discover_capability(&Capability::Storage)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::Storage)"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn nestgate_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address, self.nestgate_port)
    }

    /// ⚠️ DEPRECATED: Get Squirrel endpoint
    ///
    /// **Use instead**: `RuntimeDiscovery::discover_capability(&Capability::MCP)`
    #[deprecated(
        since = "0.3.0",
        note = "Use RuntimeDiscovery::discover_capability(&Capability::MCP)"
    )]
    #[must_use]
    #[allow(deprecated)]
    pub fn squirrel_endpoint(&self) -> String {
        format!("http://{}:{}", self.bind_address, self.squirrel_port)
    }
}

/// Configuration for resource-related environment variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceEnvConfig {
    /// Maximum CPU usage percentage
    pub max_cpu_percent: f64,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Maximum storage usage in bytes
    pub max_storage_bytes: u64,
    /// Maximum network bandwidth in Mbps
    pub max_network_mbps: f64,
    /// Maximum GPU usage percentage
    pub max_gpu_percent: f64,
    /// Maximum concurrent executions
    pub max_concurrent_executions: u32,
    /// Worker thread count
    pub worker_threads: u32,
    /// Queue size
    pub queue_size: u32,
    /// Batch size
    pub batch_size: u32,
}

impl ResourceEnvConfig {
    /// Load resource configuration from environment variables
    #[must_use]
    pub fn from_env() -> Self {
        let loader = EnvConfigLoader::new();

        Self {
            max_cpu_percent: loader.get_f64("MAX_CPU_PERCENT", 90.0),
            max_memory_bytes: loader.get_u64("MAX_MEMORY_BYTES", 8 * 1024 * 1024 * 1024), // 8GB
            max_storage_bytes: loader.get_u64("MAX_STORAGE_BYTES", 100 * 1024 * 1024 * 1024), // 100GB
            max_network_mbps: loader.get_f64("MAX_NETWORK_MBPS", 1000.0),
            max_gpu_percent: loader.get_f64("MAX_GPU_PERCENT", 95.0),
            max_concurrent_executions: loader.get_u32("MAX_CONCURRENT_EXECUTIONS", 100),
            worker_threads: loader.get_u32(
                "WORKER_THREADS",
                u32::try_from(num_cpus::get()).unwrap_or(4),
            ),
            queue_size: loader.get_u32("QUEUE_SIZE", 10000),
            batch_size: loader.get_u32("BATCH_SIZE", 1000),
        }
    }
}

/// Configuration for monitoring-related environment variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringEnvConfig {
    /// Enable metrics collection
    pub metrics_enabled: bool,
    /// Metrics collection interval
    pub metrics_interval_secs: u64,
    /// Metrics retention period in days
    pub metrics_retention_days: u64,
    /// Enable health checks
    pub health_checks_enabled: bool,
    /// Health check interval
    pub health_check_interval_secs: u64,
    /// Enable logging
    pub logging_enabled: bool,
    /// Log level
    pub log_level: String,
    /// Log directory
    pub log_dir: PathBuf,
    /// Enable alerts
    pub alerts_enabled: bool,
    /// Alert thresholds
    pub cpu_alert_threshold: f64,
    pub memory_alert_threshold: f64,
    pub storage_alert_threshold: f64,
}

impl MonitoringEnvConfig {
    /// Load monitoring configuration from environment variables
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

/// Configuration for security-related environment variables
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEnvConfig {
    /// Enable authentication
    pub auth_enabled: bool,
    /// Authentication token expiry
    pub auth_token_expiry_secs: u64,
    /// Enable sandboxing
    pub sandboxing_enabled: bool,
    /// Sandbox isolation level
    pub isolation_level: String,
    /// Enable encryption
    pub encryption_enabled: bool,
    /// Encryption key path
    pub encryption_key_path: PathBuf,
    /// Enable rate limiting
    pub rate_limiting_enabled: bool,
    /// Requests per second limit
    pub rate_limit_rps: u32,
    /// Rate limit burst size
    pub rate_limit_burst: u32,
    /// Enable CORS
    pub cors_enabled: bool,
    /// Allowed origins
    pub cors_allowed_origins: Vec<String>,
}

impl SecurityEnvConfig {
    /// Load security configuration from environment variables
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

/// Comprehensive environment configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvironmentConfig {
    /// Network configuration
    pub network: NetworkEnvConfig,
    /// Resource configuration
    pub resources: ResourceEnvConfig,
    /// Monitoring configuration
    pub monitoring: MonitoringEnvConfig,
    /// Security configuration
    pub security: SecurityEnvConfig,
    /// Environment name
    pub environment: String,
    /// Debug mode
    pub debug: bool,
    /// Verbose mode
    pub verbose: bool,
    /// Data directory
    pub data_dir: PathBuf,
    /// Cache directory
    pub cache_dir: PathBuf,
    /// Temp directory
    pub temp_dir: PathBuf,
}

impl EnvironmentConfig {
    /// Load all environment configuration
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

    /// Apply environment configuration to `ToadStool` config
    pub fn apply_to_config(&self, config: &mut ToadStoolConfig) {
        // Apply network configuration
        if let Ok(addr) = format!(
            "{}:{}",
            self.network.bind_address, self.network.toadstool_port
        )
        .parse()
        {
            config.network.bind_address = addr;
        }

        // Legacy endpoint configuration (deprecated - use capability-based discovery)
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

        // Apply resource configuration
        config.runtime.resource_limits.max_cpu_usage = self.resources.max_cpu_percent;
        config.runtime.resource_limits.max_memory_usage = self.resources.max_memory_bytes as f64;
        config.app.worker_threads = self.resources.worker_threads as usize;
        config.app.queue_size = self.resources.queue_size as usize;
        config.app.batch_size = self.resources.batch_size as usize;

        // Apply monitoring configuration
        config.logging.level = self.monitoring.log_level.clone();
        // Note: log_file configuration would be applied here if the field exists

        // Apply security configuration
        config.security.auth.enabled = self.security.auth_enabled;
        config.security.sandbox.enabled = self.security.sandboxing_enabled;

        // Apply general configuration
        config.app.environment = self.environment.clone();
        config.app.data_dir = self.data_dir.to_string_lossy().to_string();
        config.app.cache_dir = self.cache_dir.to_string_lossy().to_string();
        config.app.temp_dir = self.temp_dir.to_string_lossy().to_string();

        debug!("Applied environment configuration to ToadStool config");
    }
}

/// Helper function to get environment variable with custom prefix
#[must_use]
pub fn get_env_with_prefix(prefix: &str, key: &str, default: &str) -> String {
    let env_key = format!("{prefix}_{key}");
    env::var(&env_key).unwrap_or_else(|_| default.to_string())
}

/// Helper function to get environment variable as boolean
#[must_use]
pub fn get_env_bool(key: &str, default: bool) -> bool {
    env::var(key)
        .map(|v| v.to_lowercase() == "true" || v == "1")
        .unwrap_or(default)
}

/// Helper function to get environment variable as number
pub fn get_env_number<T: FromStr>(key: &str, default: T) -> T {
    env::var(key)
        .and_then(|v| v.parse().map_err(|_| env::VarError::NotPresent))
        .unwrap_or(default)
}

/// Helper function to get environment variable as duration
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

/// Load global environment configuration
pub fn load_global_env_config() -> EnvironmentConfig {
    let config = EnvironmentConfig::from_env();
    debug!(
        "Loaded global environment configuration for {} environment",
        config.environment
    );
    config
}

/// Apply environment configuration to existing `ToadStool` config
pub fn apply_env_config(config: &mut ToadStoolConfig) {
    let env_config = EnvironmentConfig::from_env();
    env_config.apply_to_config(config);
    debug!("Applied environment configuration overrides");
}

#[cfg(test)]
#[allow(deprecated)] // Tests for legacy APIs during migration period
pub(crate) mod tests {
    use super::*;
    use std::env;
    use std::sync::Mutex;

    // ✅ MODERN: Scoped lock for environment variable tests (shared across all config tests)
    static ENV_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

    pub(crate) fn get_env_lock() -> &'static Mutex<()> {
        ENV_LOCK.get_or_init(|| Mutex::new(()))
    }

    #[test]
    fn test_env_config_loader() {
        let loader = EnvConfigLoader::new();

        // Test string loading
        env::set_var("TOADSTOOL_TEST_STRING", "test_value");
        assert_eq!(loader.get_string("TEST_STRING", "default"), "test_value");

        // Test boolean loading
        env::set_var("TOADSTOOL_TEST_BOOL", "true");
        assert!(loader.get_bool("TEST_BOOL", false));

        // Test number loading
        env::set_var("TOADSTOOL_TEST_NUMBER", "42");
        assert_eq!(loader.get_u32("TEST_NUMBER", 0), 42);

        // Clean up
        env::remove_var("TOADSTOOL_TEST_STRING");
        env::remove_var("TOADSTOOL_TEST_BOOL");
        env::remove_var("TOADSTOOL_TEST_NUMBER");
    }

    #[test]
    #[allow(deprecated)] // Testing deprecated method during migration
    fn test_network_env_config() {
        // ✅ MODERN: Recover from poisoned lock (robust concurrent testing)
        let _guard = get_env_lock().lock().unwrap_or_else(|e| e.into_inner());

        // Save original environment state
        let original_port = env::var("TOADSTOOL_SONGBIRD_PORT").ok();
        let original_addr = env::var("TOADSTOOL_BIND_ADDRESS").ok();

        // Set test values
        env::set_var("TOADSTOOL_SONGBIRD_PORT", "9080");
        env::set_var("TOADSTOOL_BIND_ADDRESS", "0.0.0.0");

        let config = NetworkEnvConfig::from_env();
        assert_eq!(config.songbird_port, 9080);
        assert_eq!(config.bind_address, "0.0.0.0");
        assert_eq!(config.songbird_endpoint(), "http://0.0.0.0:9080");

        // ✅ MODERN: Restore original environment state
        if let Some(val) = original_port {
            env::set_var("TOADSTOOL_SONGBIRD_PORT", val);
        } else {
            env::remove_var("TOADSTOOL_SONGBIRD_PORT");
        }
        if let Some(val) = original_addr {
            env::set_var("TOADSTOOL_BIND_ADDRESS", val);
        } else {
            env::remove_var("TOADSTOOL_BIND_ADDRESS");
        }
    }

    #[test]
    fn test_environment_config() {
        // ✅ MODERN: Recover from poisoned lock (robust concurrent testing)
        let _guard = get_env_lock().lock().unwrap_or_else(|e| e.into_inner());

        // Save original environment state
        let original_env = env::var("TOADSTOOL_ENV").ok();
        let original_debug = env::var("TOADSTOOL_DEBUG").ok();

        // ✅ MODERN: Set test values explicitly
        env::set_var("TOADSTOOL_ENV", "development");
        env::set_var("TOADSTOOL_DEBUG", "false");

        let config = EnvironmentConfig::from_env();
        assert_eq!(config.environment, "development");
        assert!(!config.debug);

        // ✅ MODERN: Restore original environment state
        match original_env {
            Some(val) => env::set_var("TOADSTOOL_ENV", val),
            None => env::remove_var("TOADSTOOL_ENV"),
        }
        match original_debug {
            Some(val) => env::set_var("TOADSTOOL_DEBUG", val),
            None => env::remove_var("TOADSTOOL_DEBUG"),
        }
    }
}
