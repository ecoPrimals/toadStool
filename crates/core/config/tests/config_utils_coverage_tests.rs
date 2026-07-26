// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration coverage for [`toadstool_config::config_utils::ConfigUtils`]: path helpers,
//! network getters, environment flags, default-backed accessors, env edge cases, serde
//! stability for env config structs, and [`EnvConfigLoader`]’s [`Default`] implementation.

use std::path::PathBuf;
use std::time::Duration;

use serde_json::json;
use toadstool_common::constants::primal_identity::PRIMAL_NAME;
use toadstool_config::config_utils::ConfigUtils;
use toadstool_config::defaults::network as defaults_network;
use toadstool_config::defaults::ports as defaults_ports;
use toadstool_config::defaults::storage as defaults_storage;
use toadstool_config::env_config::{
    EnvConfigLoader, EnvironmentConfig, MonitoringEnvConfig, NetworkEnvConfig, ResourceEnvConfig,
    SecurityEnvConfig,
};
use toadstool_config::network as network_defaults;
use toadstool_config::ports::{capability_fallback, resolve_capability_port};

#[test]
fn env_config_loader_default_matches_new() {
    let a = EnvConfigLoader::default();
    let b = EnvConfigLoader::new();
    temp_env::with_var_unset("TOADSTOOL_ENV", || {
        assert_eq!(
            a.get_string("ENV", "development"),
            b.get_string("ENV", "development")
        );
    });
}

#[test]
fn primal_ports_resolve_when_legacy_env_unset() {
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_COORDINATION_PORT",
            "COORDINATION_PORT",
            "SONGBIRD_PORT",
            "TOADSTOOL_SECURITY_PORT",
            "SECURITY_PORT",
            "BEARDOG_PORT",
            "TOADSTOOL_STORAGE_PORT",
            "STORAGE_PORT",
            "NESTGATE_PORT",
            "TOADSTOOL_PLATFORM_PORT",
            "PLATFORM_PORT",
            "SQUIRREL_PORT",
        ],
        || {
            assert_eq!(
                resolve_capability_port("COORDINATION", capability_fallback::COORDINATION),
                capability_fallback::COORDINATION
            );
            assert_eq!(
                resolve_capability_port("SECURITY", capability_fallback::SECURITY),
                capability_fallback::SECURITY
            );
            assert_eq!(
                resolve_capability_port("STORAGE", capability_fallback::STORAGE),
                capability_fallback::STORAGE
            );
            assert_eq!(
                resolve_capability_port("PLATFORM", capability_fallback::PLATFORM),
                capability_fallback::PLATFORM
            );
        },
    );
}

#[test]
fn network_ports_and_bind_strings_follow_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_PORT", Some("7777")),
            ("TOADSTOOL_FEDERATION_PORT", Some("7001")),
            ("TOADSTOOL_METRICS_PORT", Some("7002")),
            ("TOADSTOOL_HEALTH_PORT", Some("7003")),
            ("TOADSTOOL_EVENTS_PORT", Some("7004")),
            ("BIND_ADDRESS", Some("127.0.0.1")),
            ("TOADSTOOL_EXTERNAL_HOSTNAME", Some("svc.local")),
        ],
        || {
            assert_eq!(ConfigUtils::get_toadstool_port(), 7777);
            assert_eq!(ConfigUtils::get_federation_port(), 7001);
            assert_eq!(ConfigUtils::get_metrics_port(), 7002);
            assert_eq!(ConfigUtils::get_health_port(), 7003);
            assert_eq!(ConfigUtils::get_events_port(), 7004);
            assert_eq!(ConfigUtils::get_bind_address(), "127.0.0.1");
            assert_eq!(ConfigUtils::get_external_hostname(), "svc.local");
        },
    );
}

