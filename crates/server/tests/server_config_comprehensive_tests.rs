//! Comprehensive tests for server configuration types

use std::collections::HashMap;
use std::time::Duration;
use toadstool_server::{
    AuthenticationConfig, HealthCheckConfig, LoggingConfig, RateLimitingConfig, ServerConfig,
};

// ============================================================================
// ServerConfig Tests
// ============================================================================

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();

    assert!(!config.bind_address.is_empty());
    assert!(config.enable_api);
    assert!(!config.enable_websocket); // Disabled by default for security
    assert!(config.enable_cors);
    assert_eq!(config.max_concurrent_executions, 100);
    assert_eq!(config.default_timeout, Duration::from_secs(300));
    assert!(config.auth.is_none());
    assert!(config.rate_limiting.is_none());
}

#[test]
fn test_server_config_custom_bind_address() {
    let config = ServerConfig::default().bind_address("0.0.0.0:3000");

    assert_eq!(config.bind_address, "0.0.0.0:3000");
}

#[test]
fn test_server_config_enable_api() {
    let config = ServerConfig::default().enable_api(false);

    assert!(!config.enable_api);
}

#[test]
fn test_server_config_enable_websocket() {
    let config = ServerConfig::default().enable_websocket(false);

    assert!(!config.enable_websocket);
}

#[test]
fn test_server_config_max_concurrent_executions() {
    let config = ServerConfig::default().max_concurrent_executions(50);

    assert_eq!(config.max_concurrent_executions, 50);
}

#[test]
fn test_server_config_default_timeout() {
    let timeout = Duration::from_secs(600);
    let config = ServerConfig::default().default_timeout(timeout);

    assert_eq!(config.default_timeout, timeout);
}

#[test]
fn test_server_config_with_auth() {
    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec!["key1".to_string()],
        jwt_secret: Some("secret".to_string()),
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    let config = ServerConfig::default().auth(auth);

    assert!(config.auth.is_some());
    assert!(config.auth.unwrap().required);
}

#[test]
fn test_server_config_with_rate_limiting() {
    let rate_limiting = RateLimitingConfig {
        requests_per_minute: 200,
        concurrent_executions_per_client: 20,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    let config = ServerConfig::default().rate_limiting(rate_limiting);

    assert!(config.rate_limiting.is_some());
    let rl = config.rate_limiting.unwrap();
    assert_eq!(rl.requests_per_minute, 200);
}

#[test]
fn test_server_config_builder_pattern() {
    let config = ServerConfig::default()
        .bind_address("127.0.0.1:8080")
        .enable_api(true)
        .enable_websocket(true)
        .max_concurrent_executions(150)
        .default_timeout(Duration::from_secs(180));

    assert_eq!(config.bind_address, "127.0.0.1:8080");
    assert_eq!(config.max_concurrent_executions, 150);
    assert_eq!(config.default_timeout, Duration::from_secs(180));
}

#[test]
fn test_server_config_zero_concurrent_executions() {
    let config = ServerConfig::default().max_concurrent_executions(0);

    assert_eq!(config.max_concurrent_executions, 0);
}

#[test]
fn test_server_config_very_high_concurrent_executions() {
    let config = ServerConfig::default().max_concurrent_executions(10000);

    assert_eq!(config.max_concurrent_executions, 10000);
}

// ============================================================================
// AuthenticationConfig Tests
// ============================================================================

#[test]
fn test_authentication_config_default() {
    let config = AuthenticationConfig::default();

    assert!(!config.required);
    assert!(config.api_keys.is_empty());
    assert!(config.jwt_secret.is_none());
    assert!(config.basic_auth.is_empty());
    assert!(config.custom_validator.is_none());
}

#[test]
fn test_authentication_config_with_api_keys() {
    let config = AuthenticationConfig {
        required: true,
        api_keys: vec!["key1".to_string(), "key2".to_string(), "key3".to_string()],
        jwt_secret: None,
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert!(config.required);
    assert_eq!(config.api_keys.len(), 3);
}

#[test]
fn test_authentication_config_with_jwt_secret() {
    let config = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: Some("my-secret-key".to_string()),
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert!(config.jwt_secret.is_some());
    assert_eq!(config.jwt_secret.unwrap(), "my-secret-key");
}

#[test]
fn test_authentication_config_with_basic_auth() {
    let mut basic_auth = HashMap::new();
    basic_auth.insert("user1".to_string(), "password1".to_string());
    basic_auth.insert("user2".to_string(), "password2".to_string());

    let config = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: None,
        basic_auth: basic_auth.clone(),
        custom_validator: None,
    };

    assert_eq!(config.basic_auth.len(), 2);
    assert_eq!(
        config.basic_auth.get("user1"),
        Some(&"password1".to_string())
    );
}

#[test]
fn test_authentication_config_with_custom_validator() {
    let config = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: None,
        basic_auth: HashMap::new(),
        custom_validator: Some("custom_auth_fn".to_string()),
    };

    assert!(config.custom_validator.is_some());
}

