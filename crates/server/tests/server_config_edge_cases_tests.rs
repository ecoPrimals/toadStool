// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Edge case tests for server configuration

use std::collections::HashMap;
use std::time::Duration;
use toadstool_server::{AuthenticationConfig, RateLimitingConfig, ServerConfig};

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
    let config = ServerConfig::default().default_timeout(Duration::from_hours(24));
    assert_eq!(config.default_timeout, Duration::from_hours(24));
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
    let api_keys: Vec<String> = (0..100).map(|i| format!("key{i}")).collect();
    let config = AuthenticationConfig {
        required: true,
        api_keys,
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
fn test_server_config_all_features_disabled() {
    let config = ServerConfig::default().enable_api(false);
    assert!(!config.enable_api);
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
fn test_server_config_default_has_logging_and_health_check() {
    let config = ServerConfig::default();
    assert!(!config.logging.level.is_empty());
    assert!(config.health_check.interval > Duration::from_secs(0));
}
