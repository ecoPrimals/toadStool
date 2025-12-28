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
        info!("    WebSocket: {}", self.features.enable_websocket);
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
        let _guard = get_env_lock().lock().unwrap_or_else(|e| e.into_inner()); // ✅ MODERN: Concurrent-safe + poison recovery
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
    fn test_current_environment_detection() {
        let _guard = get_env_lock().lock().unwrap_or_else(|e| e.into_inner()); // ✅ MODERN: Concurrent-safe + poison recovery
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
