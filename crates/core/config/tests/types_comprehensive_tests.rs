// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for backward compatibility with deprecated endpoint configuration
//! These tests validate that legacy hardcoded endpoints still work
#![allow(deprecated)]

//! Comprehensive tests for config types module
//! Target: config/src/types.rs (58% → 80%+ coverage)
//!
//! Focus: Test all Default implementations, serialization, and validation

use std::collections::HashMap;
use std::time::Duration;
use toadstool_config::types::*;

// ============================================================================
// ToadStoolConfig Tests
// ============================================================================

#[test]
fn test_toadstool_config_default() {
    let config = ToadStoolConfig::default();
    assert!(!config.app.name.is_empty());
    assert!(!config.app.version.is_empty());
    assert!(!config.network.endpoints.songbird.is_empty());
    assert!(config.features.enable_distributed);
}

#[test]
fn test_toadstool_config_serialization() {
    let config = ToadStoolConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: ToadStoolConfig = serde_json::from_str(&serialized).unwrap();

    assert_eq!(config.app.name, deserialized.app.name);
    assert_eq!(config.logging.level, deserialized.logging.level);
}

#[test]
fn test_toadstool_config_clone() {
    let config1 = ToadStoolConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.app.name, config2.app.name);
    assert_eq!(config1.app.environment, config2.app.environment);
}

// ============================================================================
// ApplicationConfig Tests
// ============================================================================

#[test]
fn test_application_config_default() {
    let config = ApplicationConfig::default();

    assert!(!config.name.is_empty());
    assert!(!config.version.is_empty());
    assert!(!config.data_dir.is_empty());
    assert!(!config.cache_dir.is_empty());
    assert!(!config.logs_dir.is_empty());
    assert!(!config.temp_dir.is_empty());
    assert!(config.worker_threads > 0);
    assert!(config.queue_size > 0);
    assert!(config.batch_size > 0);
    assert!(config.shutdown_timeout.as_secs() > 0);
}

#[test]
fn test_application_config_custom_values() {
    let config = ApplicationConfig {
        name: "custom-app".to_string(),
        version: "2.0.0".to_string(),
        environment: "staging".to_string(),
        data_dir: "/custom/data".to_string(),
        cache_dir: "/custom/cache".to_string(),
        logs_dir: "/custom/logs".to_string(),
        temp_dir: "/custom/tmp".to_string(),
        worker_threads: 16,
        queue_size: 2000,
        batch_size: 200,
        shutdown_timeout: Duration::from_secs(60),
    };

    assert_eq!(config.name, "custom-app");
    assert_eq!(config.worker_threads, 16);
    assert_eq!(config.shutdown_timeout.as_secs(), 60);
}

// ============================================================================
// NetworkConfig Tests
// ============================================================================

#[test]
fn test_network_config_default() {
    let config = NetworkConfig::default();

    assert!(!config.endpoints.songbird.is_empty());
    assert!(!config.endpoints.beardog.is_empty());
    assert!(!config.endpoints.nestgate.is_empty());
    assert!(config.connection.request_timeout.as_secs() > 0);
}

#[test]
fn test_network_config_tls_optional() {
    let config = NetworkConfig::default();
    assert!(config.tls.is_none(), "TLS should be optional by default");
}

// ============================================================================
// EndpointConfig Tests
// ============================================================================

#[test]
fn test_endpoint_config_all_services() {
    let config = EndpointConfig::default();

    assert!(config.songbird.starts_with("http"));
    assert!(config.beardog.starts_with("http"));
    assert!(config.nestgate.starts_with("http"));
    assert!(config.squirrel.starts_with("http"));
    assert!(config.federation.starts_with("http"));
    assert!(config.metrics.starts_with("http"));
    assert!(config.health.starts_with("http"));
}

// ============================================================================
// ConnectionConfig Tests
// ============================================================================

#[test]
fn test_connection_config_default() {
    let config = ConnectionConfig::default();

    assert!(config.request_timeout.as_secs() > 0);
    assert!(config.connection_timeout.as_secs() > 0);
    assert!(config.max_retries > 0);
    assert!(config.keepalive_interval.as_secs() > 0);
    assert!(config.max_connections_per_host > 0);
    assert!(config.pool_size > 0);
    assert!(config.enable_http2);
    assert!(config.enable_compression);
}

