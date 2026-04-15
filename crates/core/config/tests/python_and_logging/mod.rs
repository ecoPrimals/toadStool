// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_config::ToadStoolConfig;

/// Test Python executable validation
#[test]
fn test_validation_python_executable_empty() {
    let mut config = ToadStoolConfig::default();
    config.runtime.python.executable = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Python executable cannot be empty")
    );
}

/// Test Python index URL: empty allowed (discovered at runtime)
#[test]
fn test_validation_python_index_url_empty() {
    let mut config = ToadStoolConfig::default();
    config.runtime.python.index_url = String::new();

    let result = config.validate_runtime_config();
    assert!(
        result.is_ok(),
        "empty index_url allowed (sovereignty: no external defaults)"
    );
}

/// Test Python max memory validation
#[test]
fn test_validation_python_max_memory_zero() {
    let mut config = ToadStoolConfig::default();
    config.runtime.python.max_memory = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Python max memory must be greater than 0")
    );
}

/// Test Python execution time validation
#[test]
fn test_validation_python_execution_time_zero() {
    let mut config = ToadStoolConfig::default();
    config.runtime.python.max_execution_time = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Python max execution time must be greater than 0")
    );
}

/// Test log level validation
#[test]
fn test_validation_log_level_empty() {
    let mut config = ToadStoolConfig::default();
    config.logging.level = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Log level cannot be empty")
    );
}

/// Test log format validation
#[test]
fn test_validation_log_format_empty() {
    let mut config = ToadStoolConfig::default();
    config.logging.format = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Log format cannot be empty")
    );
}

/// Test max log size validation
#[test]
fn test_validation_max_log_size_zero() {
    let mut config = ToadStoolConfig::default();
    config.logging.max_log_size = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Max log size must be greater than 0")
    );
}

/// Test max log files validation
#[test]
fn test_validation_max_log_files_zero() {
    let mut config = ToadStoolConfig::default();
    config.logging.max_log_files = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Max log files must be greater than 0")
    );
}