#[test]
fn network_timeouts_and_limits_follow_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_REQUEST_TIMEOUT_SECS", Some("99")),
            ("TOADSTOOL_CONNECTION_TIMEOUT_SECS", Some("8")),
            ("TOADSTOOL_MAX_RETRIES", Some("7")),
            ("TOADSTOOL_MAX_CONNECTIONS_PER_HOST", Some("42")),
            ("TOADSTOOL_KEEPALIVE_INTERVAL_SECS", Some("15")),
        ],
        || {
            assert_eq!(ConfigUtils::get_request_timeout(), Duration::from_secs(99));
            assert_eq!(
                ConfigUtils::get_connection_timeout(),
                Duration::from_secs(8)
            );
            assert_eq!(ConfigUtils::get_max_retries(), 7);
            assert_eq!(ConfigUtils::get_max_connections_per_host(), 42);
            assert_eq!(
                ConfigUtils::get_keepalive_interval(),
                Duration::from_secs(15)
            );
        },
    );
}

#[test]
fn invalid_numeric_env_falls_back_to_defaults() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_REQUEST_TIMEOUT_SECS", Some("not-a-number")),
            ("TOADSTOOL_MAX_RETRIES", Some("")),
            ("TOADSTOOL_TOADSTOOL_PORT", Some("bogus")),
        ],
        || {
            assert_eq!(
                ConfigUtils::get_request_timeout(),
                Duration::from_secs(network_defaults::DEFAULT_REQUEST_TIMEOUT_SECS)
            );
            assert_eq!(
                ConfigUtils::get_max_retries(),
                network_defaults::DEFAULT_MAX_RETRIES
            );
        },
    );
}

#[test]
fn port_ranges_follow_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_CONTAINER_PORT_START", Some("3100")),
            ("TOADSTOOL_CONTAINER_PORT_END", Some("3200")),
            ("TOADSTOOL_PORT_RANGE_START", Some("9000")),
            ("TOADSTOOL_PORT_RANGE_END", Some("9100")),
        ],
        || {
            assert_eq!(ConfigUtils::get_container_port_range(), (3100, 3200));
            assert_eq!(ConfigUtils::get_port_allocation_range(), (9000, 9100));
        },
    );
}

#[test]
fn port_ranges_default_when_missing() {
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_CONTAINER_PORT_START",
            "TOADSTOOL_CONTAINER_PORT_END",
            "TOADSTOOL_PORT_RANGE_START",
            "TOADSTOOL_PORT_RANGE_END",
        ],
        || {
            assert_eq!(
                ConfigUtils::get_container_port_range(),
                (
                    defaults_ports::CONTAINER_START,
                    defaults_ports::CONTAINER_END
                )
            );
            assert_eq!(
                ConfigUtils::get_port_allocation_range(),
                (defaults_ports::RANGE_START, defaults_ports::RANGE_END)
            );
        },
    );
}

#[test]
fn service_maps_reflect_ports_and_endpoint() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_PORT", Some("3333")),
            ("TOADSTOOL_FEDERATION_PORT", Some("3334")),
            ("TOADSTOOL_METRICS_PORT", Some("3335")),
            ("TOADSTOOL_HEALTH_PORT", Some("3336")),
            ("TOADSTOOL_EVENTS_PORT", Some("3337")),
            ("TOADSTOOL_TOADSTOOL_PORT", Some("4444")),
            ("TOADSTOOL_EXTERNAL_HOSTNAME", Some("host.test")),
        ],
        || {
            let ports = ConfigUtils::get_service_ports();
            assert_eq!(ports.get(PRIMAL_NAME).copied(), Some(3333));
            assert_eq!(ports.get("federation").copied(), Some(3334));
            assert_eq!(ports.get("metrics").copied(), Some(3335));
            assert_eq!(ports.get("health").copied(), Some(3336));
            assert_eq!(ports.get("events").copied(), Some(3337));

            let endpoints = ConfigUtils::get_service_endpoints();
            assert_eq!(
                endpoints.get(PRIMAL_NAME).map(String::as_str),
                Some("http://host.test:4444")
            );
        },
    );
}

