// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for environment variable configuration overrides
//!
//! Coverage expansion: `env_overrides.rs` had ZERO test coverage
//!
//! ✅ MODERNIZED: Uses `temp_env` for thread-safe, isolated env var testing

use std::time::Duration;
use toadstool_config::{BackendCacheConfig, MetricsConfig, ToadStoolConfig};

/// All `TOADSTOOL_*` vars that `apply_env_overrides` reads (for tests needing clean slate)
const ENV_OVERRIDE_VARS: &[&str] = &[
    "TOADSTOOL_ENV",
    "TOADSTOOL_DEBUG",
    "TOADSTOOL_VERBOSE",
    "TOADSTOOL_BIND_ADDRESS",
    "TOADSTOOL_PORT",
    "TOADSTOOL_SONGBIRD_ENDPOINT",
    "TOADSTOOL_BEARDOG_ENDPOINT",
    "TOADSTOOL_NESTGATE_ENDPOINT",
    "TOADSTOOL_SQUIRREL_ENDPOINT",
    "TOADSTOOL_MAX_CPU",
    "TOADSTOOL_MAX_MEMORY",
    "TOADSTOOL_LOG_LEVEL",
    "TOADSTOOL_DATA_DIR",
    "TOADSTOOL_CACHE_DIR",
    "TOADSTOOL_WORKER_THREADS",
    "TOADSTOOL_MAX_CONCURRENT_EXECUTIONS",
    "TOADSTOOL_EXECUTION_TIMEOUT",
    "TOADSTOOL_REQUEST_TIMEOUT",
    "TOADSTOOL_ENABLE_METRICS",
    "TOADSTOOL_ENABLE_CACHE",
    "TOADSTOOL_ENABLE_AUTH",
    "TOADSTOOL_ENABLE_SANDBOX",
    "TOADSTOOL_ENABLE_FEDERATION",
    "TOADSTOOL_ENABLE_DISTRIBUTED",
    "TOADSTOOL_ENABLE_AUTO_CONFIG",
    "TOADSTOOL_ENABLE_HOT_RELOAD",
    "TOADSTOOL_ENABLE_EXPERIMENTAL",
    "TOADSTOOL_ENABLE_BETA",
    "TOADSTOOL_ENABLE_PROFILING",
    "TOADSTOOL_ENABLE_OPENAPI",
    "TOADSTOOL_ENABLE_GRPC",
    "TOADSTOOL_ENABLE_GRAPHQL",
    "TOADSTOOL_CONTAINER_RUNTIME",
    "TOADSTOOL_CONTAINER_REGISTRY",
    "TOADSTOOL_CONTAINER_NETWORK_MODE",
    "TOADSTOOL_WASM_ENGINE",
    "TOADSTOOL_WASM_MAX_MEMORY",
    "TOADSTOOL_WASM_ENABLE_WASI",
    "TOADSTOOL_PYTHON_EXECUTABLE",
    "TOADSTOOL_PYTHON_VENV_PATH",
    "TOADSTOOL_PYTHON_INDEX_URL",
    "TOADSTOOL_PYTHON_MAX_MEMORY",
    "TOADSTOOL_JWT_SECRET",
    "TOADSTOOL_SESSION_TIMEOUT",
    "TOADSTOOL_MAX_LOGIN_ATTEMPTS",
    "TOADSTOOL_LOCKOUT_DURATION",
    "TOADSTOOL_ENCRYPTION_ENABLED",
    "TOADSTOOL_ENCRYPTION_ALGORITHM",
    "TOADSTOOL_ENCRYPTION_KEY_LENGTH",
    "TOADSTOOL_AUDIT_ENABLED",
    "TOADSTOOL_AUDIT_LOG_FILE",
    "TOADSTOOL_AUDIT_LOG_LEVEL",
    "TOADSTOOL_SANDBOX_TYPE",
    "TOADSTOOL_SANDBOX_ALLOW_NETWORK",
    "TOADSTOOL_SANDBOX_ALLOW_FILE_ACCESS",
    "TOADSTOOL_LOG_TO_FILE",
    "TOADSTOOL_LOG_FILE",
    "TOADSTOOL_LOG_FORMAT",
    "TOADSTOOL_LOG_COLORS",
    "TOADSTOOL_LOG_TIMESTAMPS",
    "TOADSTOOL_LOG_THREAD_IDS",
    "TOADSTOOL_LOG_MODULE_PATHS",
    "TOADSTOOL_LOG_ROTATION",
    "TOADSTOOL_LOG_MAX_SIZE",
    "TOADSTOOL_LOG_MAX_FILES",
];

