// SPDX-License-Identifier: AGPL-3.0-or-later
//! Coverage expansion tests for under-covered modules in toadstool-common
//!
//! Tests public functions and types from:
//! - `unix_jsonrpc_client`, `platform_paths`, error (types, context, constructors)
//! - auth, `capability_discovery`, `config_bases`, `modern_utils`

use std::error::Error as StdError;
use std::path::PathBuf;
use std::time::Duration;

use toadstool_common::auth::{AuthCredentials, AuthType, ServiceAuthConfig};
use toadstool_common::capability_discovery::{
    CapabilityDiscovery, DiscoveryConfig, DiscoveryError, DiscoveryMethod,
};
use toadstool_common::config_bases::{
    BackendEndpoint, ResourceLimit, TelemetryConfig, TimeoutConfig,
};
use toadstool_common::error::{
    ConfigError, ExecutionError, IntegrationError, NetworkError, ResourceError, SecurityError,
    SystemError, ToadStoolError, ToadStoolErrorExt,
};
use toadstool_common::error_codes::codes;
use toadstool_common::modern_utils::{
    UtilError, batch_process, clamp, in_range, lerp, maybe_clone_str, normalize,
    retry_with_backoff, safe_divide, safe_percentage, with_timeout,
};
use toadstool_common::platform_paths::{PathEnv, Platform, PlatformPaths};
use toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient;

// ============================================================================
// Error Types Tests
// ============================================================================

#[test]
fn test_toadstool_error_runtime_variant() {
    let err = ToadStoolError::Runtime("test runtime".to_string());
    assert!(err.to_string().contains("Runtime error"));
    assert!(err.to_string().contains("test runtime"));
}

#[test]
fn test_toadstool_error_not_found_variant() {
    let err = ToadStoolError::NotFound("resource-xyz".to_string());
    assert!(err.to_string().contains("Not found"));
    assert!(err.to_string().contains("resource-xyz"));
}

#[test]
fn test_execution_error_resource_exhaustion() {
    let err = ExecutionError::ResourceExhaustion {
        resource: "memory".to_string(),
    };
    assert!(err.to_string().contains("memory"));
}

#[test]
fn test_execution_error_unsupported_workload_type() {
    let err = ExecutionError::UnsupportedWorkloadType {
        workload_type: "cuda".to_string(),
    };
    assert!(err.to_string().contains("cuda"));
}

#[test]
fn test_config_error_missing_field() {
    let err = ConfigError::MissingField {
        field: "port".to_string(),
    };
    assert!(err.to_string().contains("port"));
}

#[test]
fn test_config_error_load_error() {
    let err = ConfigError::LoadError {
        config_source: "file.toml".to_string(),
        reason: "permission denied".to_string(),
    };
    assert!(err.to_string().contains("file.toml"));
    assert!(err.to_string().contains("permission denied"));
}

#[test]
fn test_resource_error_allocation_failure() {
    let err = ResourceError::allocation_failure("GPU", "OOM");
    assert!(err.to_string().contains("GPU"));
}

#[test]
fn test_resource_error_not_found() {
    let err = ResourceError::NotFound {
        resource: "workload".to_string(),
        id: "w-1".to_string(),
    };
    assert!(err.to_string().contains("workload"));
}

#[test]
fn test_integration_error_invalid_response() {
    let err = IntegrationError::InvalidResponse {
        service: "api".to_string(),
        reason: "malformed JSON".to_string(),
    };
    assert!(err.to_string().contains("api"));
}

#[test]
fn test_security_error_authentication_failed() {
    let err = SecurityError::AuthenticationFailed {
        reason: "invalid token".to_string(),
    };
    assert!(err.to_string().contains("invalid token"));
}

#[test]
fn test_network_error_protocol_error() {
    let err = NetworkError::ProtocolError {
        reason: "invalid frame".to_string(),
    };
    assert!(err.to_string().contains("invalid frame"));
}

