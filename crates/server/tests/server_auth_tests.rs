// SPDX-License-Identifier: AGPL-3.0-only
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding
)]
//! AuthenticationConfig tests

use std::collections::HashMap;
use toadstool_server::AuthenticationConfig;

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