#[test]
fn test_env_override_app_environment() {
    temp_env::with_var("TOADSTOOL_ENV", Some("production"), || {
        let mut config = ToadStoolConfig::default();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.app.environment, "production");
    });
}

#[test]
fn test_env_override_debug_flag() {
    temp_env::with_var("TOADSTOOL_DEBUG", Some("true"), || {
        let mut config = ToadStoolConfig::default();
        config.features.enable_debug = false;
        config.apply_env_overrides().unwrap();
        assert!(config.features.enable_debug);
    });
    temp_env::with_var("TOADSTOOL_DEBUG", Some("TRUE"), || {
        let mut config2 = ToadStoolConfig::default();
        config2.apply_env_overrides().unwrap();
        assert!(config2.features.enable_debug);
    });
    temp_env::with_var("TOADSTOOL_DEBUG", Some("false"), || {
        let mut config3 = ToadStoolConfig::default();
        config3.features.enable_debug = true;
        config3.apply_env_overrides().unwrap();
        assert!(!config3.features.enable_debug);
    });
}

#[test]
fn test_env_override_verbose() {
    temp_env::with_var("TOADSTOOL_VERBOSE", Some("true"), || {
        let mut config = ToadStoolConfig::default();
        config.logging.level = "info".to_string();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.logging.level, "debug");
    });
    temp_env::with_var("TOADSTOOL_VERBOSE", Some("false"), || {
        let mut config2 = ToadStoolConfig::default();
        config2.apply_env_overrides().unwrap();
        assert_eq!(config2.logging.level, "info");
    });
}

#[test]
fn test_env_override_bind_address() {
    temp_env::with_var("TOADSTOOL_BIND_ADDRESS", Some("0.0.0.0:8080"), || {
        let mut config = ToadStoolConfig::default();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.network.bind_address.to_string(), "0.0.0.0:8080");
    });
}

#[test]
fn test_env_override_bind_address_invalid() {
    temp_env::with_var("TOADSTOOL_BIND_ADDRESS", Some("invalid_address"), || {
        let mut config = ToadStoolConfig::default();
        let result = config.apply_env_overrides();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid bind address")
        );
    });
}

#[test]
fn test_env_override_port() {
    temp_env::with_var("TOADSTOOL_PORT", Some("9090"), || {
        let mut config = ToadStoolConfig::default();
        config.network.bind_address = "127.0.0.1:3000".parse().unwrap();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.network.bind_address.port(), 9090);
    });
}

#[test]
fn test_env_override_port_invalid() {
    temp_env::with_var("TOADSTOOL_PORT", Some("invalid"), || {
        let mut config = ToadStoolConfig::default();
        let result = config.apply_env_overrides();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid port"));
    });
}

#[test]
fn test_env_override_primal_endpoints() {
    temp_env::with_var(
        "TOADSTOOL_SONGBIRD_ENDPOINT",
        Some("http://songbird:8001"),
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();
        },
    );
}

#[test]
fn test_env_override_resource_limits() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_MAX_CPU", Some("80.5")),
            ("TOADSTOOL_MAX_MEMORY", Some("4294967296")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();
            assert!((config.runtime.resource_limits.max_cpu_usage - 80.5).abs() < 0.001);
            assert!(
                (config.runtime.resource_limits.max_memory_usage - 4_294_967_296.0).abs() < 0.1
            );
        },
    );
}

#[test]
fn test_env_override_resource_limits_invalid() {
    temp_env::with_var("TOADSTOOL_MAX_CPU", Some("invalid"), || {
        let mut config = ToadStoolConfig::default();
        let result = config.apply_env_overrides();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid max CPU"));
    });
    temp_env::with_var("TOADSTOOL_MAX_MEMORY", Some("not_a_number"), || {
        let mut config2 = ToadStoolConfig::default();
        let result2 = config2.apply_env_overrides();
        assert!(result2.is_err());
        assert!(
            result2
                .unwrap_err()
                .to_string()
                .contains("Invalid max memory")
        );
    });
}

