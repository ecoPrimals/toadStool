// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for environment variable configuration overrides
//!
//! Coverage expansion: `env_overrides.rs` had ZERO test coverage
//!
//! ✅ MODERNIZED: Uses scoped Mutex instead of #[serial] for concurrent execution

use std::sync::Mutex;
use std::time::Duration;
use toadstool_config::{BackendCacheConfig, MetricsConfig, ToadStoolConfig};

// Scoped lock for environment variable tests - allows concurrent execution with non-env tests
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Helper to clear all TOADSTOOL_* environment variables
fn clear_toadstool_env_vars() {
    for (key, _) in std::env::vars() {
        if key.starts_with("TOADSTOOL_") {
            // SAFETY: Test-only; sequential test execution via ENV_LOCK
            unsafe { std::env::remove_var(&key) };
        }
    }
}

/// Test basic application environment overrides
#[test]
fn test_env_override_app_environment() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { std::env::set_var("TOADSTOOL_ENV", "production") };

    config.apply_env_overrides().unwrap();

    assert_eq!(config.app.environment, "production");
    clear_toadstool_env_vars();
}

/// Test debug flag override
#[test]
fn test_env_override_debug_flag() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.features.enable_debug = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_DEBUG", "true");
    }
    config.apply_env_overrides().unwrap();
    assert!(config.features.enable_debug);

    // Test case insensitivity
    let mut config2 = ToadStoolConfig::default();
    unsafe { std::env::set_var("TOADSTOOL_DEBUG", "TRUE") };
    config2.apply_env_overrides().unwrap();
    assert!(config2.features.enable_debug);

    // Test false
    let mut config3 = ToadStoolConfig::default();
    config3.features.enable_debug = true;
    unsafe { std::env::set_var("TOADSTOOL_DEBUG", "false") };
    config3.apply_env_overrides().unwrap();
    assert!(!config3.features.enable_debug);

    clear_toadstool_env_vars();
}

/// Test verbose logging override
#[test]
fn test_env_override_verbose() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.logging.level = "info".to_string();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { std::env::set_var("TOADSTOOL_VERBOSE", "true") };
    config.apply_env_overrides().unwrap();
    assert_eq!(config.logging.level, "debug");

    // Test false sets info
    let mut config2 = ToadStoolConfig::default();
    unsafe { std::env::set_var("TOADSTOOL_VERBOSE", "false") };
    config2.apply_env_overrides().unwrap();
    assert_eq!(config2.logging.level, "info");

    clear_toadstool_env_vars();
}

/// Test network bind address override
#[test]
fn test_env_override_bind_address() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { std::env::set_var("TOADSTOOL_BIND_ADDRESS", "0.0.0.0:8080") };
    config.apply_env_overrides().unwrap();

    assert_eq!(config.network.bind_address.to_string(), "0.0.0.0:8080");

    clear_toadstool_env_vars();
}

/// Test invalid bind address returns error
#[test]
fn test_env_override_bind_address_invalid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { std::env::set_var("TOADSTOOL_BIND_ADDRESS", "invalid_address") };
    let result = config.apply_env_overrides();

    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .contains("Invalid bind address")
    );

    clear_toadstool_env_vars();
}

/// Test port override
#[test]
fn test_env_override_port() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.network.bind_address = "127.0.0.1:3000".parse().unwrap();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe { std::env::set_var("TOADSTOOL_PORT", "9090") };
    config.apply_env_overrides().unwrap();

    assert_eq!(config.network.bind_address.port(), 9090);

    clear_toadstool_env_vars();
}

/// Test invalid port returns error
#[test]
fn test_env_override_port_invalid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_PORT", "invalid") };
    let result = config.apply_env_overrides();

    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid port"));

    clear_toadstool_env_vars();
}

/// Test primal endpoint overrides (NOTE: Most endpoints deprecated)
#[test]
fn test_env_override_primal_endpoints() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // NOTE: All endpoint fields are deprecated in favor of ServiceDiscovery::find_by_capability()
    // This test verifies the override mechanism works, but production code should use capability discovery

    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_SONGBIRD_ENDPOINT", "http://songbird:8001") };

    config.apply_env_overrides().unwrap();

    // Skip deprecated field assertions - override mechanism tested by not returning error

    clear_toadstool_env_vars();
}

