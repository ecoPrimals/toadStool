// SPDX-License-Identifier: AGPL-3.0-only

use crate::ToadStoolConfig;
use crate::runtime_defaults::{ConfigError, ConfigResult};

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    if let Some(cache_config) = &config.cache {
        if cache_config.cache_type.is_empty() {
            return Err(ConfigError::Invalid(
                "Cache type cannot be empty".to_string(),
            ));
        }

        if cache_config.max_size == 0 {
            return Err(ConfigError::Invalid(
                "Cache max size must be greater than 0".to_string(),
            ));
        }

        if cache_config.ttl.is_zero() {
            return Err(ConfigError::Invalid(
                "Cache TTL must be greater than 0".to_string(),
            ));
        }
    }

    Ok(())
}
