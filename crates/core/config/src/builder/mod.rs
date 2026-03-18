// SPDX-License-Identifier: AGPL-3.0-or-later
//! Universal Configuration Builder Pattern
//!
//! **Deep Debt Principle**: No hardcoded values, all runtime configurable
//!
//! This module provides a unified builder pattern for all ToadStool configurations,
//! enabling runtime flexibility, TOML file support, and environment variable integration.
//!
//! # Example
//!
//! ```rust,ignore
//! use toadstool_config::builder::*;
//!
//! // Method 1: Builder pattern
//! let config = ProfilerConfigBuilder::new()
//!     .warmup_iterations(20)
//!     .benchmark_iterations(500)
//!     .timeout_ms(30000)
//!     .parallel()
//!     .build();
//!
//! // Method 2: From TOML file
//! let config = ProfilerConfig::from_file("profiler.toml")?;
//!
//! // Method 3: From environment variables
//! let config = ProfilerConfig::from_env()?;
//!
//! // Method 4: Quick presets
//! let config = ProfilerConfig::quick();  // Fast benchmarks
//! let config = ProfilerConfig::thorough();  // Comprehensive benchmarks
//! ```

mod profiler;
mod substrate;

pub use profiler::{OutputFormat, ProfilerConfig, ProfilerConfigBuilder};
pub use substrate::{
    PerformanceTarget, SubstrateConfig, SubstrateConfigBuilder, SubstratePreference, SubstrateType,
};

use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

/// Configuration errors
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Environment variable error: {0}")]
    EnvVar(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, ConfigError>;

/// Base trait for all ToadStool configurations
///
/// **Deep Debt**: All configs support multiple sources (file, env, builder)
pub trait ToadStoolConfigTrait: Serialize + for<'de> Deserialize<'de> + Default + Sized {
    /// Load from TOML file
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if the file cannot be read or TOML parsing fails.
    fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&contents)?)
    }

    /// Load from environment variables (with TOADSTOOL_ prefix)
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if required environment variables are missing or invalid.
    fn from_env() -> Result<Self>;

    /// Save to TOML file
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if serialization fails or the file cannot be written.
    fn to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let contents =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Validation(e.to_string()))?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Merge with defaults (self takes precedence)
    #[must_use]
    fn with_defaults(self) -> Self {
        self
    }

    /// Validate configuration
    ///
    /// # Errors
    ///
    /// Returns [`ConfigError`] if validation fails (implementation-specific).
    fn validate(&self) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::Validation("test".to_string());
        assert_eq!(err.to_string(), "Validation error: test");
    }

    #[test]
    fn test_config_error_io_display() {
        let err = ConfigError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "file not found",
        ));
        assert!(err.to_string().contains("file not found"));
    }
}
