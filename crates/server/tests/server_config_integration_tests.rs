// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! Integration tests for server configuration

use std::collections::HashMap;
use std::time::Duration;
use toadstool_server::{AuthenticationConfig, RateLimitingConfig, ServerConfig};

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
        .max_concurrent_executions(10);
    assert!(!config.enable_api);
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
