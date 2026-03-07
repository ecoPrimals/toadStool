// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for server configuration
//!
//! Week 13 Day 1: Configuration and Type Tests
//! Target: Verify `ServerConfig`, `HealthCheckConfig`, and related configuration types

use std::time::Duration;
use toadstool_server::config::{
    AuthenticationConfig, HealthCheckConfig, LoggingConfig, RateLimitingConfig, ServerConfig,
};

// =============================================================================
// ServerConfig Tests
// =============================================================================

#[test]
fn test_server_config_default_values() {
    let config = ServerConfig::default();

    // Verify default values are sensible
    assert!(config.bind_address.contains(':'));
    assert!(config.enable_api);
    assert!(config.enable_cors);
    assert_eq!(config.max_concurrent_executions, 100);
    assert_eq!(config.default_timeout, Duration::from_secs(300));
    assert_eq!(config.resource_monitoring_interval, Duration::from_secs(30));
}

#[test]
fn test_server_config_builder_pattern() {
    let config = ServerConfig::default()
        .bind_address("127.0.0.1:3000")
        .enable_api(true)
        .max_concurrent_executions(50)
        .default_timeout(Duration::from_secs(600));

    assert_eq!(config.bind_address, "127.0.0.1:3000");
    assert!(config.enable_api);
    assert_eq!(config.max_concurrent_executions, 50);
    assert_eq!(config.default_timeout, Duration::from_secs(600));
}

#[test]
fn test_server_config_with_authentication() {
    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec!["test-key-1".to_string(), "test-key-2".to_string()],
        jwt_secret: Some("secret-key".to_string()),
        ..Default::default()
    };

    let config = ServerConfig::default().auth(auth.clone());

    assert!(config.auth.is_some());
    let configured_auth = config.auth.unwrap();
    assert!(configured_auth.required);
    assert_eq!(configured_auth.api_keys.len(), 2);
    assert_eq!(configured_auth.jwt_secret, Some("secret-key".to_string()));
}

#[test]
fn test_server_config_with_rate_limiting() {
    let rate_limiting = RateLimitingConfig {
        requests_per_minute: 200,
        concurrent_executions_per_client: 20,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    let config = ServerConfig::default().rate_limiting(rate_limiting.clone());

    assert!(config.rate_limiting.is_some());
    let configured_rl = config.rate_limiting.unwrap();
    assert_eq!(configured_rl.requests_per_minute, 200);
    assert_eq!(configured_rl.concurrent_executions_per_client, 20);
    assert!(configured_rl.limit_by_ip);
    assert!(configured_rl.limit_by_api_key);
}

#[test]
fn test_server_config_chaining_multiple_settings() {
    let config = ServerConfig::default()
        .bind_address("0.0.0.0:8080")
        .enable_api(true)
        .max_concurrent_executions(150)
        .default_timeout(Duration::from_secs(900));

    assert_eq!(config.bind_address, "0.0.0.0:8080");
    assert!(config.enable_api);
    assert_eq!(config.max_concurrent_executions, 150);
    assert_eq!(config.default_timeout, Duration::from_secs(900));
}

// =============================================================================
// HealthCheckConfig Tests
// =============================================================================

#[test]
fn test_health_check_config_defaults() {
    let config = HealthCheckConfig::default();

    assert_eq!(config.interval, Duration::from_secs(30));
    assert!(config.check_runtime_engines);
    assert!(config.check_resources);
    assert_eq!(config.memory_threshold_percent, 90.0);
    assert_eq!(config.cpu_threshold_percent, 95.0);
}

#[test]
fn test_health_check_config_custom_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(60),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 80.0,
        cpu_threshold_percent: 85.0,
    };

    assert_eq!(config.interval, Duration::from_secs(60));
    assert_eq!(config.memory_threshold_percent, 80.0);
    assert_eq!(config.cpu_threshold_percent, 85.0);
}

