// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;
use toadstool_config::ToadStoolConfig;

/// Test request timeout validation
#[test]
fn test_validation_request_timeout_zero() {
    let mut config = ToadStoolConfig::default();
    config.network.connection.request_timeout = Duration::from_secs(0);

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Request timeout must be greater than 0")
    );
}

/// Test connection timeout validation
#[test]
fn test_validation_connection_timeout_zero() {
    let mut config = ToadStoolConfig::default();
    config.network.connection.connection_timeout = Duration::from_secs(0);

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Connection timeout must be greater than 0")
    );
}

/// Test max retries validation
#[test]
fn test_validation_max_retries_zero() {
    let mut config = ToadStoolConfig::default();
    config.network.connection.max_retries = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Max retries must be greater than 0")
    );
}

/// Test max connections per host validation
#[test]
fn test_validation_max_connections_zero() {
    let mut config = ToadStoolConfig::default();
    config.network.connection.max_connections_per_host = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Max connections per host must be greater than 0")
    );
}
