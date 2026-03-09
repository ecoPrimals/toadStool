// SPDX-License-Identifier: AGPL-3.0-only

use crate::runtime_defaults::{ConfigError, ConfigResult};
use crate::ToadStoolConfig;

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    if config.app.name.is_empty() {
        return Err(ConfigError::Invalid(
            "Application name cannot be empty".to_string(),
        ));
    }

    if config.app.worker_threads == 0 {
        return Err(ConfigError::Invalid(
            "Worker threads must be greater than 0".to_string(),
        ));
    }

    if config.app.queue_size == 0 {
        return Err(ConfigError::Invalid(
            "Queue size must be greater than 0".to_string(),
        ));
    }

    if config.app.batch_size == 0 {
        return Err(ConfigError::Invalid(
            "Batch size must be greater than 0".to_string(),
        ));
    }

    Ok(())
}
