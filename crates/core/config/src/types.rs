//! Configuration type definitions
//!
//! This module contains all configuration struct definitions for ToadStool.
//! Types are organized by domain for easy discovery.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::time::Duration;

// Import from parent modules
use crate::{app, development, network, production, testing};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ToadStoolConfig {
    /// Application configuration
    pub app: ApplicationConfig,
    /// Network configuration
    pub network: NetworkConfig,
    /// Runtime configuration
    pub runtime: RuntimeConfig,
    /// Security configuration
    pub security: SecurityConfig,
    /// Logging configuration
    pub logging: LoggingConfig,
    /// Database configuration
    pub database: Option<DatabaseConfig>,
    /// Cache configuration
    pub cache: Option<BackendCacheConfig>,
    /// Metrics configuration
    pub metrics: Option<MetricsConfig>,
    /// Feature flags
    pub features: FeatureFlags,
    /// Environment-specific overrides
    pub overrides: HashMap<String, serde_json::Value>,
}

/// Application configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationConfig {
    /// Application name
    pub name: String,
    /// Application version
    pub version: String,
    /// Environment (development, staging, production)
    pub environment: String,
    /// Data directory
    pub data_dir: String,
    /// Cache directory
    pub cache_dir: String,
    /// Logs directory
    pub logs_dir: String,
    /// Temporary directory
    pub temp_dir: String,
    /// Worker thread count
    pub worker_threads: usize,
    /// Queue size
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

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// Bind address
    pub bind_address: SocketAddr,
    /// External endpoints
    pub endpoints: EndpointConfig,
    /// Connection settings
    pub connection: ConnectionConfig,
    /// TLS configuration
    pub tls: Option<TlsConfig>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        let config = crate::env_config::EnvironmentConfig::from_env();
        Self {
            bind_address: format!(
                "{}:{}",
                config.network.bind_address, config.network.toadstool_port
            )
            .parse()
            .expect("Invalid default bind address"),
            endpoints: EndpointConfig::default(),
            connection: ConnectionConfig::default(),
            tls: None,
        }
    }
}

/// External service endpoints
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointConfig {
    /// Songbird service endpoint
    pub songbird: String,
    /// `BearDog` service endpoint
    pub beardog: String,
    /// `NestGate` service endpoint
    pub nestgate: String,
    /// Squirrel MCP service endpoint
    pub squirrel: String,
    /// Federation endpoint
    pub federation: String,
    /// Metrics endpoint
    pub metrics: String,
    /// Health check endpoint
    pub health: String,
}

impl Default for EndpointConfig {
    fn default() -> Self {
        let config = crate::env_config::EnvironmentConfig::from_env();
        Self {
            songbird: network::default_songbird_endpoint(),
            beardog: network::default_beardog_endpoint(),
            nestgate: network::default_nestgate_endpoint(),
            squirrel: network::default_squirrel_endpoint(),
            federation: format!(
                "http://{}:{}",
                config.network.bind_address, config.network.federation_port
            ),
            metrics: format!(
                "http://{}:{}",
                config.network.bind_address, config.network.metrics_port
            ),
            health: format!(
                "http://{}:{}",
                config.network.bind_address, config.network.health_port
            ),
        }
    }
}

/// Connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Request timeout
    pub request_timeout: Duration,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Max retry attempts
    pub max_retries: u32,
    /// Keepalive interval
    pub keepalive_interval: Duration,
    /// Max connections per host
    pub max_connections_per_host: u32,
    /// Connection pool size
    pub pool_size: u32,
    /// Enable HTTP/2
    pub enable_http2: bool,
    /// Enable compression
    pub enable_compression: bool,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(network::DEFAULT_REQUEST_TIMEOUT_SECS),
            connection_timeout: Duration::from_secs(network::DEFAULT_CONNECTION_TIMEOUT_SECS),
            max_retries: network::DEFAULT_MAX_RETRIES,
            keepalive_interval: Duration::from_secs(network::DEFAULT_KEEPALIVE_INTERVAL_SECS),
            max_connections_per_host: network::DEFAULT_MAX_CONNECTIONS_PER_HOST,
            pool_size: 10,
            enable_http2: true,
            enable_compression: true,
        }
    }
}

