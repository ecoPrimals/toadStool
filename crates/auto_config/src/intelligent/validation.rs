// SPDX-License-Identifier: AGPL-3.0-or-later
//! Configuration validation (Pipeline Stage 4)

use tracing::warn;

use crate::{ToadStoolError, ToadStoolResult};
use toadstool_config::ToadStoolConfig;

/// Configuration validator
pub struct ConfigValidator;

impl Default for ConfigValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigValidator {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Validate the generated configuration
    pub fn validate_configuration(&self, config: &ToadStoolConfig) -> ToadStoolResult<()> {
        // Basic validation checks
        if config.runtime.max_concurrent_executions == 0 {
            return Err(ToadStoolError::configuration(
                "No concurrent executions configured",
            ));
        }

        if config.runtime.resource_limits.max_memory_usage == 0.0 {
            return Err(ToadStoolError::configuration("No memory limit configured"));
        }

        // Advanced validation
        if config.runtime.wasm.max_memory == 0 {
            warn!("WASM runtime has no memory limit set");
        }

        if config.runtime.gpu.is_some()
            && let Some(gpu_config) = &config.runtime.gpu
            && gpu_config.max_memory_per_device == 0
        {
            warn!("GPU runtime enabled but no memory limit set");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validator_new() {
        let validator = ConfigValidator::new();
        let config = toadstool_config::ToadStoolConfig::default();
        assert!(validator.validate_configuration(&config).is_ok());
    }

    #[test]
    fn test_validate_configuration_zero_concurrent() {
        let validator = ConfigValidator::new();
        let mut config = toadstool_config::ToadStoolConfig::default();
        config.runtime.max_concurrent_executions = 0;
        let result = validator.validate_configuration(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("concurrent"));
    }

    #[test]
    fn test_validate_configuration_zero_memory() {
        let validator = ConfigValidator::new();
        let mut config = toadstool_config::ToadStoolConfig::default();
        config.runtime.resource_limits.max_memory_usage = 0.0;
        let result = validator.validate_configuration(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("memory"));
    }
}
