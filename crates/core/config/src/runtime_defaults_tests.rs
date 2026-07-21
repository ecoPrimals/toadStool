// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use tempfile::NamedTempFile;

#[test]
fn test_development_config() {
    let config = ToadStoolConfig::development();
    assert_eq!(config.app.environment, "development");
    assert_eq!(config.logging.level, "debug");
    assert!(config.features.enable_debug);
    assert!(config.features.enable_hot_reload);
    assert!(!config.security.auth.enabled);
}

#[test]
fn test_production_config() {
    let config = ToadStoolConfig::production();
    assert_eq!(config.app.environment, "production");
    assert_eq!(config.logging.level, "info");
    assert!(!config.features.enable_debug);
    assert!(!config.features.enable_hot_reload);
    assert!(config.security.auth.enabled);
}

#[test]
fn test_testing_config() {
    let config = ToadStoolConfig::testing();
    assert_eq!(config.app.environment, "test");
    assert_eq!(config.logging.level, "debug");
    assert!(!config.security.auth.enabled);
}

#[test]
#[expect(deprecated)] // Testing legacy endpoint configuration
fn test_env_overrides() {
    let coord_url = crate::defaults::endpoints::coordination_localhost_bootstrap_url();
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENV", Some("test")),
            ("TOADSTOOL_DEBUG", Some("true")),
            ("TOADSTOOL_LOG_LEVEL", Some("debug")),
            ("TOADSTOOL_WORKER_THREADS", Some("8")),
            ("TOADSTOOL_COORDINATION_ENDPOINT", Some(coord_url.as_str())),
            ("TOADSTOOL_BIND_ADDRESS", Some("127.0.0.1:3000")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();

            assert_eq!(config.app.environment, "test");
            assert!(config.features.enable_debug);
            assert_eq!(config.logging.level, "debug");
            assert_eq!(config.app.worker_threads, 8);
            // `TOADSTOOL_COORDINATION_ENDPOINT` overrides default coordination URL (default bootstrap: `coordination_localhost_bootstrap_url()`).
            assert_eq!(
                config.network.endpoints.coordination,
                crate::defaults::endpoints::coordination_localhost_bootstrap_url()
            );
        },
    );
}

#[test]
fn test_config_validation() {
    let config = ToadStoolConfig::default();
    assert!(config.validate_runtime_config().is_ok());

    let mut invalid_config = config.clone();
    invalid_config.app.name = String::new();
    assert!(invalid_config.validate_runtime_config().is_err());

    let mut invalid_config = config.clone();
    invalid_config.app.worker_threads = 0;
    assert!(invalid_config.validate_runtime_config().is_err());

    let mut invalid_config = config.clone();
    invalid_config.runtime.resource_limits.max_cpu_usage = 150.0;
    assert!(invalid_config.validate_runtime_config().is_err());

    let mut invalid_config = config;
    invalid_config.runtime.max_concurrent_executions = 0;
    assert!(invalid_config.validate_runtime_config().is_err());
}

#[test]
fn test_config_file_operations() {
    let config = ToadStoolConfig::development();
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();

    // Test save
    config.save_to_file(temp_path).unwrap();

    // Test load
    let loaded_config = ToadStoolConfig::load_from_file(temp_path).unwrap();
    assert_eq!(loaded_config.app.environment, config.app.environment);
    assert_eq!(loaded_config.logging.level, config.logging.level);
    assert_eq!(loaded_config.app.worker_threads, config.app.worker_threads);
}

#[test]
fn test_config_json_serialization() {
    let config = ToadStoolConfig::default();
    let json = config.to_json().unwrap();
    assert!(json.contains("app"));
    assert!(json.contains("network"));
    assert!(json.contains("runtime"));
    assert!(json.contains("security"));
    assert!(json.contains("logging"));
}

#[test]
fn test_current_environment_detection() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENVIRONMENT", Some("production")),
            ("TOADSTOOL_ENV", Some("production")),
            ("ENVIRONMENT", Some("production")),
            ("ENV", Some("production")),
        ],
        || {
            let config = ToadStoolConfig::for_current_environment();
            assert_eq!(config.app.environment, "production");
        },
    );

    temp_env::with_vars(
        [
            ("TOADSTOOL_ENVIRONMENT", Some("staging")),
            ("TOADSTOOL_ENV", Some("staging")),
            ("ENVIRONMENT", Some("staging")),
            ("ENV", Some("staging")),
        ],
        || {
            let config = ToadStoolConfig::for_current_environment();
            assert_eq!(config.app.environment, "staging");
        },
    );
}

// ═══════════════════════════════════════════════════════════════════
// Additional tests for uncovered runtime default functions
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_for_current_environment_env_var_priority_toadstool_environment() {
    // TOADSTOOL_ENVIRONMENT has highest priority for initial env detection.
    // Must unset TOADSTOOL_ENV so apply_env_overrides doesn't overwrite.
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENVIRONMENT", Some("prod")),
            ("TOADSTOOL_ENV", None),
            ("ENVIRONMENT", Some("test")),
            ("ENV", Some("dev")),
        ],
        || {
            let config = ToadStoolConfig::for_current_environment();
            assert_eq!(config.app.environment, "prod");
        },
    );
}

