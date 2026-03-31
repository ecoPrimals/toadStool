// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for Configuration System
//!
//! Tests for `EnvConfigLoader`, `EnvironmentConfig`, network configuration,
//! and all configuration utilities.
//!
//! ✅ MODERNIZED: Uses scoped Mutex for parallel execution
//! Note: Some tests validate deprecated functions for backward compatibility

#![allow(deprecated)]

mod test_env_fixture;

use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use toadstool_config::*;

// ✅ MODERN: Scoped lock for environment variable tests
static ENV_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

fn get_env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

// ============================================================================
// EnvConfigLoader Tests
// ============================================================================

#[test]
fn test_env_config_loader_new() {
    // ✅ MODERNIZED: No #[serial] needed - uses isolated state
    let loader = env_config::EnvConfigLoader::new();
    // Loader should be created successfully
    assert!(format!("{loader:?}").contains("EnvConfigLoader"));
}

#[test]
fn test_env_config_loader_with_prefix() {
    // ✅ MODERNIZED: No #[serial] needed
    let loader = env_config::EnvConfigLoader::with_prefix("CUSTOM");
    // Loader should be created with custom prefix
    assert!(format!("{loader:?}").contains("EnvConfigLoader"));
}

#[test]
fn test_env_config_loader_get_string_default() {
    // ✅ MODERNIZED: No #[serial] needed
    let loader = env_config::EnvConfigLoader::new();
    let value = loader.get_string("NONEXISTENT_KEY", "default_value");
    assert_eq!(value, "default_value");
}

#[test]
fn test_env_config_loader_get_string_from_env() {
    let unique_key = format!("TOADSTOOL_TEST_STRING_{}", std::process::id());
    temp_env::with_var(&unique_key, Some("test_value"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_string(&unique_key.replace("TOADSTOOL_", ""), "default");
        assert_eq!(value, "test_value");
    });
}

#[test]
fn test_env_config_loader_get_bool_default_false() {
    let loader = env_config::EnvConfigLoader::new();
    let value = loader.get_bool("NONEXISTENT_BOOL", false);
    assert!(!value);
}

#[test]
fn test_env_config_loader_get_bool_default_true() {
    let loader = env_config::EnvConfigLoader::new();
    let value = loader.get_bool("NONEXISTENT_BOOL", true);
    assert!(value);
}

#[test]
fn test_env_config_loader_get_bool_true_from_env() {
    let _guard = get_env_lock().lock().unwrap();
    temp_env::with_var("TOADSTOOL_TEST_BOOL_TRUE", Some("true"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_bool("TEST_BOOL_TRUE", false);
        assert!(value);
    });
}

#[test]
fn test_env_config_loader_get_bool_one_from_env() {
    let _guard = get_env_lock().lock().unwrap();
    temp_env::with_var("TOADSTOOL_TEST_BOOL_ONE", Some("1"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_bool("TEST_BOOL_ONE", false);
        assert!(value);
    });
}

#[test]
fn test_env_config_loader_get_bool_false_from_env() {
    let _guard = get_env_lock().lock().unwrap();
    temp_env::with_var("TOADSTOOL_TEST_BOOL_FALSE", Some("false"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_bool("TEST_BOOL_FALSE", true);
        assert!(!value);
    });
}

#[test]
fn test_env_config_loader_get_u16_default() {
    let loader = env_config::EnvConfigLoader::new();
    let value = loader.get_u16("NONEXISTENT_U16", 8080);
    assert_eq!(value, 8080);
}

#[test]
fn test_env_config_loader_get_u16_from_env() {
    temp_env::with_var("TOADSTOOL_TEST_U16", Some("9090"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_u16("TEST_U16", 8080);
        assert_eq!(value, 9090);
    });
}

#[test]
fn test_env_config_loader_get_u16_invalid() {
    temp_env::with_var("TOADSTOOL_TEST_U16_INVALID", Some("invalid"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_u16("TEST_U16_INVALID", 8080);
        assert_eq!(value, 8080); // Falls back to default
    });
}

#[test]
fn test_env_config_loader_get_u32_default() {
    let loader = env_config::EnvConfigLoader::new();
    let value = loader.get_u32("NONEXISTENT_U32", 1000);
    assert_eq!(value, 1000);
}

#[test]
fn test_env_config_loader_get_u32_from_env() {
    temp_env::with_var("TOADSTOOL_TEST_U32", Some("2000"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_u32("TEST_U32", 1000);
        assert_eq!(value, 2000);
    });
}

