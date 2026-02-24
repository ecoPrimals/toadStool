//! Configuration validation
//!
//! Validates ToadStool configuration values to ensure they are within acceptable ranges

use super::{ConfigError, ConfigResult};
use crate::ToadStoolConfig;

impl ToadStoolConfig {
    /// Validate configuration values
    ///
    /// # Errors
    ///
    /// Returns an error if any configuration value is invalid:
    /// - Port numbers are 0 or out of valid range
    /// - Resource limits are outside 0-100% range
    /// - Required fields (endpoints, names) are empty
    /// - Thread counts are 0
    /// - Timeout values are 0
    /// - Port ranges are invalid (start >= end)
    pub fn validate_runtime_config(&self) -> ConfigResult<()> {
        // Bind address port 0 is valid (OS-assigned at bind time)

        // Validate legacy endpoints (deprecated - use capability-based discovery)
        // These validations are kept for backward compatibility only
        #[allow(deprecated)]
        {
            if self.network.endpoints.songbird.is_empty() {
                return Err(ConfigError::Invalid(
                    "Songbird endpoint cannot be empty (use capability-based discovery instead)"
                        .to_string(),
                ));
            }

            if self.network.endpoints.beardog.is_empty() {
                return Err(ConfigError::Invalid(
                    "BearDog endpoint cannot be empty (use capability-based discovery instead)"
                        .to_string(),
                ));
            }

            if self.network.endpoints.nestgate.is_empty() {
                return Err(ConfigError::Invalid(
                    "NestGate endpoint cannot be empty (use capability-based discovery instead)"
                        .to_string(),
                ));
            }

            if self.network.endpoints.squirrel.is_empty() {
                return Err(ConfigError::Invalid(
                    "Squirrel endpoint cannot be empty (use capability-based discovery instead)"
                        .to_string(),
                ));
            }
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

#[cfg(test)]
mod tests {
    use crate::{BackendCacheConfig, DatabaseConfig, MetricsConfig, ToadStoolConfig};
    use std::time::Duration;

    fn valid_config() -> ToadStoolConfig {
        ToadStoolConfig::default()
    }

    #[test]
    fn test_valid_config_passes() {
        let config = valid_config();
        assert!(config.validate_runtime_config().is_ok());
    }

    #[test]
    fn test_bind_address_port_zero_allowed() {
        let mut config = valid_config();
        config.network.bind_address = "127.0.0.1:0".parse().unwrap();
        // Port 0 is valid (OS-assigned at bind time)
        assert!(config.validate_runtime_config().is_ok());
    }

    #[test]
    #[allow(deprecated)]
    fn test_empty_songbird_endpoint() {
        let mut config = valid_config();
        config.network.endpoints.songbird = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Songbird endpoint"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_empty_beardog_endpoint() {
        let mut config = valid_config();
        config.network.endpoints.beardog = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("BearDog endpoint"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_empty_nestgate_endpoint() {
        let mut config = valid_config();
        config.network.endpoints.nestgate = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("NestGate endpoint"));
    }

    #[test]
    #[allow(deprecated)]
    fn test_empty_squirrel_endpoint() {
        let mut config = valid_config();
        config.network.endpoints.squirrel = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Squirrel endpoint"));
    }

    #[test]
    fn test_cpu_usage_invalid() {
        let mut config = valid_config();
        config.runtime.resource_limits.max_cpu_usage = 150.0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max CPU usage"));
    }

    #[test]
    fn test_memory_usage_invalid() {
        let mut config = valid_config();
        config.runtime.resource_limits.max_memory_usage = 0.0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max memory usage"));
    }

    #[test]
    fn test_disk_usage_invalid() {
        let mut config = valid_config();
        config.runtime.resource_limits.max_disk_usage = 101.0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max disk usage"));
    }

    #[test]
    fn test_empty_app_name() {
        let mut config = valid_config();
        config.app.name = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Application name"));
    }

    #[test]
    fn test_zero_worker_threads() {
        let mut config = valid_config();
        config.app.worker_threads = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Worker threads"));
    }

    #[test]
    fn test_zero_queue_size() {
        let mut config = valid_config();
        config.app.queue_size = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Queue size"));
    }

    #[test]
    fn test_zero_batch_size() {
        let mut config = valid_config();
        config.app.batch_size = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Batch size"));
    }

    #[test]
    fn test_zero_max_concurrent_executions() {
        let mut config = valid_config();
        config.runtime.max_concurrent_executions = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max concurrent executions"));
    }

    #[test]
    fn test_zero_execution_timeout() {
        let mut config = valid_config();
        config.runtime.execution_timeout = Duration::ZERO;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Execution timeout"));
    }

    #[test]
    fn test_zero_request_timeout() {
        let mut config = valid_config();
        config.network.connection.request_timeout = Duration::ZERO;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Request timeout"));
    }

    #[test]
    fn test_zero_connection_timeout() {
        let mut config = valid_config();
        config.network.connection.connection_timeout = Duration::ZERO;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Connection timeout"));
    }

    #[test]
    fn test_zero_max_retries() {
        let mut config = valid_config();
        config.network.connection.max_retries = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max retries"));
    }

    #[test]
    fn test_zero_max_connections_per_host() {
        let mut config = valid_config();
        config.network.connection.max_connections_per_host = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max connections per host"));
    }

    #[test]
    fn test_empty_container_runtime() {
        let mut config = valid_config();
        config.runtime.container.runtime = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Container runtime"));
    }

    #[test]
    fn test_empty_default_registry() {
        let mut config = valid_config();
        config.runtime.container.default_registry = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Default registry"));
    }

    #[test]
    fn test_invalid_port_range() {
        let mut config = valid_config();
        config.runtime.container.port_range = (9000, 8000); // start >= end
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("port range"));
    }

    #[test]
    fn test_empty_wasm_engine() {
        let mut config = valid_config();
        config.runtime.wasm.engine = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("WASM engine"));
    }

    #[test]
    fn test_zero_wasm_max_memory() {
        let mut config = valid_config();
        config.runtime.wasm.max_memory = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("WASM max memory"));
    }

    #[test]
    fn test_zero_wasm_max_execution_time() {
        let mut config = valid_config();
        config.runtime.wasm.max_execution_time = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("WASM max execution time"));
    }