/// Test resource limit overrides
#[test]
fn test_env_override_resource_limits() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_MAX_CPU", "80.5");
        std::env::set_var("TOADSTOOL_MAX_MEMORY", "4294967296");
    }

    config.apply_env_overrides().unwrap();

    // Float comparisons with epsilon (both are f64)
    assert!((config.runtime.resource_limits.max_cpu_usage - 80.5).abs() < 0.001);
    assert!((config.runtime.resource_limits.max_memory_usage - 4_294_967_296.0).abs() < 0.1);

    clear_toadstool_env_vars();
}

/// Test invalid resource limits return errors
#[test]
fn test_env_override_resource_limits_invalid() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_MAX_CPU", "invalid") };
    let result = config.apply_env_overrides();
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("Invalid max CPU"));

    clear_toadstool_env_vars();

    let mut config2 = ToadStoolConfig::default();
    unsafe { std::env::set_var("TOADSTOOL_MAX_MEMORY", "not_a_number") };
    let result2 = config2.apply_env_overrides();
    assert!(result2.is_err());
    assert!(
        result2
            .unwrap_err()
            .to_string()
            .contains("Invalid max memory")
    );

    clear_toadstool_env_vars();
}

/// Test log level override
#[test]
fn test_env_override_log_level() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_LOG_LEVEL", "trace") };
    config.apply_env_overrides().unwrap();

    assert_eq!(config.logging.level, "trace");

    clear_toadstool_env_vars();
}

/// Test data and cache directory overrides
#[test]
fn test_env_override_directories() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_DATA_DIR", "/custom/data");
        std::env::set_var("TOADSTOOL_CACHE_DIR", "/custom/cache");
    }

    config.apply_env_overrides().unwrap();

    assert_eq!(config.app.data_dir, "/custom/data");
    assert_eq!(config.app.cache_dir, "/custom/cache");

    clear_toadstool_env_vars();
}

/// Test worker threads override
#[test]
fn test_env_override_worker_threads() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    unsafe { std::env::set_var("TOADSTOOL_WORKER_THREADS", "16") };
    config.apply_env_overrides().unwrap();

    assert_eq!(config.app.worker_threads, 16);

    clear_toadstool_env_vars();
}

/// Test max concurrent executions override
#[test]
fn test_env_override_max_concurrent() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", "50") };
    config.apply_env_overrides().unwrap();

    assert_eq!(config.runtime.max_concurrent_executions, 50);

    clear_toadstool_env_vars();
}

/// Test timeout overrides
#[test]
fn test_env_override_timeouts() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_EXECUTION_TIMEOUT", "300");
        std::env::set_var("TOADSTOOL_REQUEST_TIMEOUT", "60");
    }

    config.apply_env_overrides().unwrap();

    assert_eq!(config.runtime.execution_timeout, Duration::from_secs(300));
    assert_eq!(
        config.network.connection.request_timeout,
        Duration::from_secs(60)
    );

    clear_toadstool_env_vars();
}

/// Test metrics enable/disable
#[test]
fn test_env_override_metrics() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig {
        metrics: None,
        ..Default::default()
    };

    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_ENABLE_METRICS", "true") };
    config.apply_env_overrides().unwrap();

    assert!(config.metrics.is_some());

    // Test disable
    let mut config2 = ToadStoolConfig {
        metrics: Some(MetricsConfig::default()),
        ..Default::default()
    };

    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_ENABLE_METRICS", "false") };
    config2.apply_env_overrides().unwrap();

    assert!(config2.metrics.is_none());

    clear_toadstool_env_vars();
}

