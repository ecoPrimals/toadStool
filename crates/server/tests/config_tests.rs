// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Comprehensive tests for server configuration
//!
//! Tests for `ServerConfig`, `AuthenticationConfig`, `RateLimitingConfig`,
//! `LoggingConfig`, and `HealthCheckConfig`.

use std::collections::HashMap;
use std::time::Duration;
use toadstool_server::*;

// ============================================================================
// ServerConfig Tests
// ============================================================================

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();

    assert!(config.enable_api);
    assert!(config.enable_cors);
    assert_eq!(config.max_concurrent_executions, 100);
    assert_eq!(config.default_timeout, Duration::from_mins(5));
    assert_eq!(config.resource_monitoring_interval, Duration::from_secs(30));
}

#[test]
fn test_server_config_bind_address_builder() {
    let config = ServerConfig::default().bind_address("127.0.0.1:9000");

    assert_eq!(config.bind_address, "127.0.0.1:9000");
}

#[test]
fn test_server_config_enable_api() {
    let config = ServerConfig::default().enable_api(false);

    assert!(!config.enable_api);
}

#[test]
fn test_server_config_max_concurrent_executions() {
    let config = ServerConfig::default().max_concurrent_executions(250);

    assert_eq!(config.max_concurrent_executions, 250);
}

#[test]
fn test_server_config_default_timeout() {
    let config = ServerConfig::default().default_timeout(Duration::from_mins(10));

    assert_eq!(config.default_timeout, Duration::from_mins(10));
}

#[test]
fn test_server_config_chained_builders() {
    let config = ServerConfig::default()
        .bind_address("0.0.0.0:8080")
        .enable_api(true)
        .max_concurrent_executions(50)
        .default_timeout(Duration::from_mins(2));

    assert_eq!(config.bind_address, "0.0.0.0:8080");
    assert!(config.enable_api);
    assert_eq!(config.max_concurrent_executions, 50);
    assert_eq!(config.default_timeout, Duration::from_mins(2));
}

#[test]
fn test_server_config_with_auth() {
    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec!["key1".to_string(), "key2".to_string()],
        jwt_secret: Some("secret123".to_string()),
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
        limit_by_api_key: false,
    };

    let config = ServerConfig::default().rate_limiting(rate_limiting);

    assert!(config.rate_limiting.is_some());
    let rl = config.rate_limiting.unwrap();
    assert_eq!(rl.requests_per_minute, 200);
    assert_eq!(rl.concurrent_executions_per_client, 20);
}

#[test]
fn test_server_config_minimal() {
    let config = ServerConfig::default()
        .enable_api(false)
        .max_concurrent_executions(1);

    assert!(!config.enable_api);
    assert_eq!(config.max_concurrent_executions, 1);
}

#[test]
fn test_server_config_maximal() {
    let config = ServerConfig::default()
        .enable_api(true)
        .max_concurrent_executions(1000)
        .default_timeout(Duration::from_hours(1));

    assert!(config.enable_api);
    assert_eq!(config.max_concurrent_executions, 1000);
    assert_eq!(config.default_timeout, Duration::from_hours(1));
}

// ============================================================================
// AuthenticationConfig Tests
// ============================================================================

#[test]
fn test_authentication_config_default() {
    let auth = AuthenticationConfig::default();

    assert!(!auth.required);
    assert!(auth.api_keys.is_empty());
    assert_eq!(auth.jwt_secret, None);
    assert!(auth.basic_auth.is_empty());
    assert_eq!(auth.custom_validator, None);
}

#[test]
fn test_authentication_config_with_api_keys() {
    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec![
            "api-key-1".to_string(),
            "api-key-2".to_string(),
            "api-key-3".to_string(),
        ],
        jwt_secret: None,
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert!(auth.required);
    assert_eq!(auth.api_keys.len(), 3);
    assert!(auth.api_keys.contains(&"api-key-1".to_string()));
}

#[test]
fn test_authentication_config_with_jwt() {
    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: Some("my-super-secret-jwt-key".to_string()),
        basic_auth: HashMap::new(),
        custom_validator: None,
    };

    assert!(auth.jwt_secret.is_some());
    assert_eq!(auth.jwt_secret.unwrap(), "my-super-secret-jwt-key");
}

#[test]
fn test_authentication_config_with_basic_auth() {
    let mut basic_auth = HashMap::new();
    basic_auth.insert("admin".to_string(), "admin123".to_string());
    basic_auth.insert("user".to_string(), "user456".to_string());

    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: None,
        basic_auth,
        custom_validator: None,
    };

    assert_eq!(auth.basic_auth.len(), 2);
    assert_eq!(auth.basic_auth.get("admin"), Some(&"admin123".to_string()));
}

#[test]
fn test_authentication_config_with_custom_validator() {
    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: None,
        basic_auth: HashMap::new(),
        custom_validator: Some("custom_auth_handler".to_string()),
    };

    assert!(auth.custom_validator.is_some());
    assert_eq!(auth.custom_validator.unwrap(), "custom_auth_handler");
}