#[test]
fn test_system_error_serialization() {
    let err = SystemError::Serialization {
        reason: "invalid UTF-8".to_string(),
    };
    assert!(err.to_string().contains("invalid UTF-8"));
}

// ============================================================================
// Error Context (Convenience Methods) Tests
// ============================================================================

#[test]
fn test_error_context_configuration() {
    let err = ToadStoolError::configuration("bad config");
    assert!(err.to_string().contains("Configuration error"));
}

#[test]
fn test_error_context_runtime() {
    let err = ToadStoolError::runtime("workload failed");
    assert!(err.to_string().contains("Runtime error"));
}

#[test]
fn test_error_context_not_found() {
    let err = ToadStoolError::not_found("item-123");
    assert!(err.to_string().contains("Not found"));
}

#[test]
fn test_error_context_with_code() {
    let err = ToadStoolError::runtime("test").with_code(codes::EXEC_RUNTIME_001);
    assert!(err.error_code().is_some());
    assert_eq!(err.error_code_str(), Some("EXEC-RUNTIME-001"));
}

// ============================================================================
// Error Constructors Tests
// ============================================================================

#[test]
fn test_execution_error_constructors() {
    let err = ExecutionError::runtime_failure("k8s", "wk-1", "image pull failed");
    assert!(err.to_string().contains("k8s"));
    let err = ExecutionError::workload_failure("wk-2", "oom");
    assert!(err.to_string().contains("oom"));
}

#[test]
fn test_config_error_constructors() {
    let err = ConfigError::not_found("/etc/config.toml");
    assert!(err.to_string().contains("/etc/config.toml"));
}

#[test]
fn test_resource_error_constructors() {
    let err = ResourceError::limit_exceeded("cpu", "8", "4");
    assert!(err.to_string().contains("cpu"));
}

#[test]
fn test_integration_error_constructors() {
    let err = IntegrationError::connection_failed("db", "timeout");
    assert!(err.to_string().contains("db"));
}

// ============================================================================
// Platform Paths Tests
// ============================================================================

#[test]
fn test_platform_paths_runtime_dir_with_xdg() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(paths.runtime_dir(), PathBuf::from("/run/user/1000"));
}

#[test]
fn test_platform_paths_toadstool_jsonrpc_socket() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.toadstool_jsonrpc_socket(),
        PathBuf::from("/run/user/1000/biomeos/toadstool.jsonrpc.sock")
    );
}

