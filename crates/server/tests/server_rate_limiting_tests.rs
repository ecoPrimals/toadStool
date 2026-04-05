// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::doc_markdown,
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! RateLimitingConfig tests

use toadstool_server::RateLimitingConfig;

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