#[test]
fn test_env_override_log_level() {
    temp_env::with_var("TOADSTOOL_LOG_LEVEL", Some("trace"), || {
        let mut config = ToadStoolConfig::default();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.logging.level, "trace");
    });
}

#[test]
fn test_env_override_directories() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_DATA_DIR", Some("/custom/data")),
            ("TOADSTOOL_CACHE_DIR", Some("/custom/cache")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();
            assert_eq!(config.app.data_dir, "/custom/data");
            assert_eq!(config.app.cache_dir, "/custom/cache");
        },
    );
}

#[test]
fn test_env_override_worker_threads() {
    temp_env::with_var("TOADSTOOL_WORKER_THREADS", Some("16"), || {
        let mut config = ToadStoolConfig::default();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.app.worker_threads, 16);
    });
}

#[test]
fn test_env_override_max_concurrent() {
    temp_env::with_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", Some("50"), || {
        let mut config = ToadStoolConfig::default();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.runtime.max_concurrent_executions, 50);
    });
}

#[test]
fn test_env_override_timeouts() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_EXECUTION_TIMEOUT", Some("300")),
            ("TOADSTOOL_REQUEST_TIMEOUT", Some("60")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();
            assert_eq!(config.runtime.execution_timeout, Duration::from_secs(300));
            assert_eq!(
                config.network.connection.request_timeout,
                Duration::from_secs(60)
            );
        },
    );
}

#[test]
fn test_env_override_metrics() {
    temp_env::with_var("TOADSTOOL_ENABLE_METRICS", Some("true"), || {
        let mut config = ToadStoolConfig {
            metrics: None,
            ..Default::default()
        };
        config.apply_env_overrides().unwrap();
        assert!(config.metrics.is_some());
    });
    temp_env::with_var("TOADSTOOL_ENABLE_METRICS", Some("false"), || {
        let mut config2 = ToadStoolConfig {
            metrics: Some(MetricsConfig::default()),
            ..Default::default()
        };
        config2.apply_env_overrides().unwrap();
        assert!(config2.metrics.is_none());
    });
}

#[test]
fn test_env_override_cache() {
    temp_env::with_var("TOADSTOOL_ENABLE_CACHE", Some("true"), || {
        let mut config = ToadStoolConfig {
            cache: None,
            ..Default::default()
        };
        config.apply_env_overrides().unwrap();
        assert!(config.cache.is_some());
    });
    temp_env::with_var("TOADSTOOL_ENABLE_CACHE", Some("false"), || {
        let mut config2 = ToadStoolConfig {
            cache: Some(BackendCacheConfig::default()),
            ..Default::default()
        };
        config2.apply_env_overrides().unwrap();
        assert!(config2.cache.is_none());
    });
}

#[test]
fn test_env_override_security_features() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENABLE_AUTH", Some("true")),
            ("TOADSTOOL_ENABLE_SANDBOX", Some("true")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.security.auth.enabled = false;
            config.security.sandbox.enabled = false;
            config.apply_env_overrides().unwrap();
            assert!(config.security.auth.enabled);
            assert!(config.security.sandbox.enabled);
        },
    );
}

#[test]
fn test_env_override_feature_flags() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENABLE_FEDERATION", Some("true")),
            ("TOADSTOOL_ENABLE_DISTRIBUTED", Some("true")),
            ("TOADSTOOL_ENABLE_AUTO_CONFIG", Some("true")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.features.enable_federation = false;
            config.features.enable_distributed = false;
            config.features.enable_auto_config = false;
            config.apply_env_overrides().unwrap();
            assert!(config.features.enable_federation);
            assert!(config.features.enable_distributed);
            assert!(config.features.enable_auto_config);
        },
    );
}