#[test]
fn test_platform_paths_toadstool_data_dir() {
    let env = PathEnv {
        xdg_data_home: Some("/home/user/.local/share".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.toadstool_data_dir(),
        PathBuf::from("/home/user/.local/share/toadstool")
    );
}

#[test]
fn test_platform_paths_toadstool_cache_dir() {
    let env = PathEnv {
        xdg_cache_home: Some("/home/user/.cache".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.toadstool_cache_dir(),
        PathBuf::from("/home/user/.cache/toadstool")
    );
}

#[test]
fn test_platform_paths_toadstool_log_dir() {
    let env = PathEnv {
        xdg_data_home: Some("/data".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.toadstool_log_dir(),
        PathBuf::from("/data/toadstool/logs")
    );
}

#[test]
fn test_platform_paths_sandbox_dir() {
    let env = PathEnv {
        xdg_data_home: Some("/data".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.sandbox_dir(),
        PathBuf::from("/data/toadstool/sandbox")
    );
}

#[test]
fn test_platform_paths_discovery_dir() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert!(
        paths
            .discovery_dir()
            .to_string_lossy()
            .contains("ecoPrimals")
    );
}

#[test]
fn test_platform_paths_jsonrpc_port_file() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert!(
        paths
            .jsonrpc_port_file()
            .to_string_lossy()
            .contains("toadstool-jsonrpc-port")
    );
}

#[test]
fn test_platform_paths_biomeos_runtime_dir() {
    let env = PathEnv {
        xdg_runtime_dir: Some("/run/user/1000".to_string()),
        ..Default::default()
    };
    let paths = PlatformPaths::new(&env);
    assert_eq!(
        paths.biomeos_runtime_dir(),
        PathBuf::from("/run/user/1000/biomeos")
    );
}

#[test]
fn test_platform_convenience_functions() {
    let _runtime = toadstool_common::platform_paths::runtime_dir();
    let _temp = toadstool_common::platform_paths::temp_dir();
    let _socket_dir = toadstool_common::platform_paths::toadstool_socket_dir();
    let _socket = toadstool_common::platform_paths::toadstool_socket();
    let _temp_dir = toadstool_common::platform_paths::toadstool_temp_dir();
    let _biomeos = toadstool_common::platform_paths::biomeos_runtime_dir();
}

#[test]
fn test_platform_enum_variants() {
    assert!(matches!(Platform::Linux, Platform::Linux));
    assert!(matches!(Platform::MacOS, Platform::MacOS));
    assert!(matches!(Platform::Windows, Platform::Windows));
    assert!(matches!(Platform::Android, Platform::Android));
    assert!(matches!(Platform::Wasm, Platform::Wasm));
    assert!(matches!(Platform::Unknown, Platform::Unknown));
}

// ============================================================================
// Auth Tests
// ============================================================================

#[test]
fn test_auth_type_custom() {
    let t = AuthType::Custom("MyAuth".to_string());
    assert!(format!("{t:?}").contains("Custom"));
}

#[test]
fn test_auth_credentials_serialization_roundtrip() {
    let creds = AuthCredentials::bearer("token-123");
    let json = serde_json::to_string(&creds).unwrap();
    let restored: AuthCredentials = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.token, creds.token);
}

#[test]
fn test_service_auth_config_serialization() {
    let config = ServiceAuthConfig::bearer("secret");
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("Bearer"));
}

// ============================================================================
// Capability Discovery Tests
// ============================================================================

#[test]
fn test_discovery_config_default_timeout() {
    let config = DiscoveryConfig::default();
    assert_eq!(config.timeout, Duration::from_secs(5));
}

#[test]
fn test_discovery_error_display() {
    let err = DiscoveryError::Timeout;
    assert_eq!(err.to_string(), "Discovery timeout");

    let err = DiscoveryError::NoServicesFound("cap".to_string());
    assert!(err.to_string().contains("cap"));
}

#[test]
fn test_capability_discovery_new() {
    let result = CapabilityDiscovery::new();
    assert!(result.is_ok());
}

#[test]
fn test_capability_discovery_with_config() {
    let config = DiscoveryConfig {
        timeout: Duration::from_millis(100),
        enable_localhost_fallback: true,
        methods: vec![DiscoveryMethod::Environment],
    };
    let result = CapabilityDiscovery::with_config(&config);
    assert!(result.is_ok());
}

#[test]
fn test_discovery_error_std_error() {
    let err = DiscoveryError::InvalidConfig("bad".to_string());
    let dyn_err: &dyn StdError = &err;
    assert!(dyn_err.source().is_none());
}

// ============================================================================
// Config Bases Tests
// ============================================================================

#[test]
fn test_timeout_config_all_fields() {
    let config = TimeoutConfig::default();
    assert_eq!(config.connection_timeout, Duration::from_secs(30));
    assert_eq!(config.read_timeout, Duration::from_secs(30));
    assert_eq!(config.write_timeout, Duration::from_secs(30));
}

#[test]
fn test_resource_limit_default() {
    let limit = ResourceLimit::default();
    assert!(limit.limit.is_none());
    assert!(limit.request.is_none());
}

#[test]
fn test_backend_endpoint_new() {
    let ep = BackendEndpoint::new("api", "localhost", 8080);
    assert_eq!(ep.name, "api");
    assert_eq!(ep.address, "localhost");
    assert_eq!(ep.port, 8080);
    assert!(ep.enabled);
}

#[test]
fn test_backend_endpoint_url() {
    let ep = BackendEndpoint::new("svc", "example.com", 443);
    assert_eq!(ep.url("https"), "https://example.com:443");
}

