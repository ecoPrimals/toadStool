// SPDX-License-Identifier: AGPL-3.0-only
//! Unit tests for environment variable overrides.

use crate::ToadStoolConfig;

#[test]
fn apply_env_overrides_sets_environment() {
    temp_env::with_var("TOADSTOOL_ENV", Some("staging"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert_eq!(c.app.environment, "staging");
    });
}

#[test]
fn apply_env_overrides_debug_true() {
    temp_env::with_var("TOADSTOOL_DEBUG", Some("true"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert!(c.features.enable_debug);
    });
}

#[test]
fn apply_env_overrides_verbose_sets_debug_level() {
    temp_env::with_var("TOADSTOOL_VERBOSE", Some("true"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert_eq!(c.logging.level, "debug");
    });
}

#[test]
fn apply_env_overrides_bind_address() {
    temp_env::with_var("TOADSTOOL_BIND_ADDRESS", Some("0.0.0.0:9000"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert_eq!(c.network.bind_address.port(), 9000);
    });
}

#[test]
fn apply_env_overrides_invalid_bind_address_returns_error() {
    temp_env::with_var("TOADSTOOL_BIND_ADDRESS", Some("not-valid"), || {
        let mut c = ToadStoolConfig::default();
        let r = c.apply_env_overrides();
        assert!(r.is_err());
    });
}

#[test]
fn apply_env_overrides_port() {
    temp_env::with_var("TOADSTOOL_PORT", Some("7777"), || {
        let mut c = ToadStoolConfig::default();
        c.network.bind_address = "127.0.0.1:3000".parse().unwrap();
        c.apply_env_overrides().unwrap();
        assert_eq!(c.network.bind_address.port(), 7777);
    });
}

#[test]
fn apply_env_overrides_invalid_port_returns_error() {
    temp_env::with_var("TOADSTOOL_PORT", Some("abc"), || {
        let mut c = ToadStoolConfig::default();
        let r = c.apply_env_overrides();
        assert!(r.is_err());
    });
}

#[allow(deprecated)]
#[test]
fn apply_env_overrides_legacy_endpoints() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_SONGBIRD_ENDPOINT", Some("http://sb:8000")),
            ("TOADSTOOL_BEARDOG_ENDPOINT", Some("http://bd:8001")),
            ("TOADSTOOL_NESTGATE_ENDPOINT", Some("http://ng:8002")),
            ("TOADSTOOL_SQUIRREL_ENDPOINT", Some("http://sq:8003")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert_eq!(c.network.endpoints.coordination, "http://sb:8000");
            assert_eq!(c.network.endpoints.security, "http://bd:8001");
            assert_eq!(c.network.endpoints.storage, "http://ng:8002");
            assert_eq!(c.network.endpoints.ai_processing, "http://sq:8003");
        },
    );
}

#[test]
fn apply_env_overrides_resource_limits() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_MAX_CPU", Some("75.0")),
            ("TOADSTOOL_MAX_MEMORY", Some("2147483648")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert!((c.runtime.resource_limits.max_cpu_usage - 75.0).abs() < 0.01);
            assert!((c.runtime.resource_limits.max_memory_usage - 2_147_483_648.0).abs() < 1.0);
        },
    );
}

#[test]
fn apply_env_overrides_invalid_max_cpu_returns_error() {
    temp_env::with_var("TOADSTOOL_MAX_CPU", Some("not-a-float"), || {
        let mut c = ToadStoolConfig::default();
        let r = c.apply_env_overrides();
        assert!(r.is_err());
    });
}

#[test]
fn apply_env_overrides_data_cache_dirs() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_DATA_DIR", Some("/data")),
            ("TOADSTOOL_CACHE_DIR", Some("/cache")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert_eq!(c.app.data_dir, "/data");
            assert_eq!(c.app.cache_dir, "/cache");
        },
    );
}

#[test]
fn apply_env_overrides_worker_threads() {
    temp_env::with_var("TOADSTOOL_WORKER_THREADS", Some("32"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert_eq!(c.app.worker_threads, 32);
    });
}

#[test]
fn apply_env_overrides_execution_timeout() {
    temp_env::with_var("TOADSTOOL_EXECUTION_TIMEOUT", Some("120"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert_eq!(
            c.runtime.execution_timeout,
            std::time::Duration::from_secs(120)
        );
    });
}

#[test]
fn apply_env_overrides_enable_metrics_true() {
    temp_env::with_var("TOADSTOOL_ENABLE_METRICS", Some("true"), || {
        let mut c = ToadStoolConfig {
            metrics: None,
            ..Default::default()
        };
        c.apply_env_overrides().unwrap();
        assert!(c.metrics.is_some());
    });
}

#[test]
fn apply_env_overrides_enable_cache_true() {
    temp_env::with_var("TOADSTOOL_ENABLE_CACHE", Some("true"), || {
        let mut c = ToadStoolConfig {
            cache: None,
            ..Default::default()
        };
        c.apply_env_overrides().unwrap();
        assert!(c.cache.is_some());
    });
}

#[test]
fn apply_env_overrides_feature_flags() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENABLE_FEDERATION", Some("true")),
            ("TOADSTOOL_ENABLE_GRPC", Some("true")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert!(c.features.enable_federation);
            #[allow(deprecated)]
            let grpc_enabled = c.features.enable_grpc;
            assert!(grpc_enabled);
        },
    );
}

