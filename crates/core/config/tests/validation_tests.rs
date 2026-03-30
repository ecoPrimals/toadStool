// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for configuration validation
//!
//! Coverage expansion: validation.rs had ZERO test coverage (346 lines, 40+ validations)

use std::time::Duration;
use toadstool_config::{BackendCacheConfig, DatabaseConfig, MetricsConfig, ToadStoolConfig};

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
#[allow(deprecated)]
fn test_validation_empty_songbird_endpoint() {
    let mut config = ToadStoolConfig::default();
    config.network.endpoints.songbird = String::new();

    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Songbird endpoint cannot be empty")
    );
}

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

/// Test cache configuration validation
#[test]
fn test_validation_cache_config() {
    let mut config = ToadStoolConfig::default();
    // Test empty cache type
    let cache_config = BackendCacheConfig {
        cache_type: String::new(),
        ..Default::default()
    };
    config.cache = Some(cache_config);
    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Cache type cannot be empty")
    );

    // Test zero max size
    let cache_config2 = BackendCacheConfig {
        max_size: 0,
        ..Default::default()
    };
    config.cache = Some(cache_config2);
    let result2 = config.validate_runtime_config();
    assert!(result2.is_err());
    assert!(
        result2
            .unwrap_err()
            .to_string()
            .contains("Cache max size must be greater than 0")
    );

    // Test zero TTL
    let cache_config3 = BackendCacheConfig {
        ttl: Duration::from_secs(0),
        ..Default::default()
    };
    config.cache = Some(cache_config3);
    let result3 = config.validate_runtime_config();
    assert!(result3.is_err());
    assert!(
        result3
            .unwrap_err()
            .to_string()
            .contains("Cache TTL must be greater than 0")
    );
}

/// Test metrics configuration validation
#[test]
fn test_validation_metrics_config() {
    let mut config = ToadStoolConfig::default();
    // Test empty endpoint
    let metrics_config = MetricsConfig {
        endpoint: String::new(),
        ..Default::default()
    };
    config.metrics = Some(metrics_config);
    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Metrics endpoint cannot be empty")
    );

    // Test empty format
    let metrics_config2 = MetricsConfig {
        format: String::new(),
        ..Default::default()
    };
    config.metrics = Some(metrics_config2);
    let result2 = config.validate_runtime_config();
    assert!(result2.is_err());
    assert!(
        result2
            .unwrap_err()
            .to_string()
            .contains("Metrics format cannot be empty")
    );

    // Test zero collection interval
    let metrics_config3 = MetricsConfig {
        collection_interval: Duration::from_secs(0),
        ..Default::default()
    };
    config.metrics = Some(metrics_config3);
    let result3 = config.validate_runtime_config();
    assert!(result3.is_err());
    assert!(
        result3
            .unwrap_err()
            .to_string()
            .contains("Metrics collection interval must be greater than 0")
    );
}

/// Test database configuration validation
#[test]
fn test_validation_database_config() {
    let mut config = ToadStoolConfig::default();

    // Create DatabaseConfig manually (no Default impl)
    let db_config = DatabaseConfig {
        url: String::new(), // Empty URL (invalid)
        database_type: "postgres".to_string(),
        max_connections: 10,
        connection_timeout: Duration::from_secs(30),
        query_timeout: Duration::from_secs(30),
        enable_migrations: false,
        migration_dir: String::new(),
    };

    config.database = Some(db_config);
    let result = config.validate_runtime_config();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Database URL cannot be empty")
    );

    // Test empty database type
    let db_config2 = DatabaseConfig {
        url: "postgres://localhost".to_string(),
        database_type: String::new(), // Empty type (invalid)
        max_connections: 10,
        connection_timeout: Duration::from_secs(30),
        query_timeout: Duration::from_secs(30),
        enable_migrations: false,
        migration_dir: String::new(),
    };
    config.database = Some(db_config2);
    let result2 = config.validate_runtime_config();
    assert!(result2.is_err());
    assert!(
        result2
            .unwrap_err()
            .to_string()
            .contains("Database type cannot be empty")
    );

    // Test zero max connections
    let db_config3 = DatabaseConfig {
        url: "postgres://localhost".to_string(),
        database_type: "postgres".to_string(),
        max_connections: 0, // Zero (invalid)
        connection_timeout: Duration::from_secs(30),
        query_timeout: Duration::from_secs(30),
        enable_migrations: false,
        migration_dir: String::new(),
    };
    config.database = Some(db_config3);
    let result3 = config.validate_runtime_config();
    assert!(result3.is_err());
    assert!(
        result3
            .unwrap_err()
            .to_string()
            .contains("Database max connections must be greater than 0")
    );

    // Test zero connection timeout
    let db_config4 = DatabaseConfig {
        url: "postgres://localhost".to_string(),
        database_type: "postgres".to_string(),
        max_connections: 10,
        connection_timeout: Duration::from_secs(0), // Zero (invalid)
        query_timeout: Duration::from_secs(30),
        enable_migrations: false,
        migration_dir: String::new(),
    };
    config.database = Some(db_config4);
    let result4 = config.validate_runtime_config();
    assert!(result4.is_err());
    assert!(
        result4
            .unwrap_err()
            .to_string()
            .contains("Database connection timeout must be greater than 0")
    );

    // Test zero query timeout
    let db_config5 = DatabaseConfig {
        url: "postgres://localhost".to_string(),
        database_type: "postgres".to_string(),
        max_connections: 10,
        connection_timeout: Duration::from_secs(30),
        query_timeout: Duration::from_secs(0), // Zero (invalid)
        enable_migrations: false,
        migration_dir: String::new(),
    };
    config.database = Some(db_config5);
    let result5 = config.validate_runtime_config();
    assert!(result5.is_err());
    assert!(
        result5
            .unwrap_err()
            .to_string()
            .contains("Database query timeout must be greater than 0")
    );
}

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