#[test]
fn test_authentication_config_not_required() {
    let config = AuthenticationConfig {
        required: false,
        api_keys: vec!["key1".to_string()],
        jwt_secret: Some("secret".to_string()),
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert!(!config.required);
}

// ============================================================================
// RateLimitingConfig Tests
// ============================================================================

#[test]
fn test_rate_limiting_config_default() {
    let config = RateLimitingConfig::default();

    assert_eq!(config.requests_per_minute, 100);
    assert_eq!(config.concurrent_executions_per_client, 10);
    assert!(config.limit_by_ip);
    assert!(config.limit_by_api_key);
}

#[test]
fn test_rate_limiting_config_custom_requests() {
    let config = RateLimitingConfig {
        requests_per_minute: 500,
        concurrent_executions_per_client: 10,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.requests_per_minute, 500);
}

#[test]
fn test_rate_limiting_config_custom_executions() {
    let config = RateLimitingConfig {
        requests_per_minute: 100,
        concurrent_executions_per_client: 50,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.concurrent_executions_per_client, 50);
}

#[test]
fn test_rate_limiting_config_ip_only() {
    let config = RateLimitingConfig {
        requests_per_minute: 100,
        concurrent_executions_per_client: 10,
        limit_by_ip: true,
        limit_by_api_key: false,
    };

    assert!(config.limit_by_ip);
    assert!(!config.limit_by_api_key);
}

#[test]
fn test_rate_limiting_config_api_key_only() {
    let config = RateLimitingConfig {
        requests_per_minute: 100,
        concurrent_executions_per_client: 10,
        limit_by_ip: false,
        limit_by_api_key: true,
    };

    assert!(!config.limit_by_ip);
    assert!(config.limit_by_api_key);
}

#[test]
fn test_rate_limiting_config_no_limits() {
    let config = RateLimitingConfig {
        requests_per_minute: 100,
        concurrent_executions_per_client: 10,
        limit_by_ip: false,
        limit_by_api_key: false,
    };

    assert!(!config.limit_by_ip);
    assert!(!config.limit_by_api_key);
}

#[test]
fn test_rate_limiting_config_zero_requests() {
    let config = RateLimitingConfig {
        requests_per_minute: 0,
        concurrent_executions_per_client: 10,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.requests_per_minute, 0);
}

#[test]
fn test_rate_limiting_config_very_high_requests() {
    let config = RateLimitingConfig {
        requests_per_minute: 100000,
        concurrent_executions_per_client: 10,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.requests_per_minute, 100000);
}

// ============================================================================
// LoggingConfig Tests
// ============================================================================

#[test]
fn test_logging_config_default() {
    let config = LoggingConfig::default();

    assert_eq!(config.level, "info");
    assert!(config.log_requests);
    assert!(config.log_executions);
    assert!(config.log_metrics);
}

#[test]
fn test_logging_config_debug_level() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };

    assert_eq!(config.level, "debug");
}

#[test]
fn test_logging_config_warn_level() {
    let config = LoggingConfig {
        level: "warn".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };

    assert_eq!(config.level, "warn");
}

#[test]
fn test_logging_config_error_level() {
    let config = LoggingConfig {
        level: "error".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };

    assert_eq!(config.level, "error");
}

#[test]
fn test_logging_config_no_request_logging() {
    let config = LoggingConfig {
        level: "info".to_string(),
        log_requests: false,
        log_executions: true,
        log_metrics: true,
    };

    assert!(!config.log_requests);
}

#[test]
fn test_logging_config_no_execution_logging() {
    let config = LoggingConfig {
        level: "info".to_string(),
        log_requests: true,
        log_executions: false,
        log_metrics: true,
    };

    assert!(!config.log_executions);
}

#[test]
fn test_logging_config_no_metrics_logging() {
    let config = LoggingConfig {
        level: "info".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: false,
    };

    assert!(!config.log_metrics);
}

#[test]
fn test_logging_config_minimal() {
    let config = LoggingConfig {
        level: "error".to_string(),
        log_requests: false,
        log_executions: false,
        log_metrics: false,
    };

    assert_eq!(config.level, "error");
    assert!(!config.log_requests);
    assert!(!config.log_executions);
    assert!(!config.log_metrics);
}

// ============================================================================
// HealthCheckConfig Tests
// ============================================================================

#[test]
fn test_health_check_config_default() {
    let config = HealthCheckConfig::default();

    assert_eq!(config.interval, Duration::from_secs(30));
    assert!(config.check_runtime_engines);
    assert!(config.check_resources);
    assert_eq!(config.memory_threshold_percent, 90.0);
    assert_eq!(config.cpu_threshold_percent, 95.0);
}

