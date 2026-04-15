// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_config::ToadStoolConfig;

/// Test default config passes validation
#[test]
fn test_default_config_valid() {
    let config = ToadStoolConfig::default();
    let result = config.validate_runtime_config();
    assert!(result.is_ok(), "Default config should be valid");
}

/// Test port 0 is allowed (OS-assigned at bind time)
#[test]
fn test_validation_port_zero_allowed() {
    let mut config = ToadStoolConfig::default();
    config.network.bind_address = "127.0.0.1:0".parse().unwrap();

    let result = config.validate_runtime_config();
    // Port 0 is valid for bind addresses (OS-assigned)
    assert!(result.is_ok() || !result.unwrap_err().to_string().contains("port cannot be 0"));
}

/// Test empty songbird endpoint validation (deprecated but still validated)
#[test]
#[expect(deprecated)]
fn test_validation_empty_coordination_endpoint() {
    let mut config = ToadStoolConfig::default();
    config.network.endpoints.coordination = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Coordination service endpoint cannot be empty")
    );
}