#[test]
fn test_connection_config_custom() {
    let config = ConnectionConfig {
        request_timeout: Duration::from_secs(60),
        connection_timeout: Duration::from_secs(15),
        max_retries: 5,
        keepalive_interval: Duration::from_secs(45),
        max_connections_per_host: 50,
        pool_size: 20,
        enable_http2: false,
        enable_compression: false,
    };

    assert_eq!(config.request_timeout.as_secs(), 60);
    assert_eq!(config.max_retries, 5);
    assert!(!config.enable_http2);
}

// ============================================================================
// TlsConfig Tests
// ============================================================================

#[test]
fn test_tls_config_structure() {
    let config = TlsConfig {
        cert_file: "/path/to/cert.pem".to_string(),
        key_file: "/path/to/key.pem".to_string(),
        ca_file: Some("/path/to/ca.pem".to_string()),
        verify_certs: true,
        tls_version: "1.3".to_string(),
        cipher_suites: vec!["TLS_AES_256_GCM_SHA384".to_string()],
    };

    assert_eq!(config.cert_file, "/path/to/cert.pem");
    assert!(config.verify_certs);
    assert_eq!(config.cipher_suites.len(), 1);
}

// ============================================================================
// RuntimeConfig Tests
// ============================================================================

#[test]
fn test_runtime_config_default() {
    let config = RuntimeConfig::default();

    assert!(config.execution_timeout.as_secs() > 0);
    assert!(config.max_concurrent_executions > 0);
    assert!(config.resource_limits.max_cpu_usage > 0.0);
    assert!(config.resource_limits.max_memory_usage > 0.0);
    assert_eq!(config.container.runtime, "docker");
    assert_eq!(config.wasm.engine, "wasmtime");
    assert_eq!(config.python.executable, "python3");
    assert!(config.gpu.is_none());
}

// ============================================================================
// ResourceLimits Tests
// ============================================================================

#[test]
fn test_resource_limits_default() {
    let limits = ResourceLimits::default();

    assert!(limits.max_cpu_usage > 0.0 && limits.max_cpu_usage <= 100.0);
    assert!(limits.max_memory_usage > 0.0 && limits.max_memory_usage <= 100.0);
    assert!(limits.max_disk_usage > 0.0 && limits.max_disk_usage <= 100.0);
    assert!(limits.max_network_bandwidth > 0);
    assert!(limits.max_open_files > 0);
    assert!(limits.max_processes > 0);
}

#[test]
#[allow(clippy::float_cmp)] // comparing against exact literal initialization
fn test_resource_limits_custom() {
    let limits = ResourceLimits {
        max_cpu_usage: 80.0,
        max_memory_usage: 75.0,
        max_disk_usage: 90.0,
        max_network_bandwidth: 1024 * 1024 * 100, // 100 MB/s
        max_open_files: 2048,
        max_processes: 200,
    };

    assert_eq!(limits.max_cpu_usage, 80.0);
    assert_eq!(limits.max_open_files, 2048);
}

// ============================================================================
// ContainerConfig Tests
// ============================================================================

#[test]
fn test_container_config_default() {
    let config = ContainerConfig::default();

    assert_eq!(config.runtime, "docker");
    assert_eq!(config.default_registry, "docker.io");
    assert!(config.port_range.0 < config.port_range.1);
    assert_eq!(config.network_mode, "bridge");
    assert!(config
        .security_opts
        .contains(&"no-new-privileges".to_string()));
    assert!(config.volume_mounts.is_empty());
    assert!(config.environment.is_empty());
}

#[test]
fn test_container_config_custom() {
    let mut env = HashMap::new();
    env.insert("VAR1".to_string(), "value1".to_string());

    let config = ContainerConfig {
        runtime: "podman".to_string(),
        default_registry: "quay.io".to_string(),
        port_range: (8000, 9000),
        network_mode: "host".to_string(),
        security_opts: vec!["apparmor=unconfined".to_string()],
        volume_mounts: vec!["/data:/data".to_string()],
        environment: env,
    };

    assert_eq!(config.runtime, "podman");
    assert_eq!(config.port_range, (8000, 9000));
    assert_eq!(config.environment.len(), 1);
}

// ============================================================================
// WasmConfig Tests
// ============================================================================

#[test]
fn test_wasm_config_default() {
    let config = WasmConfig::default();

    assert_eq!(config.engine, "wasmtime");
    assert_eq!(config.max_memory, 64 * 1024 * 1024);
    assert_eq!(config.max_execution_time, 300);
    assert!(config.enable_wasi);
    assert!(config.wasi_allowed_dirs.contains(&"/tmp".to_string()));
    assert!(config.wasi_env.is_empty());
}