#[test]
fn test_health_check_config_disabled_checks() {
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
fn test_health_check_config_aggressive_thresholds() {
    let config = HealthCheckConfig {
        interval: Duration::from_secs(10),
        check_runtime_engines: true,
        check_resources: true,
        memory_threshold_percent: 70.0,
        cpu_threshold_percent: 75.0,
    };

    assert_eq!(config.interval, Duration::from_secs(10));
    assert_eq!(config.memory_threshold_percent, 70.0);
    assert_eq!(config.cpu_threshold_percent, 75.0);
}

// =============================================================================
// LoggingConfig Tests
// =============================================================================

#[test]
fn test_logging_config_defaults() {
    let config = LoggingConfig::default();

    assert_eq!(config.level, "info");
    assert!(config.log_requests);
    assert!(config.log_executions);
    assert!(config.log_metrics);
}

#[test]
fn test_logging_config_custom_level() {
    let config = LoggingConfig {
        level: "debug".to_string(),
        log_requests: true,
        log_executions: true,
        log_metrics: true,
    };

    assert_eq!(config.level, "debug");
}

#[test]
fn test_logging_config_selective_logging() {
    let config = LoggingConfig {
        level: "warn".to_string(),
        log_requests: false,
        log_executions: true,
        log_metrics: false,
    };

    assert_eq!(config.level, "warn");
    assert!(!config.log_requests);
    assert!(config.log_executions);
    assert!(!config.log_metrics);
}

// =============================================================================
// RateLimitingConfig Tests
// =============================================================================

#[test]
fn test_rate_limiting_config_defaults() {
    let config = RateLimitingConfig::default();

    assert_eq!(config.requests_per_minute, 100);
    assert_eq!(config.concurrent_executions_per_client, 10);
    assert!(config.limit_by_ip);
    assert!(config.limit_by_api_key);
}

#[test]
fn test_rate_limiting_config_custom_limits() {
    let config = RateLimitingConfig {
        requests_per_minute: 500,
        concurrent_executions_per_client: 50,
        limit_by_ip: true,
        limit_by_api_key: true,
    };

    assert_eq!(config.requests_per_minute, 500);
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

// =============================================================================
// AuthenticationConfig Tests
// =============================================================================

#[test]
fn test_authentication_config_defaults() {
    let config = AuthenticationConfig::default();

    assert!(!config.required);
    assert!(config.api_keys.is_empty());
    assert!(config.jwt_secret.is_none());
    assert!(config.basic_auth.is_empty());
}

#[test]
fn test_authentication_config_with_api_keys() {
    let config = AuthenticationConfig {
        required: true,
        api_keys: vec!["key1".to_string(), "key2".to_string(), "key3".to_string()],
        jwt_secret: None,
        ..Default::default()
    };

    assert!(config.required);
    assert_eq!(config.api_keys.len(), 3);
    assert!(config.api_keys.contains(&"key1".to_string()));
}

#[test]
fn test_authentication_config_with_jwt() {
    let config = AuthenticationConfig {
        required: true,
        api_keys: vec![],
        jwt_secret: Some("my-secret-key-123".to_string()),
        ..Default::default()
    };

    assert!(config.required);
    assert_eq!(config.jwt_secret, Some("my-secret-key-123".to_string()));
}

#[test]
fn test_authentication_config_with_basic_auth() {
    let mut config = AuthenticationConfig::default();
    config
        .basic_auth
        .insert("admin".to_string(), "password123".to_string());
    config
        .basic_auth
        .insert("user".to_string(), "userpass".to_string());

    assert_eq!(config.basic_auth.len(), 2);
    assert_eq!(
        config.basic_auth.get("admin"),
        Some(&"password123".to_string())
    );
    assert_eq!(config.basic_auth.get("user"), Some(&"userpass".to_string()));
}

// =============================================================================
// Configuration Integration Tests
// =============================================================================

#[test]
fn test_full_production_config() {
    let auth = AuthenticationConfig {
        required: true,
        api_keys: vec!["prod-key-1".to_string()],
        jwt_secret: Some("production-secret".to_string()),
        ..Default::default()
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
        .default_timeout(Duration::from_secs(1800))
        .auth(auth)
        .rate_limiting(rate_limiting);

    // Verify production settings
    assert_eq!(config.bind_address, "0.0.0.0:443");
    assert_eq!(config.max_concurrent_executions, 500);
    assert_eq!(config.default_timeout, Duration::from_secs(1800));
    assert!(config.auth.is_some());
    assert!(config.rate_limiting.is_some());
}

#[test]
fn test_development_config() {
    let config = ServerConfig::default()
        .bind_address("127.0.0.1:8080")
        .enable_api(true)
        .max_concurrent_executions(10);

    // Verify development settings
    assert_eq!(config.bind_address, "127.0.0.1:8080");
    assert_eq!(config.max_concurrent_executions, 10);
    assert!(config.auth.is_none()); // No auth in dev
    assert!(config.rate_limiting.is_none()); // No rate limiting in dev
}

#[test]
fn test_minimal_config() {
    let config = ServerConfig::default()
        .bind_address("127.0.0.1:3000")
        .enable_api(true);

    assert_eq!(config.bind_address, "127.0.0.1:3000");
    assert!(config.enable_api);
}

#[test]
fn test_config_clone_and_modify() {
    let base_config = ServerConfig::default()
        .bind_address("127.0.0.1:8080")
        .max_concurrent_executions(100);

    let modified_config = base_config
        .clone()
        .bind_address("0.0.0.0:8080")
        .max_concurrent_executions(200);

    assert_eq!(modified_config.bind_address, "0.0.0.0:8080");
    assert_eq!(modified_config.max_concurrent_executions, 200);
}
