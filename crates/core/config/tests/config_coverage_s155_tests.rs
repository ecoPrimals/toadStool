// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive coverage tests for under-covered config modules
//!
//! Targets: `runtime_defaults/validation`, `runtime_defaults`, `discovery_integration`,
//! builder/profiler, `config_utils`, `network_config`, ports, validation, network, `env_overrides`

#![allow(deprecated)] // Testing legacy endpoint/port APIs for backwards compatibility

use std::sync::Mutex;
use std::time::Duration;
use tempfile::NamedTempFile;
use toadstool_common::primal_identity::{Capability, CoordinationCapability};
use toadstool_config::builder::{OutputFormat, ProfilerConfig, ProfilerConfigBuilder};
use toadstool_config::config_utils::ConfigUtils;
use toadstool_config::discovery_integration::{create_discovery, discover_or_fallback};
use toadstool_config::network_config::{BindMode, EndpointBuilder, NetworkConfig};
use toadstool_config::ports::{
    PortRegistry, capability_fallback, get_capability_port, get_port_with_env, get_toadstool_port,
    resolve_capability_port, resolve_port, test, toadstool,
};
use toadstool_config::{ConfigError, ToadStoolConfig};

static ENV_LOCK: Mutex<()> = Mutex::new(());

// ============================================================================
// 1. Config Validation
// ============================================================================

#[test]
fn test_validate_runtime_config_valid_default() {
    let config = ToadStoolConfig::default();
    assert!(config.validate_runtime_config().is_ok());
}

#[test]
fn test_validate_runtime_config_empty_app_name_fails() {
    let mut config = ToadStoolConfig::default();
    config.app.name = String::new();
    let err = config.validate_runtime_config().unwrap_err();
    assert!(err.to_string().contains("Application name"));
}

#[test]
fn test_validate_runtime_config_zero_worker_threads_fails() {
    let mut config = ToadStoolConfig::default();
    config.app.worker_threads = 0;
    let err = config.validate_runtime_config().unwrap_err();
    assert!(err.to_string().contains("Worker threads"));
}

#[test]
fn test_validate_runtime_config_invalid_port_range_fails() {
    let mut config = ToadStoolConfig::default();
    config.runtime.container.port_range = (9000, 8000);
    let err = config.validate_runtime_config().unwrap_err();
    assert!(err.to_string().contains("port range"));
}

#[test]
fn test_validate_runtime_config_cpu_usage_out_of_range_fails() {
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_cpu_usage = 150.0;
    let err = config.validate_runtime_config().unwrap_err();
    assert!(err.to_string().contains("Max CPU usage"));
}

#[test]
fn test_validate_runtime_config_zero_execution_timeout_fails() {
    let mut config = ToadStoolConfig::default();
    config.runtime.execution_timeout = Duration::ZERO;
    let err = config.validate_runtime_config().unwrap_err();
    assert!(err.to_string().contains("Execution timeout"));
}

#[test]
fn test_validate_runtime_config_optional_sections_none_passes() {
    let config = ToadStoolConfig {
        cache: None,
        metrics: None,
        database: None,
        ..ToadStoolConfig::default()
    };
    assert!(config.validate_runtime_config().is_ok());
}

// ============================================================================
// 2. Runtime Defaults and Overrides
// ============================================================================

#[test]
fn test_development_config() {
    let config = ToadStoolConfig::development();
    assert_eq!(config.app.environment, "development");
    assert_eq!(config.logging.level, "debug");
    assert!(config.features.enable_debug);
}

#[test]
fn test_production_config() {
    let config = ToadStoolConfig::production();
    assert_eq!(config.app.environment, "production");
    assert_eq!(config.logging.level, "info");
    assert!(!config.features.enable_debug);
}

#[test]
fn test_testing_config() {
    let config = ToadStoolConfig::testing();
    assert_eq!(config.app.environment, "test");
}

#[test]
fn test_for_environment_development() {
    let config = ToadStoolConfig::default().for_environment("development");
    assert_eq!(config.app.environment, "development");
}

#[test]
fn test_for_environment_production() {
    let config = ToadStoolConfig::default().for_environment("production");
    assert_eq!(config.app.environment, "production");
}

#[test]
fn test_for_environment_custom() {
    let config = ToadStoolConfig::default().for_environment("staging");
    assert_eq!(config.app.environment, "staging");
}

#[test]
fn test_for_current_environment_with_temp_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_ENVIRONMENT",
            "TOADSTOOL_ENV",
            "ENVIRONMENT",
            "ENV",
        ],
        || {
            temp_env::with_var("TOADSTOOL_ENVIRONMENT", Some("production"), || {
                let config = ToadStoolConfig::for_current_environment();
                assert_eq!(config.app.environment, "production");
            });
        },
    );
}

#[test]
fn test_load_with_overrides_success() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_ENVIRONMENT",
            "TOADSTOOL_ENV",
            "ENVIRONMENT",
            "ENV",
        ],
        || {
            let config = ToadStoolConfig::development();
            let temp_file = NamedTempFile::new().unwrap();
            config.save_to_file(temp_file.path()).unwrap();

            let result = ToadStoolConfig::load_with_overrides(temp_file.path());
            assert!(result.is_ok());
        },
    );
}

