// SPDX-License-Identifier: AGPL-3.0-or-later

use std::time::Duration;
use toadstool_config::{BackendCacheConfig, DatabaseConfig, MetricsConfig, ToadStoolConfig};

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
