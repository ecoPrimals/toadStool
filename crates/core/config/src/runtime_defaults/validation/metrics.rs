// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::runtime_defaults::{ConfigError, ConfigResult};
use crate::ToadStoolConfig;

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    if let Some(metrics_config) = &config.metrics {
        if metrics_config.endpoint.is_empty() {
            return Err(ConfigError::Invalid(
                "Metrics endpoint cannot be empty".to_string(),
            ));
        }

        if metrics_config.format.is_empty() {
            return Err(ConfigError::Invalid(
                "Metrics format cannot be empty".to_string(),
            ));
        }

        if metrics_config.collection_interval.is_zero() {
            return Err(ConfigError::Invalid(
                "Metrics collection interval must be greater than 0".to_string(),
            ));
        }
    }

    Ok(())
}