#[test]
fn test_load_from_env_only() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_var("TOADSTOOL_ENV", Some("test"), || {
        let result = ToadStoolConfig::load_from_env_only();
        assert!(result.is_ok());
    });
}

#[test]
fn test_save_to_file_roundtrip() {
    let config = ToadStoolConfig::testing();
    let temp_file = NamedTempFile::new().unwrap();
    config.save_to_file(temp_file.path()).unwrap();
    let content = std::fs::read_to_string(temp_file.path()).unwrap();
    assert!(content.contains("environment"));
}

#[test]
fn test_to_json_serialization() {
    let config = ToadStoolConfig::default();
    let json = config.to_json().unwrap();
    assert!(json.contains("\"app\""));
    assert!(json.contains("\"network\""));
}

#[test]
fn test_config_error_variants() {
    let invalid = ConfigError::Invalid("bad".into());
    assert!(invalid.to_string().contains("bad"));

    let missing = ConfigError::MissingField("name".into());
    assert!(missing.to_string().contains("name"));
}

// ============================================================================
// 3. Port Allocation and Validation
// ============================================================================

#[test]
fn test_ports_resolve_port() {
    assert_eq!(resolve_port(Some("9999"), 8080), 9999);
    assert_eq!(resolve_port(None, 8080), 8080);
    assert_eq!(resolve_port(Some("invalid"), 8080), 8080);
}

#[test]
fn test_ports_toadstool_constants() {
    assert_eq!(toadstool::SERVER, 0);
    assert_eq!(toadstool::DAEMON_API, 0);
}

#[test]
fn test_ports_capability_fallback_constants() {
    assert_eq!(capability_fallback::COORDINATION, 8080);
    assert_eq!(capability_fallback::SECURITY, 8081);
    assert_eq!(capability_fallback::STORAGE, 8082);
    assert_eq!(capability_fallback::PLATFORM, 8083);
}

#[test]
fn test_ports_get_port_with_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_var("TOADSTOOL_SERVER_PORT", Some("9000"), || {
        let port = get_port_with_env(0, "TOADSTOOL_SERVER_PORT");
        assert_eq!(port, 9000);
    });
}

#[test]
fn test_ports_get_toadstool_port() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_var("TOADSTOOL_DAEMON_API_PORT", Some("9090"), || {
        let port = get_toadstool_port("DAEMON_API", toadstool::DAEMON_API);
        assert_eq!(port, 9090);
    });
}

#[test]
fn test_ports_get_capability_port() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_var("TOADSTOOL_SECURITY_PORT", Some("9001"), || {
        let port = get_capability_port("SECURITY", capability_fallback::SECURITY);
        assert_eq!(port, 9001);
    });
}

#[test]
fn test_ports_resolve_capability_port() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_var("TOADSTOOL_STORAGE_PORT", Some("8200"), || {
        let port = resolve_capability_port("STORAGE", 8082);
        assert_eq!(port, 8200);
    });
}

#[test]
fn test_ports_resolve_capability_port_capability_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_vars(
        [
            ("TOADSTOOL_STORAGE_PORT", None::<&str>),
            ("STORAGE_PORT", Some("8300")),
        ],
        || {
            let port = resolve_capability_port("STORAGE", 8082);
            assert_eq!(port, 8300);
        },
    );
}

#[test]
fn test_ports_port_registry_default() {
    let registry = PortRegistry::default();
    // Port 0 = OS-assigned; registry provides valid u16 ports
    let _ = (registry.server, registry.metrics);
}

#[test]
fn test_ports_test_unique_port() {
    let p1 = test::unique_port(1);
    let p2 = test::unique_port(2);
    assert!(p1 >= test::BASE);
    assert!(p2 >= test::BASE);
    assert_ne!(p1, p2);
}

// ============================================================================
// 4. Network Configuration Builders
// ============================================================================

#[test]
fn test_network_config_default() {
    let config = NetworkConfig::default();
    assert_eq!(config.bind_mode, BindMode::Localhost);
    assert!(config.enable_mdns);
}

#[test]
fn test_network_config_production() {
    let config = NetworkConfig::production();
    assert_eq!(config.bind_mode, BindMode::AllInterfaces);
}

#[test]
fn test_network_config_development() {
    let config = NetworkConfig::development();
    assert_eq!(config.bind_mode, BindMode::Localhost);
}

#[test]
fn test_network_config_test() {
    let config = NetworkConfig::test();
    assert_eq!(config.service_port, 0);
}

#[test]
fn test_network_config_socket_addrs() {
    let config = NetworkConfig::default();
    let _service = config.service_addr();
    let _api = config.api_addr();
    let _metrics = config.metrics_addr();
    let _health = config.health_addr();
}