#[test]
fn test_authentication_config_all_methods() {
    let mut basic_auth = HashMap::new();
    basic_auth.insert("user".to_string(), "pass".to_string());

    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec!["key1".to_string()],
        jwt_secret: Some("secret".to_string()),
        basic_auth,
        custom_validator: Some("validator".to_string()),
    };

    assert!(auth.required);
    assert_eq!(auth.api_keys.len(), 1);
    assert!(auth.jwt_secret.is_some());
    assert_eq!(auth.basic_auth.len(), 1);
    assert!(auth.custom_validator.is_some());
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
fn test_rate_limiting_config_custom() {
    let config = RateLimitingConfig {
        requests_per_minute: 500,
        concurrent_executions_per_client: 50,
        limit_by_ip: false,
        limit_by_api_key: true,
    };

    assert_eq!(config.requests_per_minute, 500);
    assert_eq!(config.concurrent_executions_per_client, 50);
    assert!(!config.limit_by_ip);
    assert!(config.limit_by_api_key);
}

#[test]
fn test_rate_limiting_config_strict() {
    let config = RateLimitingConfig {
        requests_per_minute: 10,
        concurrent_executions_per_client: 1,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.requests_per_minute, 10);
    assert_eq!(config.concurrent_executions_per_client, 1);
}

#[test]
fn test_rate_limiting_config_permissive() {
    let config = RateLimitingConfig {
        requests_per_minute: 10000,
        concurrent_executions_per_client: 1000,
        limit_by_ip: false,
        limit_by_api_key: false,
    };

    assert_eq!(config.requests_per_minute, 10000);
    assert_eq!(config.concurrent_executions_per_client, 1000);
    assert!(!config.limit_by_ip);
    assert!(!config.limit_by_api_key);
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
fn test_logging_config_error_level() {
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

#[test]
fn test_logging_config_selective() {
    let config = LoggingConfig {
        level: "warn".to_string(),
        log_requests: true,
        log_executions: false,
        log_metrics: true,
    };

    assert_eq!(config.level, "warn");
    assert!(config.log_requests);
    assert!(!config.log_executions);
    assert!(config.log_metrics);
}

#[test]
fn test_logging_config_all_levels() {
    let levels = vec!["debug", "info", "warn", "error", "trace"];

    for level in levels {
        let config = LoggingConfig {
            level: level.to_string(),
            log_requests: true,
            log_executions: true,
            log_metrics: true,
        };

        assert_eq!(config.level, level);
    }
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
        interval: Duration::from_secs(10),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert_eq!(config.interval, Duration::from_secs(10));
}

#[test]
fn test_health_check_config_custom_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 80.0,
        cpu_threshold_percent: 85.0,
    };

    assert_eq!(config.memory_threshold_percent, 80.0);
    assert_eq!(config.cpu_threshold_percent, 85.0);
}

#[test]
fn test_health_check_config_runtime_only() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: false,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert!(config.check_runtime_engines);
    assert!(!config.check_resources);
}

#[test]
fn test_health_check_config_resources_only() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: false,
        check_resources: true,
        memory_threshold_percent: 90.0,
        cpu_threshold_percent: 95.0,
    };

    assert!(!config.check_runtime_engines);
    assert!(config.check_resources);
}

#[test]
fn test_health_check_config_no_checks() {
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
fn test_health_check_config_strict_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 70.0,
        cpu_threshold_percent: 75.0,
    };

    assert_eq!(config.memory_threshold_percent, 70.0);
    assert_eq!(config.cpu_threshold_percent, 75.0);
}

#[test]
fn test_health_check_config_permissive_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(30),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 98.0,
        cpu_threshold_percent: 99.0,
    };

    assert_eq!(config.memory_threshold_percent, 98.0);
    assert_eq!(config.cpu_threshold_percent, 99.0);
}

// ============================================================================
// Configuration Integration Tests
// ============================================================================

#[test]
fn test_full_server_config_production() {
    let mut basic_auth = HashMap::new();
    basic_auth.insert("admin".to_string(), "secure_password".to_string());

    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec!["prod-key-1".to_string(), "prod-key-2".to_string()],
        jwt_secret: Some("prod-jwt-secret-key".to_string()),
        basic_auth,
        custom_validator: None,
    };

    let rate_limiting = RateLimitingConfig {
        requests_per_minute: 1000,
        concurrent_executions_per_client: 100,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    let config = ServerConfig::default()
        .bind_address("0.0.0.0:443")
        .enable_api(true)
        .max_concurrent_executions(500)
        .default_timeout(Duration::from_mins(10))
        .auth(auth)
        .rate_limiting(rate_limiting);

    assert_eq!(config.bind_address, "0.0.0.0:443");
    assert!(config.enable_api);
    assert_eq!(config.max_concurrent_executions, 500);
    assert!(config.auth.is_some());
    assert!(config.rate_limiting.is_some());
}

#[test]
fn test_full_server_config_development() {
    let config = ServerConfig::default()
        .bind_address("127.0.0.1:8080")
        .enable_api(true)
        .max_concurrent_executions(10)
        .default_timeout(Duration::from_mins(1));

    assert_eq!(config.bind_address, "127.0.0.1:8080");
    assert_eq!(config.max_concurrent_executions, 10);
    assert!(config.auth.is_none()); // No auth in dev
    assert!(config.rate_limiting.is_none()); // No rate limiting in dev
}
