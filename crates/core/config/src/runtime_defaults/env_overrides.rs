//! Environment variable configuration overrides
//!
//! Handles applying environment variable overrides to ToadStool configuration.
//!
//! # Network Environment Variables (primary; current values are fallback defaults)
//!
//! | Variable | Fallback | Description |
//! |----------|----------|-------------|
//! | `TOADSTOOL_BIND_ADDRESS` | `127.0.0.1` (from config) | Full socket address (host:port) |
//! | `TOADSTOOL_PORT` | port from config | Override port only |
//! | `TOADSTOOL_SONGBIRD_ENDPOINT` | (deprecated) | Legacy coordination endpoint |
//! | `TOADSTOOL_BEARDOG_ENDPOINT` | (deprecated) | Legacy PKI endpoint |
//! | `TOADSTOOL_NESTGATE_ENDPOINT` | (deprecated) | Legacy storage endpoint |
//! | `TOADSTOOL_SQUIRREL_ENDPOINT` | (deprecated) | Legacy AI endpoint |

use super::{ConfigError, ConfigResult};
use crate::{BackendCacheConfig, MetricsConfig, ToadStoolConfig};
use std::time::Duration;

impl ToadStoolConfig {
    /// Apply environment variable overrides to configuration
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Environment variables contain invalid values (e.g., non-numeric for numbers)
    /// - Port numbers are invalid
    /// - Resource limits are out of range
    pub fn apply_env_overrides(&mut self) -> ConfigResult<()> {
        // Application overrides
        if let Ok(env_name) = std::env::var("TOADSTOOL_ENV") {
            self.app.environment = env_name;
        }

        if let Ok(debug) = std::env::var("TOADSTOOL_DEBUG") {
            self.features.enable_debug = debug.to_lowercase() == "true";
        }

        if let Ok(verbose) = std::env::var("TOADSTOOL_VERBOSE") {
            self.logging.level = if verbose.to_lowercase() == "true" {
                "debug".to_string()
            } else {
                "info".to_string()
            };
        }

        // TOADSTOOL_BIND_ADDRESS: full "host:port" (e.g. 0.0.0.0:9000, 127.0.0.1:3000)
        // Fallback: existing config value (from defaults::network or env_config)
        if let Ok(bind_address) = std::env::var("TOADSTOOL_BIND_ADDRESS") {
            self.network.bind_address = bind_address
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid bind address: {e}")))?;
        }

        // TOADSTOOL_PORT: override port only; fallback: existing config port
        if let Ok(port) = std::env::var("TOADSTOOL_PORT") {
            let port: u16 = port
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid port: {e}")))?;
            self.network.bind_address.set_port(port);
        }

        // Legacy endpoint overrides (deprecated - use capability-based discovery)
        #[allow(deprecated)]
        {
            if let Ok(songbird_endpoint) = std::env::var("TOADSTOOL_SONGBIRD_ENDPOINT") {
                self.network.endpoints.songbird = songbird_endpoint;
            }

            if let Ok(beardog_endpoint) = std::env::var("TOADSTOOL_BEARDOG_ENDPOINT") {
                self.network.endpoints.beardog = beardog_endpoint;
            }

            if let Ok(nestgate_endpoint) = std::env::var("TOADSTOOL_NESTGATE_ENDPOINT") {
                self.network.endpoints.nestgate = nestgate_endpoint;
            }

            if let Ok(squirrel_endpoint) = std::env::var("TOADSTOOL_SQUIRREL_ENDPOINT") {
                self.network.endpoints.squirrel = squirrel_endpoint;
            }
        }

        if let Ok(max_cpu) = std::env::var("TOADSTOOL_MAX_CPU") {
            self.runtime.resource_limits.max_cpu_usage = max_cpu
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid max CPU: {e}")))?;
        }

        if let Ok(max_memory) = std::env::var("TOADSTOOL_MAX_MEMORY") {
            self.runtime.resource_limits.max_memory_usage = max_memory
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid max memory: {e}")))?;
        }

        if let Ok(log_level) = std::env::var("TOADSTOOL_LOG_LEVEL") {
            self.logging.level = log_level;
        }

        if let Ok(data_dir) = std::env::var("TOADSTOOL_DATA_DIR") {
            self.app.data_dir = data_dir;
        }

        if let Ok(cache_dir) = std::env::var("TOADSTOOL_CACHE_DIR") {
            self.app.cache_dir = cache_dir;
        }

        if let Ok(worker_threads) = std::env::var("TOADSTOOL_WORKER_THREADS") {
            self.app.worker_threads = worker_threads
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid worker threads: {e}")))?;
        }

        if let Ok(max_concurrent) = std::env::var("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS") {
            self.runtime.max_concurrent_executions = max_concurrent.parse().map_err(|e| {
                ConfigError::Invalid(format!("Invalid max concurrent executions: {e}"))
            })?;
        }

        if let Ok(timeout) = std::env::var("TOADSTOOL_EXECUTION_TIMEOUT") {
            let timeout_secs: u64 = timeout
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid execution timeout: {e}")))?;
            self.runtime.execution_timeout = Duration::from_secs(timeout_secs);
        }

        if let Ok(request_timeout) = std::env::var("TOADSTOOL_REQUEST_TIMEOUT") {
            let timeout_secs: u64 = request_timeout
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid request timeout: {e}")))?;
            self.network.connection.request_timeout = Duration::from_secs(timeout_secs);
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_METRICS") {
            self.metrics = if enabled.to_lowercase() == "true" {
                Some(MetricsConfig::default())
            } else {
                None
            };
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_CACHE") {
            self.cache = if enabled.to_lowercase() == "true" {
                Some(BackendCacheConfig::default())
            } else {
                None
            };
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_AUTH") {
            self.security.auth.enabled = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_SANDBOX") {
            self.security.sandbox.enabled = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_FEDERATION") {
            self.features.enable_federation = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_DISTRIBUTED") {
            self.features.enable_distributed = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_AUTO_CONFIG") {
            self.features.enable_auto_config = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_HOT_RELOAD") {
            self.features.enable_hot_reload = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_EXPERIMENTAL") {
            self.features.enable_experimental = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_BETA") {
            self.features.enable_beta = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_PROFILING") {
            self.features.enable_profiling = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_OPENAPI") {
            self.features.enable_openapi = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_GRPC") {
            #[allow(deprecated)]
            {
                self.features.enable_grpc = enabled.to_lowercase() == "true";
            }
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENABLE_GRAPHQL") {
            self.features.enable_graphql = enabled.to_lowercase() == "true";
        }

        if let Ok(container_runtime) = std::env::var("TOADSTOOL_CONTAINER_RUNTIME") {
            self.runtime.container.runtime = container_runtime;
        }

        if let Ok(registry) = std::env::var("TOADSTOOL_CONTAINER_REGISTRY") {
            self.runtime.container.default_registry = registry;
        }

        if let Ok(network_mode) = std::env::var("TOADSTOOL_CONTAINER_NETWORK_MODE") {
            self.runtime.container.network_mode = network_mode;
        }

        if let Ok(wasm_engine) = std::env::var("TOADSTOOL_WASM_ENGINE") {
            self.runtime.wasm.engine = wasm_engine;
        }

        if let Ok(max_memory) = std::env::var("TOADSTOOL_WASM_MAX_MEMORY") {
            self.runtime.wasm.max_memory = max_memory
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid WASM max memory: {e}")))?;
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_WASM_ENABLE_WASI") {
            self.runtime.wasm.enable_wasi = enabled.to_lowercase() == "true";
        }

        if let Ok(python_exe) = std::env::var("TOADSTOOL_PYTHON_EXECUTABLE") {
            self.runtime.python.executable = python_exe;
        }

        if let Ok(venv_path) = std::env::var("TOADSTOOL_PYTHON_VENV_PATH") {
            self.runtime.python.venv_path = Some(venv_path);
        }

        if let Ok(index_url) = std::env::var("TOADSTOOL_PYTHON_INDEX_URL") {
            self.runtime.python.index_url = index_url;
        }

        if let Ok(max_memory) = std::env::var("TOADSTOOL_PYTHON_MAX_MEMORY") {
            self.runtime.python.max_memory = max_memory
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid Python max memory: {e}")))?;
        }

        if let Ok(jwt_secret) = std::env::var("TOADSTOOL_JWT_SECRET") {
            self.security.auth.jwt_secret = Some(jwt_secret);
        }

        if let Ok(session_timeout) = std::env::var("TOADSTOOL_SESSION_TIMEOUT") {
            let timeout_secs: u64 = session_timeout
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid session timeout: {e}")))?;
            self.security.auth.session_timeout = Duration::from_secs(timeout_secs);
        }

        if let Ok(max_attempts) = std::env::var("TOADSTOOL_MAX_LOGIN_ATTEMPTS") {
            self.security.auth.max_login_attempts = max_attempts
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid max login attempts: {e}")))?;
        }

        if let Ok(lockout_duration) = std::env::var("TOADSTOOL_LOCKOUT_DURATION") {
            let duration_secs: u64 = lockout_duration
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid lockout duration: {e}")))?;
            self.security.auth.lockout_duration = Duration::from_secs(duration_secs);
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_ENCRYPTION_ENABLED") {
            self.security.encryption.enabled = enabled.to_lowercase() == "true";
        }

        if let Ok(algorithm) = std::env::var("TOADSTOOL_ENCRYPTION_ALGORITHM") {
            self.security.encryption.algorithm = algorithm;
        }

        if let Ok(key_length) = std::env::var("TOADSTOOL_ENCRYPTION_KEY_LENGTH") {
            self.security.encryption.key_length = key_length
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid encryption key length: {e}")))?;
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_AUDIT_ENABLED") {
            self.security.audit.enabled = enabled.to_lowercase() == "true";
        }

        if let Ok(log_file) = std::env::var("TOADSTOOL_AUDIT_LOG_FILE") {
            self.security.audit.log_file = log_file;
        }

        if let Ok(log_level) = std::env::var("TOADSTOOL_AUDIT_LOG_LEVEL") {
            self.security.audit.log_level = log_level;
        }

        if let Ok(sandbox_type) = std::env::var("TOADSTOOL_SANDBOX_TYPE") {
            self.security.sandbox.sandbox_type = sandbox_type;
        }

        if let Ok(allow_network) = std::env::var("TOADSTOOL_SANDBOX_ALLOW_NETWORK") {
            self.security.sandbox.allow_network = allow_network.to_lowercase() == "true";
        }

        if let Ok(allow_file_access) = std::env::var("TOADSTOOL_SANDBOX_ALLOW_FILE_ACCESS") {
            self.security.sandbox.allow_file_access = allow_file_access.to_lowercase() == "true";
        }

        if let Ok(log_to_file) = std::env::var("TOADSTOOL_LOG_TO_FILE") {
            self.logging.log_to_file = log_to_file.to_lowercase() == "true";
        }

        if let Ok(log_file) = std::env::var("TOADSTOOL_LOG_FILE") {
            self.logging.log_file = log_file;
        }

        if let Ok(log_format) = std::env::var("TOADSTOOL_LOG_FORMAT") {
            self.logging.format = log_format;
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_COLORS") {
            self.logging.enable_colors = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_TIMESTAMPS") {
            self.logging.enable_timestamps = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_THREAD_IDS") {
            self.logging.enable_thread_ids = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_MODULE_PATHS") {
            self.logging.enable_module_paths = enabled.to_lowercase() == "true";
        }

        if let Ok(enabled) = std::env::var("TOADSTOOL_LOG_ROTATION") {
            self.logging.log_rotation = enabled.to_lowercase() == "true";
        }

        if let Ok(max_size) = std::env::var("TOADSTOOL_LOG_MAX_SIZE") {
            self.logging.max_log_size = max_size
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid log max size: {e}")))?;
        }

        if let Ok(max_files) = std::env::var("TOADSTOOL_LOG_MAX_FILES") {
            self.logging.max_log_files = max_files
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid log max files: {e}")))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
                assert_eq!(c.network.endpoints.songbird, "http://sb:8000");
                assert_eq!(c.network.endpoints.beardog, "http://bd:8001");
                assert_eq!(c.network.endpoints.nestgate, "http://ng:8002");
                assert_eq!(c.network.endpoints.squirrel, "http://sq:8003");
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
}
