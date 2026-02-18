//! Configuration types module
//!
//! This module contains all configuration type definitions organized by domain:
//! - `application`: Application lifecycle configuration
//! - `network`: Network and communication configuration
//! - `runtime`: Runtime execution configuration (WASM, Container, Python, GPU)
//! - `security`: Security and access control configuration
//! - `observability`: Logging, metrics, and monitoring configuration
//! - `features`: Feature flags and toggles
//!
//! ## Usage
//!
//! Import the root config:
//! ```
//! use toadstool_config::types::ToadStoolConfig;
//!
//! let config = ToadStoolConfig::default();
//! ```
//!
//! Or import specific domain types:
//! ```
//! use toadstool_config::types::{ApplicationConfig, RuntimeConfig};
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// Domain modules
pub mod application;
pub mod features;
pub mod network;
pub mod observability;
pub mod runtime;
pub mod security;

// Re-export all public types for convenient access
pub use application::ApplicationConfig;
pub use features::FeatureFlags;
pub use network::{ConnectionConfig, EndpointConfig, NetworkConfig, TlsConfig};
pub use observability::{BackendCacheConfig, DatabaseConfig, LoggingConfig, MetricsConfig};
pub use runtime::{
    ContainerConfig, GpuConfig, PythonConfig, ResourceLimits, RuntimeConfig, WasmConfig,
};
pub use security::{
    AuditConfig, AuthConfig, AuthzConfig, EncryptionConfig, SandboxConfig, SecurityConfig,
};

use crate::{development, production, testing};

/// Main ToadStool configuration structure
///
/// This is the root configuration that orchestrates all domain-specific configs.
/// It provides methods for loading, validating, and managing configuration.
///
/// # Example
/// ```
/// use toadstool_config::types::ToadStoolConfig;
///
/// // Load default configuration
/// let config = ToadStoolConfig::default();
///
/// // Access configurations
/// assert!(!config.app.name.is_empty());
/// assert!(!config.logging.level.is_empty());
/// ```
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

    /// Port registry (centralized port management)
    pub ports: crate::ports::PortRegistry,

    /// Service registry (dynamic service discovery)
    pub services: crate::services::ServiceRegistry,

    /// Database configuration (optional)
    pub database: Option<DatabaseConfig>,

    /// Cache configuration (optional)
    pub cache: Option<BackendCacheConfig>,

    /// Metrics configuration (optional)
    pub metrics: Option<MetricsConfig>,

    /// Feature flags
    pub features: FeatureFlags,

    /// Environment-specific overrides
    pub overrides: HashMap<String, serde_json::Value>,
}

/// Configuration loading and management
impl ToadStoolConfig {
    /// Load configuration from file
    ///
    /// # Errors
    /// Returns an error if the configuration file cannot be read or parsed
    ///
    /// # Example
    /// ```no_run
    /// # use toadstool_config::types::ToadStoolConfig;
    /// let config = ToadStoolConfig::load_from_file("config.toml")?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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
    ///
    /// # Example
    /// ```
    /// # use toadstool_config::types::ToadStoolConfig;
    /// let config = ToadStoolConfig::load_from_env()?;
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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

        // Legacy endpoint override (deprecated - use capability-based discovery)
        #[allow(deprecated)]
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

        // Validate legacy network configuration (deprecated - use capability-based discovery)
        #[allow(deprecated)]
        if self.network.endpoints.songbird.is_empty() {
            return Err(
                "Songbird endpoint cannot be empty (use capability-based discovery instead)".into(),
            );
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
    ///
    /// Applies environment-specific defaults for development, production, or test.
    ///
    /// # Example
    /// ```
    /// # use toadstool_config::types::ToadStoolConfig;
    /// let config = ToadStoolConfig::default().for_environment("production");
    /// assert_eq!(config.app.environment, "production");
    /// ```
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
                    std::time::Duration::from_secs(testing::DEFAULT_TEST_EXECUTION_TIMEOUT_SECS);
                self.security.auth.enabled = false;
            }
            _ => {
                // Use default configuration
            }
        }

        self
    }

    /// Merge with override configuration
    ///
    /// # Example
    /// ```
    /// # use toadstool_config::types::ToadStoolConfig;
    /// # use std::collections::HashMap;
    /// let mut overrides = HashMap::new();
    /// overrides.insert("custom_key".to_string(), serde_json::json!(true));
    ///
    /// let config = ToadStoolConfig::default().merge(overrides);
    /// ```
    #[must_use]
    pub fn merge(mut self, overrides: HashMap<String, serde_json::Value>) -> Self {
        self.overrides.extend(overrides);
        self
    }

    /// Get configuration value with override support
    ///
    /// # Example
    /// ```
    /// # use toadstool_config::types::ToadStoolConfig;
    /// let config = ToadStoolConfig::default();
    /// let value: i32 = config.get_override("some_key", 42);
    /// assert_eq!(value, 42); // Returns default if not found
    /// ```
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
    use crate::app;

    #[test]
    fn test_default_configuration() {
        let config = ToadStoolConfig::default();
        assert_eq!(config.app.name, app::DEFAULT_APP_NAME);
        assert_eq!(config.app.environment, app::DEFAULT_ENVIRONMENT);
        assert_eq!(config.logging.level, app::DEFAULT_LOG_LEVEL);
        assert!(config.features.enable_federation);
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