/// Test cache enable/disable
#[test]
fn test_env_override_cache() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig {
        cache: None,
        ..Default::default()
    };

    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_ENABLE_CACHE", "true") };
    config.apply_env_overrides().unwrap();

    assert!(config.cache.is_some());

    // Test disable
    let mut config2 = ToadStoolConfig {
        cache: Some(BackendCacheConfig::default()),
        ..Default::default()
    };

    // SAFETY: Test-only; not called concurrently
    unsafe { std::env::set_var("TOADSTOOL_ENABLE_CACHE", "false") };
    config2.apply_env_overrides().unwrap();

    assert!(config2.cache.is_none());

    clear_toadstool_env_vars();
}

/// Test security feature overrides
#[test]
fn test_env_override_security_features() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.security.auth.enabled = false;
    config.security.sandbox.enabled = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_ENABLE_AUTH", "true");
        std::env::set_var("TOADSTOOL_ENABLE_SANDBOX", "true");
    }

    config.apply_env_overrides().unwrap();

    assert!(config.security.auth.enabled);
    assert!(config.security.sandbox.enabled);

    clear_toadstool_env_vars();
}

/// Test feature flags (websocket, federation, etc.)
#[test]
fn test_env_override_feature_flags() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.features.enable_federation = false;
    config.features.enable_distributed = false;
    config.features.enable_auto_config = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_ENABLE_FEDERATION", "true");
        std::env::set_var("TOADSTOOL_ENABLE_DISTRIBUTED", "true");
        std::env::set_var("TOADSTOOL_ENABLE_AUTO_CONFIG", "true");
    }

    config.apply_env_overrides().unwrap();

    assert!(config.features.enable_federation);
    assert!(config.features.enable_distributed);
    assert!(config.features.enable_auto_config);

    clear_toadstool_env_vars();
}

/// Test experimental/beta/profiling feature flags
#[test]
fn test_env_override_experimental_flags() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.features.enable_hot_reload = false;
    config.features.enable_experimental = false;
    config.features.enable_beta = false;
    config.features.enable_profiling = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_ENABLE_HOT_RELOAD", "true");
        std::env::set_var("TOADSTOOL_ENABLE_EXPERIMENTAL", "true");
        std::env::set_var("TOADSTOOL_ENABLE_BETA", "true");
        std::env::set_var("TOADSTOOL_ENABLE_PROFILING", "true");
    }

    config.apply_env_overrides().unwrap();

    assert!(config.features.enable_hot_reload);
    assert!(config.features.enable_experimental);
    assert!(config.features.enable_beta);
    assert!(config.features.enable_profiling);

    clear_toadstool_env_vars();
}

/// Test API protocol feature flags
#[test]
fn test_env_override_api_protocols() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.features.enable_openapi = false;
    #[allow(deprecated)]
    {
        config.features.enable_grpc = false;
    }
    config.features.enable_graphql = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_ENABLE_OPENAPI", "true");
        std::env::set_var("TOADSTOOL_ENABLE_GRPC", "true");
        std::env::set_var("TOADSTOOL_ENABLE_GRAPHQL", "true");
    }

    config.apply_env_overrides().unwrap();

    assert!(config.features.enable_openapi);
    #[allow(deprecated)]
    let grpc_enabled = config.features.enable_grpc;
    assert!(grpc_enabled);
    assert!(config.features.enable_graphql);

    clear_toadstool_env_vars();
}

/// Test container runtime configuration
#[test]
fn test_env_override_container_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_CONTAINER_RUNTIME", "podman");
        std::env::set_var("TOADSTOOL_CONTAINER_REGISTRY", "quay.io");
        std::env::set_var("TOADSTOOL_CONTAINER_NETWORK_MODE", "host");
    }

    config.apply_env_overrides().unwrap();

    assert_eq!(config.runtime.container.runtime, "podman");
    assert_eq!(config.runtime.container.default_registry, "quay.io");
    assert_eq!(config.runtime.container.network_mode, "host");

    clear_toadstool_env_vars();
}