#[test]
fn path_helpers_follow_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_DATA_DIR", Some("/data")),
            ("TOADSTOOL_CACHE_DIR", Some("/cache")),
            ("TOADSTOOL_TEMP_DIR", Some("/tmp/ts")),
            ("TOADSTOOL_LOG_DIR", Some("/logs")),
            ("TOADSTOOL_ENCRYPTION_KEY_PATH", Some("/keys/k")),
            ("TOADSTOOL_TLS_CERT_PATH", Some("/certs/crt")),
            ("TOADSTOOL_TLS_KEY_PATH", Some("/certs/key")),
            ("TOADSTOOL_CA_CERT_PATH", Some("/certs/ca")),
        ],
        || {
            assert_eq!(ConfigUtils::get_data_dir(), "/data");
            assert_eq!(ConfigUtils::get_cache_dir(), "/cache");
            assert_eq!(ConfigUtils::get_temp_dir(), "/tmp/ts");
            assert_eq!(ConfigUtils::get_log_dir(), "/logs");
            assert_eq!(ConfigUtils::get_encryption_key_path(), "/keys/k");
            assert_eq!(ConfigUtils::get_tls_cert_path(), "/certs/crt");
            assert_eq!(ConfigUtils::get_tls_key_path(), "/certs/key");
            assert_eq!(ConfigUtils::get_ca_cert_path(), "/certs/ca");
        },
    );
}

#[test]
fn path_defaults_when_env_missing() {
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_DATA_DIR",
            "TOADSTOOL_CACHE_DIR",
            "TOADSTOOL_TEMP_DIR",
            "TOADSTOOL_LOG_DIR",
            "TOADSTOOL_ENCRYPTION_KEY_PATH",
            "TOADSTOOL_TLS_CERT_PATH",
            "TOADSTOOL_TLS_KEY_PATH",
            "TOADSTOOL_CA_CERT_PATH",
        ],
        || {
            assert_eq!(ConfigUtils::get_data_dir(), "./data");
            assert_eq!(ConfigUtils::get_cache_dir(), "./cache");
            assert_eq!(ConfigUtils::get_temp_dir(), "./tmp");
            assert_eq!(ConfigUtils::get_log_dir(), "./logs");
            assert_eq!(
                ConfigUtils::get_encryption_key_path(),
                "./keys/encryption.key"
            );
            assert_eq!(ConfigUtils::get_tls_cert_path(), "./certs/tls.crt");
            assert_eq!(ConfigUtils::get_tls_key_path(), "./certs/tls.key");
            assert_eq!(ConfigUtils::get_ca_cert_path(), "./certs/ca.crt");
        },
    );
}

#[test]
fn empty_path_env_returns_empty_string() {
    temp_env::with_var("TOADSTOOL_DATA_DIR", Some(""), || {
        assert!(ConfigUtils::get_data_dir().is_empty());
    });
}

#[test]
fn environment_flags_follow_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ENV", Some("staging")),
            ("TOADSTOOL_DEBUG", Some("true")),
            ("TOADSTOOL_VERBOSE", Some("1")),
        ],
        || {
            assert_eq!(ConfigUtils::get_environment(), "staging");
            assert!(ConfigUtils::get_debug_mode());
            assert!(ConfigUtils::get_verbose_mode());
        },
    );
}

#[test]
fn bool_invalid_falls_back_to_default() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_DEBUG", Some("maybe")),
            ("TOADSTOOL_VERBOSE", Some("")),
        ],
        || {
            assert!(!ConfigUtils::get_debug_mode());
            assert!(!ConfigUtils::get_verbose_mode());
        },
    );
}

#[test]
fn get_all_toadstool_env_vars_filters_prefix() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_ALPHA", Some("a")),
            ("TOADSTOOL_BETA", Some("b")),
            ("OTHER_PREFIX_GAMMA", Some("c")),
        ],
        || {
            let map = ConfigUtils::get_all_toadstool_env_vars();
            assert_eq!(map.get("TOADSTOOL_ALPHA").map(String::as_str), Some("a"));
            assert_eq!(map.get("TOADSTOOL_BETA").map(String::as_str), Some("b"));
            assert!(!map.contains_key("OTHER_PREFIX_GAMMA"));
        },
    );
}

