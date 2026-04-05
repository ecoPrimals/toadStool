// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::doc_markdown,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! ServerConfig tests

use std::collections::HashMap;
use std::time::Duration;
use toadstool_server::{AuthenticationConfig, RateLimitingConfig, ServerConfig};

#[test]
fn test_server_config_default() {
    let config = ServerConfig::default();
    assert!(!config.bind_address.is_empty());
    assert!(config.enable_api);
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
