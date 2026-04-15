// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_config::ToadStoolConfig;

/// Test CPU usage range validation
#[test]
fn test_validation_cpu_usage_range() {
    // Test negative CPU
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_cpu_usage = -1.0;
    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("between 0 and 100")
    );

    // Test zero CPU
    let mut config2 = ToadStoolConfig::default();
    config2.runtime.resource_limits.max_cpu_usage = 0.0;
    let result2 = config2.validate_runtime_config();
    assert!(result2.is_err());

    // Test CPU > 100
    let mut config3 = ToadStoolConfig::default();
    config3.runtime.resource_limits.max_cpu_usage = 101.0;
    let result3 = config3.validate_runtime_config();
    assert!(result3.is_err());

    // Test valid CPU (edge case 100.0)
    let mut config4 = ToadStoolConfig::default();
    config4.runtime.resource_limits.max_cpu_usage = 100.0;
    let result4 = config4.validate_runtime_config();
    assert!(result4.is_ok());
}

/// Test memory usage range validation
#[test]
fn test_validation_memory_usage_range() {
    // Test negative memory
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_memory_usage = -1.0;
    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Max memory usage must be between 0 and 100")
    );

    // Test memory > 100
    let mut config2 = ToadStoolConfig::default();
    config2.runtime.resource_limits.max_memory_usage = 150.0;
    let result2 = config2.validate_runtime_config();
    assert!(result2.is_err());
}

/// Test disk usage range validation
#[test]
fn test_validation_disk_usage_range() {
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_disk_usage = 101.0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Max disk usage must be between 0 and 100")
    );
}
