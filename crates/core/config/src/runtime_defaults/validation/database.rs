// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::runtime_defaults::{ConfigError, ConfigResult};
use crate::ToadStoolConfig;

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    if let Some(database_config) = &config.database {
        if database_config.url.is_empty() {
            return Err(ConfigError::Invalid(
                "Database URL cannot be empty".to_string(),
            ));
        }

        if database_config.database_type.is_empty() {
            return Err(ConfigError::Invalid(
                "Database type cannot be empty".to_string(),
            ));
        }

        if database_config.max_connections == 0 {
            return Err(ConfigError::Invalid(
                "Database max connections must be greater than 0".to_string(),
            ));
        }

        if database_config.connection_timeout.is_zero() {
            return Err(ConfigError::Invalid(
                "Database connection timeout must be greater than 0".to_string(),
            ));
        }

        if database_config.query_timeout.is_zero() {
            return Err(ConfigError::Invalid(
                "Database query timeout must be greater than 0".to_string(),
            ));
        }
    }

    Ok(())
}