/// TLS configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    /// Certificate file path
    pub cert_file: String,
    /// Private key file path
    pub key_file: String,
    /// CA certificate file path
    pub ca_file: Option<String>,
    /// Verify certificates
    pub verify_certs: bool,
    /// TLS version
    pub tls_version: String,
    /// Cipher suites
    pub cipher_suites: Vec<String>,
}

/// Runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeConfig {
    /// Execution timeout
    pub execution_timeout: Duration,
    /// Max concurrent executions
    pub max_concurrent_executions: u32,
    /// Resource limits
    pub resource_limits: ResourceLimits,
    /// Container settings
    pub container: ContainerConfig,
    /// WASM settings
    pub wasm: WasmConfig,
    /// Python settings
    pub python: PythonConfig,
    /// GPU settings
    pub gpu: Option<GpuConfig>,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            execution_timeout: Duration::from_secs(app::DEFAULT_EXECUTION_TIMEOUT_SECS),
            max_concurrent_executions: app::DEFAULT_MAX_CONCURRENT_EXECUTIONS,
            resource_limits: ResourceLimits::default(),
            container: ContainerConfig::default(),
            wasm: WasmConfig::default(),
            python: PythonConfig::default(),
            gpu: None,
        }
    }
}

/// Resource limits configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Max CPU usage percentage
    pub max_cpu_usage: f64,
    /// Max memory usage percentage
    pub max_memory_usage: f64,
    /// Max disk usage percentage
    pub max_disk_usage: f64,
    /// Max network bandwidth in bytes per second
    pub max_network_bandwidth: u64,
    /// Max open file descriptors
    pub max_open_files: u64,
    /// Max process count
    pub max_processes: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_cpu_usage: app::DEFAULT_MAX_CPU_USAGE,
            max_memory_usage: app::DEFAULT_MAX_MEMORY_USAGE,
            max_disk_usage: app::DEFAULT_MAX_DISK_USAGE,
            max_network_bandwidth: 1024 * 1024 * 1024, // 1 GB/s
            max_open_files: 1024,
            max_processes: 100,
        }
    }
}

/// Container runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerConfig {
    /// Container runtime (docker, podman, etc.)
    pub runtime: String,
    /// Default image registry
    pub default_registry: String,
    /// Port range for container ports
    pub port_range: (u16, u16),
    /// Network mode
    pub network_mode: String,
    /// Security options
    pub security_opts: Vec<String>,
    /// Volume mounts
    pub volume_mounts: Vec<String>,
    /// Environment variables
    pub environment: HashMap<String, String>,
}

impl Default for ContainerConfig {
    fn default() -> Self {
        Self {
            runtime: "docker".to_string(),
            default_registry: "docker.io".to_string(),
            port_range: crate::config_utils::ConfigUtils::get_container_port_range(),
            network_mode: "bridge".to_string(),
            security_opts: vec!["no-new-privileges".to_string()],
            volume_mounts: vec![],
            environment: HashMap::new(),
        }
    }
}

/// WASM runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WasmConfig {
    /// WASM runtime engine
    pub engine: String,
    /// Max memory size in bytes
    pub max_memory: u64,
    /// Max execution time in seconds
    pub max_execution_time: u64,
    /// Enable WASI
    pub enable_wasi: bool,
    /// WASI allowed directories
    pub wasi_allowed_dirs: Vec<String>,
    /// WASI environment variables
    pub wasi_env: HashMap<String, String>,
}

impl Default for WasmConfig {
    fn default() -> Self {
        Self {
            engine: "wasmtime".to_string(),
            max_memory: 64 * 1024 * 1024, // 64MB
            max_execution_time: 300,      // 5 minutes
            enable_wasi: true,
            wasi_allowed_dirs: vec!["/tmp".to_string()],
            wasi_env: HashMap::new(),
        }
    }
}

/// Python runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PythonConfig {
    /// Python executable path
    pub executable: String,
    /// Virtual environment path
    pub venv_path: Option<String>,
    /// Package index URL
    pub index_url: String,
    /// Max memory size in bytes
    pub max_memory: u64,
    /// Max execution time in seconds
    pub max_execution_time: u64,
    /// Allowed modules
    pub allowed_modules: Vec<String>,
    /// Restricted modules
    pub restricted_modules: Vec<String>,
}