#[test]
fn test_health_check_config_custom_interval() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(60),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert_eq!(config.interval, Duration::from_secs(60));
}

#[test]
fn test_health_check_config_no_runtime_checks() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: false,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert!(!config.check_runtime_engines);
}

#[test]
fn test_health_check_config_no_resource_checks() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: false,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert!(!config.check_resources);
}

#[test]
fn test_health_check_config_custom_memory_threshold() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 80.0,
        cpu_threshold_percent: 95.0,
    };

    assert_eq!(config.memory_threshold_percent, 80.0);
}

#[test]
fn test_health_check_config_custom_cpu_threshold() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 85.0,
    };

    assert_eq!(config.cpu_threshold_percent, 85.0);
}

#[test]
fn test_health_check_config_strict_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(15),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 70.0,
        cpu_threshold_percent: 75.0,
    };

    assert_eq!(config.memory_threshold_percent, 70.0);
    assert_eq!(config.cpu_threshold_percent, 75.0);
}

#[test]
fn test_health_check_config_lenient_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(60),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 98.0,
        cpu_threshold_percent: 99.0,
    };

    assert_eq!(config.memory_threshold_percent, 98.0);
    assert_eq!(config.cpu_threshold_percent, 99.0);
}

// ============================================================================
// Additional Edge Case Tests
// ============================================================================

#[test]
fn test_server_config_ipv6_bind_address() {
    let config = ServerConfig::default().bind_address("[::1]:8080");

    assert_eq!(config.bind_address, "[::1]:8080");
}

#[test]
fn test_server_config_hostname_bind_address() {
    let config = ServerConfig::default().bind_address("localhost:8080");

    assert_eq!(config.bind_address, "localhost:8080");
}

#[test]
fn test_server_config_very_short_timeout() {
    let config = ServerConfig::default().default_timeout(Duration::from_secs(1));

    assert_eq!(config.default_timeout, Duration::from_secs(1));
}

#[test]
fn test_server_config_very_long_timeout() {
    let config = ServerConfig::default().default_timeout(Duration::from_secs(86400));

    assert_eq!(config.default_timeout, Duration::from_secs(86400));
}

#[test]
fn test_authentication_config_empty_api_keys() {
    let config = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: None,
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert!(config.api_keys.is_empty());
}

#[test]
fn test_authentication_config_single_api_key() {
    let config = AuthenticationConfig {
        required: true,
        api_keys: vec!["single_key".to_string()],
        jwt_secret: None,
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert_eq!(config.api_keys.len(), 1);
}

#[test]
fn test_authentication_config_many_api_keys() {
    let api_keys: Vec<String> = (0..100).map(|i| format!("key{}", i)).collect();

    let config = AuthenticationConfig {
        required: true,
        api_keys: api_keys.clone(),
        jwt_secret: None,
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert_eq!(config.api_keys.len(), 100);
}

#[test]
fn test_authentication_config_empty_basic_auth() {
    let config = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: None,
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert!(config.basic_auth.is_empty());
}

#[test]
fn test_authentication_config_single_basic_auth_user() {
    let mut basic_auth = HashMap::new();
    basic_auth.insert("admin".to_string(), "password".to_string());

    let config = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: None,
        basic_auth,
        custom_validator: None,
    };

    assert_eq!(config.basic_auth.len(), 1);
}

#[test]
fn test_rate_limiting_config_one_request_per_minute() {
    let config = RateLimitingConfig {
        requests_per_minute: 1,
        concurrent_executions_per_client: 10,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.requests_per_minute, 1);
}

#[test]
fn test_rate_limiting_config_zero_concurrent_executions() {
    let config = RateLimitingConfig {
        requests_per_minute: 100,
        concurrent_executions_per_client: 0,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.concurrent_executions_per_client, 0);
}

#[test]
fn test_rate_limiting_config_one_concurrent_execution() {
    let config = RateLimitingConfig {
        requests_per_minute: 100,
        concurrent_executions_per_client: 1,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.concurrent_executions_per_client, 1);
}

#[test]
fn test_logging_config_trace_level() {
    let config = LoggingConfig {
        level: "trace".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };

    assert_eq!(config.level, "trace");
}

#[test]
fn test_logging_config_all_logging_enabled() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };

    assert!(config.log_requests);
    assert!(config.log_executions);
    assert!(config.log_metrics);
}

#[test]
fn test_logging_config_all_logging_disabled() {
    let config = LoggingConfig {
        level: "off".to_string(),
        log_requests: false,
        log_executions: false,
        log_metrics: false,
    };

    assert!(!config.log_requests);
    assert!(!config.log_executions);
    assert!(!config.log_metrics);
}

#[test]
fn test_health_check_config_very_short_interval() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(1),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert_eq!(config.interval, Duration::from_secs(1));
}

