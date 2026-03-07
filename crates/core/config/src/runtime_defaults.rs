// SPDX-License-Identifier: AGPL-3.0-or-later
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
    #[allow(deprecated)] // Testing legacy endpoint configuration
    fn test_env_overrides() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENV", Some("test")),
                ("TOADSTOOL_DEBUG", Some("true")),
                ("TOADSTOOL_LOG_LEVEL", Some("debug")),
                ("TOADSTOOL_WORKER_THREADS", Some("8")),
                ("TOADSTOOL_SONGBIRD_ENDPOINT", Some("http://localhost:8080")),
                ("TOADSTOOL_BIND_ADDRESS", Some("127.0.0.1:3000")),
            ],
            || {
                let mut config = ToadStoolConfig::default();
                config.apply_env_overrides().unwrap();

                assert_eq!(config.app.environment, "test");
                assert!(config.features.enable_debug);
                assert_eq!(config.logging.level, "debug");
                assert_eq!(config.app.worker_threads, 8);
                assert_eq!(config.network.endpoints.songbird, "http://localhost:8080");
            },
        );
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
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENVIRONMENT", Some("production")),
                ("TOADSTOOL_ENV", Some("production")),
                ("ENVIRONMENT", Some("production")),
                ("ENV", Some("production")),
            ],
            || {
                let config = ToadStoolConfig::for_current_environment();
                assert_eq!(config.app.environment, "production");
            },
        );

        temp_env::with_vars(
            [
                ("TOADSTOOL_ENVIRONMENT", Some("staging")),
                ("TOADSTOOL_ENV", Some("staging")),
                ("ENVIRONMENT", Some("staging")),
                ("ENV", Some("staging")),
            ],
            || {
                let config = ToadStoolConfig::for_current_environment();
                assert_eq!(config.app.environment, "staging");
            },
        );
    }

    // ═══════════════════════════════════════════════════════════════════
    // Additional tests for uncovered runtime default functions
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_for_current_environment_env_var_priority_toadstool_environment() {
        // TOADSTOOL_ENVIRONMENT has highest priority for initial env detection.
        // Must unset TOADSTOOL_ENV so apply_env_overrides doesn't overwrite.
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENVIRONMENT", Some("prod")),
                ("TOADSTOOL_ENV", None),
                ("ENVIRONMENT", Some("test")),
                ("ENV", Some("dev")),
            ],
            || {
                let config = ToadStoolConfig::for_current_environment();
                assert_eq!(config.app.environment, "prod");
            },
        );
    }

    #[test]
    fn test_for_current_environment_env_var_priority_toadstool_env_fallback() {
        temp_env::with_vars(
            [
                ("TOADSTOOL_ENVIRONMENT", None),
                ("TOADSTOOL_ENV", Some("staging")),
                ("ENVIRONMENT", None),
                ("ENV", None),
            ],
            || {
                let config = ToadStoolConfig::for_current_environment();
                assert_eq!(config.app.environment, "staging");
            },
        );
    }

    #[test]
    fn test_load_with_overrides_success() {
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
                assert!(result.is_ok(), "load failed: {result:?}");
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
            "expected Io error, got {err:?}"
        );
    }

    #[test]
    fn test_load_from_env_only_success() {
        temp_env::with_var("TOADSTOOL_ENV", Some("test"), || {
            let result = ToadStoolConfig::load_from_env_only();
            assert!(result.is_ok());
        });
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
        temp_env::with_vars_unset(
            [
                "TOADSTOOL_ENVIRONMENT",
                "TOADSTOOL_ENV",
                "ENVIRONMENT",
                "ENV",
            ],
            || {
                let config = ToadStoolConfig::for_current_environment();
                assert_eq!(config.app.environment, "development");
            },
        );
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
            "expected Toml error, got {err:?}"
        );
    }
}
