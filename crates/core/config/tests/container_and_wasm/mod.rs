// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_config::ToadStoolConfig;

/// Test container runtime validation
#[test]
fn test_validation_container_runtime_empty() {
    let mut config = ToadStoolConfig::default();
    config.runtime.container.runtime = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Container runtime cannot be empty")
    );
}

/// Test default registry validation
#[test]
fn test_validation_default_registry_empty() {
    let mut config = ToadStoolConfig::default();
    config.runtime.container.default_registry = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Default registry cannot be empty")
    );
}

/// Test port range validation
#[test]
fn test_validation_port_range_invalid() {
    let mut config = ToadStoolConfig::default();
    // Start >= End
    config.runtime.container.port_range = (8080, 8080);

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("port range start must be less than end")
    );

    // Start > End
    let mut config2 = ToadStoolConfig::default();
    config2.runtime.container.port_range = (9000, 8000);
    let result2 = config2.validate_runtime_config();
    assert!(result2.is_err());
}

/// Test WASM engine validation
#[test]
fn test_validation_wasm_engine_empty() {
    let mut config = ToadStoolConfig::default();
    config.runtime.wasm.engine = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("WASM engine cannot be empty")
    );
}

/// Test WASM max memory validation
#[test]
fn test_validation_wasm_max_memory_zero() {
    let mut config = ToadStoolConfig::default();
    config.runtime.wasm.max_memory = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("WASM max memory must be greater than 0")
    );
}

/// Test WASM execution time validation
#[test]
fn test_validation_wasm_execution_time_zero() {
    let mut config = ToadStoolConfig::default();
    config.runtime.wasm.max_execution_time = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("WASM max execution time must be greater than 0")
    );
}