#[test]
fn test_env_override_experimental_flags() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENABLE_HOT_RELOAD", Some("true")),
            ("TOADSTOOL_ENABLE_EXPERIMENTAL", Some("true")),
            ("TOADSTOOL_ENABLE_BETA", Some("true")),
            ("TOADSTOOL_ENABLE_PROFILING", Some("true")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.features.enable_hot_reload = false;
            config.features.enable_experimental = false;
            config.features.enable_beta = false;
            config.features.enable_profiling = false;
            config.apply_env_overrides().unwrap();
            assert!(config.features.enable_hot_reload);
            assert!(config.features.enable_experimental);
            assert!(config.features.enable_beta);
            assert!(config.features.enable_profiling);
        },
    );
}

#[test]
fn test_env_override_api_protocols() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENABLE_OPENAPI", Some("true")),
            ("TOADSTOOL_ENABLE_GRPC", Some("true")),
            ("TOADSTOOL_ENABLE_GRAPHQL", Some("true")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.features.enable_openapi = false;
            #[expect(deprecated)]
            {
                config.features.enable_grpc = false;
            }
            config.features.enable_graphql = false;
            config.apply_env_overrides().unwrap();
            assert!(config.features.enable_openapi);
            #[expect(deprecated)]
            let grpc_enabled = config.features.enable_grpc;
            assert!(grpc_enabled);
            assert!(config.features.enable_graphql);
        },
    );
}

#[test]
fn test_env_override_container_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_CONTAINER_RUNTIME", Some("podman")),
            ("TOADSTOOL_CONTAINER_REGISTRY", Some("quay.io")),
            ("TOADSTOOL_CONTAINER_NETWORK_MODE", Some("host")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();
            assert_eq!(config.runtime.container.runtime, "podman");
            assert_eq!(config.runtime.container.default_registry, "quay.io");
            assert_eq!(config.runtime.container.network_mode, "host");
        },
    );
}

#[test]
fn test_env_override_wasm_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_WASM_ENGINE", Some("wasmi")),
            ("TOADSTOOL_WASM_MAX_MEMORY", Some("536870912")),
            ("TOADSTOOL_WASM_ENABLE_WASI", Some("true")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.runtime.wasm.enable_wasi = false;
            config.apply_env_overrides().unwrap();
            assert_eq!(config.runtime.wasm.engine, "wasmi");
            assert_eq!(config.runtime.wasm.max_memory, 536_870_912);
            assert!(config.runtime.wasm.enable_wasi);
        },
    );
}

#[test]
fn test_env_override_python_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_PYTHON_EXECUTABLE", Some("/usr/bin/python3.11")),
            ("TOADSTOOL_PYTHON_VENV_PATH", Some("/opt/venv")),
            (
                "TOADSTOOL_PYTHON_INDEX_URL",
                Some("https://pypi.org/simple"),
            ),
            ("TOADSTOOL_PYTHON_MAX_MEMORY", Some("2147483648")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();
            assert_eq!(config.runtime.python.executable, "/usr/bin/python3.11");
            assert_eq!(
                config.runtime.python.venv_path,
                Some("/opt/venv".to_string())
            );
            assert_eq!(config.runtime.python.index_url, "https://pypi.org/simple");
            assert_eq!(config.runtime.python.max_memory, 2_147_483_648);
        },
    );
}

#[test]
fn test_env_override_jwt_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_JWT_SECRET", Some("super_secret_key_12345")),
            ("TOADSTOOL_SESSION_TIMEOUT", Some("7200")),
            ("TOADSTOOL_MAX_LOGIN_ATTEMPTS", Some("5")),
            ("TOADSTOOL_LOCKOUT_DURATION", Some("1800")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.security.auth.jwt_secret = None;
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
        },
    );
}

#[test]
fn test_env_override_encryption_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENABLED", Some("true")),
            ("TOADSTOOL_ENCRYPTION_ALGORITHM", Some("AES-256-GCM")),
            ("TOADSTOOL_ENCRYPTION_KEY_LENGTH", Some("256")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.security.encryption.enabled = false;
            config.apply_env_overrides().unwrap();
            assert!(config.security.encryption.enabled);
            assert_eq!(config.security.encryption.algorithm, "AES-256-GCM");
            assert_eq!(config.security.encryption.key_length, 256);
        },
    );
}