#[test]
fn test_bind_mode_from_str() {
    use std::str::FromStr;
    assert_eq!(
        BindMode::from_str("localhost").unwrap(),
        BindMode::Localhost
    );
    assert_eq!(BindMode::from_str("all").unwrap(), BindMode::AllInterfaces);
    assert!(BindMode::from_str("invalid").is_err());
}

#[test]
fn test_endpoint_builder() {
    let config = NetworkConfig::default();
    let builder = EndpointBuilder::new(config);
    let _ = builder.service_url();
    let _ = builder.api_url();
    let _ = builder.metrics_url();
    let _ = builder.health_url();
}

// ============================================================================
// 5. Discovery Integration
// ============================================================================

#[tokio::test]
async fn test_create_discovery() {
    let result = create_discovery();
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_discover_or_fallback_uses_fallback_when_no_services() {
    let discovery = create_discovery().unwrap();
    let fallback = "http://localhost:50001";
    let result = discover_or_fallback(
        &discovery,
        &Capability::Coordination(CoordinationCapability::default()),
        fallback,
    )
    .await
    .unwrap();
    assert_eq!(result, fallback);
}

// ============================================================================
// 6. Environment Variable Overrides
// ============================================================================

#[test]
fn test_env_override_bind_address() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_var("TOADSTOOL_BIND_ADDRESS", Some("0.0.0.0:8080"), || {
        let mut config = ToadStoolConfig::default();
        config.apply_env_overrides().unwrap();
        assert_eq!(config.network.bind_address.to_string(), "0.0.0.0:8080");
    });
}

#[test]
fn test_env_override_invalid_bind_address_fails() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_var("TOADSTOOL_BIND_ADDRESS", Some("invalid"), || {
        let mut config = ToadStoolConfig::default();
        let result = config.apply_env_overrides();
        assert!(result.is_err());
    });
}

#[test]
fn test_env_override_with_temp_env() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENV", Some("test")),
            ("TOADSTOOL_LOG_LEVEL", Some("trace")),
            ("TOADSTOOL_WORKER_THREADS", Some("12")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            config.apply_env_overrides().unwrap();
            assert_eq!(config.app.environment, "test");
            assert_eq!(config.logging.level, "trace");
            assert_eq!(config.app.worker_threads, 12);
        },
    );
}

#[test]
fn test_env_override_enable_metrics() {
    let _guard = ENV_LOCK.lock().unwrap();
    temp_env::with_var("TOADSTOOL_ENABLE_METRICS", Some("true"), || {
        let mut config = ToadStoolConfig {
            metrics: None,
            ..Default::default()
        };
        config.apply_env_overrides().unwrap();
        assert!(config.metrics.is_some());
    });
}

// ============================================================================
// 7. ConfigUtils (config_utils module)
// ============================================================================

#[test]
fn test_config_utils_get_toadstool_port() {
    let port = ConfigUtils::get_toadstool_port();
    // Port 0 = OS-assigned; or explicit from env
    let _ = port;
}

#[test]
fn test_config_utils_get_bind_address() {
    let addr = ConfigUtils::get_bind_address();
    assert!(!addr.is_empty());
}

#[test]
fn test_config_utils_get_environment() {
    let env = ConfigUtils::get_environment();
    assert!(!env.is_empty());
}

#[test]
fn test_config_utils_get_worker_threads() {
    let threads = ConfigUtils::get_worker_threads();
    assert!(threads > 0);
}

#[test]
fn test_config_utils_get_service_ports() {
    let ports = ConfigUtils::get_service_ports();
    assert!(!ports.is_empty());
}

#[test]
fn test_config_utils_get_container_port_range() {
    let (start, end) = ConfigUtils::get_container_port_range();
    assert!(start <= end);
}

// ============================================================================
// 8. Profiler Builder (builder/profiler)
// ============================================================================

#[test]
fn test_profiler_config_builder() {
    let config = ProfilerConfigBuilder::new()
        .warmup_iterations(5)
        .benchmark_iterations(50)
        .timeout_ms(5000)
        .build()
        .expect("valid");
    assert_eq!(config.warmup_iterations, 5);
    assert_eq!(config.benchmark_iterations, 50);
    assert_eq!(config.timeout_ms, Some(5000));
}

#[test]
fn test_profiler_config_presets() {
    let quick = ProfilerConfig::quick();
    assert_eq!(quick.warmup_iterations, 5);

    let thorough = ProfilerConfig::thorough();
    assert_eq!(thorough.benchmark_iterations, 500);

    let production = ProfilerConfig::production();
    assert!(production.parallel);
}

#[test]
fn test_profiler_config_validation_fails_on_zero_warmup() {
    let err = ProfilerConfigBuilder::new()
        .warmup_iterations(0)
        .build()
        .expect_err("should fail");
    assert!(err.to_string().contains("warmup"));
}

#[test]
fn test_profiler_output_format_variants() {
    let config = ProfilerConfigBuilder::new()
        .output_format(OutputFormat::Json)
        .build()
        .unwrap();
    assert!(matches!(config.output_format, OutputFormat::Json));
}