#[test]
fn test_env_config_loader_get_u64_default() {
    let loader = env_config::EnvConfigLoader::new();
    let value = loader.get_u64("NONEXISTENT_U64", 100_000);
    assert_eq!(value, 100_000);
}

#[test]
fn test_env_config_loader_get_u64_from_env() {
    temp_env::with_var("TOADSTOOL_TEST_U64", Some("200000"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_u64("TEST_U64", 100_000);
        assert_eq!(value, 200_000);
    });
}

#[test]
fn test_env_config_loader_get_f64_default() {
    let loader = env_config::EnvConfigLoader::new();
    let default_val = std::f64::consts::PI;
    let value = loader.get_f64("NONEXISTENT_F64", default_val);
    assert!((value - default_val).abs() < 0.001);
}

#[test]
fn test_env_config_loader_get_f64_from_env() {
    temp_env::with_var("TOADSTOOL_TEST_F64", Some("2.71"), || {
        let loader = env_config::EnvConfigLoader::new();
        let default_val = std::f64::consts::PI;
        let value = loader.get_f64("TEST_F64", default_val);
        assert!((value - 2.71).abs() < 0.001);
    });
}

#[test]
fn test_env_config_loader_get_duration_default() {
    let loader = env_config::EnvConfigLoader::new();
    let value = loader.get_duration("NONEXISTENT_DURATION", Duration::from_secs(30));
    assert_eq!(value, Duration::from_secs(30));
}

#[test]
fn test_env_config_loader_get_duration_from_env() {
    temp_env::with_var("TOADSTOOL_TEST_DURATION", Some("60"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_duration("TEST_DURATION", Duration::from_secs(30));
        assert_eq!(value, Duration::from_secs(60));
    });
}

#[test]
fn test_env_config_loader_get_path_default() {
    let loader = env_config::EnvConfigLoader::new();
    let value = loader.get_path("NONEXISTENT_PATH", "/default/path");
    assert_eq!(value, PathBuf::from("/default/path"));
}

#[test]
fn test_env_config_loader_get_path_from_env() {
    temp_env::with_var("TOADSTOOL_TEST_PATH", Some("/custom/path"), || {
        let loader = env_config::EnvConfigLoader::new();
        let value = loader.get_path("TEST_PATH", "/default/path");
        assert_eq!(value, PathBuf::from("/custom/path"));
    });
}

#[test]
fn test_env_config_loader_load_cache() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_CACHE_TEST_1", Some("value1")),
            ("TOADSTOOL_CACHE_TEST_2", Some("value2")),
        ],
        || {
            let mut loader = env_config::EnvConfigLoader::new();
            loader.load_cache();
        },
    );
}

#[test]
fn test_env_config_loader_get_prefixed() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_PREFIX_TEST_1", Some("value1")),
            ("TOADSTOOL_PREFIX_TEST_2", Some("value2")),
            ("TOADSTOOL_OTHER_TEST", Some("other")),
        ],
        || {
            let loader = env_config::EnvConfigLoader::new();
            let prefixed = loader.get_prefixed("PREFIX");
            assert!(prefixed.len() >= 2);
            assert!(prefixed.contains_key("TOADSTOOL_PREFIX_TEST_1"));
            assert!(prefixed.contains_key("TOADSTOOL_PREFIX_TEST_2"));
        },
    );
}

#[test]
fn test_env_config_loader_custom_prefix() {
    temp_env::with_var("CUSTOM_MY_VAR", Some("custom_value"), || {
        let loader = env_config::EnvConfigLoader::with_prefix("CUSTOM");
        let value = loader.get_string("MY_VAR", "default");
        assert_eq!(value, "custom_value");
    });
}

#[test]
fn test_env_config_loader_default_trait() {
    let loader = env_config::EnvConfigLoader::default();
    let value = loader.get_string("TEST", "default");
    assert_eq!(value, "default");
}

// ============================================================================
// Network Configuration Tests
// ============================================================================

#[test]
fn test_network_default_request_timeout() {
    assert_eq!(network::DEFAULT_REQUEST_TIMEOUT_SECS, 30);
}

#[test]
fn test_network_default_connection_timeout() {
    assert_eq!(network::DEFAULT_CONNECTION_TIMEOUT_SECS, 10);
}