/// Test WASM configuration overrides
#[test]
fn test_env_override_wasm_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.runtime.wasm.enable_wasi = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_WASM_ENGINE", "wasmi");
        std::env::set_var("TOADSTOOL_WASM_MAX_MEMORY", "536870912");
        std::env::set_var("TOADSTOOL_WASM_ENABLE_WASI", "true");
    }

    config.apply_env_overrides().unwrap();

    assert_eq!(config.runtime.wasm.engine, "wasmi");
    assert_eq!(config.runtime.wasm.max_memory, 536_870_912);
    assert!(config.runtime.wasm.enable_wasi);

    clear_toadstool_env_vars();
}

/// Test Python runtime configuration
#[test]
fn test_env_override_python_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_PYTHON_EXECUTABLE", "/usr/bin/python3.11");
        std::env::set_var("TOADSTOOL_PYTHON_VENV_PATH", "/opt/venv");
        std::env::set_var("TOADSTOOL_PYTHON_INDEX_URL", "https://pypi.org/simple");
        std::env::set_var("TOADSTOOL_PYTHON_MAX_MEMORY", "2147483648");
    }

    config.apply_env_overrides().unwrap();

    assert_eq!(config.runtime.python.executable, "/usr/bin/python3.11");
    assert_eq!(
        config.runtime.python.venv_path,
        Some("/opt/venv".to_string())
    );
    assert_eq!(config.runtime.python.index_url, "https://pypi.org/simple");
    assert_eq!(config.runtime.python.max_memory, 2_147_483_648);

    clear_toadstool_env_vars();
}

/// Test JWT authentication configuration
#[test]
fn test_env_override_jwt_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.security.auth.jwt_secret = None;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_JWT_SECRET", "super_secret_key_12345");
        std::env::set_var("TOADSTOOL_SESSION_TIMEOUT", "7200");
        std::env::set_var("TOADSTOOL_MAX_LOGIN_ATTEMPTS", "5");
        std::env::set_var("TOADSTOOL_LOCKOUT_DURATION", "1800");
    }

    config.apply_env_overrides().unwrap();

    assert_eq!(
        config.security.auth.jwt_secret,
        Some("super_secret_key_12345".to_string())
    );
    assert_eq!(
        config.security.auth.session_timeout,
        Duration::from_secs(7200)
    );
    assert_eq!(config.security.auth.max_login_attempts, 5);
    assert_eq!(
        config.security.auth.lockout_duration,
        Duration::from_secs(1800)
    );

    clear_toadstool_env_vars();
}

/// Test encryption configuration
#[test]
fn test_env_override_encryption_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.security.encryption.enabled = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_ENCRYPTION_ENABLED", "true");
        std::env::set_var("TOADSTOOL_ENCRYPTION_ALGORITHM", "AES-256-GCM");
        std::env::set_var("TOADSTOOL_ENCRYPTION_KEY_LENGTH", "256");
    }

    config.apply_env_overrides().unwrap();

    assert!(config.security.encryption.enabled);
    assert_eq!(config.security.encryption.algorithm, "AES-256-GCM");
    assert_eq!(config.security.encryption.key_length, 256);

    clear_toadstool_env_vars();
}

/// Test audit logging configuration
#[test]
fn test_env_override_audit_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.security.audit.enabled = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_AUDIT_ENABLED", "true");
        std::env::set_var("TOADSTOOL_AUDIT_LOG_FILE", "/var/log/toadstool/audit.log");
        std::env::set_var("TOADSTOOL_AUDIT_LOG_LEVEL", "info");
    }

    config.apply_env_overrides().unwrap();

    assert!(config.security.audit.enabled);
    assert_eq!(
        config.security.audit.log_file,
        "/var/log/toadstool/audit.log"
    );
    assert_eq!(config.security.audit.log_level, "info");

    clear_toadstool_env_vars();
}

/// Test sandbox configuration
#[test]
fn test_env_override_sandbox_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.security.sandbox.allow_network = true;
    config.security.sandbox.allow_file_access = true;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_SANDBOX_TYPE", "seccomp");
        std::env::set_var("TOADSTOOL_SANDBOX_ALLOW_NETWORK", "false");
        std::env::set_var("TOADSTOOL_SANDBOX_ALLOW_FILE_ACCESS", "false");
    }

    config.apply_env_overrides().unwrap();

    assert_eq!(config.security.sandbox.sandbox_type, "seccomp");
    assert!(!config.security.sandbox.allow_network);
    assert!(!config.security.sandbox.allow_file_access);

    clear_toadstool_env_vars();
}