#[test]
fn test_for_current_environment_env_var_priority_toadstool_env_fallback() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENVIRONMENT", None),
            ("TOADSTOOL_ENV", Some("staging")),
            ("ENVIRONMENT", None),
            ("ENV", None),
        ],
        || {
            let config = ToadStoolConfig::for_current_environment();
            assert_eq!(config.app.environment, "staging");
        },
    );
}

#[test]
fn test_load_with_overrides_success() {
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_ENVIRONMENT",
            "TOADSTOOL_ENV",
            "TOADSTOOL_VERBOSE",
            "TOADSTOOL_WORKER_THREADS",
            "TOADSTOOL_REQUEST_TIMEOUT",
            "TOADSTOOL_EXECUTION_TIMEOUT",
            "TOADSTOOL_MAX_CONCURRENT_EXECUTIONS",
            "TOADSTOOL_BIND_ADDRESS",
            "TOADSTOOL_PORT",
        ],
        || {
            let config = ToadStoolConfig::development();
            let temp_file = NamedTempFile::new().unwrap();
            config.save_to_file(temp_file.path()).unwrap();

            let result = ToadStoolConfig::load_with_overrides(temp_file.path());
            assert!(result.is_ok(), "load failed: {result:?}");
            let loaded = result.unwrap();
            assert_eq!(loaded.app.environment, config.app.environment);
        },
    );
}

#[test]
fn test_load_with_overrides_nonexistent_file() {
    let result = ToadStoolConfig::load_with_overrides("/nonexistent/path/config.toml");
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ConfigError::Io(_)),
        "expected Io error, got {err:?}"
    );
}

#[test]
fn test_load_from_env_only_success() {
    temp_env::with_var("TOADSTOOL_ENV", Some("test"), || {
        let result = ToadStoolConfig::load_from_env_only();
        assert!(result.is_ok());
    });
}

#[test]
fn test_save_to_file_then_load_roundtrip() {
    let config = ToadStoolConfig::testing();
    let temp_file = NamedTempFile::new().unwrap();
    let path = temp_file.path();

    config.save_to_file(path).unwrap();
    let content = std::fs::read_to_string(path).unwrap();
    assert!(content.contains("environment"));
    assert!(content.contains("test"));
}

#[test]
fn test_save_to_file_invalid_path() {
    let config = ToadStoolConfig::default();
    let result = config.save_to_file("/nonexistent/directory/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_to_json_success() {
    let config = ToadStoolConfig::default();
    let json = config.to_json().unwrap();
    assert!(json.contains("\"app\""));
    assert!(json.contains("\"network\""));
    assert!(json.contains("\"runtime\""));
}

#[test]
fn test_config_error_variants() {
    let invalid = ConfigError::Invalid("bad".into());
    assert!(invalid.to_string().contains("bad"));

    let missing = ConfigError::MissingField("name".into());
    assert!(missing.to_string().contains("name"));

    let io_err = ConfigError::Io(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "file not found",
    ));
    assert!(io_err.to_string().contains("file not found"));

    let addr_err: ConfigError = "invalid"
        .parse::<std::net::SocketAddr>()
        .unwrap_err()
        .into();
    assert!(addr_err.to_string().contains("Address"));

    let env_err = ConfigError::Env("TOADSTOOL_ENV".into());
    assert!(env_err.to_string().contains("TOADSTOOL_ENV"));
}

#[test]
fn test_print_summary_no_panic() {
    let config = ToadStoolConfig::default();
    config.print_summary();
}

#[test]
fn test_print_summary_with_cache() {
    let config = ToadStoolConfig {
        cache: Some(crate::BackendCacheConfig::default()),
        ..Default::default()
    };
    config.print_summary();
}

#[test]
fn test_print_summary_with_metrics() {
    let config = ToadStoolConfig {
        metrics: Some(crate::MetricsConfig::default()),
        ..Default::default()
    };
    config.print_summary();
}

#[test]
fn test_print_summary_with_database() {
    let config = ToadStoolConfig {
        database: Some(crate::DatabaseConfig {
            url: "sqlite::memory:".to_string(),
            database_type: "sqlite".to_string(),
            max_connections: 10,
            connection_timeout: std::time::Duration::from_secs(30),
            query_timeout: std::time::Duration::from_mins(1),
            enable_migrations: false,
            migration_dir: "migrations".to_string(),
        }),
        ..Default::default()
    };
    config.print_summary();
}

#[test]
fn test_for_current_environment_defaults_to_development_when_unset() {
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_ENVIRONMENT",
            "TOADSTOOL_ENV",
            "ENVIRONMENT",
            "ENV",
        ],
        || {
            let config = ToadStoolConfig::for_current_environment();
            assert_eq!(config.app.environment, "development");
        },
    );
}

#[test]
fn test_load_with_overrides_invalid_toml() {
    let temp_file = NamedTempFile::new().unwrap();
    std::fs::write(temp_file.path(), "invalid toml [[[").unwrap();

    let result = ToadStoolConfig::load_with_overrides(temp_file.path());
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ConfigError::Toml(_)),
        "expected Toml error, got {err:?}"
    );
}