impl Default for PythonConfig {
    fn default() -> Self {
        Self {
            executable: "python3".to_string(),
            venv_path: None,
            index_url: "https://pypi.org/simple".to_string(),
            max_memory: 128 * 1024 * 1024, // 128MB
            max_execution_time: 300,       // 5 minutes
            allowed_modules: vec!["numpy".to_string(), "pandas".to_string()],
            restricted_modules: vec!["os".to_string(), "subprocess".to_string()],
        }
    }
}

/// GPU runtime configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuConfig {
    /// GPU runtime (cuda, opencl, etc.)
    pub runtime: String,
    /// GPU device IDs to use
    pub device_ids: Vec<u32>,
    /// Max memory usage per device
    pub max_memory_per_device: u64,
    /// Max execution time in seconds
    pub max_execution_time: u64,
    /// Enable profiling
    pub enable_profiling: bool,
}

impl Default for GpuConfig {
    fn default() -> Self {
        Self {
            runtime: "cuda".to_string(),
            device_ids: vec![0],
            max_memory_per_device: 2 * 1024 * 1024 * 1024, // 2GB
            max_execution_time: 300,                       // 5 minutes
            enable_profiling: false,
        }
    }
}

/// Security configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SecurityConfig {
    /// Authentication settings
    pub auth: AuthConfig,
    /// Authorization settings
    pub authz: AuthzConfig,
    /// Encryption settings
    pub encryption: EncryptionConfig,
    /// Audit settings
    pub audit: AuditConfig,
    /// Sandbox settings
    pub sandbox: SandboxConfig,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Enable authentication
    pub enabled: bool,
    /// Authentication provider
    pub provider: String,
    /// JWT secret key
    pub jwt_secret: Option<String>,
    /// Session timeout
    pub session_timeout: Duration,
    /// Max login attempts
    pub max_login_attempts: u32,
    /// Lockout duration
    pub lockout_duration: Duration,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local".to_string(),
            jwt_secret: None,
            session_timeout: Duration::from_secs(app::DEFAULT_SESSION_TIMEOUT_SECS),
            max_login_attempts: 5,
            lockout_duration: Duration::from_secs(300), // 5 minutes
        }
    }
}

/// Authorization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthzConfig {
    /// Enable authorization
    pub enabled: bool,
    /// Authorization provider
    pub provider: String,
    /// Default permissions
    pub default_permissions: Vec<String>,
    /// Admin permissions
    pub admin_permissions: Vec<String>,
}

impl Default for AuthzConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            provider: "local".to_string(),
            default_permissions: vec!["read".to_string()],
            admin_permissions: vec!["read".to_string(), "write".to_string(), "admin".to_string()],
        }
    }
}

/// Encryption configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptionConfig {
    /// Enable encryption
    pub enabled: bool,
    /// Encryption algorithm
    pub algorithm: String,
    /// Key derivation function
    pub key_derivation: String,
    /// Key length in bytes
    pub key_length: usize,
    /// Encryption at rest
    pub encrypt_at_rest: bool,
    /// Encryption in transit
    pub encrypt_in_transit: bool,
}

impl Default for EncryptionConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            algorithm: "aes-256-gcm".to_string(),
            key_derivation: "pbkdf2".to_string(),
            key_length: app::DEFAULT_ENCRYPTION_KEY_LENGTH,
            encrypt_at_rest: false,
            encrypt_in_transit: true,
        }
    }
}

/// Audit configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditConfig {
    /// Enable audit logging
    pub enabled: bool,
    /// Audit log file path
    pub log_file: String,
    /// Audit log level
    pub log_level: String,
    /// Audit log format
    pub log_format: String,
    /// Audit log rotation
    pub log_rotation: bool,
    /// Max audit log size
    pub max_log_size: u64,
    /// Max audit log files
    pub max_log_files: u32,
}

impl Default for AuditConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            log_file: "audit.log".to_string(),
            log_level: "info".to_string(),
            log_format: "json".to_string(),
            log_rotation: true,
            max_log_size: app::DEFAULT_MAX_LOG_SIZE,
            max_log_files: app::DEFAULT_MAX_LOG_FILES,
        }
    }
}

/// Sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable sandboxing
    pub enabled: bool,
    /// Sandbox type
    pub sandbox_type: String,
    /// Allowed system calls
    pub allowed_syscalls: Vec<String>,
    /// Blocked system calls
    pub blocked_syscalls: Vec<String>,
    /// Allowed network access
    pub allow_network: bool,
    /// Allowed file access
    pub allow_file_access: bool,
    /// Allowed directories
    pub allowed_dirs: Vec<String>,
    /// Blocked directories
    pub blocked_dirs: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            sandbox_type: "seccomp".to_string(),
            allowed_syscalls: vec![
                "read".to_string(),
                "write".to_string(),
                "open".to_string(),
                "close".to_string(),
            ],
            blocked_syscalls: vec![
                "execve".to_string(),
                "fork".to_string(),
                "clone".to_string(),
            ],
            allow_network: false,
            allow_file_access: true,
            allowed_dirs: vec!["/tmp".to_string()],
            blocked_dirs: vec!["/etc".to_string(), "/proc".to_string(), "/sys".to_string()],
        }
    }
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level
    pub level: String,
    /// Log format
    pub format: String,
    /// Log to file
    pub log_to_file: bool,
    /// Log file path
    pub log_file: String,
    /// Log rotation
    pub log_rotation: bool,
    /// Max log size
    pub max_log_size: u64,
    /// Max log files
    pub max_log_files: u32,
    /// Enable colors
    pub enable_colors: bool,
    /// Enable timestamps
    pub enable_timestamps: bool,
    /// Enable thread IDs
    pub enable_thread_ids: bool,
    /// Enable module paths
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

/// Database configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    /// Database URL
    pub url: String,
    /// Database type
    pub database_type: String,
    /// Max connections
    pub max_connections: u32,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Query timeout
    pub query_timeout: Duration,
    /// Enable migrations
    pub enable_migrations: bool,
    /// Migration directory
    pub migration_dir: String,
}

/// Backend cache configuration for distributed caching systems
///
/// This is distinct from `toadstool::config_bases::CacheConfig` which is for
/// simple in-memory caching. This config is for distributed cache backends
/// like Redis, Memcached, etc. with compression and persistence support.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackendCacheConfig {
    /// Cache type (redis, memcached, memory, etc.)
    pub cache_type: String,
    /// Cache backend URL (for distributed caches)
    pub url: Option<String>,
    /// Max size in bytes
    pub max_size: u64,
    /// TTL in seconds
    pub ttl: Duration,
    /// Enable compression
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

/// Metrics configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    /// Enable metrics
    pub enabled: bool,
    /// Metrics endpoint
    pub endpoint: String,
    /// Metrics format
    pub format: String,
    /// Collection interval
    pub collection_interval: Duration,
    /// Retention period
    pub retention_period: Duration,
    /// Enable histograms
    pub enable_histograms: bool,
    /// Enable counters
    pub enable_counters: bool,
    /// Enable gauges
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

/// Feature flags configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureFlags {
    /// Enable experimental features
    pub enable_experimental: bool,
    /// Enable beta features
    pub enable_beta: bool,
    /// Enable debug features
    pub enable_debug: bool,
    /// Enable profiling features
    pub enable_profiling: bool,
    /// Enable distributed mode
    pub enable_distributed: bool,
    /// Enable federation
    pub enable_federation: bool,
    /// Enable WebSocket
    pub enable_websocket: bool,
    /// Enable GraphQL
    pub enable_graphql: bool,
    /// Enable gRPC
    pub enable_grpc: bool,
    /// Enable `OpenAPI`
    pub enable_openapi: bool,
    /// Enable auto-configuration
    pub enable_auto_config: bool,
    /// Enable hot reload
    pub enable_hot_reload: bool,
    /// Enable live reload
    pub enable_live_reload: bool,
    /// Enable watch mode
    pub enable_watch_mode: bool,
    /// Custom feature flags
    pub custom: HashMap<String, bool>,
}