#[test]
fn test_wasm_config_custom() {
    let mut env = HashMap::new();
    env.insert("WASI_VAR".to_string(), "value".to_string());

    let config = WasmConfig {
        engine: "wasmer".to_string(),
        max_memory: 128 * 1024 * 1024,
        max_execution_time: 600,
        enable_wasi: false,
        wasi_allowed_dirs: vec!["/data".to_string()],
        wasi_env: env,
    };

    assert_eq!(config.engine, "wasmer");
    assert!(!config.enable_wasi);
    assert_eq!(config.wasi_env.len(), 1);
}

// ============================================================================
// PythonConfig Tests
// ============================================================================

#[test]
fn test_python_config_default() {
    let config = PythonConfig::default();

    assert_eq!(config.executable, "python3");
    assert!(config.venv_path.is_none());
    assert!(
        config.index_url.is_empty(),
        "index_url discovered via config (sovereignty)"
    );
    assert_eq!(config.max_memory, 128 * 1024 * 1024);
    assert_eq!(config.max_execution_time, 300);
    assert!(config.allowed_modules.contains(&"numpy".to_string()));
    assert!(config.restricted_modules.contains(&"os".to_string()));
}

#[test]
fn test_python_config_with_venv() {
    let config = PythonConfig {
        executable: "/opt/python3.11/bin/python3".to_string(),
        venv_path: Some("/opt/venv".to_string()),
        index_url: "https://custom-pypi.org/simple".to_string(),
        max_memory: 256 * 1024 * 1024,
        max_execution_time: 600,
        allowed_modules: vec!["pandas".to_string(), "scipy".to_string()],
        restricted_modules: vec!["subprocess".to_string()],
    };

    assert!(config.venv_path.is_some());
    assert_eq!(config.allowed_modules.len(), 2);
}

// ============================================================================
// GpuConfig Tests
// ============================================================================

#[test]
fn test_gpu_config_default() {
    let config = GpuConfig::default();

    assert_eq!(config.runtime, "cuda");
    assert_eq!(config.device_ids, vec![0]);
    assert_eq!(config.max_memory_per_device, 2 * 1024 * 1024 * 1024);
    assert_eq!(config.max_execution_time, 300);
    assert!(!config.enable_profiling);
}

#[test]
fn test_gpu_config_multi_device() {
    let config = GpuConfig {
        runtime: "opencl".to_string(),
        device_ids: vec![0, 1, 2],
        max_memory_per_device: 4 * 1024 * 1024 * 1024,
        max_execution_time: 600,
        enable_profiling: true,
    };

    assert_eq!(config.device_ids.len(), 3);
    assert!(config.enable_profiling);
}

// ============================================================================
// SecurityConfig Tests
// ============================================================================

#[test]
fn test_security_config_default() {
    let config = SecurityConfig::default();

    assert!(!config.auth.enabled);
    assert!(!config.authz.enabled);
    assert!(!config.encryption.enabled);
    assert!(!config.audit.enabled);
    assert!(config.sandbox.enabled);
}

// ============================================================================
// AuthConfig Tests
// ============================================================================

#[test]
fn test_auth_config_default() {
    let config = AuthConfig::default();

    assert!(!config.enabled);
    assert_eq!(config.provider, "local");
    assert!(config.jwt_secret.is_none());
    assert!(config.session_timeout.as_secs() > 0);
    assert_eq!(config.max_login_attempts, 5);
    assert_eq!(config.lockout_duration.as_secs(), 300);
}

#[test]
fn test_auth_config_enabled() {
    let config = AuthConfig {
        enabled: true,
        provider: "oauth2".to_string(),
        jwt_secret: Some("secret-key-here".to_string()),
        session_timeout: Duration::from_secs(7200),
        max_login_attempts: 3,
        lockout_duration: Duration::from_secs(600),
    };

    assert!(config.enabled);
    assert!(config.jwt_secret.is_some());
    assert_eq!(config.max_login_attempts, 3);
}

// ============================================================================
// AuthzConfig Tests
// ============================================================================

#[test]
fn test_authz_config_default() {
    let config = AuthzConfig::default();

    assert!(!config.enabled);
    assert_eq!(config.provider, "local");
    assert!(config.default_permissions.contains(&"read".to_string()));
    assert!(config.admin_permissions.contains(&"admin".to_string()));
}

// ============================================================================
// EncryptionConfig Tests
// ============================================================================

