// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_config::ToadStoolConfig;

/// Test multiple validation failures return first error
#[test]
fn test_validation_multiple_failures_returns_first() {
    let mut config = ToadStoolConfig::default();
    config.app.worker_threads = 0;
    config.runtime.max_concurrent_executions = 0;

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    // Port 0 is now allowed; first error is worker_threads or max_concurrent
    let err = result.unwrap_err().to_string();
    assert!(err.contains("Worker") || err.contains("concurrent") || err.contains("worker"));
}

/// Test config with all optional sections None passes basic validation
#[test]
fn test_validation_optional_sections_none() {
    let config = ToadStoolConfig {
        cache: None,
        metrics: None,
        database: None,
        ..Default::default()
    };

    let result = config.validate_runtime_config();
    // Should pass validation (optional sections don't need validation when None)
    assert!(result.is_ok());
}

/// Test edge cases for valid values
#[test]
fn test_validation_edge_cases_valid() {
    let mut config = ToadStoolConfig::default();

    // Edge case: CPU at exactly 100%
    config.runtime.resource_limits.max_cpu_usage = 100.0;
    assert!(config.validate_runtime_config().is_ok());

    // Edge case: Memory at exactly 100%
    config.runtime.resource_limits.max_memory_usage = 100.0;
    assert!(config.validate_runtime_config().is_ok());

    // Edge case: Disk at exactly 100%
    config.runtime.resource_limits.max_disk_usage = 100.0;
    assert!(config.validate_runtime_config().is_ok());

    // Edge case: worker_threads = 1
    config.app.worker_threads = 1;
    assert!(config.validate_runtime_config().is_ok());
}
