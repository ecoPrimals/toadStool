// SPDX-License-Identifier: AGPL-3.0-only

use crate::ToadStoolConfig;
use crate::runtime_defaults::{ConfigError, ConfigResult};

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    if config.logging.level.is_empty() {
        return Err(ConfigError::Invalid(
            "Log level cannot be empty".to_string(),
        ));
    }

    if config.logging.format.is_empty() {
        return Err(ConfigError::Invalid(
            "Log format cannot be empty".to_string(),
        ));
    }

    if config.logging.max_log_size == 0 {
        return Err(ConfigError::Invalid(
            "Max log size must be greater than 0".to_string(),
        ));
    }

    if config.logging.max_log_files == 0 {
        return Err(ConfigError::Invalid(
            "Max log files must be greater than 0".to_string(),
        ));
    }

    Ok(())
}
