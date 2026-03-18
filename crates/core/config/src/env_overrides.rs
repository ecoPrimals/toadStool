// SPDX-License-Identifier: AGPL-3.0-or-later
//! Environment variable configuration overrides
//!
//! Handles applying environment variable overrides to ToadStool configuration

use std::time::Duration;
use crate::{BackendCacheConfig, ConfigError, ConfigResult, MetricsConfig, ToadStoolConfig};

impl ToadStoolConfig {
    /// Apply environment variable overrides
    ///
    /// # Errors
    ///
    /// Returns `ConfigError` if environment variable parsing fails
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

        if let Ok(bind_address) = std::env::var("TOADSTOOL_BIND_ADDRESS") {
            self.network.bind_address = bind_address
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid bind address: {e}")))?;
        }

        if let Ok(port) = std::env::var("TOADSTOOL_PORT") {
            let port: u16 = port
                .parse()
                .map_err(|e| ConfigError::Invalid(format!("Invalid port: {e}")))?;
            self.network.bind_address.set_port(port);
        }

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