#[test]
fn test_health_check_config_very_long_interval() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(3600),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert_eq!(config.interval, Duration::from_secs(3600));
}

#[test]
fn test_health_check_config_no_checks_enabled() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: false,
        check_resources: false,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert!(!config.check_runtime_engines);
    assert!(!config.check_resources);
}

#[test]
fn test_health_check_config_low_memory_threshold() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 50.0,
        cpu_threshold_percent: 95.0,
    };

    assert_eq!(config.memory_threshold_percent, 50.0);
}

#[test]
fn test_health_check_config_low_cpu_threshold() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 60.0,
    };

    assert_eq!(config.cpu_threshold_percent, 60.0);
}

#[test]
fn test_health_check_config_100_percent_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 100.0,
        cpu_threshold_percent: 100.0,
    };

    assert_eq!(config.memory_threshold_percent, 100.0);
    assert_eq!(config.cpu_threshold_percent, 100.0);
}

#[test]
fn test_server_config_all_features_disabled() {
    let config = ServerConfig::default()
        .enable_api(false)
        .enable_websocket(false);

    assert!(!config.enable_api);
    assert!(!config.enable_websocket);
}

#[test]
fn test_server_config_minimal_concurrent_executions() {
    let config = ServerConfig::default().max_concurrent_executions(1);

    assert_eq!(config.max_concurrent_executions, 1);
}

#[test]
fn test_authentication_config_all_methods() {
    let mut basic_auth = HashMap::new();
    basic_auth.insert("user".to_string(), "pass".to_string());

    let config = AuthenticationConfig {
        required: true,
        api_keys: vec!["key".to_string()],
        jwt_secret: Some("secret".to_string()),
        basic_auth,
        custom_validator: Some("validator".to_string()),
    };

    assert!(!config.api_keys.is_empty());
    assert!(config.jwt_secret.is_some());
    assert!(!config.basic_auth.is_empty());
    assert!(config.custom_validator.is_some());
}

#[test]
fn test_rate_limiting_config_high_concurrent_executions() {
    let config = RateLimitingConfig {
        requests_per_minute: 100,
        concurrent_executions_per_client: 1000,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.concurrent_executions_per_client, 1000);
}

#[test]
fn test_logging_config_custom_level() {
    let config = LoggingConfig {
        level: "custom".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };

    assert_eq!(config.level, "custom");
}

#[test]
fn test_server_config_default_has_logging_and_health_check() {
    let config = ServerConfig::default();

    assert!(!config.logging.level.is_empty());
    assert!(config.health_check.interval > Duration::from_secs(0));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_complete_server_config_with_all_options() {
    let mut basic_auth = HashMap::new();
    basic_auth.insert("admin".to_string(), "admin123".to_string());

    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec!["key1".to_string()],
        jwt_secret: Some("secret".to_string()),
        basic_auth,
        custom_validator: None,
    };

    let rate_limiting = RateLimitingConfig {
        requests_per_minute: 200,
        concurrent_executions_per_client: 20,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    let config = ServerConfig::default()
        .bind_address("0.0.0.0:9000")
        .enable_api(true)
        .enable_websocket(true)
        .max_concurrent_executions(200)
        .default_timeout(Duration::from_secs(600))
        .auth(auth)
        .rate_limiting(rate_limiting);

    assert_eq!(config.bind_address, "0.0.0.0:9000");
    assert_eq!(config.max_concurrent_executions, 200);
    assert!(config.auth.is_some());
    assert!(config.rate_limiting.is_some());
}

#[test]
fn test_minimal_server_config() {
    let config = ServerConfig::default()
        .bind_address("127.0.0.1:8080")
        .enable_api(false)
        .enable_websocket(false)
        .max_concurrent_executions(10);

    assert!(!config.enable_api);
    assert!(!config.enable_websocket);
    assert_eq!(config.max_concurrent_executions, 10);
}

#[test]
fn test_server_config_clone() {
    let config1 = ServerConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.bind_address, config2.bind_address);
    assert_eq!(config1.enable_api, config2.enable_api);
}

#[test]
fn test_authentication_config_clone() {
    let config1 = AuthenticationConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.required, config2.required);
    assert_eq!(config1.api_keys, config2.api_keys);
}

#[test]
fn test_rate_limiting_config_clone() {
    let config1 = RateLimitingConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.requests_per_minute, config2.requests_per_minute);
    assert_eq!(config1.limit_by_ip, config2.limit_by_ip);
}

#[test]
fn test_logging_config_clone() {
    let config1 = LoggingConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.level, config2.level);
    assert_eq!(config1.log_requests, config2.log_requests);
}

#[test]
fn test_health_check_config_clone() {
    let config1 = HealthCheckConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.interval, config2.interval);
    assert_eq!(
        config1.memory_threshold_percent,
        config2.memory_threshold_percent
    );
}
