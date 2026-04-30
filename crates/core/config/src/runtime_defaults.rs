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

/// Configuration error type for runtime config operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// Invalid configuration value or structure.
    #[error("Invalid configuration: {0}")]
    Invalid(String),
    /// Required configuration field is missing.
    #[error("Missing required field: {0}")]
    MissingField(String),
    /// File I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// TOML parse error.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    /// JSON parse error.
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
    /// Socket address parse error.
    #[error("Address parse error: {0}")]
    AddrParse(#[from] std::net::AddrParseError),
    /// Environment variable error.
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
        #[expect(
            deprecated,
            reason = "logging legacy endpoint fallbacks for diagnostics"
        )]
        {
            info!(
                "    Coordination (fallback): {}",
                self.network.endpoints.coordination
            );
            info!("    Crypto (fallback): {}", self.network.endpoints.security);
            info!("    Storage (fallback): {}", self.network.endpoints.storage);
            info!(
                "    AI (fallback): {}",
                self.network.endpoints.ai_processing
            );
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
#[allow(
    deprecated,
    reason = "tests exercise legacy runtime defaults pending migration"
)]
#[path = "runtime_defaults_tests.rs"]
mod tests;