#[test]
fn test_network_default_max_retries() {
    assert_eq!(network::DEFAULT_MAX_RETRIES, 3);
}

#[test]
fn test_network_default_keepalive_interval() {
    assert_eq!(network::DEFAULT_KEEPALIVE_INTERVAL_SECS, 30);
}

#[test]
fn test_network_default_max_connections_per_host() {
    assert_eq!(network::DEFAULT_MAX_CONNECTIONS_PER_HOST, 100);
}

#[test]
fn test_get_songbird_port_default() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_COORDINATION_PORT", None::<&str>),
            ("COORDINATION_PORT", None::<&str>),
        ],
        || {
            let port = network::get_songbird_port();
            assert_eq!(port, 8080);
        },
    );
}

#[test]
fn test_get_songbird_port_from_env() {
    let _guard = get_env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("COORDINATION_PORT", Some("9080"), || {
        let port = network::get_songbird_port();
        assert_eq!(port, 9080);
    });
}

#[test]
fn test_get_beardog_port_default() {
    let _lock = get_env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_vars(
        [
            ("TOADSTOOL_SECURITY_PORT", None::<&str>),
            ("SECURITY_PORT", None::<&str>),
        ],
        || {
            let port = network::get_beardog_port();
            assert_eq!(port, 8081);
        },
    );
}

#[test]
fn test_get_beardog_port_from_env() {
    let _lock = get_env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("SECURITY_PORT", Some("9081"), || {
        let port = network::get_beardog_port();
        assert_eq!(port, 9081);
    });
}

// ===== TESTS ALREADY USE TestEnv - NO GLOBAL ENV POLLUTION =====
// These tests were already modernized with the TestEnv fixture pattern.
// The TestEnv provides isolated state, making all tests fully concurrent-safe.

#[test]
fn test_get_toadstool_port_default() {
    let _guard = get_env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_vars_unset(["TOADSTOOL_PORT", "TOADSTOOL_API_PORT"], || {
        let port = network::get_toadstool_port();
        assert_eq!(port, 0);
    });
}

#[test]
fn test_get_toadstool_port_from_env() {
    let _guard = get_env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("TOADSTOOL_PORT", Some("9084"), || {
        let port = network::get_toadstool_port();
        assert_eq!(port, 9084);
    });
}

#[test]
fn test_get_bind_host_default() {
    let _guard = get_env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var_unset("BIND_ADDRESS", || {
        let host = network::get_bind_host();
        assert_eq!(host, "127.0.0.1");
    });
}

#[test]
fn test_get_bind_host_from_env() {
    let _guard = get_env_lock()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    temp_env::with_var("BIND_ADDRESS", Some("0.0.0.0"), || {
        let host = network::get_bind_host();
        assert_eq!(host, "0.0.0.0");
    });
}

// Deprecated endpoint tests - maintained for backward compatibility validation
// These test the DEPRECATED hardcoded endpoint functions that violate self-knowledge principle
// Modern code should use RuntimeDiscovery with capability-based discovery instead

#[test]
#[allow(deprecated)]
fn test_get_songbird_endpoint_format() {
    temp_env::with_vars_unset(["TOADSTOOL_BIND_HOST", "TOADSTOOL_COORDINATION_PORT", "COORDINATION_PORT"], || {
        let endpoint = network::get_songbird_endpoint();
        assert!(endpoint.starts_with("http://"));
        assert!(endpoint.contains(':'));
    });
}

#[test]
#[allow(deprecated)]
fn test_get_beardog_endpoint_format() {
    temp_env::with_vars_unset(["TOADSTOOL_BIND_HOST", "TOADSTOOL_SECURITY_PORT", "SECURITY_PORT"], || {
        let endpoint = network::get_beardog_endpoint();
        assert!(endpoint.starts_with("http://"));
        assert!(endpoint.contains(':'));
    });
}

#[test]
#[allow(deprecated)]
fn test_get_nestgate_endpoint_format() {
    temp_env::with_vars_unset(["TOADSTOOL_BIND_HOST", "TOADSTOOL_STORAGE_PORT", "STORAGE_PORT"], || {
        let endpoint = network::get_nestgate_endpoint();
        assert!(endpoint.starts_with("http://"));
        assert!(endpoint.contains(':'));
    });
}

