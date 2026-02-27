//! Runtime configuration defaults and validation
//!
//! This module provides default values and validation for runtime configuration
//! to eliminate hardcoded values scattered throughout the codebase.

// Module declarations
mod env_overrides;
pub mod validation;

use std::path::Path;

use tracing::{info, warn};

use crate::ToadStoolConfig;

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
    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
    #[error("Environment error: {0}")]
    Env(String),
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

    // apply_env_overrides method is now in env_overrides.rs
    // validate_runtime_config method is now in validation.rs

    /// Get optimized configuration for current environment
    ///
    /// Detects the environment from environment variables in this order:
    /// 1. `TOADSTOOL_ENVIRONMENT`
    /// 2. `TOADSTOOL_ENV`
    /// 3. `ENVIRONMENT`
    /// 4. `ENV`
    ///
    /// Defaults to "development" if no environment variable is set.
    /// Automatically applies environment variable overrides.
    #[must_use = "Configuration should be used or it will be dropped"]
    pub fn for_current_environment() -> Self {
        // Try multiple environment variable names in priority order
        let environment = std::env::var("TOADSTOOL_ENVIRONMENT")
            .or_else(|_| std::env::var("TOADSTOOL_ENV"))
            .or_else(|_| std::env::var("ENVIRONMENT"))
            .or_else(|_| std::env::var("ENV"))
            .unwrap_or_else(|_| {
                // Default to development environment if no variable set
                info!("No environment variable set, defaulting to 'development'");
                "development".to_string()
            });

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

    /// Print configuration summary to logs
    ///
    /// Displays a comprehensive overview of the current configuration including:
    /// - Environment and network settings
    /// - Runtime and resource limits
    /// - Enabled features
    /// - Security settings
    /// - External service endpoints
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
        info!("    Federation: {}", self.features.enable_federation);
        info!("    Distributed: {}", self.features.enable_distributed);
        info!("    Auto-Config: {}", self.features.enable_auto_config);
        info!("    Debug: {}", self.features.enable_debug);
        info!("  External Services (LEGACY - use capability-based discovery):");
        #[allow(deprecated)]
        {
            info!(
                "    Coordination (fallback): {}",
                self.network.endpoints.songbird
            );
            info!("    Crypto (fallback): {}", self.network.endpoints.beardog);
            info!(
                "    Storage (fallback): {}",
                self.network.endpoints.nestgate
            );
            info!("    AI (fallback): {}", self.network.endpoints.squirrel);
        }
        info!("    💡 Use ServiceDiscovery::find_by_capability() for runtime discovery");
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
#[allow(deprecated)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::NamedTempFile;

    // ✅ MODERN: Use shared lock from env_config to prevent test races
    use crate::env_config::tests::get_env_lock;

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
    #[allow(deprecated)] // Testing legacy endpoint configuration
    fn test_env_overrides() {
        let _guard = get_env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner); // ✅ MODERN: Concurrent-safe + poison recovery
                                                                 // Save original environment state
        let original_env = env::var("TOADSTOOL_ENV").ok();
        let original_debug = env::var("TOADSTOOL_DEBUG").ok();
        let original_log_level = env::var("TOADSTOOL_LOG_LEVEL").ok();
        let original_threads = env::var("TOADSTOOL_WORKER_THREADS").ok();
        let original_endpoint = env::var("TOADSTOOL_SONGBIRD_ENDPOINT").ok();
        let original_bind_address = env::var("TOADSTOOL_BIND_ADDRESS").ok();

        // ✅ MODERN: Set test values (use BIND_ADDRESS with port, not BIND_HOST)
        env::set_var("TOADSTOOL_ENV", "test");
        env::set_var("TOADSTOOL_DEBUG", "true");
        env::set_var("TOADSTOOL_LOG_LEVEL", "debug");
        env::set_var("TOADSTOOL_WORKER_THREADS", "8");
        env::set_var("TOADSTOOL_SONGBIRD_ENDPOINT", "http://localhost:8080");
        env::set_var("TOADSTOOL_BIND_ADDRESS", "127.0.0.1:3000"); // Fixed: full socket address

        let mut config = ToadStoolConfig::default();
        config.apply_env_overrides().unwrap();

        assert_eq!(config.app.environment, "test");
        assert!(config.features.enable_debug);
        assert_eq!(config.logging.level, "debug");
        assert_eq!(config.app.worker_threads, 8);
        assert_eq!(config.network.endpoints.songbird, "http://localhost:8080");

        // ✅ MODERN: Restore original environment state
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
        match original_bind_address {
            Some(val) => env::set_var("TOADSTOOL_BIND_ADDRESS", val),
            None => env::remove_var("TOADSTOOL_BIND_ADDRESS"),
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

        let mut invalid_config = config;
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
    fn test_current_environment_detection() {
        let _guard = get_env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner); // ✅ MODERN: Concurrent-safe + poison recovery
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

    // ═══════════════════════════════════════════════════════════════════
    // Additional tests for uncovered runtime default functions
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_for_current_environment_env_var_priority_toadstool_environment() {
        let _guard = get_env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let orig_env = env::var("TOADSTOOL_ENVIRONMENT").ok();
        let orig_env_short = env::var("TOADSTOOL_ENV").ok();
        let orig_generic = env::var("ENVIRONMENT").ok();
        let orig_env_generic = env::var("ENV").ok();

        // TOADSTOOL_ENVIRONMENT has highest priority for initial env detection.
        // Must unset TOADSTOOL_ENV so apply_env_overrides doesn't overwrite.
        env::set_var("TOADSTOOL_ENVIRONMENT", "prod");
        env::remove_var("TOADSTOOL_ENV");
        env::set_var("ENVIRONMENT", "test");
        env::set_var("ENV", "dev");

        let config = ToadStoolConfig::for_current_environment();
        assert_eq!(config.app.environment, "prod");

        if let Some(v) = orig_env {
            env::set_var("TOADSTOOL_ENVIRONMENT", v);
        } else {
            env::remove_var("TOADSTOOL_ENVIRONMENT");
        }
        if let Some(v) = orig_env_short {
            env::set_var("TOADSTOOL_ENV", v);
        } else {
            env::remove_var("TOADSTOOL_ENV");
        }
        if let Some(v) = orig_generic {
            env::set_var("ENVIRONMENT", v);
        } else {
            env::remove_var("ENVIRONMENT");
        }
        if let Some(v) = orig_env_generic {
            env::set_var("ENV", v);
        } else {
            env::remove_var("ENV");
        }
    }

    #[test]
    fn test_for_current_environment_env_var_priority_toadstool_env_fallback() {
        let _guard = get_env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        env::remove_var("TOADSTOOL_ENVIRONMENT");
        let orig = env::var("TOADSTOOL_ENV").ok();
        env::set_var("TOADSTOOL_ENV", "staging");
        env::remove_var("ENVIRONMENT");
        env::remove_var("ENV");

        let config = ToadStoolConfig::for_current_environment();
        assert_eq!(config.app.environment, "staging");

        if let Some(v) = orig {
            env::set_var("TOADSTOOL_ENV", v);
        } else {
            env::remove_var("TOADSTOOL_ENV");
        }
    }

    #[test]
    fn test_load_with_overrides_success() {
        let _guard = crate::env_config::tests::get_env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        temp_env::with_vars_unset(
            [
                "TOADSTOOL_ENVIRONMENT",
                "TOADSTOOL_ENV",
                "TOADSTOOL_VERBOSE",
                "TOADSTOOL_WORKER_THREADS",
                "TOADSTOOL_REQUEST_TIMEOUT",
                "TOADSTOOL_EXECUTION_TIMEOUT",
                "TOADSTOOL_MAX_CONCURRENT_EXECUTIONS",
                "TOADSTOOL_BIND_ADDRESS",
                "TOADSTOOL_PORT",
            ],
            || {
                let config = ToadStoolConfig::development();
                let temp_file = NamedTempFile::new().unwrap();
                config.save_to_file(temp_file.path()).unwrap();

                let result = ToadStoolConfig::load_with_overrides(temp_file.path());
                assert!(result.is_ok(), "load failed: {:?}", result);
                let loaded = result.unwrap();
                assert_eq!(loaded.app.environment, config.app.environment);
            },
        );
    }

    #[test]
    fn test_load_with_overrides_nonexistent_file() {
        let result = ToadStoolConfig::load_with_overrides("/nonexistent/path/config.toml");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ConfigError::Io(_)),
            "expected Io error, got {:?}",
            err
        );
    }

    #[test]
    fn test_load_from_env_only_success() {
        let _guard = get_env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let orig_env = env::var("TOADSTOOL_ENV").ok();
        env::set_var("TOADSTOOL_ENV", "test");

        let result = ToadStoolConfig::load_from_env_only();
        assert!(result.is_ok());

        if let Some(v) = orig_env {
            env::set_var("TOADSTOOL_ENV", v);
        } else {
            env::remove_var("TOADSTOOL_ENV");
        }
    }

    #[test]
    fn test_save_to_file_then_load_roundtrip() {
        let config = ToadStoolConfig::testing();
        let temp_file = NamedTempFile::new().unwrap();
        let path = temp_file.path();

        config.save_to_file(path).unwrap();
        let content = std::fs::read_to_string(path).unwrap();
        assert!(content.contains("environment"));
        assert!(content.contains("test"));
    }

    #[test]
    fn test_save_to_file_invalid_path() {
        let config = ToadStoolConfig::default();
        let result = config.save_to_file("/nonexistent/directory/config.toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_to_json_success() {
        let config = ToadStoolConfig::default();
        let json = config.to_json().unwrap();
        assert!(json.contains("\"app\""));
        assert!(json.contains("\"network\""));
        assert!(json.contains("\"runtime\""));
    }

    #[test]
    fn test_config_error_variants() {
        let invalid = ConfigError::Invalid("bad".into());
        assert!(invalid.to_string().contains("bad"));

        let missing = ConfigError::MissingField("name".into());
        assert!(missing.to_string().contains("name"));

        let io_err = ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(io_err.to_string().contains("file not found"));

        let addr_err: ConfigError = "invalid"
            .parse::<std::net::SocketAddr>()
            .unwrap_err()
            .into();
        assert!(addr_err.to_string().contains("Address"));

        let env_err = ConfigError::Env("TOADSTOOL_ENV".into());
        assert!(env_err.to_string().contains("TOADSTOOL_ENV"));
    }

    #[test]
    fn test_print_summary_no_panic() {
        let config = ToadStoolConfig::default();
        config.print_summary();
    }

    #[test]
    fn test_print_summary_with_cache() {
        let config = ToadStoolConfig {
            cache: Some(crate::BackendCacheConfig::default()),
            ..Default::default()
        };
        config.print_summary();
    }

    #[test]
    fn test_print_summary_with_metrics() {
        let config = ToadStoolConfig {
            metrics: Some(crate::MetricsConfig::default()),
            ..Default::default()
        };
        config.print_summary();
    }

    #[test]
    fn test_print_summary_with_database() {
        let config = ToadStoolConfig {
            database: Some(crate::DatabaseConfig {
                url: "sqlite::memory:".to_string(),
                database_type: "sqlite".to_string(),
                max_connections: 10,
                connection_timeout: std::time::Duration::from_secs(30),
                query_timeout: std::time::Duration::from_secs(60),
                enable_migrations: false,
                migration_dir: "migrations".to_string(),
            }),
            ..Default::default()
        };
        config.print_summary();
    }

    #[test]
    fn test_for_current_environment_defaults_to_development_when_unset() {
        let _guard = get_env_lock()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        env::remove_var("TOADSTOOL_ENVIRONMENT");
        env::remove_var("TOADSTOOL_ENV");
        env::remove_var("ENVIRONMENT");
        env::remove_var("ENV");

        let config = ToadStoolConfig::for_current_environment();
        assert_eq!(config.app.environment, "development");
    }

    #[test]
    fn test_load_with_overrides_invalid_toml() {
        let temp_file = NamedTempFile::new().unwrap();
        std::fs::write(temp_file.path(), "invalid toml [[[").unwrap();

        let result = ToadStoolConfig::load_with_overrides(temp_file.path());
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            matches!(err, ConfigError::Toml(_)),
            "expected Toml error, got {:?}",
            err
        );
    }
}