#[test]
fn apply_env_overrides_container_runtime() {
    temp_env::with_var("TOADSTOOL_CONTAINER_RUNTIME", Some("containerd"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert_eq!(c.runtime.container.runtime, "containerd");
    });
}

#[test]
fn apply_env_overrides_wasm_settings() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_WASM_ENGINE", Some("wasmtime")),
            ("TOADSTOOL_WASM_MAX_MEMORY", Some("128")),
            ("TOADSTOOL_WASM_ENABLE_WASI", Some("true")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert_eq!(c.runtime.wasm.engine, "wasmtime");
            assert_eq!(c.runtime.wasm.max_memory, 128);
            assert!(c.runtime.wasm.enable_wasi);
        },
    );
}

#[test]
fn apply_env_overrides_python_settings() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_PYTHON_EXECUTABLE", Some("/usr/bin/python3")),
            ("TOADSTOOL_PYTHON_VENV_PATH", Some("/venv")),
            (
                "TOADSTOOL_PYTHON_INDEX_URL",
                Some("https://pypi.org/simple"),
            ),
            ("TOADSTOOL_PYTHON_MAX_MEMORY", Some("512")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert_eq!(c.runtime.python.executable, "/usr/bin/python3");
            assert_eq!(c.runtime.python.venv_path, Some("/venv".to_string()));
            assert_eq!(c.runtime.python.index_url, "https://pypi.org/simple");
            assert_eq!(c.runtime.python.max_memory, 512);
        },
    );
}

#[test]
fn apply_env_overrides_security_auth() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_JWT_SECRET", Some("secret123")),
            ("TOADSTOOL_SESSION_TIMEOUT", Some("600")),
            ("TOADSTOOL_MAX_LOGIN_ATTEMPTS", Some("5")),
            ("TOADSTOOL_LOCKOUT_DURATION", Some("300")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert_eq!(c.security.auth.jwt_secret, Some("secret123".to_string()));
            assert_eq!(
                c.security.auth.session_timeout,
                std::time::Duration::from_secs(600)
            );
            assert_eq!(c.security.auth.max_login_attempts, 5);
            assert_eq!(
                c.security.auth.lockout_duration,
                std::time::Duration::from_secs(300)
            );
        },
    );
}

#[test]
fn apply_env_overrides_encryption() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENCRYPTION_ENABLED", Some("true")),
            ("TOADSTOOL_ENCRYPTION_ALGORITHM", Some("AES-256-GCM")),
            ("TOADSTOOL_ENCRYPTION_KEY_LENGTH", Some("256")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert!(c.security.encryption.enabled);
            assert_eq!(c.security.encryption.algorithm, "AES-256-GCM");
            assert_eq!(c.security.encryption.key_length, 256);
        },
    );
}

#[test]
fn apply_env_overrides_logging() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_LOG_FORMAT", Some("json")),
            ("TOADSTOOL_LOG_COLORS", Some("true")),
            ("TOADSTOOL_LOG_MAX_SIZE", Some("50")),
            ("TOADSTOOL_LOG_MAX_FILES", Some("10")),
        ],
        || {
            let mut c = ToadStoolConfig::default();
            c.apply_env_overrides().unwrap();
            assert_eq!(c.logging.format, "json");
            assert!(c.logging.enable_colors);
            assert_eq!(c.logging.max_log_size, 50);
            assert_eq!(c.logging.max_log_files, 10);
        },
    );
}

#[test]
fn apply_env_overrides_invalid_worker_threads_returns_error() {
    temp_env::with_var("TOADSTOOL_WORKER_THREADS", Some("xyz"), || {
        let mut c = ToadStoolConfig::default();
        let r = c.apply_env_overrides();
        assert!(r.is_err());
    });
}

#[test]
fn apply_env_overrides_invalid_execution_timeout_returns_error() {
    temp_env::with_var("TOADSTOOL_EXECUTION_TIMEOUT", Some("abc"), || {
        let mut c = ToadStoolConfig::default();
        let r = c.apply_env_overrides();
        assert!(r.is_err());
    });
}

#[test]
fn apply_env_overrides_verbose_false_sets_info() {
    temp_env::with_var("TOADSTOOL_VERBOSE", Some("false"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert_eq!(c.logging.level, "info");
    });
}

#[test]
fn apply_env_overrides_invalid_max_memory_returns_error() {
    temp_env::with_var("TOADSTOOL_MAX_MEMORY", Some("not-a-number"), || {
        let mut c = ToadStoolConfig::default();
        let r = c.apply_env_overrides();
        assert!(r.is_err());
    });
}

#[test]
fn apply_env_overrides_invalid_max_concurrent_returns_error() {
    temp_env::with_var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", Some("xyz"), || {
        let mut c = ToadStoolConfig::default();
        let r = c.apply_env_overrides();
        assert!(r.is_err());
    });
}

#[test]
fn apply_env_overrides_request_timeout() {
    temp_env::with_var("TOADSTOOL_REQUEST_TIMEOUT", Some("60"), || {
        let mut c = ToadStoolConfig::default();
        c.apply_env_overrides().unwrap();
        assert_eq!(
            c.network.connection.request_timeout,
            std::time::Duration::from_secs(60)
        );
    });
}

#[test]
fn apply_env_overrides_invalid_request_timeout_returns_error() {
    temp_env::with_var("TOADSTOOL_REQUEST_TIMEOUT", Some("invalid"), || {
        let mut c = ToadStoolConfig::default();
        let r = c.apply_env_overrides();
        assert!(r.is_err());
    });
}
