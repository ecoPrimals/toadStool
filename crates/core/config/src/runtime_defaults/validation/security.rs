// SPDX-License-Identifier: AGPL-3.0-only

use crate::ToadStoolConfig;
use crate::runtime_defaults::{ConfigError, ConfigResult};

pub(super) fn validate(config: &ToadStoolConfig) -> ConfigResult<()> {
    if config.security.auth.enabled && config.security.auth.jwt_secret.is_none() {
        return Err(ConfigError::Invalid(
            "JWT secret is required when authentication is enabled".to_string(),
        ));
    }

    if config.security.auth.session_timeout.is_zero() {
        return Err(ConfigError::Invalid(
            "Session timeout must be greater than 0".to_string(),
        ));
    }

    if config.security.auth.max_login_attempts == 0 {
        return Err(ConfigError::Invalid(
            "Max login attempts must be greater than 0".to_string(),
        ));
    }

    if config.security.auth.lockout_duration.is_zero() {
        return Err(ConfigError::Invalid(
            "Lockout duration must be greater than 0".to_string(),
        ));
    }

    if config.security.encryption.enabled && config.security.encryption.algorithm.is_empty() {
        return Err(ConfigError::Invalid(
            "Encryption algorithm is required when encryption is enabled".to_string(),
        ));
    }

    if config.security.encryption.key_length == 0 {
        return Err(ConfigError::Invalid(
            "Encryption key length must be greater than 0".to_string(),
        ));
    }

    if config.security.sandbox.enabled && config.security.sandbox.sandbox_type.is_empty() {
        return Err(ConfigError::Invalid(
            "Sandbox type is required when sandboxing is enabled".to_string(),
        ));
    }

    Ok(())
}