impl Default for FeatureFlags {
    fn default() -> Self {
        Self {
            enable_experimental: false,
            enable_beta: false,
            enable_debug: cfg!(debug_assertions),
            enable_profiling: false,
            enable_distributed: true,
            enable_federation: true,
            enable_websocket: true,
            enable_graphql: false,
            enable_grpc: false,
            enable_openapi: true,
            enable_auto_config: true,
            enable_hot_reload: cfg!(debug_assertions),
            enable_live_reload: cfg!(debug_assertions),
            enable_watch_mode: cfg!(debug_assertions),
            custom: HashMap::new(),
        }
    }
}

/// Configuration loading and validation
impl ToadStoolConfig {
    /// Load configuration from file
    ///
    /// # Errors
    /// Returns an error if the configuration file cannot be read or parsed
    pub fn load_from_file<P: AsRef<std::path::Path>>(
        path: P,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let content = std::fs::read_to_string(path)?;
        let config: Self = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Load configuration from environment variables
    ///
    /// # Errors
    /// Returns an error if the environment variables cannot be parsed
    pub fn load_from_env() -> Result<Self, Box<dyn std::error::Error>> {
        let mut config = Self::default();

        // Override with environment variables
        if let Ok(env) = std::env::var("TOADSTOOL_ENVIRONMENT") {
            config.app.environment = env;
        }

        if let Ok(log_level) = std::env::var("TOADSTOOL_LOG_LEVEL") {
            config.logging.level = log_level;
        }

        if let Ok(bind_address) = std::env::var("TOADSTOOL_BIND_ADDRESS") {
            config.network.bind_address = bind_address.parse()?;
        }

        if let Ok(songbird_endpoint) = std::env::var("TOADSTOOL_SONGBIRD_ENDPOINT") {
            config.network.endpoints.songbird = songbird_endpoint;
        }

        config.validate()?;
        Ok(config)
    }

    /// Validate configuration
    ///
    /// # Errors
    /// Returns an error if the configuration is invalid
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Validate application configuration
        if self.app.name.is_empty() {
            return Err("Application name cannot be empty".into());
        }

        if self.app.worker_threads == 0 {
            return Err("Worker threads must be greater than 0".into());
        }

        // Validate network configuration
        if self.network.endpoints.songbird.is_empty() {
            return Err("Songbird endpoint cannot be empty".into());
        }

        // Validate runtime configuration
        if self.runtime.max_concurrent_executions == 0 {
            return Err("Max concurrent executions must be greater than 0".into());
        }

        // Validate resource limits
        if self.runtime.resource_limits.max_cpu_usage <= 0.0
            || self.runtime.resource_limits.max_cpu_usage > 100.0
        {
            return Err("Max CPU usage must be between 0 and 100".into());
        }

        if self.runtime.resource_limits.max_memory_usage <= 0.0
            || self.runtime.resource_limits.max_memory_usage > 100.0
        {
            return Err("Max memory usage must be between 0 and 100".into());
        }

        Ok(())
    }

    /// Get environment-specific configuration
    #[must_use]
    pub fn for_environment(mut self, environment: &str) -> Self {
        self.app.environment = environment.to_string();

        // Apply environment-specific defaults
        match environment {
            "development" => {
                self.logging.level = development::DEFAULT_DEV_LOG_LEVEL.to_string();
                self.logging.enable_colors = true;
                self.features.enable_debug = development::DEFAULT_DEV_DEBUG_MODE;
                self.features.enable_hot_reload = development::DEFAULT_DEV_HOT_RELOAD;
                self.security.auth.enabled = false;
            }
            "production" => {
                self.logging.level = production::DEFAULT_PROD_LOG_LEVEL.to_string();
                self.logging.enable_colors = production::DEFAULT_PROD_PRETTY_LOGS;
                self.features.enable_debug = production::DEFAULT_PROD_DEBUG_MODE;
                self.features.enable_hot_reload = production::DEFAULT_PROD_HOT_RELOAD;
                self.security.auth.enabled = true;
            }
            "test" => {
                self.logging.level = testing::DEFAULT_TEST_LOG_LEVEL.to_string();
                self.app.data_dir = testing::DEFAULT_TEST_DATA_DIR.to_string();
                self.app.cache_dir = testing::DEFAULT_TEST_CACHE_DIR.to_string();
                self.runtime.execution_timeout =
                    Duration::from_secs(testing::DEFAULT_TEST_EXECUTION_TIMEOUT_SECS);
                self.security.auth.enabled = false;
            }
            _ => {
                // Use default configuration
            }
        }

        self
    }

