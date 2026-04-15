// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;
use toadstool_config::ToadStoolConfig;

/// Test JWT secret required when auth enabled
#[test]
fn test_validation_jwt_secret_required_when_auth_enabled() {
    let mut config = ToadStoolConfig::default();
    config.security.auth.enabled = true;
    config.security.auth.jwt_secret = None;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("JWT secret is required when authentication is enabled")
    );
}

/// Test JWT secret not required when auth disabled
#[test]
fn test_validation_jwt_secret_not_required_when_auth_disabled() {
    let mut config = ToadStoolConfig::default();
    config.security.auth.enabled = false;
    config.security.auth.jwt_secret = None;

    let result = config.validate_runtime_config();
    // Should pass other validations even without JWT secret when auth disabled
    assert!(result.is_ok() || !result.unwrap_err().to_string().contains("JWT secret"));
}

/// Test session timeout validation
#[test]
fn test_validation_session_timeout_zero() {
    let mut config = ToadStoolConfig::default();
    config.security.auth.session_timeout = Duration::from_secs(0);

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Session timeout must be greater than 0")
    );
}

/// Test max login attempts validation
#[test]
fn test_validation_max_login_attempts_zero() {
    let mut config = ToadStoolConfig::default();
    config.security.auth.max_login_attempts = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Max login attempts must be greater than 0")
    );
}

/// Test lockout duration validation
#[test]
fn test_validation_lockout_duration_zero() {
    let mut config = ToadStoolConfig::default();
    config.security.auth.lockout_duration = Duration::from_secs(0);

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Lockout duration must be greater than 0")
    );
}

/// Test encryption algorithm required when encryption enabled
#[test]
fn test_validation_encryption_algorithm_required() {
    let mut config = ToadStoolConfig::default();
    config.security.encryption.enabled = true;
    config.security.encryption.algorithm = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Encryption algorithm is required when encryption is enabled")
    );
}

/// Test encryption key length validation
#[test]
fn test_validation_encryption_key_length_zero() {
    let mut config = ToadStoolConfig::default();
    config.security.encryption.key_length = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Encryption key length must be greater than 0")
    );
}

/// Test sandbox type required when sandbox enabled
#[test]
fn test_validation_sandbox_type_required() {
    let mut config = ToadStoolConfig::default();
    config.security.sandbox.enabled = true;
    config.security.sandbox.sandbox_type = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Sandbox type is required when sandboxing is enabled")
    );
}