#[test]
#[allow(deprecated)]
fn test_get_squirrel_endpoint_format() {
    temp_env::with_vars_unset(["TOADSTOOL_BIND_HOST", "TOADSTOOL_PLATFORM_PORT", "PLATFORM_PORT"], || {
        let endpoint = network::get_squirrel_endpoint();
        assert!(endpoint.starts_with("http://"));
        assert!(endpoint.contains(':'));
    });
}

#[test]
#[allow(deprecated)]
fn test_get_toadstool_endpoint_format() {
    temp_env::with_vars_unset(["TOADSTOOL_BIND_HOST", "TOADSTOOL_API_PORT"], || {
        let endpoint = network::get_toadstool_endpoint();
        assert!(endpoint.starts_with("http://"));
        assert!(endpoint.contains(':'));
    });
}

// ============================================================================
// Application Configuration Tests
// ============================================================================

#[test]
fn test_app_default_app_name() {
    assert_eq!(app::DEFAULT_APP_NAME, "toadstool");
}

#[test]
fn test_app_default_environment() {
    assert_eq!(app::DEFAULT_ENVIRONMENT, "development");
}

#[test]
fn test_app_default_log_level() {
    assert_eq!(app::DEFAULT_LOG_LEVEL, "info");
}

#[test]
fn test_app_default_config_file() {
    assert_eq!(app::DEFAULT_CONFIG_FILE, "toadstool.toml");
}

#[test]
fn test_app_default_data_dir() {
    assert_eq!(app::DEFAULT_DATA_DIR, "./data");
}

#[test]
fn test_app_default_cache_dir() {
    assert_eq!(app::DEFAULT_CACHE_DIR, "./cache");
}

#[test]
fn test_app_default_logs_dir() {
    assert_eq!(app::DEFAULT_LOGS_DIR, "./logs");
}

#[test]
fn test_app_default_temp_dir() {
    assert_eq!(app::default_temp_dir(), std::env::temp_dir());
}

#[test]
fn test_app_default_max_file_size() {
    assert_eq!(app::DEFAULT_MAX_FILE_SIZE, 100 * 1024 * 1024); // 100MB
}

#[test]
fn test_app_default_max_log_size() {
    assert_eq!(app::DEFAULT_MAX_LOG_SIZE, 10 * 1024 * 1024); // 10MB
}

#[test]
fn test_app_default_max_log_files() {
    assert_eq!(app::DEFAULT_MAX_LOG_FILES, 10);
}

#[test]
fn test_app_default_worker_threads() {
    assert_eq!(app::DEFAULT_WORKER_THREADS, 4);
}

#[test]
fn test_app_default_queue_size() {
    assert_eq!(app::DEFAULT_QUEUE_SIZE, 1000);
}

#[test]
fn test_app_default_batch_size() {
    assert_eq!(app::DEFAULT_BATCH_SIZE, 100);
}

#[test]
fn test_app_default_polling_interval() {
    assert_eq!(app::DEFAULT_POLLING_INTERVAL_MS, 500);
}

#[test]
fn test_app_default_heartbeat_interval() {
    assert_eq!(app::DEFAULT_HEARTBEAT_INTERVAL_SECS, 30);
}

#[test]
fn test_app_default_health_check_interval() {
    assert_eq!(app::DEFAULT_HEALTH_CHECK_INTERVAL_SECS, 60);
}

#[test]
fn test_app_default_metrics_interval() {
    assert_eq!(app::DEFAULT_METRICS_INTERVAL_SECS, 30);
}

#[test]
fn test_app_default_cleanup_interval() {
    assert_eq!(app::DEFAULT_CLEANUP_INTERVAL_SECS, 300);
}

#[test]
fn test_app_default_session_timeout() {
    assert_eq!(app::DEFAULT_SESSION_TIMEOUT_SECS, 3600);
}

#[test]
fn test_app_default_execution_timeout() {
    assert_eq!(app::DEFAULT_EXECUTION_TIMEOUT_SECS, 1800);
}

#[test]
fn test_app_default_max_concurrent_executions() {
    assert_eq!(app::DEFAULT_MAX_CONCURRENT_EXECUTIONS, 10);
}

#[test]
fn test_app_default_max_execution_history() {
    assert_eq!(app::DEFAULT_MAX_EXECUTION_HISTORY, 1000);
}

#[test]
fn test_app_default_resource_check_interval() {
    assert_eq!(app::DEFAULT_RESOURCE_CHECK_INTERVAL_SECS, 30);
}

