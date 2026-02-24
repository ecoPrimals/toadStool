//! Configuration validation
//!
//! Validates ToadStool configuration values to ensure they are within acceptable ranges

use crate::{ConfigError, ConfigResult, ToadStoolConfig};

impl ToadStoolConfig {
    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if validation fails for:
    /// - Invalid port numbers (port == 0)
    /// - Invalid worker thread counts
    /// - Invalid timeout values
    pub fn validate_runtime_config(&self) -> ConfigResult<()> {
        // Bind address port 0 is valid (OS-assigned at bind time)

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

        // index_url may be empty (discovered at runtime); must be set before package installs

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
}