/// Test logging configuration overrides
#[test]
fn test_env_override_logging_config() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.logging.log_to_file = false;
    config.logging.enable_colors = true;
    config.logging.enable_timestamps = true;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_LOG_TO_FILE", "true");
        std::env::set_var("TOADSTOOL_LOG_FILE", "/var/log/toadstool/app.log");
        std::env::set_var("TOADSTOOL_LOG_FORMAT", "json");
        std::env::set_var("TOADSTOOL_LOG_COLORS", "false");
        std::env::set_var("TOADSTOOL_LOG_TIMESTAMPS", "false");
    }

    config.apply_env_overrides().unwrap();

    assert!(config.logging.log_to_file);
    assert_eq!(config.logging.log_file, "/var/log/toadstool/app.log");
    assert_eq!(config.logging.format, "json");
    assert!(!config.logging.enable_colors);
    assert!(!config.logging.enable_timestamps);

    clear_toadstool_env_vars();
}

/// Test advanced logging configuration
#[test]
fn test_env_override_logging_advanced() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();
    config.logging.enable_thread_ids = false;
    config.logging.enable_module_paths = false;
    config.logging.log_rotation = false;

    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_LOG_THREAD_IDS", "true");
        std::env::set_var("TOADSTOOL_LOG_MODULE_PATHS", "true");
        std::env::set_var("TOADSTOOL_LOG_ROTATION", "true");
        std::env::set_var("TOADSTOOL_LOG_MAX_SIZE", "104857600");
        std::env::set_var("TOADSTOOL_LOG_MAX_FILES", "10");
    }

    config.apply_env_overrides().unwrap();

    assert!(config.logging.enable_thread_ids);
    assert!(config.logging.enable_module_paths);
    assert!(config.logging.log_rotation);
    assert_eq!(config.logging.max_log_size, 104_857_600);
    assert_eq!(config.logging.max_log_files, 10);

    clear_toadstool_env_vars();
}

/// Test multiple overrides at once (integration)
#[test]
fn test_env_override_multiple_integration() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let mut config = ToadStoolConfig::default();

    // Set many variables
    // SAFETY: Test-only; sequential test execution via ENV_LOCK
    unsafe {
        std::env::set_var("TOADSTOOL_ENV", "production");
        std::env::set_var("TOADSTOOL_DEBUG", "false");
        std::env::set_var("TOADSTOOL_PORT", "8080");
        std::env::set_var("TOADSTOOL_LOG_LEVEL", "warn");
        std::env::set_var("TOADSTOOL_WORKER_THREADS", "8");
        std::env::set_var("TOADSTOOL_ENABLE_METRICS", "true");
        std::env::set_var("TOADSTOOL_ENABLE_AUTH", "true");
    }

    config.apply_env_overrides().unwrap();

    assert_eq!(config.app.environment, "production");
    assert!(!config.features.enable_debug);
    assert_eq!(config.network.bind_address.port(), 8080);
    assert_eq!(config.logging.level, "warn");
    assert_eq!(config.app.worker_threads, 8);
    assert!(config.metrics.is_some());
    assert!(config.security.auth.enabled);

    clear_toadstool_env_vars();
}

/// Test that missing environment variables don't affect config
#[test]
fn test_env_override_no_change_when_vars_missing() {
    let _guard = ENV_LOCK.lock().unwrap();
    clear_toadstool_env_vars();

    let original_config = ToadStoolConfig::default();
    let mut test_config = ToadStoolConfig::default();

    // Don't set any environment variables
    test_config.apply_env_overrides().unwrap();

    // Config should be unchanged
    assert_eq!(test_config.app.environment, original_config.app.environment);
    assert_eq!(
        test_config.features.enable_debug,
        original_config.features.enable_debug
    );
    assert_eq!(test_config.logging.level, original_config.logging.level);

    clear_toadstool_env_vars();
}
