// SPDX-License-Identifier: AGPL-3.0-only

use crate::ToadStoolConfig;
use crate::runtime_defaults::{ConfigError, ConfigResult};

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    #[allow(deprecated)]
    {
        if config.network.endpoints.coordination.is_empty() {
            return Err(ConfigError::Invalid(
                "Songbird endpoint cannot be empty (use capability-based discovery instead)"
                    .to_string(),
            ));
        }

        if config.network.endpoints.security.is_empty() {
            return Err(ConfigError::Invalid(
                "BearDog endpoint cannot be empty (use capability-based discovery instead)"
                    .to_string(),
            ));
        }

        if config.network.endpoints.storage.is_empty() {
            return Err(ConfigError::Invalid(
                "NestGate endpoint cannot be empty (use capability-based discovery instead)"
                    .to_string(),
            ));
        }

        if config.network.endpoints.ai_processing.is_empty() {
            return Err(ConfigError::Invalid(
                "Squirrel endpoint cannot be empty (use capability-based discovery instead)"
                    .to_string(),
            ));
        }
    }

    if config.network.connection.request_timeout.is_zero() {
        return Err(ConfigError::Invalid(
            "Request timeout must be greater than 0".to_string(),
        ));
    }

    if config.network.connection.connection_timeout.is_zero() {
        return Err(ConfigError::Invalid(
            "Connection timeout must be greater than 0".to_string(),
        ));
    }

    if config.network.connection.max_retries == 0 {
        return Err(ConfigError::Invalid(
            "Max retries must be greater than 0".to_string(),
        ));
    }

    if config.network.connection.max_connections_per_host == 0 {
        return Err(ConfigError::Invalid(
            "Max connections per host must be greater than 0".to_string(),
        ));
    }

    Ok(())
}