#[test]
fn test_env_override_audit_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_AUDIT_ENABLED", Some("true")),
            (
                "TOADSTOOL_AUDIT_LOG_FILE",
                Some("/var/log/toadstool/audit.log"),
            ),
            ("TOADSTOOL_AUDIT_LOG_LEVEL", Some("info")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.security.audit.enabled = false;
            config.apply_env_overrides().unwrap();
            assert!(config.security.audit.enabled);
            assert_eq!(
                config.security.audit.log_file,
                "/var/log/toadstool/audit.log"
            );
            assert_eq!(config.security.audit.log_level, "info");
        },
    );
}

#[test]
fn test_env_override_sandbox_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SANDBOX_TYPE", Some("seccomp")),
            ("TOADSTOOL_SANDBOX_ALLOW_NETWORK", Some("false")),
            ("TOADSTOOL_SANDBOX_ALLOW_FILE_ACCESS", Some("false")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.security.sandbox.allow_network = true;
            config.security.sandbox.allow_file_access = true;
            config.apply_env_overrides().unwrap();
            assert_eq!(config.security.sandbox.sandbox_type, "seccomp");
            assert!(!config.security.sandbox.allow_network);
            assert!(!config.security.sandbox.allow_file_access);
        },
    );
}

#[test]
fn test_env_override_logging_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_LOG_TO_FILE", Some("true")),
            ("TOADSTOOL_LOG_FILE", Some("/var/log/toadstool/app.log")),
            ("TOADSTOOL_LOG_FORMAT", Some("json")),
            ("TOADSTOOL_LOG_COLORS", Some("false")),
            ("TOADSTOOL_LOG_TIMESTAMPS", Some("false")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.logging.log_to_file = false;
            config.logging.enable_colors = true;
            config.logging.enable_timestamps = true;
            config.apply_env_overrides().unwrap();
            assert!(config.logging.log_to_file);
            assert_eq!(config.logging.log_file, "/var/log/toadstool/app.log");
            assert_eq!(config.logging.format, "json");
            assert!(!config.logging.enable_colors);
            assert!(!config.logging.enable_timestamps);
        },
    );
}

#[test]
fn test_env_override_logging_advanced() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_LOG_THREAD_IDS", Some("true")),
            ("TOADSTOOL_LOG_MODULE_PATHS", Some("true")),
            ("TOADSTOOL_LOG_ROTATION", Some("true")),
            ("TOADSTOOL_LOG_MAX_SIZE", Some("104857600")),
            ("TOADSTOOL_LOG_MAX_FILES", Some("10")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.logging.enable_thread_ids = false;
            config.logging.enable_module_paths = false;
            config.logging.log_rotation = false;
            config.apply_env_overrides().unwrap();
            assert!(config.logging.enable_thread_ids);
            assert!(config.logging.enable_module_paths);
            assert!(config.logging.log_rotation);
            assert_eq!(config.logging.max_log_size, 104_857_600);
            assert_eq!(config.logging.max_log_files, 10);
        },
    );
}

#[test]
fn test_env_override_multiple_integration() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENV", Some("production")),
            ("TOADSTOOL_DEBUG", Some("false")),
            ("TOADSTOOL_PORT", Some("8080")),
            ("TOADSTOOL_LOG_LEVEL", Some("warn")),
            ("TOADSTOOL_WORKER_THREADS", Some("8")),
            ("TOADSTOOL_ENABLE_METRICS", Some("true")),
            ("TOADSTOOL_ENABLE_AUTH", Some("true")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();
            assert_eq!(config.app.environment, "production");
            assert!(!config.features.enable_debug);
            assert_eq!(config.network.bind_address.port(), 8080);
            assert_eq!(config.logging.level, "warn");
            assert_eq!(config.app.worker_threads, 8);
            assert!(config.metrics.is_some());
            assert!(config.security.auth.enabled);
        },
    );
}

#[test]
fn test_env_override_no_change_when_vars_missing() {
    temp_env::with_vars_unset(ENV_OVERRIDE_VARS, || {
        let original_config = ToadStoolConfig::default();
        let mut test_config = ToadStoolConfig::default();
        test_config.apply_env_overrides().unwrap();
        assert_eq!(test_config.app.environment, original_config.app.environment);
        assert_eq!(
            test_config.features.enable_debug,
            original_config.features.enable_debug
        );
        assert_eq!(test_config.logging.level, original_config.logging.level);
    });
}
