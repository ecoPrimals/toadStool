// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for `ToadStoolClient`

use super::config::{AuthConfig, ClientConfig};
use std::collections::HashMap;
use std::time::Duration;

// Helper to create test client without connecting
fn create_test_config() -> ClientConfig {
    ClientConfig {
        base_url: "http://localhost:3000".to_string(),
        request_timeout: Duration::from_secs(10),
        retry_backoff: Duration::from_millis(100),
        max_retries: 3,
        auth: None,
        custom_headers: HashMap::new(),
    }
}

#[test]
fn test_client_config_creation() {
    let config = create_test_config();
    assert_eq!(config.base_url, "http://localhost:3000");
    assert_eq!(config.request_timeout, Duration::from_secs(10));
    assert_eq!(config.max_retries, 3);
}

#[test]
fn test_client_config_with_api_key_auth() {
    let auth = AuthConfig::ApiKey {
        key: "test-key-123".to_string(),
        header_name: "X-API-Key".to_string(),
    };
    let config = ClientConfig {
        auth: Some(auth),
        ..create_test_config()
    };

    assert!(config.auth.is_some());
}

#[test]
fn test_client_config_with_bearer_token() {
    let auth = AuthConfig::BearerToken {
        token: "bearer-token-abc".to_string(),
    };
    let config = ClientConfig {
        auth: Some(auth),
        ..create_test_config()
    };

    assert!(config.auth.is_some());
}

#[test]
fn test_client_config_with_basic_auth() {
    let auth = AuthConfig::Basic {
        username: "user".to_string(),
        password: "pass".to_string(),
    };
    let config = ClientConfig {
        auth: Some(auth),
        ..create_test_config()
    };

    assert!(config.auth.is_some());
}

#[test]
fn test_client_config_with_custom_headers() {
    let mut custom_headers = HashMap::new();
    custom_headers.insert("X-Custom-Header".to_string(), "custom-value".to_string());

    let config = ClientConfig {
        custom_headers,
        ..create_test_config()
    };

    assert_eq!(config.custom_headers.len(), 1);
}

#[test]
fn test_client_config_api_url() {
    let config = create_test_config();
    let api_url = config.api_url("health");
    assert_eq!(api_url, "http://localhost:3000/api/v1/health");
}

#[test]
fn test_client_config_api_url_with_path() {
    let config = create_test_config();
    let api_url = config.api_url("executions/123");
    assert_eq!(api_url, "http://localhost:3000/api/v1/executions/123");
}

// Note: Integration tests that actually connect to a server
// should be in tests/ directory with #[tokio::test]
