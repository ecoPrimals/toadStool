// SPDX-License-Identifier: AGPL-3.0-only

use crate::ToadStoolConfig;
use crate::runtime_defaults::{ConfigError, ConfigResult};

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    if config.runtime.max_concurrent_executions == 0 {
        return Err(ConfigError::Invalid(
            "Max concurrent executions must be greater than 0".to_string(),
        ));
    }

    if config.runtime.execution_timeout.is_zero() {
        return Err(ConfigError::Invalid(
            "Execution timeout must be greater than 0".to_string(),
        ));
    }

    if config.runtime.container.runtime.is_empty() {
        return Err(ConfigError::Invalid(
            "Container runtime cannot be empty".to_string(),
        ));
    }

    if config.runtime.container.default_registry.is_empty() {
        return Err(ConfigError::Invalid(
            "Default registry cannot be empty".to_string(),
        ));
    }

    if config.runtime.container.port_range.0 >= config.runtime.container.port_range.1 {
        return Err(ConfigError::Invalid(
            "Container port range start must be less than end".to_string(),
        ));
    }

    if config.runtime.wasm.engine.is_empty() {
        return Err(ConfigError::Invalid(
            "WASM engine cannot be empty".to_string(),
        ));
    }

    if config.runtime.wasm.max_memory == 0 {
        return Err(ConfigError::Invalid(
            "WASM max memory must be greater than 0".to_string(),
        ));
    }

    if config.runtime.wasm.max_execution_time == 0 {
        return Err(ConfigError::Invalid(
            "WASM max execution time must be greater than 0".to_string(),
        ));
    }

    if config.runtime.python.executable.is_empty() {
        return Err(ConfigError::Invalid(
            "Python executable cannot be empty".to_string(),
        ));
    }

    if config.runtime.python.max_memory == 0 {
        return Err(ConfigError::Invalid(
            "Python max memory must be greater than 0".to_string(),
        ));
    }

    if config.runtime.python.max_execution_time == 0 {
        return Err(ConfigError::Invalid(
            "Python max execution time must be greater than 0".to_string(),
        ));
    }

    Ok(())
}