#[test]
fn defaults_getters_follow_env() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_WORKER_THREADS", Some("16")),
            ("TOADSTOOL_MAX_CONCURRENT_EXECUTIONS", Some("77")),
            ("TOADSTOOL_EXECUTION_TIMEOUT_SECS", Some("120")),
            ("TOADSTOOL_MAX_CPU_PERCENT", Some("12.5")),
            ("TOADSTOOL_MAX_MEMORY_BYTES", Some("4096")),
            ("TOADSTOOL_MAX_STORAGE_BYTES", Some("8192")),
            ("TOADSTOOL_METRICS_INTERVAL_SECS", Some("3")),
            ("TOADSTOOL_HEALTH_CHECK_INTERVAL_SECS", Some("5")),
            ("TOADSTOOL_LOG_LEVEL", Some("warn")),
            ("TOADSTOOL_TLS_ENABLED", Some("true")),
            ("TOADSTOOL_AUTH_ENABLED", Some("yes")),
            ("TOADSTOOL_SANDBOXING_ENABLED", Some("off")),
            ("TOADSTOOL_METRICS_ENABLED", Some("0")),
            ("TOADSTOOL_HEALTH_CHECKS_ENABLED", Some("false")),
            ("TOADSTOOL_DATABASE_URL", Some("postgres://db")),
            ("TOADSTOOL_REDIS_URL", Some("redis://cache:9")),
            ("TOADSTOOL_AMQP_URL", Some("amqp://q:1")),
            ("TOADSTOOL_METRICS_URL", Some("http://m:9")),
            (
                "TOADSTOOL_ALERT_WEBHOOK_URL",
                Some("https://hooks.example/h"),
            ),
            ("TOADSTOOL_JWT_SECRET", Some("jwt")),
            ("TOADSTOOL_API_KEY", Some("k")),
            ("TOADSTOOL_WEBHOOK_SECRET", Some("wh")),
            ("TOADSTOOL_FEDERATION_TRUST_DOMAIN", Some("trust.example")),
            ("TOADSTOOL_CLUSTER_NAME", Some("c1")),
            ("TOADSTOOL_NODE_NAME", Some("n1")),
        ],
        || {
            assert_eq!(ConfigUtils::get_worker_threads(), 16);
            assert_eq!(ConfigUtils::get_max_concurrent_executions(), 77);
            assert_eq!(ConfigUtils::get_execution_timeout(), Duration::from_mins(2));
            assert!((ConfigUtils::get_max_cpu_usage() - 12.5_f64).abs() < f64::EPSILON);
            assert_eq!(ConfigUtils::get_max_memory_usage(), 4096);
            assert_eq!(ConfigUtils::get_max_storage_usage(), 8192);
            assert_eq!(ConfigUtils::get_metrics_interval(), Duration::from_secs(3));
            assert_eq!(
                ConfigUtils::get_health_check_interval(),
                Duration::from_secs(5)
            );
            assert_eq!(ConfigUtils::get_log_level(), "warn");
            assert!(ConfigUtils::get_tls_enabled());
            assert!(ConfigUtils::get_auth_enabled());
            assert!(!ConfigUtils::get_sandboxing_enabled());
            assert!(!ConfigUtils::get_metrics_enabled());
            assert!(!ConfigUtils::get_health_checks_enabled());
            assert_eq!(ConfigUtils::get_database_url(), "postgres://db");
            assert_eq!(ConfigUtils::get_cache_url(), "redis://cache:9");
            assert_eq!(ConfigUtils::get_message_broker_url(), "amqp://q:1");
            assert_eq!(ConfigUtils::get_monitoring_endpoint(), "http://m:9");
            assert_eq!(
                ConfigUtils::get_alert_webhook_url(),
                "https://hooks.example/h"
            );
            assert_eq!(ConfigUtils::get_jwt_secret(), "jwt");
            assert_eq!(ConfigUtils::get_api_key(), "k");
            assert_eq!(ConfigUtils::get_webhook_secret(), "wh");
            assert_eq!(ConfigUtils::get_federation_trust_domain(), "trust.example");
            assert_eq!(ConfigUtils::get_cluster_name(), "c1");
            assert_eq!(ConfigUtils::get_node_name(), "n1");
        },
    );
}