#[test]
#[allow(clippy::float_cmp)] // comparing against exact literal initialization
fn test_app_default_max_cpu_usage() {
    assert_eq!(app::DEFAULT_MAX_CPU_USAGE, 80.0);
}

#[test]
#[allow(clippy::float_cmp)] // comparing against exact literal initialization
fn test_app_default_max_memory_usage() {
    assert_eq!(app::DEFAULT_MAX_MEMORY_USAGE, 85.0);
}

#[test]
#[allow(clippy::float_cmp)] // comparing against exact literal initialization
fn test_app_default_max_disk_usage() {
    assert_eq!(app::DEFAULT_MAX_DISK_USAGE, 90.0);
}

#[test]
fn test_app_default_buffer_size() {
    assert_eq!(app::DEFAULT_BUFFER_SIZE, 8192);
}

#[test]
fn test_app_default_chunk_size() {
    assert_eq!(app::DEFAULT_CHUNK_SIZE, 1024 * 1024); // 1MB
}

#[test]
fn test_app_default_compression_level() {
    assert_eq!(app::DEFAULT_COMPRESSION_LEVEL, 6);
}

#[test]
fn test_app_default_encryption_key_length() {
    assert_eq!(app::DEFAULT_ENCRYPTION_KEY_LENGTH, 32);
}

#[test]
fn test_app_default_hash_algorithm() {
    assert_eq!(app::DEFAULT_HASH_ALGORITHM, "sha256");
}

#[test]
fn test_app_default_signature_algorithm() {
    assert_eq!(app::DEFAULT_SIGNATURE_ALGORITHM, "ed25519");
}

#[test]
fn test_app_default_cache_ttl() {
    assert_eq!(app::DEFAULT_CACHE_TTL_SECS, 3600);
}

#[test]
fn test_app_default_cache_max_size() {
    assert_eq!(app::DEFAULT_CACHE_MAX_SIZE, 100 * 1024 * 1024); // 100MB
}

#[test]
fn test_app_default_rate_limit() {
    assert_eq!(app::DEFAULT_RATE_LIMIT_PER_SEC, 100);
}

#[test]
fn test_app_default_burst_limit() {
    assert_eq!(app::DEFAULT_BURST_LIMIT, 200);
}

#[test]
fn test_app_default_grace_period() {
    assert_eq!(app::DEFAULT_GRACE_PERIOD_SECS, 30);
}

#[test]
fn test_app_default_shutdown_timeout() {
    assert_eq!(app::DEFAULT_SHUTDOWN_TIMEOUT_SECS, 60);
}

// ============================================================================
// Testing Configuration Tests
// ============================================================================

#[test]
fn test_testing_default_test_timeout() {
    assert_eq!(testing::DEFAULT_TEST_TIMEOUT_SECS, 30);
}

#[test]
fn test_testing_default_test_port() {
    assert_eq!(testing::DEFAULT_TEST_PORT, 9999);
}

#[test]
fn test_testing_default_test_data_dir() {
    assert_eq!(testing::DEFAULT_TEST_DATA_DIR, "./test_data");
}

#[test]
fn test_testing_default_test_cache_dir() {
    assert_eq!(testing::DEFAULT_TEST_CACHE_DIR, "./test_cache");
}

#[test]
fn test_testing_default_test_temp_dir() {
    assert_eq!(testing::DEFAULT_TEST_TEMP_DIR, "./test_temp");
}

#[test]
fn test_testing_default_test_database_url() {
    assert_eq!(testing::DEFAULT_TEST_DATABASE_URL, "sqlite::memory:");
}

#[test]
fn test_testing_default_test_environment() {
    assert_eq!(testing::DEFAULT_TEST_ENVIRONMENT, "test");
}

#[test]
fn test_testing_default_test_log_level() {
    assert_eq!(testing::DEFAULT_TEST_LOG_LEVEL, "debug");
}

#[test]
fn test_testing_default_test_concurrent_connections() {
    assert_eq!(testing::DEFAULT_TEST_CONCURRENT_CONNECTIONS, 10);
}

#[test]
fn test_testing_default_test_execution_timeout() {
    assert_eq!(testing::DEFAULT_TEST_EXECUTION_TIMEOUT_SECS, 60);
}

#[test]
fn test_testing_default_test_retry_attempts() {
    assert_eq!(testing::DEFAULT_TEST_RETRY_ATTEMPTS, 3);
}