    #[test]
    fn test_empty_python_executable() {
        let mut config = valid_config();
        config.runtime.python.executable = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Python executable"));
    }

    #[test]
    fn test_empty_python_index_url_allowed() {
        let mut config = valid_config();
        config.runtime.python.index_url = String::new();
        assert!(
            config.validate_runtime_config().is_ok(),
            "empty index_url allowed (discovered at runtime)"
        );
    }

    #[test]
    fn test_zero_python_max_memory() {
        let mut config = valid_config();
        config.runtime.python.max_memory = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Python max memory"));
    }

    #[test]
    fn test_zero_python_max_execution_time() {
        let mut config = valid_config();
        config.runtime.python.max_execution_time = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Python max execution time"));
    }

    #[test]
    fn test_empty_log_level() {
        let mut config = valid_config();
        config.logging.level = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Log level"));
    }

    #[test]
    fn test_empty_log_format() {
        let mut config = valid_config();
        config.logging.format = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Log format"));
    }

    #[test]
    fn test_zero_max_log_size() {
        let mut config = valid_config();
        config.logging.max_log_size = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max log size"));
    }

    #[test]
    fn test_zero_max_log_files() {
        let mut config = valid_config();
        config.logging.max_log_files = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max log files"));
    }

    #[test]
    fn test_jwt_secret_required_when_auth_enabled() {
        let mut config = valid_config();
        config.security.auth.enabled = true;
        config.security.auth.jwt_secret = None;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("JWT secret"));
    }

    #[test]
    fn test_zero_session_timeout() {
        let mut config = valid_config();
        config.security.auth.session_timeout = Duration::ZERO;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Session timeout"));
    }

    #[test]
    fn test_zero_max_login_attempts() {
        let mut config = valid_config();
        config.security.auth.max_login_attempts = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max login attempts"));
    }

    #[test]
    fn test_zero_lockout_duration() {
        let mut config = valid_config();
        config.security.auth.lockout_duration = Duration::ZERO;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Lockout duration"));
    }

    #[test]
    fn test_encryption_algorithm_required_when_enabled() {
        let mut config = valid_config();
        config.security.encryption.enabled = true;
        config.security.encryption.algorithm = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Encryption algorithm"));
    }

    #[test]
    fn test_zero_encryption_key_length() {
        let mut config = valid_config();
        config.security.encryption.key_length = 0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Encryption key length"));
    }

    #[test]
    fn test_sandbox_type_required_when_enabled() {
        let mut config = valid_config();
        config.security.sandbox.enabled = true;
        config.security.sandbox.sandbox_type = String::new();
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Sandbox type"));
    }

    #[test]
    fn test_cache_empty_type() {
        let mut config = valid_config();
        config.cache = Some(BackendCacheConfig {
            cache_type: String::new(),
            ..Default::default()
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Cache type"));
    }

    #[test]
    fn test_cache_zero_max_size() {
        let mut config = valid_config();
        config.cache = Some(BackendCacheConfig {
            cache_type: "redis".to_string(),
            max_size: 0,
            ..Default::default()
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Cache max size"));
    }

    #[test]
    fn test_cache_zero_ttl() {
        let mut config = valid_config();
        config.cache = Some(BackendCacheConfig {
            cache_type: "redis".to_string(),
            max_size: 1000,
            ttl: Duration::ZERO,
            ..Default::default()
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Cache TTL"));
    }

    #[test]
    fn test_metrics_empty_endpoint() {
        let mut config = valid_config();
        config.metrics = Some(MetricsConfig {
            endpoint: String::new(),
            ..Default::default()
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Metrics endpoint"));
    }

    #[test]
    fn test_metrics_empty_format() {
        let mut config = valid_config();
        config.metrics = Some(MetricsConfig {
            endpoint: "http://localhost:9090".to_string(),
            format: String::new(),
            ..Default::default()
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Metrics format"));
    }

    #[test]
    fn test_metrics_zero_collection_interval() {
        let mut config = valid_config();
        config.metrics = Some(MetricsConfig {
            endpoint: "http://localhost:9090".to_string(),
            format: "prometheus".to_string(),
            collection_interval: Duration::ZERO,
            ..Default::default()
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Metrics collection interval"));
    }

    #[test]
    fn test_database_empty_url() {
        let mut config = valid_config();
        config.database = Some(DatabaseConfig {
            url: String::new(),
            database_type: "postgres".to_string(),
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(30),
            enable_migrations: false,
            migration_dir: String::new(),
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Database URL"));
    }

    #[test]
    fn test_database_empty_type() {
        let mut config = valid_config();
        config.database = Some(DatabaseConfig {
            url: "postgres://localhost".to_string(),
            database_type: String::new(),
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(30),
            enable_migrations: false,
            migration_dir: String::new(),
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Database type"));
    }

    #[test]
    fn test_database_zero_max_connections() {
        let mut config = valid_config();
        config.database = Some(DatabaseConfig {
            url: "postgres://localhost".to_string(),
            database_type: "postgres".to_string(),
            max_connections: 0,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::from_secs(30),
            enable_migrations: false,
            migration_dir: String::new(),
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Database max connections"));
    }

    #[test]
    fn test_database_zero_connection_timeout() {
        let mut config = valid_config();
        config.database = Some(DatabaseConfig {
            url: "postgres://localhost".to_string(),
            database_type: "postgres".to_string(),
            max_connections: 10,
            connection_timeout: Duration::ZERO,
            query_timeout: Duration::from_secs(30),
            enable_migrations: false,
            migration_dir: String::new(),
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Database connection timeout"));
    }

    #[test]
    fn test_database_zero_query_timeout() {
        let mut config = valid_config();
        config.database = Some(DatabaseConfig {
            url: "postgres://localhost".to_string(),
            database_type: "postgres".to_string(),
            max_connections: 10,
            connection_timeout: Duration::from_secs(30),
            query_timeout: Duration::ZERO,
            enable_migrations: false,
            migration_dir: String::new(),
        });
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Database query timeout"));
    }

    #[test]
    fn test_optional_sections_none_passes() {
        let config = ToadStoolConfig {
            cache: None,
            metrics: None,
            database: None,
            ..valid_config()
        };
        assert!(config.validate_runtime_config().is_ok());
    }

    #[test]
    fn test_port_range_start_equals_end_fails() {
        let mut config = valid_config();
        config.runtime.container.port_range = (9000, 9000);
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("port range"));
    }

    #[test]
    fn test_cpu_usage_zero_invalid() {
        let mut config = valid_config();
        config.runtime.resource_limits.max_cpu_usage = 0.0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max CPU usage"));
    }

    #[test]
    fn test_disk_usage_zero_invalid() {
        let mut config = valid_config();
        config.runtime.resource_limits.max_disk_usage = 0.0;
        let err = config.validate_runtime_config().unwrap_err();
        assert!(err.to_string().contains("Max disk usage"));
    }
}
