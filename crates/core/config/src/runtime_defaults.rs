//! Runtime configuration defaults and validation
//!
//! This module provides default values and validation for runtime configuration
//! to eliminate hardcoded values scattered throughout the codebase.

use std::path::Path;
use std::time::Duration;

use tracing::{info, warn};

use crate::{BackendCacheConfig, MetricsConfig, ToadStoolConfig};

/// Configuration error type
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Invalid configuration: {0}")]
    Invalid(String),
    #[error("Missing required field: {0}")]
    MissingField(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Generic error: {0}")]
    Generic(#[from] Box<dyn std::error::Error>),
}

/// Configuration result type
pub type ConfigResult<T> = Result<T, ConfigError>;

/// Runtime configuration defaults
impl ToadStoolConfig {
    /// Create optimized configuration for development
    #[must_use]
    pub fn development() -> Self {
        Self::default().for_environment("development")
    }

    /// Create optimized configuration for production
    #[must_use]
    pub fn production() -> Self {
        Self::default().for_environment("production")
    }

    /// Create optimized configuration for testing
    #[must_use]
    pub fn testing() -> Self {
        Self::default().for_environment("test")
    }

    /// Apply environment variable overrides
    pub fn apply_env_overrides(&mut self) -> ConfigResult<()> {
        // Application overrides
        if let Ok(env_name) = std::env::var("TOADSTOOL_ENV") {
            self.app.environment = env_name;
        }

        if let Ok(debug) = std::env::var("TOADSTOOL_DEBUG") {
            self.features.enable_debug = debug.to_lowercase() == "true";
        }

        if let Ok(verbose) = std::env::var("TOADSTOOL_VERBOSE") {
            self.logging.level = if verbose.to_lowercase() == "true" {
                "debug".to_string()
            } else {
                "info".to_string()
            };
        }

        if let Ok(bind_address) = std::env::var("TOADSTOOL_BIND_ADDRESS") {
            self.network.bind_address = bind_address
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid bind address: {e}")))?;
        }

        if let Ok(port) = std::env::var("TOADSTOOL_PORT") {
            let port: u16 = port
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid port: {e}")))?;
            self.network.bind_address.set_port(port);
        }

        if let Ok(songbird_endpoint) = std::env::var("TOADSTOOL_SONGBIRD_ENDPOINT") {
            self.network.endpoints.songbird = songbird_endpoint;
        }

        if let Ok(beardog_endpoint) = std::env::var("TOADSTOOL_BEARDOG_ENDPOINT") {
            self.network.endpoints.beardog = beardog_endpoint;
        }

        if let Ok(nestgate_endpoint) = std::env::var("TOADSTOOL_NESTGATE_ENDPOINT") {
            self.network.endpoints.nestgate = nestgate_endpoint;
        }

        if let Ok(squirrel_endpoint) = std::env::var("TOADSTOOL_SQUIRREL_ENDPOINT") {
            self.network.endpoints.squirrel = squirrel_endpoint;
        }

        if let Ok(max_cpu) = std::env::var("TOADSTOOL_MAX_CPU") {
            self.runtime.resource_limits.max_cpu_usage = max_cpu
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid max CPU: {e}")))?;
        }

        if let Ok(max_memory) = std::env::var("TOADSTOOL_MAX_MEMORY") {
            self.runtime.resource_limits.max_memory_usage = max_memory
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid max memory: {e}")))?;
        }

        if let Ok(log_level) = std::env::var("TOADSTOOL_LOG_LEVEL") {
            self.logging.level = log_level;
        }

        if let Ok(data_dir) = std::env::var("TOADSTOOL_DATA_DIR") {
            self.app.data_dir = data_dir;
        }

        if let Ok(cache_dir) = std::env::var("TOADSTOOL_CACHE_DIR") {
            self.app.cache_dir = cache_dir;
        }

        if let Ok(worker_threads) = std::env::var("TOADSTOOL_WORKER_THREADS") {
            self.app.worker_threads = worker_threads
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid worker threads: {e}")))?;
        }

        if let Ok(max_concurrent) = std::env::var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS") {
            self.runtime.max_concurrent_executions = max_concurrent.parse().map_err(|e| {
                ConfigError::Invalid(format!("Invalid max concurrent executions: {e}"))
            })?;
        }

        if let Ok(timeout) = std::env::var("TOADSTOOL_EXECUTION_TIMEOUT") {
            let timeout_secs: u64 = timeout
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid execution timeout: {e}")))?;
            self.runtime.execution_timeout = Duration::from_secs(timeout_secs);
        }

        if let Ok(request_timeout) = std::env::var("TOADSTOOL_REQUEST_TIMEOUT") {
            let timeout_secs: u64 = request_timeout
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid request timeout: {e}")))?;
            self.network.connection.request_timeout = Duration::from_secs(timeout_secs);
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_METRICS") {
            self.metrics = if enabled.to_lowercase() == "true" {
                Some(MetricsConfig::default())
            } else {
                None
            };
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_CACHE") {
            self.cache = if enabled.to_lowercase() == "true" {
                Some(BackendCacheConfig::default())
            } else {
                None
            };
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_AUTH") {
            self.security.auth.enabled = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_SANDBOX") {
            self.security.sandbox.enabled = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_WEBSOCKET") {
            self.features.enable_websocket = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_FEDERATION") {
            self.features.enable_federation = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_DISTRIBUTED") {
            self.features.enable_distributed = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_AUTO_CONFIG") {
            self.features.enable_auto_config = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_HOT_RELOAD") {
            self.features.enable_hot_reload = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_EXPERIMENTAL") {
            self.features.enable_experimental = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_BETA") {
            self.features.enable_beta = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_PROFILING") {
            self.features.enable_profiling = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_OPENAPI") {
            self.features.enable_openapi = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_GRPC") {
            self.features.enable_grpc = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_GRAPHQL") {
            self.features.enable_graphql = enabled.to_lowercase() == "true";
        }

        if let Ok(container_runtime) = std::env::var("TOADSTOOL_CONTAINER_RUNTIME") {
            self.runtime.container.runtime = container_runtime;
        }

        if let Ok(registry) = std::env::var("TOADSTOOL_CONTAINER_REGISTRY") {
            self.runtime.container.default_registry = registry;
        }

        if let Ok(network_mode) = std::env::var("TOADSTOOL_CONTAINER_NETWORK_MODE") {
            self.runtime.container.network_mode = network_mode;
        }

        if let Ok(wasm_engine) = std::env::var("TOADSTOOL_WASM_ENGINE") {
            self.runtime.wasm.engine = wasm_engine;
        }

        if let Ok(max_memory) = std::env::var("TOADSTOOL_WASM_MAX_MEMORY") {
            self.runtime.wasm.max_memory = max_memory
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid WASM max memory: {e}")))?;
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_WASM_ENABLE_WASI") {
            self.runtime.wasm.enable_wasi = enabled.to_lowercase() == "true";
        }

        if let Ok(python_exe) = std::env::var("TOADSTOOL_PYTHON_EXECUTABLE") {
            self.runtime.python.executable = python_exe;
        }

        if let Ok(venv_path) = std::env::var("TOADSTOOL_PYTHON_VENV_PATH") {
            self.runtime.python.venv_path = Some(venv_path);
        }

        if let Ok(index_url) = std::env::var("TOADSTOOL_PYTHON_INDEX_URL") {
            self.runtime.python.index_url = index_url;
        }

        if let Ok(max_memory) = std::env::var("TOADSTOOL_PYTHON_MAX_MEMORY") {
            self.runtime.python.max_memory = max_memory
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid Python max memory: {e}")))?;
        }

        if let Ok(jwt_secret) = std::env::var("TOADSTOOL_JWT_SECRET") {
            self.security.auth.jwt_secret = Some(jwt_secret);
        }

        if let Ok(session_timeout) = std::env::var("TOADSTOOL_SESSION_TIMEOUT") {
            let timeout_secs: u64 = session_timeout
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid session timeout: {e}")))?;
            self.security.auth.session_timeout = Duration::from_secs(timeout_secs);
        }

        if let Ok(max_attempts) = std::env::var("TOADSTOOL_MAX_LOGIN_ATTEMPTS") {
            self.security.auth.max_login_attempts = max_attempts
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid max login attempts: {e}")))?;
        }

        if let Ok(lockout_duration) = std::env::var("TOADSTOOL_LOCKOUT_DURATION") {
            let duration_secs: u64 = lockout_duration
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid lockout duration: {e}")))?;
            self.security.auth.lockout_duration = Duration::from_secs(duration_secs);
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENCRYPTION_ENABLED") {
            self.security.encryption.enabled = enabled.to_lowercase() == "true";
        }

        if let Ok(algorithm) = std::env::var("TOADSTOOL_ENCRYPTION_ALGORITHM") {
            self.security.encryption.algorithm = algorithm;
        }

        if let Ok(key_length) = std::env::var("TOADSTOOL_ENCRYPTION_KEY_LENGTH") {
            self.security.encryption.key_length = key_length
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid encryption key length: {e}")))?;
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_AUDIT_ENABLED") {
            self.security.audit.enabled = enabled.to_lowercase() == "true";
        }

        if let Ok(log_file) = std::env::var("TOADSTOOL_AUDIT_LOG_FILE") {
            self.security.audit.log_file = log_file;
        }

        if let Ok(log_level) = std::env::var("TOADSTOOL_AUDIT_LOG_LEVEL") {
            self.security.audit.log_level = log_level;
        }

        if let Ok(sandbox_type) = std::env::var("TOADSTOOL_SANDBOX_TYPE") {
            self.security.sandbox.sandbox_type = sandbox_type;
        }

        if let Ok(allow_network) = std::env::var("TOADSTOOL_SANDBOX_ALLOW_NETWORK") {
            self.security.sandbox.allow_network = allow_network.to_lowercase() == "true";
        }

        if let Ok(allow_file_access) = std::env::var("TOADSTOOL_SANDBOX_ALLOW_FILE_ACCESS") {
            self.security.sandbox.allow_file_access = allow_file_access.to_lowercase() == "true";
        }

        if let Ok(log_to_file) = std::env::var("TOADSTOOL_LOG_TO_FILE") {
            self.logging.log_to_file = log_to_file.to_lowercase() == "true";
        }

        if let Ok(log_file) = std::env::var("TOADSTOOL_LOG_FILE") {
            self.logging.log_file = log_file;
        }

        if let Ok(log_format) = std::env::var("TOADSTOOL_LOG_FORMAT") {
            self.logging.format = log_format;
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_COLORS") {
            self.logging.enable_colors = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_TIMESTAMPS") {
            self.logging.enable_timestamps = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_THREAD_IDS") {
            self.logging.enable_thread_ids = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_MODULE_PATHS") {
            self.logging.enable_module_paths = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_ROTATION") {
            self.logging.log_rotation = enabled.to_lowercase() == "true";
        }

        if let Ok(max_size) = std::env::var("TOADSTOOL_LOG_MAX_SIZE") {
            self.logging.max_log_size = max_size
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid log max size: {e}")))?;
        }

        if let Ok(max_files) = std::env::var("TOADSTOOL_LOG_MAX_FILES") {
            self.logging.max_log_files = max_files
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid log max files: {e}")))?;
        }

        Ok(())
    }

    /// Validate configuration values
    pub fn validate_runtime_config(&self) -> ConfigResult<()> {
        // Validate bind address
        if self.network.bind_address.port() == 0 {
            return Err(ConfigError::Invalid(
                "Bind address port cannot be 0".to_string(),
            ));
        }

        // Validate endpoints
        if self.network.endpoints.songbird.is_empty() {
            return Err(ConfigError::Invalid(
                "Songbird endpoint cannot be empty".to_string(),
            ));
        }

        if self.network.endpoints.beardog.is_empty() {
            return Err(ConfigError::Invalid(
                "BearDog endpoint cannot be empty".to_string(),
            ));
        }

        if self.network.endpoints.nestgate.is_empty() {
            return Err(ConfigError::Invalid(
                "NestGate endpoint cannot be empty".to_string(),
            ));
        }

        if self.network.endpoints.squirrel.is_empty() {
            return Err(ConfigError::Invalid(
                "Squirrel endpoint cannot be empty".to_string(),
            ));
        }

        // Validate resource limits
        if self.runtime.resource_limits.max_cpu_usage <= 0.0
            || self.runtime.resource_limits.max_cpu_usage > 100.0
        {
            return Err(ConfigError::Invalid(
                "Max CPU usage must be between 0 and 100".to_string(),
            ));
        }

        if self.runtime.resource_limits.max_memory_usage <= 0.0
            || self.runtime.resource_limits.max_memory_usage > 100.0
        {
            return Err(ConfigError::Invalid(
                "Max memory usage must be between 0 and 100".to_string(),
            ));
        }

        if self.runtime.resource_limits.max_disk_usage <= 0.0
            || self.runtime.resource_limits.max_disk_usage > 100.0
        {
            return Err(ConfigError::Invalid(
                "Max disk usage must be between 0 and 100".to_string(),
            ));
        }

        // Validate application settings
        if self.app.name.is_empty() {
            return Err(ConfigError::Invalid(
                "Application name cannot be empty".to_string(),
            ));
        }

        if self.app.worker_threads == 0 {
            return Err(ConfigError::Invalid(
                "Worker threads must be greater than 0".to_string(),
            ));
        }

        if self.app.queue_size == 0 {
            return Err(ConfigError::Invalid(
                "Queue size must be greater than 0".to_string(),
            ));
        }

        if self.app.batch_size == 0 {
            return Err(ConfigError::Invalid(
                "Batch size must be greater than 0".to_string(),
            ));
        }

        // Validate runtime settings
        if self.runtime.max_concurrent_executions == 0 {
            return Err(ConfigError::Invalid(
                "Max concurrent executions must be greater than 0".to_string(),
            ));
        }

        if self.runtime.execution_timeout.is_zero() {
            return Err(ConfigError::Invalid(
                "Execution timeout must be greater than 0".to_string(),
            ));
        }

        // Validate network settings
        if self.network.connection.request_timeout.is_zero() {
            return Err(ConfigError::Invalid(
                "Request timeout must be greater than 0".to_string(),
            ));
        }

        if self.network.connection.connection_timeout.is_zero() {
            return Err(ConfigError::Invalid(
                "Connection timeout must be greater than 0".to_string(),
            ));
        }

        if self.network.connection.max_retries == 0 {
            return Err(ConfigError::Invalid(
                "Max retries must be greater than 0".to_string(),
            ));
        }

        if self.network.connection.max_connections_per_host == 0 {
            return Err(ConfigError::Invalid(
                "Max connections per host must be greater than 0".to_string(),
            ));
        }

        // Validate container settings
        if self.runtime.container.runtime.is_empty() {
            return Err(ConfigError::Invalid(
                "Container runtime cannot be empty".to_string(),
            ));
        }

        if self.runtime.container.default_registry.is_empty() {
            return Err(ConfigError::Invalid(
                "Default registry cannot be empty".to_string(),
            ));
        }

        if self.runtime.container.port_range.0 >= self.runtime.container.port_range.1 {
            return Err(ConfigError::Invalid(
                "Container port range start must be less than end".to_string(),
            ));
        }

        // Validate WASM settings
        if self.runtime.wasm.engine.is_empty() {
            return Err(ConfigError::Invalid(
                "WASM engine cannot be empty".to_string(),
            ));
        }

        if self.runtime.wasm.max_memory == 0 {
            return Err(ConfigError::Invalid(
                "WASM max memory must be greater than 0".to_string(),
            ));
        }

        if self.runtime.wasm.max_execution_time == 0 {
            return Err(ConfigError::Invalid(
                "WASM max execution time must be greater than 0".to_string(),
            ));
        }

        // Validate Python settings
        if self.runtime.python.executable.is_empty() {
            return Err(ConfigError::Invalid(
                "Python executable cannot be empty".to_string(),
            ));
        }

        if self.runtime.python.index_url.is_empty() {
            return Err(ConfigError::Invalid(
                "Python index URL cannot be empty".to_string(),
            ));
        }

        if self.runtime.python.max_memory == 0 {
            return Err(ConfigError::Invalid(
                "Python max memory must be greater than 0".to_string(),
            ));
        }

        if self.runtime.python.max_execution_time == 0 {
            return Err(ConfigError::Invalid(
                "Python max execution time must be greater than 0".to_string(),
            ));
        }

        // Validate logging settings
        if self.logging.level.is_empty() {
            return Err(ConfigError::Invalid(
                "Log level cannot be empty".to_string(),
            ));
        }

        if self.logging.format.is_empty() {
            return Err(ConfigError::Invalid(
                "Log format cannot be empty".to_string(),
            ));
        }

        if self.logging.max_log_size == 0 {
            return Err(ConfigError::Invalid(
                "Max log size must be greater than 0".to_string(),
            ));
        }

        if self.logging.max_log_files == 0 {
            return Err(ConfigError::Invalid(
                "Max log files must be greater than 0".to_string(),
            ));
        }

        // Validate security settings
        if self.security.auth.enabled && self.security.auth.jwt_secret.is_none() {
            return Err(ConfigError::Invalid(
                "JWT secret is required when authentication is enabled".to_string(),
            ));
        }

        if self.security.auth.session_timeout.is_zero() {
            return Err(ConfigError::Invalid(
                "Session timeout must be greater than 0".to_string(),
            ));
        }

        if self.security.auth.max_login_attempts == 0 {
            return Err(ConfigError::Invalid(
                "Max login attempts must be greater than 0".to_string(),
            ));
        }

        if self.security.auth.lockout_duration.is_zero() {
            return Err(ConfigError::Invalid(
                "Lockout duration must be greater than 0".to_string(),
            ));
        }

        if self.security.encryption.enabled && self.security.encryption.algorithm.is_empty() {
            return Err(ConfigError::Invalid(
                "Encryption algorithm is required when encryption is enabled".to_string(),
            ));
        }

        if self.security.encryption.key_length == 0 {
            return Err(ConfigError::Invalid(
                "Encryption key length must be greater than 0".to_string(),
            ));
        }

        if self.security.sandbox.enabled && self.security.sandbox.sandbox_type.is_empty() {
            return Err(ConfigError::Invalid(
                "Sandbox type is required when sandboxing is enabled".to_string(),
            ));
        }

        // Validate cache settings
        if let Some(cache_config) = &self.cache {
            if cache_config.cache_type.is_empty() {
                return Err(ConfigError::Invalid(
                    "Cache type cannot be empty".to_string(),
                ));
            }

            if cache_config.max_size == 0 {
                return Err(ConfigError::Invalid(
                    "Cache max size must be greater than 0".to_string(),
                ));
            }

            if cache_config.ttl.is_zero() {
                return Err(ConfigError::Invalid(
                    "Cache TTL must be greater than 0".to_string(),
                ));
            }
        }

        // Validate metrics settings
        if let Some(metrics_config) = &self.metrics {
            if metrics_config.endpoint.is_empty() {
                return Err(ConfigError::Invalid(
                    "Metrics endpoint cannot be empty".to_string(),
                ));
            }

            if metrics_config.format.is_empty() {
                return Err(ConfigError::Invalid(
                    "Metrics format cannot be empty".to_string(),
                ));
            }

            if metrics_config.collection_interval.is_zero() {
                return Err(ConfigError::Invalid(
                    "Metrics collection interval must be greater than 0".to_string(),
                ));
            }
        }

        // Validate database settings
        if let Some(database_config) = &self.database {
            if database_config.url.is_empty() {
                return Err(ConfigError::Invalid(
                    "Database URL cannot be empty".to_string(),
                ));
            }

            if database_config.database_type.is_empty() {
                return Err(ConfigError::Invalid(
                    "Database type cannot be empty".to_string(),
                ));
            }

            if database_config.max_connections == 0 {
                return Err(ConfigError::Invalid(
                    "Database max connections must be greater than 0".to_string(),
                ));
            }

            if database_config.connection_timeout.is_zero() {
                return Err(ConfigError::Invalid(
                    "Database connection timeout must be greater than 0".to_string(),
                ));
            }

            if database_config.query_timeout.is_zero() {
                return Err(ConfigError::Invalid(
                    "Database query timeout must be greater than 0".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Get optimized configuration for current environment
    #[must_use]
    pub fn for_current_environment() -> Self {
        let environment = std::env::var("TOADSTOOL_ENVIRONMENT")
            .or_else(|_| std::env::var("TOADSTOOL_ENV"))
            .or_else(|_| std::env::var("ENVIRONMENT"))
            .or_else(|_| std::env::var("ENV"))
            .unwrap_or_else(|_| "development".to_string());

        let mut config = Self::default().for_environment(&environment);

        // Apply environment variable overrides
        if let Err(e) = config.apply_env_overrides() {
            warn!("Failed to apply environment overrides: {}", e);
        }

        config
    }

    /// Load configuration from file with environment overrides
    ///
    /// # Errors
    /// Returns an error if the configuration file cannot be loaded or is invalid
    pub fn load_with_overrides<P: AsRef<Path>>(path: P) -> ConfigResult<Self> {
        let mut config = Self::load_from_file(path)?;
        config.apply_env_overrides()?;
        config.validate_runtime_config()?;
        Ok(config)
    }

    /// Load configuration from environment only
    ///
    /// # Errors
    /// Returns an error if the environment variables cannot be parsed
    pub fn load_from_env_only() -> ConfigResult<Self> {
        let mut config = Self::for_current_environment();
        config.apply_env_overrides()?;
        config.validate_runtime_config()?;
        Ok(config)
    }

    /// Save configuration to file
    ///
    /// # Errors
    /// Returns an error if the configuration file cannot be written
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> ConfigResult<()> {
        let content = toml::to_string_pretty(self)
            .map_err(|e| ConfigError::Invalid(format!("Failed to serialize config: {e}")))?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// Convert to JSON for API responses
    ///
    /// # Errors
    /// Returns an error if the configuration cannot be serialized to JSON
    pub fn to_json(&self) -> ConfigResult<String> {
        serde_json::to_string_pretty(self).map_err(ConfigError::Json)
    }

    /// Print configuration summary
    pub fn print_summary(&self) {
        info!("🍄 ToadStool Configuration Summary");
        info!("  Environment: {}", self.app.environment);
        info!("  Bind Address: {}", self.network.bind_address);
        info!("  Log Level: {}", self.logging.level);
        info!("  Worker Threads: {}", self.app.worker_threads);
        info!(
            "  Max Concurrent Executions: {}",
            self.runtime.max_concurrent_executions
        );
        info!("  Execution Timeout: {:?}", self.runtime.execution_timeout);
        info!("  Features:");
        info!("    WebSocket: {}", self.features.enable_websocket);
        info!("    Federation: {}", self.features.enable_federation);
        info!("    Distributed: {}", self.features.enable_distributed);
        info!("    Auto-Config: {}", self.features.enable_auto_config);
        info!("    Debug: {}", self.features.enable_debug);
        info!("  External Services:");
        info!("    Songbird: {}", self.network.endpoints.songbird);
        info!("    BearDog: {}", self.network.endpoints.beardog);
        info!("    NestGate: {}", self.network.endpoints.nestgate);
        info!("    Squirrel: {}", self.network.endpoints.squirrel);
        info!("  Security:");
        info!("    Authentication: {}", self.security.auth.enabled);
        info!("    Encryption: {}", self.security.encryption.enabled);
        info!("    Sandbox: {}", self.security.sandbox.enabled);
        info!("    Audit: {}", self.security.audit.enabled);
        info!("  Runtime:");
        info!("    Container Runtime: {}", self.runtime.container.runtime);
        info!("    WASM Engine: {}", self.runtime.wasm.engine);
        info!("    Python Executable: {}", self.runtime.python.executable);

        if let Some(cache_config) = &self.cache {
            info!("  Cache:");
            info!("    Type: {}", cache_config.cache_type);
            info!("    Max Size: {} bytes", cache_config.max_size);
            info!("    TTL: {:?}", cache_config.ttl);
        }

        if let Some(metrics_config) = &self.metrics {
            info!("  Metrics:");
            info!("    Endpoint: {}", metrics_config.endpoint);
            info!("    Format: {}", metrics_config.format);
            info!(
                "    Collection Interval: {:?}",
                metrics_config.collection_interval
            );
        }

        if let Some(database_config) = &self.database {
            info!("  Database:");
            info!("    Type: {}", database_config.database_type);
            info!("    Max Connections: {}", database_config.max_connections);
            info!(
                "    Connection Timeout: {:?}",
                database_config.connection_timeout
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;

    #[test]
    fn test_development_config() {
        let config = ToadStoolConfig::development();
        assert_eq!(config.app.environment, "development");
        assert_eq!(config.logging.level, "debug");
        assert!(config.features.enable_debug);
        assert!(config.features.enable_hot_reload);
        assert!(!config.security.auth.enabled);
    }

    #[test]
    fn test_production_config() {
        let config = ToadStoolConfig::production();
        assert_eq!(config.app.environment, "production");
        assert_eq!(config.logging.level, "info");
        assert!(!config.features.enable_debug);
        assert!(!config.features.enable_hot_reload);
        assert!(config.security.auth.enabled);
    }

    #[test]
    fn test_testing_config() {
        let config = ToadStoolConfig::testing();
        assert_eq!(config.app.environment, "test");
        assert_eq!(config.logging.level, "debug");
        assert!(!config.security.auth.enabled);
    }

    #[test]
    #[serial_test::serial]
    fn test_env_overrides() {
        // Clean up first to ensure clean state
        // Save original environment state
        let original_env = env::var("TOADSTOOL_ENV").ok();
        let original_debug = env::var("TOADSTOOL_DEBUG").ok();
        let original_log_level = env::var("TOADSTOOL_LOG_LEVEL").ok();
        let original_threads = env::var("TOADSTOOL_WORKER_THREADS").ok();
        let original_endpoint = env::var("TOADSTOOL_SONGBIRD_ENDPOINT").ok();
        let original_bind_host = env::var("TOADSTOOL_BIND_HOST").ok();

        // Set test values (ensure BIND_HOST is set to valid value)
        env::set_var("TOADSTOOL_ENV", "test");
        env::set_var("TOADSTOOL_DEBUG", "true");
        env::set_var("TOADSTOOL_LOG_LEVEL", "debug");
        env::set_var("TOADSTOOL_WORKER_THREADS", "8");
        env::set_var("TOADSTOOL_SONGBIRD_ENDPOINT", "http://localhost:8080");
        env::set_var("TOADSTOOL_BIND_HOST", "127.0.0.1");

        let mut config = ToadStoolConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.app.environment, "test");
        assert!(config.features.enable_debug);
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.app.worker_threads, 8);
        assert_eq!(config.network.endpoints.songbird, "http://localhost:8080");

        // Restore original environment state
        match original_env {
            Some(val) => env::set_var("TOADSTOOL_ENV", val),
            None => env::remove_var("TOADSTOOL_ENV"),
        }
        match original_debug {
            Some(val) => env::set_var("TOADSTOOL_DEBUG", val),
            None => env::remove_var("TOADSTOOL_DEBUG"),
        }
        match original_log_level {
            Some(val) => env::set_var("TOADSTOOL_LOG_LEVEL", val),
            None => env::remove_var("TOADSTOOL_LOG_LEVEL"),
        }
        match original_threads {
            Some(val) => env::set_var("TOADSTOOL_WORKER_THREADS", val),
            None => env::remove_var("TOADSTOOL_WORKER_THREADS"),
        }
        match original_endpoint {
            Some(val) => env::set_var("TOADSTOOL_SONGBIRD_ENDPOINT", val),
            None => env::remove_var("TOADSTOOL_SONGBIRD_ENDPOINT"),
        }
        match original_bind_host {
            Some(val) => env::set_var("TOADSTOOL_BIND_HOST", val),
            None => env::remove_var("TOADSTOOL_BIND_HOST"),
        }
    }

    #[test]
    fn test_config_validation() {
        let config = ToadStoolConfig::default();
        assert!(config.validate_runtime_config().is_ok());

        let mut invalid_config = config.clone();
        invalid_config.app.name = String::new();
        assert!(invalid_config.validate_runtime_config().is_err());

        let mut invalid_config = config.clone();
        invalid_config.app.worker_threads = 0;
        assert!(invalid_config.validate_runtime_config().is_err());

        let mut invalid_config = config.clone();
        invalid_config.runtime.resource_limits.max_cpu_usage = 150.0;
        assert!(invalid_config.validate_runtime_config().is_err());

        let mut invalid_config = config.clone();
        invalid_config.runtime.max_concurrent_executions = 0;
        assert!(invalid_config.validate_runtime_config().is_err());
    }

    #[test]
    fn test_config_file_operations() {
        let config = ToadStoolConfig::development();
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();

        // Test save
        config.save_to_file(temp_path).unwrap();

        // Test load
        let loaded_config = ToadStoolConfig::load_from_file(temp_path).unwrap();
        assert_eq!(loaded_config.app.environment, config.app.environment);
        assert_eq!(loaded_config.logging.level, config.logging.level);
        assert_eq!(loaded_config.app.worker_threads, config.app.worker_threads);
    }

    #[test]
    fn test_config_json_serialization() {
        let config = ToadStoolConfig::default();
        let json = config.to_json().unwrap();
        assert!(json.contains("app"));
        assert!(json.contains("network"));
        assert!(json.contains("runtime"));
        assert!(json.contains("security"));
        assert!(json.contains("logging"));
    }

    #[test]
    #[serial_test::serial]
    fn test_current_environment_detection() {
        // Save original environment state
        let original_toadstool_env = env::var("TOADSTOOL_ENVIRONMENT").ok();
        let original_env = env::var("ENVIRONMENT").ok();
        let original_toadstool_env_short = env::var("TOADSTOOL_ENV").ok();
        let original_env_short = env::var("ENV").ok();

        // Set all environment variables to ensure consistent state
        // Must set all variants to same value to prevent apply_env_overrides from changing it
        env::set_var("TOADSTOOL_ENVIRONMENT", "production");
        env::set_var("TOADSTOOL_ENV", "production");
        env::set_var("ENVIRONMENT", "production");
        env::set_var("ENV", "production");

        let config = ToadStoolConfig::for_current_environment();
        assert_eq!(config.app.environment, "production");

        // Test with different env var - set all to same value
        env::set_var("TOADSTOOL_ENVIRONMENT", "staging");
        env::set_var("TOADSTOOL_ENV", "staging");
        env::set_var("ENVIRONMENT", "staging");
        env::set_var("ENV", "staging");

        let config = ToadStoolConfig::for_current_environment();
        assert_eq!(config.app.environment, "staging");

        // Restore original environment state
        match original_toadstool_env {
            Some(val) => env::set_var("TOADSTOOL_ENVIRONMENT", val),
            None => env::remove_var("TOADSTOOL_ENVIRONMENT"),
        }
        match original_env {
            Some(val) => env::set_var("ENVIRONMENT", val),
            None => env::remove_var("ENVIRONMENT"),
        }
        match original_toadstool_env_short {
            Some(val) => env::set_var("TOADSTOOL_ENV", val),
            None => env::remove_var("TOADSTOOL_ENV"),
        }
        match original_env_short {
            Some(val) => env::set_var("ENV", val),
            None => env::remove_var("ENV"),
        }
    }
}