    /// Merge with override configuration
    #[must_use]
    pub fn merge(mut self, overrides: HashMap<String, serde_json::Value>) -> Self {
        self.overrides.extend(overrides);
        self
    }

    /// Get configuration value with override support
    #[must_use]
    pub fn get_override<T>(&self, key: &str, default: T) -> T
    where
        T: serde::de::DeserializeOwned + Clone,
    {
        self.overrides
            .get(key)
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or(default)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_configuration() {
        let config = ToadStoolConfig::default();
        assert_eq!(config.app.name, app::DEFAULT_APP_NAME);
        assert_eq!(config.app.environment, app::DEFAULT_ENVIRONMENT);
        assert_eq!(config.logging.level, app::DEFAULT_LOG_LEVEL);
        assert!(config.features.enable_websocket);
        assert!(config.features.enable_federation);
    }

    #[test]
    #[serial_test::serial]
    fn test_network_constants() {
        // Save original environment state
        let original_host = std::env::var("TOADSTOOL_BIND_HOST").ok();
        let original_addr = std::env::var("TOADSTOOL_BIND_ADDRESS").ok();
        let original_port = std::env::var("TOADSTOOL_SONGBIRD_PORT").ok();

        // Clear env vars to test defaults (both old BIND_HOST and new BIND_ADDRESS)
        std::env::remove_var("TOADSTOOL_BIND_HOST");
        std::env::remove_var("TOADSTOOL_BIND_ADDRESS");
        std::env::remove_var("TOADSTOOL_SONGBIRD_PORT");

        // Note: Deprecated constants were removed in 0.6.0
        // Now using EnvironmentConfig and ConfigUtils for all configuration

        let songbird_endpoint = network::default_songbird_endpoint();
        assert!(songbird_endpoint.contains("8080"));
        assert!(songbird_endpoint.contains("127.0.0.1"));

        // Restore original environment state
        match original_host {
            Some(val) => std::env::set_var("TOADSTOOL_BIND_HOST", val),
            None => std::env::remove_var("TOADSTOOL_BIND_HOST"),
        }
        match original_addr {
            Some(val) => std::env::set_var("TOADSTOOL_BIND_ADDRESS", val),
            None => std::env::remove_var("TOADSTOOL_BIND_ADDRESS"),
        }
        match original_port {
            Some(val) => std::env::set_var("TOADSTOOL_SONGBIRD_PORT", val),
            None => std::env::remove_var("TOADSTOOL_SONGBIRD_PORT"),
        }
    }

    #[test]
    fn test_configuration_validation() {
        let mut config = ToadStoolConfig::default();
        assert!(config.validate().is_ok());

        config.app.name = String::new();
        assert!(config.validate().is_err());

        config.app.name = "test".to_string();
        config.app.worker_threads = 0;
        assert!(config.validate().is_err());

        config.app.worker_threads = 4;
        config.runtime.resource_limits.max_cpu_usage = 150.0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_environment_specific_config() {
        let config = ToadStoolConfig::default().for_environment("development");
        assert_eq!(config.app.environment, "development");
        assert_eq!(config.logging.level, "debug");
        assert!(config.features.enable_debug);
        assert!(!config.security.auth.enabled);

        let config = ToadStoolConfig::default().for_environment("production");
        assert_eq!(config.app.environment, "production");
        assert_eq!(config.logging.level, "info");
        assert!(!config.features.enable_debug);
        assert!(config.security.auth.enabled);
    }

    #[test]
    fn test_configuration_overrides() {
        let mut overrides = HashMap::new();
        overrides.insert("custom_setting".to_string(), serde_json::Value::Bool(true));

        let config = ToadStoolConfig::default().merge(overrides);
        assert!(config.get_override("custom_setting", false));
        assert_eq!(config.get_override("non_existent", 42), 42);
    }
}