#[test]
fn test_telemetry_config_default() {
    let config = TelemetryConfig::default();
    assert_eq!(config.metrics_port, 9090);
}

// ============================================================================
// Modern Utils Tests
// ============================================================================

#[tokio::test]
async fn test_with_timeout_success() {
    let result = with_timeout(Duration::from_secs(1), async { 42 }).await;
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test]
async fn test_with_timeout_timeout_error() {
    let result = with_timeout(Duration::from_millis(10), std::future::pending::<()>()).await;
    assert!(matches!(result, Err(UtilError::Timeout(_))));
}

#[tokio::test]
async fn test_retry_with_backoff_success_on_first() {
    let result = retry_with_backoff(3, || async { Ok::<_, &str>(1) }).await;
    assert_eq!(result.unwrap(), 1);
}

#[test]
fn test_maybe_clone_str_boundary() {
    let r = maybe_clone_str("0123456789", 10);
    assert!(matches!(r, std::borrow::Cow::Borrowed(_)));

    let r = maybe_clone_str("01234567890", 10);
    assert!(matches!(r, std::borrow::Cow::Owned(_)));
}

#[test]
fn test_safe_divide_by_zero() {
    let result = safe_divide(10, 0);
    assert!(result.is_err());
}

#[test]
fn test_safe_percentage_by_zero() {
    let result = safe_percentage(50, 0);
    assert!(result.is_err());
}

#[test]
fn test_clamp_bounds() {
    assert_eq!(clamp(5, 0, 10), 5);
    assert_eq!(clamp(-1, 0, 10), 0);
    assert_eq!(clamp(11, 0, 10), 10);
}

#[test]
fn test_normalize_zero_range() {
    let result = normalize(5.0, 0.0, 0.0);
    assert!(result.is_err());
}

#[test]
fn test_lerp_values() {
    assert!((lerp(0.0, 10.0, 0.5) - 5.0).abs() < f64::EPSILON);
}

#[test]
fn test_in_range_boundaries() {
    assert!(in_range(&5, &0, &10));
    assert!(!in_range(&11, &0, &10));
}

#[tokio::test]
async fn test_batch_process_single_batch() {
    let items = vec![1, 2, 3];
    let results = batch_process(items, 10, |batch| async move {
        Ok::<_, std::io::Error>(batch.iter().sum::<i32>())
    })
    .await
    .unwrap();
    assert_eq!(results, vec![6]);
}

// ============================================================================
// Unix JSON-RPC Client Tests
// ============================================================================

#[test]
fn test_unix_jsonrpc_client_new() {
    let client = UnixJsonRpcClient::new("/tmp/nonexistent.sock");
    assert_eq!(
        client.socket_path(),
        std::path::Path::new("/tmp/nonexistent.sock")
    );
}

#[test]
fn test_unix_jsonrpc_client_is_available_nonexistent() {
    let client = UnixJsonRpcClient::new("/tmp/definitely-nonexistent-socket-12345.sock");
    assert!(!client.is_available());
}

#[test]
fn test_unix_jsonrpc_client_socket_path_accepts_string() {
    let client = UnixJsonRpcClient::new("/var/run/test.sock".to_string());
    assert_eq!(
        client.socket_path(),
        std::path::Path::new("/var/run/test.sock")
    );
}

#[test]
fn test_unix_jsonrpc_client_socket_path_accepts_pathbuf() {
    let path = PathBuf::from("/opt/sockets/app.sock");
    let client = UnixJsonRpcClient::new(path);
    assert_eq!(
        client.socket_path(),
        std::path::Path::new("/opt/sockets/app.sock")
    );
}

#[test]
fn test_unix_jsonrpc_client_clone() {
    let client1 = UnixJsonRpcClient::new("/tmp/a.sock");
    let client2 = client1.clone();
    assert_eq!(client1.socket_path(), client2.socket_path());
}