#[test]
fn test_encryption_config_default() {
    let config = EncryptionConfig::default();

    assert!(!config.enabled);
    assert_eq!(config.algorithm, "aes-256-gcm");
    assert_eq!(config.key_derivation, "pbkdf2");
    assert!(config.key_length > 0);
    assert!(!config.encrypt_at_rest);
    assert!(config.encrypt_in_transit);
}

#[test]
fn test_encryption_config_enabled() {
    let config = EncryptionConfig {
        enabled: true,
        algorithm: "chacha20-poly1305".to_string(),
        key_derivation: "scrypt".to_string(),
        key_length: 32,
        encrypt_at_rest: true,
        encrypt_in_transit: true,
    };

    assert!(config.enabled);
    assert!(config.encrypt_at_rest);
}

// ============================================================================
// AuditConfig Tests
// ============================================================================

#[test]
fn test_audit_config_default() {
    let config = AuditConfig::default();

    assert!(!config.enabled);
    assert_eq!(config.log_file, "audit.log");
    assert_eq!(config.log_level, "info");
    assert_eq!(config.log_format, "json");
    assert!(config.log_rotation);
    assert!(config.max_log_size > 0);
    assert!(config.max_log_files > 0);
}

// ============================================================================
// SandboxConfig Tests
// ============================================================================

#[test]
fn test_sandbox_config_default() {
    let config = SandboxConfig::default();

    assert!(config.enabled);
    assert_eq!(config.sandbox_type, "seccomp");
    assert!(config.allowed_syscalls.contains(&"read".to_string()));
    assert!(config.blocked_syscalls.contains(&"execve".to_string()));
    assert!(!config.allow_network);
    assert!(config.allow_file_access);
    assert!(config.allowed_dirs.contains(&"/tmp".to_string()));
    assert!(config.blocked_dirs.contains(&"/etc".to_string()));
}

#[test]
fn test_sandbox_config_permissive() {
    let config = SandboxConfig {
        enabled: false,
        sandbox_type: "none".to_string(),
        allowed_syscalls: vec![],
        blocked_syscalls: vec![],
        allow_network: true,
        allow_file_access: true,
        allowed_dirs: vec!["/".to_string()],
        blocked_dirs: vec![],
    };

    assert!(!config.enabled);
    assert!(config.allow_network);
}

// ============================================================================
// LoggingConfig Tests
// ============================================================================

#[test]
fn test_logging_config_default() {
    let config = LoggingConfig::default();

    assert!(!config.level.is_empty());
    assert_eq!(config.format, "pretty");
    assert!(!config.log_to_file);
    assert_eq!(config.log_file, "toadstool.log");
    assert!(config.log_rotation);
    assert!(config.max_log_size > 0);
    assert!(config.max_log_files > 0);
    assert!(config.enable_colors);
    assert!(config.enable_timestamps);
    assert!(!config.enable_thread_ids);
    assert!(!config.enable_module_paths);
}

#[test]
fn test_logging_config_production() {
    let config = LoggingConfig {
        level: "info".to_string(),
        format: "json".to_string(),
        log_to_file: true,
        log_file: "/var/log/toadstool.log".to_string(),
        log_rotation: true,
        max_log_size: 100 * 1024 * 1024,
        max_log_files: 10,
        enable_colors: false,
        enable_timestamps: true,
        enable_thread_ids: true,
        enable_module_paths: true,
    };

    assert_eq!(config.format, "json");
    assert!(config.log_to_file);
    assert!(config.enable_thread_ids);
}

// ============================================================================
// DatabaseConfig Tests
// ============================================================================

#[test]
fn test_database_config_structure() {
    let config = DatabaseConfig {
        url: "postgresql://localhost/toadstool".to_string(),
        database_type: "postgresql".to_string(),
        max_connections: 20,
        connection_timeout: Duration::from_secs(10),
        query_timeout: Duration::from_secs(30),
        enable_migrations: true,
        migration_dir: "migrations".to_string(),
    };

    assert_eq!(config.database_type, "postgresql");
    assert_eq!(config.max_connections, 20);
    assert!(config.enable_migrations);
}

// ============================================================================
// BackendCacheConfig Tests
// ============================================================================

#[test]
fn test_cache_config_default() {
    let config = BackendCacheConfig::default();

    assert_eq!(config.cache_type, "memory");
    assert!(config.url.is_none());
    assert!(config.max_size > 0);
    assert!(config.ttl.as_secs() > 0);
    assert!(!config.enable_compression);
    assert_eq!(config.compression_algorithm, "gzip");
}

