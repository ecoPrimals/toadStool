// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;
use toadstool_config::ToadStoolConfig;

/// Test app name validation
#[test]
fn test_validation_app_name_empty() {
    let mut config = ToadStoolConfig::default();
    config.app.name = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Application name cannot be empty")
    );
}

/// Test worker threads validation
#[test]
fn test_validation_worker_threads_zero() {
    let mut config = ToadStoolConfig::default();
    config.app.worker_threads = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Worker threads must be greater than 0")
    );
}

/// Test queue size validation
#[test]
fn test_validation_queue_size_zero() {
    let mut config = ToadStoolConfig::default();
    config.app.queue_size = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Queue size must be greater than 0")
    );
}

/// Test batch size validation
#[test]
fn test_validation_batch_size_zero() {
    let mut config = ToadStoolConfig::default();
    config.app.batch_size = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Batch size must be greater than 0")
    );
}

/// Test max concurrent executions validation
#[test]
fn test_validation_max_concurrent_zero() {
    let mut config = ToadStoolConfig::default();
    config.runtime.max_concurrent_executions = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Max concurrent executions must be greater than 0")
    );
}

/// Test execution timeout validation
#[test]
fn test_validation_execution_timeout_zero() {
    let mut config = ToadStoolConfig::default();
    config.runtime.execution_timeout = Duration::from_secs(0);

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Execution timeout must be greater than 0")
    );
}
