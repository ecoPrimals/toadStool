//! Tests for the environment configuration subsystem.

use std::time::Duration;

use super::*;

#[test]
fn test_env_config_loader() {
    let loader = EnvConfigLoader::new();

    temp_env::with_vars(
        [
            ("TOADSTOOL_TEST_STRING", Some("test_value")),
            ("TOADSTOOL_TEST_BOOL", Some("true")),
            ("TOADSTOOL_TEST_NUMBER", Some("42")),
        ],
        || {
            assert_eq!(loader.get_string("TEST_STRING", "default"), "test_value");
            assert!(loader.get_bool("TEST_BOOL", false));
            assert_eq!(loader.get_u32("TEST_NUMBER", 0), 42);
        },
    );
}

#[test]
#[allow(deprecated)]
fn test_network_env_config() {
    temp_env::with_vars(
        [
            ("SONGBIRD_PORT", Some("9080")),
            ("TOADSTOOL_BIND_ADDRESS", Some("0.0.0.0")),
        ],
        || {
            let config = NetworkEnvConfig::from_env();
            assert_eq!(config.songbird_port, 9080);
            assert_eq!(config.bind_address, "0.0.0.0");
            assert_eq!(config.songbird_endpoint(), "http://localhost:9080");
        },
    );
}

#[test]
fn test_environment_config() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENV", Some("development")),
            ("TOADSTOOL_DEBUG", Some("false")),
        ],
        || {
            let config = EnvironmentConfig::from_env();
            assert_eq!(config.environment, "development");
            assert!(!config.debug);
        },
    );
}

#[test]
fn test_env_config_loader_empty_prefix_get_string() {
    let loader = EnvConfigLoader::with_prefix("");
    let test_key = format!("EMPTY_PREFIX_STR_{}", std::process::id());
    temp_env::with_var(&test_key, Some("found"), || {
        let value = loader.get_string(&test_key, "default");
        assert_eq!(value, "found");
    });
}

#[test]
fn test_env_config_loader_empty_prefix_get_u16() {
    let loader = EnvConfigLoader::with_prefix("");
    let test_key = format!("EMPTY_PREFIX_U16_{}", std::process::id());
    temp_env::with_var(&test_key, Some("9999"), || {
        let value = loader.get_u16(&test_key, 0);
        assert_eq!(value, 9999);
    });
}

#[test]
fn test_env_config_loader_get_bool_yes_no_on_off() {
    let loader = EnvConfigLoader::new();

    temp_env::with_var("TOADSTOOL_BOOL_YES", Some("yes"), || {
        assert!(loader.get_bool("BOOL_YES", false));
    });
    temp_env::with_var("TOADSTOOL_BOOL_NO", Some("no"), || {
        assert!(!loader.get_bool("BOOL_NO", true));
    });
    temp_env::with_var("TOADSTOOL_BOOL_ON", Some("on"), || {
        assert!(loader.get_bool("BOOL_ON", false));
    });
    temp_env::with_var("TOADSTOOL_BOOL_OFF", Some("off"), || {
        assert!(!loader.get_bool("BOOL_OFF", true));
    });
}

#[test]
fn test_env_config_loader_default_impl() {
    let loader = EnvConfigLoader::default();
    assert_eq!(
        loader.get_string("NONEXISTENT_DEFAULT", "fallback"),
        "fallback"
    );
}

#[test]
#[allow(deprecated)]
fn test_network_env_config_toadstool_endpoint() {
    let config = NetworkEnvConfig::from_env();
    let ep = config.toadstool_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains(':'));
    assert!(ep.contains(&config.external_hostname));
    assert!(ep.contains(&config.toadstool_port.to_string()));
}

#[test]
fn test_network_env_config_federation_endpoint() {
    let config = NetworkEnvConfig::from_env();
    let ep = config.federation_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains(&config.federation_port.to_string()));
}

#[test]
fn test_network_env_config_metrics_endpoint() {
    let config = NetworkEnvConfig::from_env();
    let ep = config.metrics_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains(&config.metrics_port.to_string()));
}

#[test]
fn test_network_env_config_health_endpoint() {
    let config = NetworkEnvConfig::from_env();
    let ep = config.health_endpoint();
    assert!(ep.starts_with("http://"));
    assert!(ep.contains(&config.health_port.to_string()));
}

#[test]
fn test_network_env_config_serialization_roundtrip() {
    let config = NetworkEnvConfig::from_env();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: NetworkEnvConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.toadstool_port, parsed.toadstool_port);
    assert_eq!(config.bind_address, parsed.bind_address);
    assert_eq!(config.external_hostname, parsed.external_hostname);
}