#[test]
fn test_cache_config_redis() {
    let config = BackendCacheConfig {
        cache_type: "redis".to_string(),
        url: Some("redis://localhost:6379".to_string()),
        max_size: 1024 * 1024 * 1024,
        ttl: Duration::from_secs(7200),
        enable_compression: true,
        compression_algorithm: "lz4".to_string(),
    };

    assert_eq!(config.cache_type, "redis");
    assert!(config.url.is_some());
    assert!(config.enable_compression);
}

// ============================================================================
// MetricsConfig Tests
// ============================================================================

#[test]
fn test_metrics_config_default() {
    let config = MetricsConfig::default();

    assert!(config.enabled);
    assert!(config.endpoint.contains("/metrics"));
    assert_eq!(config.format, "prometheus");
    assert!(config.collection_interval.as_secs() > 0);
    assert!(config.retention_period.as_secs() > 0);
    assert!(config.enable_histograms);
    assert!(config.enable_counters);
    assert!(config.enable_gauges);
}

// ============================================================================
// FeatureFlags Tests
// ============================================================================

#[test]
fn test_feature_flags_default() {
    let flags = FeatureFlags::default();

    assert!(!flags.enable_experimental);
    assert!(!flags.enable_beta);
    assert!(flags.enable_distributed);
    assert!(flags.enable_federation);
    assert!(!flags.enable_graphql);
    assert!(!flags.enable_grpc);
    assert!(flags.enable_openapi);
    assert!(flags.enable_auto_config);
    assert!(flags.custom.is_empty());
}

#[test]
fn test_feature_flags_custom() {
    let mut custom = HashMap::new();
    custom.insert("custom_feature_1".to_string(), true);
    custom.insert("custom_feature_2".to_string(), false);

    let flags = FeatureFlags {
        enable_experimental: true,
        enable_beta: true,
        enable_debug: true,
        enable_profiling: true,
        enable_distributed: false,
        enable_federation: false,
        enable_graphql: true,
        enable_grpc: true,
        enable_openapi: false,
        enable_auto_config: false,
        enable_hot_reload: true,
        enable_live_reload: true,
        enable_watch_mode: true,
        custom,
    };

    assert!(flags.enable_experimental);
    assert_eq!(flags.custom.len(), 2);
}

// ============================================================================
// Configuration Methods Tests
// ============================================================================

#[test]
fn test_config_for_development() {
    let config = ToadStoolConfig::default().for_environment("development");

    assert_eq!(config.app.environment, "development");
    assert_eq!(config.logging.level, "debug");
    assert!(config.features.enable_debug);
    assert!(!config.security.auth.enabled);
}

#[test]
fn test_config_for_production() {
    let config = ToadStoolConfig::default().for_environment("production");

    assert_eq!(config.app.environment, "production");
    assert_eq!(config.logging.level, "info");
    assert!(!config.features.enable_debug);
    assert!(config.security.auth.enabled);
}

#[test]
fn test_config_for_test() {
    let config = ToadStoolConfig::default().for_environment("test");

    assert_eq!(config.app.environment, "test");
    assert!(config.app.data_dir.contains("test"));
    assert!(!config.security.auth.enabled);
}

#[test]
fn test_config_for_unknown_environment() {
    let config = ToadStoolConfig::default().for_environment("unknown");

    assert_eq!(config.app.environment, "unknown");
    // Should use defaults
}

#[test]
fn test_config_merge_overrides() {
    let mut overrides = HashMap::new();
    overrides.insert("key1".to_string(), serde_json::json!("value1"));
    overrides.insert("key2".to_string(), serde_json::json!(42));

    let config = ToadStoolConfig::default().merge(overrides.clone());

    assert_eq!(config.overrides.len(), 2);
    assert_eq!(config.get_override("key2", 0), 42);
}

#[test]
fn test_config_get_override_default() {
    let config = ToadStoolConfig::default();

    let value: String = config.get_override("nonexistent", "default".to_string());
    assert_eq!(value, "default");
}

#[test]
fn test_config_get_override_exists() {
    let mut overrides = HashMap::new();
    overrides.insert("test_key".to_string(), serde_json::json!("test_value"));

    let config = ToadStoolConfig::default().merge(overrides);
    let value: String = config.get_override("test_key", "default".to_string());

    assert_eq!(value, "test_value");
}