#[test]
fn defaults_numeric_invalid_falls_back() {
    temp_env::with_vars(
        [
            ("TOADSTOOL_MAX_CPU_PERCENT", Some("not-a-float")),
            ("TOADSTOOL_MAX_MEMORY_BYTES", Some("oops")),
            ("TOADSTOOL_EXECUTION_TIMEOUT_SECS", Some("-1")),
        ],
        || {
            assert!((ConfigUtils::get_max_cpu_usage() - 90.0_f64).abs() < f64::EPSILON);
            assert_eq!(ConfigUtils::get_max_memory_usage(), 8 * 1024 * 1024 * 1024);
            assert_eq!(ConfigUtils::get_execution_timeout(), Duration::from_mins(5));
        },
    );
}

#[test]
fn cache_and_broker_urls_default_when_missing() {
    temp_env::with_vars_unset(
        [
            "TOADSTOOL_REDIS_URL",
            "TOADSTOOL_AMQP_URL",
            "TOADSTOOL_METRICS_URL",
            "TOADSTOOL_ALERT_WEBHOOK_URL",
        ],
        || {
            assert_eq!(
                ConfigUtils::get_cache_url(),
                format!(
                    "redis://{}:{}",
                    defaults_network::LOCALHOST,
                    defaults_storage::REDIS_PORT
                )
            );
            assert_eq!(
                ConfigUtils::get_message_broker_url(),
                format!(
                    "amqp://{}:{}",
                    defaults_network::LOCALHOST,
                    defaults_storage::AMQP_PORT
                )
            );
            assert_eq!(
                ConfigUtils::get_monitoring_endpoint(),
                format!(
                    "http://{}:{}",
                    defaults_network::LOCALHOST,
                    defaults_network::METRICS_PORT
                )
            );
            assert_eq!(ConfigUtils::get_alert_webhook_url(), "");
        },
    );
}

fn sample_network_env_config() -> NetworkEnvConfig {
    NetworkEnvConfig {
        toadstool_port: 5000,
        federation_port: 5001,
        metrics_port: 5002,
        health_port: 5003,
        events_port: 5004,
        bind_address: "0.0.0.0".to_string(),
        external_hostname: "node.example".to_string(),
        tls_enabled: true,
        connection_timeout_secs: 11,
        request_timeout_secs: 22,
        max_retries: 4,
        max_connections_per_host: 50,
        coordination_port: 8080,
        security_port: 8081,
        storage_port: 8082,
        ai_processing_port: 8083,
        biomeos_port: 8005,
    }
}

#[test]
fn network_env_config_serde_json_round_trip() {
    let original = sample_network_env_config();
    let value = serde_json::to_value(&original).expect("serialize NetworkEnvConfig");
    let parsed: NetworkEnvConfig = serde_json::from_value(value.clone()).expect("deserialize");
    let again = serde_json::to_value(&parsed).expect("re-serialize");
    assert_eq!(value, again);
}

#[test]
fn environment_config_serde_json_round_trip() {
    let original = EnvironmentConfig {
        network: sample_network_env_config(),
        resources: ResourceEnvConfig {
            max_cpu_percent: 10.0,
            max_memory_bytes: 100,
            max_storage_bytes: 200,
            max_network_mbps: 50.0,
            max_gpu_percent: 80.0,
            max_concurrent_executions: 5,
            worker_threads: 2,
            queue_size: 100,
            batch_size: 10,
        },
        monitoring: MonitoringEnvConfig {
            metrics_enabled: true,
            metrics_interval_secs: 10,
            metrics_retention_days: 3,
            health_checks_enabled: true,
            health_check_interval_secs: 20,
            logging_enabled: true,
            log_level: "debug".to_string(),
            log_dir: PathBuf::from("./logs"),
            alerts_enabled: false,
            cpu_alert_threshold: 90.0,
            memory_alert_threshold: 91.0,
            storage_alert_threshold: 92.0,
        },
        security: SecurityEnvConfig {
            auth_enabled: true,
            auth_token_expiry_secs: 3600,
            sandboxing_enabled: true,
            isolation_level: "Strict".to_string(),
            encryption_enabled: false,
            encryption_key_path: PathBuf::from("./keys/x.key"),
            rate_limiting_enabled: false,
            rate_limit_rps: 10,
            rate_limit_burst: 20,
            cors_enabled: true,
            cors_allowed_origins: vec!["https://a.example".to_string()],
        },
        environment: "integration".to_string(),
        debug: false,
        verbose: true,
        data_dir: PathBuf::from("/d"),
        cache_dir: PathBuf::from("/c"),
        temp_dir: PathBuf::from("/t"),
    };
    let value = serde_json::to_value(&original).expect("serialize EnvironmentConfig");
    let parsed: EnvironmentConfig = serde_json::from_value(value.clone()).expect("deserialize");
    let again = serde_json::to_value(&parsed).expect("re-serialize");
    assert_eq!(value, again);
}