#[test]
fn test_resource_env_config_from_env() {
    let config = ResourceEnvConfig::from_env();
    assert!(config.max_cpu_percent > 0.0);
    assert!(config.max_memory_bytes > 0);
    assert!(config.max_storage_bytes > 0);
    assert!(config.worker_threads > 0);
    assert!(config.queue_size > 0);
    assert!(config.batch_size > 0);
}

#[test]
fn test_resource_env_config_serialization_roundtrip() {
    let config = ResourceEnvConfig::from_env();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: ResourceEnvConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.max_cpu_percent, parsed.max_cpu_percent);
    assert_eq!(config.max_memory_bytes, parsed.max_memory_bytes);
}

#[test]
fn test_monitoring_env_config_from_env() {
    let config = MonitoringEnvConfig::from_env();
    assert!(!config.log_level.is_empty());
    assert!(config.metrics_interval_secs > 0);
    assert!(config.metrics_retention_days > 0);
    assert!(config.health_check_interval_secs > 0);
}

#[test]
fn test_monitoring_env_config_serialization_roundtrip() {
    let config = MonitoringEnvConfig::from_env();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: MonitoringEnvConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.log_level, parsed.log_level);
    assert_eq!(config.metrics_enabled, parsed.metrics_enabled);
}

#[test]
fn test_security_env_config_from_env() {
    let config = SecurityEnvConfig::from_env();
    assert!(!config.isolation_level.is_empty());
    assert!(config.rate_limit_rps > 0);
    assert!(config.rate_limit_burst > 0);
}

#[test]
fn test_security_env_config_cors_comma_separated() {
    temp_env::with_var(
        "TOADSTOOL_CORS_ALLOWED_ORIGINS",
        Some("https://a.com, https://b.com , https://c.com"),
        || {
            let config = SecurityEnvConfig::from_env();
            assert_eq!(config.cors_allowed_origins.len(), 3);
            assert!(config
                .cors_allowed_origins
                .contains(&"https://a.com".to_string()));
        },
    );
}

#[test]
fn test_security_env_config_serialization_roundtrip() {
    let config = SecurityEnvConfig::from_env();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: SecurityEnvConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.auth_enabled, parsed.auth_enabled);
    assert_eq!(config.cors_allowed_origins, parsed.cors_allowed_origins);
}

#[test]
fn test_environment_config_apply_to_config() {
    use crate::ToadStoolConfig;

    temp_env::with_vars(
        [
            ("TOADSTOOL_ENV", Some("staging")),
            ("TOADSTOOL_DEBUG", Some("false")),
        ],
        || {
            let mut config = ToadStoolConfig::default();
            let env_config = EnvironmentConfig::from_env();

            env_config.apply_to_config(&mut config);

            assert_eq!(config.app.environment, env_config.environment);
            assert_eq!(
                config.app.data_dir,
                env_config.data_dir.to_string_lossy().to_string()
            );
            assert_eq!(
                config.security.auth.enabled,
                env_config.security.auth_enabled
            );
        },
    );
}

#[test]
fn test_environment_config_serialization_roundtrip() {
    let config = EnvironmentConfig::from_env();
    let json = serde_json::to_string(&config).unwrap();
    let parsed: EnvironmentConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(config.environment, parsed.environment);
    assert_eq!(config.debug, parsed.debug);
    assert_eq!(config.verbose, parsed.verbose);
}

#[test]
fn test_get_env_with_prefix() {
    let suffix = std::process::id();
    let key = format!("TEST_PREFIX_HELPER_{suffix}");
    temp_env::with_var(&key, Some("helper_value"), || {
        let value = get_env_with_prefix("TEST", &format!("PREFIX_HELPER_{suffix}"), "default");
        assert_eq!(value, "helper_value");
    });
}

#[test]
fn test_get_env_bool_true() {
    let key = format!("TEST_BOOL_TRUE_{}", std::process::id());
    temp_env::with_var(&key, Some("true"), || {
        assert!(get_env_bool(&key, false));
    });
}

#[test]
fn test_get_env_duration() {
    let key = format!("TEST_DURATION_{}", std::process::id());
    temp_env::with_var(&key, Some("120"), || {
        let value = get_env_duration(&key, Duration::from_secs(30));
        assert_eq!(value, Duration::from_secs(120));
    });
}

#[test]
fn test_load_global_env_config() {
    let config = load_global_env_config();
    assert!(!config.environment.is_empty());
    assert!(!config.data_dir.as_os_str().is_empty());
}

#[test]
fn test_apply_env_config() {
    use crate::ToadStoolConfig;

    let mut config = ToadStoolConfig::default();
    apply_env_config(&mut config);
    assert!(!config.app.environment.is_empty());
}