#[test]
fn environment_config_serde_yaml_round_trip() {
    let original = EnvironmentConfig {
        network: sample_network_env_config(),
        resources: ResourceEnvConfig {
            max_cpu_percent: 11.0,
            max_memory_bytes: 101,
            max_storage_bytes: 201,
            max_network_mbps: 51.0,
            max_gpu_percent: 81.0,
            max_concurrent_executions: 6,
            worker_threads: 3,
            queue_size: 101,
            batch_size: 11,
        },
        monitoring: MonitoringEnvConfig {
            metrics_enabled: false,
            metrics_interval_secs: 11,
            metrics_retention_days: 4,
            health_checks_enabled: false,
            health_check_interval_secs: 21,
            logging_enabled: false,
            log_level: "error".to_string(),
            log_dir: PathBuf::from("./var/log"),
            alerts_enabled: true,
            cpu_alert_threshold: 88.0,
            memory_alert_threshold: 89.0,
            storage_alert_threshold: 90.0,
        },
        security: SecurityEnvConfig {
            auth_enabled: false,
            auth_token_expiry_secs: 1800,
            sandboxing_enabled: false,
            isolation_level: "Minimal".to_string(),
            encryption_enabled: true,
            encryption_key_path: PathBuf::from("/secret/key"),
            rate_limiting_enabled: true,
            rate_limit_rps: 200,
            rate_limit_burst: 400,
            cors_enabled: false,
            cors_allowed_origins: vec![],
        },
        environment: "qa".to_string(),
        debug: true,
        verbose: false,
        data_dir: PathBuf::from("/data"),
        cache_dir: PathBuf::from("/cache"),
        temp_dir: PathBuf::from("/tmp"),
    };
    let yaml = serde_yaml_ng::to_string(&original).expect("yaml serialize");
    let parsed: EnvironmentConfig = serde_yaml_ng::from_str(&yaml).expect("yaml deserialize");
    let yaml_again = serde_yaml_ng::to_string(&parsed).expect("yaml re-serialize");
    assert_eq!(yaml, yaml_again);
}

#[tokio::test]
async fn config_utils_accessors_run_under_tokio() {
    tokio::task::yield_now().await;
    temp_env::with_var("TOADSTOOL_ENV", Some("async-context"), || {
        assert_eq!(ConfigUtils::get_environment(), "async-context");
    });
}

#[test]
fn json_macro_round_trip_network_subset() {
    let v = json!({
        "toadstool_port": 1,
        "federation_port": 2,
        "metrics_port": 3,
        "health_port": 4,
        "events_port": 5,
        "bind_address": "127.0.0.1",
        "external_hostname": "h",
        "tls_enabled": false,
        "connection_timeout_secs": 1,
        "request_timeout_secs": 2,
        "max_retries": 3,
        "max_connections_per_host": 4,
        "songbird_port": 6,
        "beardog_port": 7,
        "nestgate_port": 8,
        "squirrel_port": 9,
        "biomeos_port": 10
    });
    let n: NetworkEnvConfig = serde_json::from_value(v).expect("from_value");
    assert_eq!(n.coordination_port, 6);
    assert_eq!(n.security_port, 7);
    assert_eq!(n.storage_port, 8);
    assert_eq!(n.ai_processing_port, 9);
    let out = serde_json::to_value(&n).expect("to_value");
    assert_eq!(out["toadstool_port"], 1);
    assert_eq!(out["biomeos_port"], 10);
    assert_eq!(out["coordination_port"], 6);
    assert_eq!(out["security_port"], 7);
    assert_eq!(out["storage_port"], 8);
    assert_eq!(out["ai_processing_port"], 9);
}
