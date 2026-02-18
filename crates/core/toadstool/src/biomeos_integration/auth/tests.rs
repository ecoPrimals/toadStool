use std::collections::HashMap;
use std::sync::Arc;

use toadstool_common::constants::timeouts::{TIMESTAMP_VALIDATION_WINDOW, TOKEN_REFRESH_INTERVAL};

use super::*;
use crate::biomeos_integration::types::ToadStoolConfig;

fn test_config() -> AuthManagerConfig {
    AuthManagerConfig {
        beardog_endpoint: "http://localhost:9090".to_string(),
        token_refresh_interval: TOKEN_REFRESH_INTERVAL,
        signature_validation: true,
        timestamp_window: TIMESTAMP_VALIDATION_WINDOW,
        replay_protection: true,
        signing_key_seed: None,
    }
}

fn test_config_with_signing_key() -> AuthManagerConfig {
    use base64::{engine::general_purpose, Engine as _};
    let seed = [0u8; 32];
    AuthManagerConfig {
        beardog_endpoint: "http://localhost:9090".to_string(),
        token_refresh_interval: TOKEN_REFRESH_INTERVAL,
        signature_validation: true,
        timestamp_window: TIMESTAMP_VALIDATION_WINDOW,
        replay_protection: true,
        signing_key_seed: Some(general_purpose::STANDARD.encode(seed)),
    }
}

fn sample_token() -> AuthenticationToken {
    let now = chrono::Utc::now();
    let expires = now + chrono::Duration::hours(1);
    AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "encrypted-value".to_string(),
        public_key: "pk-abc".to_string(),
        expires_at: expires,
        issued_at: now,
        issuer: "beardog".to_string(),
        audience: vec!["songbird".to_string(), "biomeos".to_string()],
        scope: vec!["cross-primal".to_string()],
        claims: HashMap::new(),
    }
}

#[test]
fn test_auth_manager_config_construction() {
    let config = test_config();
    assert_eq!(config.beardog_endpoint, "http://localhost:9090");
}

#[test]
fn test_authentication_token_construction() {
    let token = sample_token();
    assert_eq!(token.id, "token-123");
    assert_eq!(token.issuer, "beardog");
}

#[test]
fn test_authentication_manager_new() {
    let config = test_config();
    let backend = crate::biomeos_integration::InMemoryAuthBackend::new();
    let _manager = AuthenticationManager::new(config, Arc::new(backend));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_manager_with_inmemory_backend() {
    let config = test_config();
    let manager = AuthenticationManager::with_inmemory(config);
    let result = manager.get_current_token().await;
    assert!(result.is_ok());
    let token = result.unwrap();
    assert_eq!(token.issuer, "beardog");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_sign_token_request_mock() {
    let config = test_config();
    let manager = AuthenticationManager::with_inmemory(config);
    let token = manager.get_current_token().await.expect("token");
    let signature = manager.sign_token_request(&token, "songbird").await;
    assert!(signature.is_ok());
    assert!(signature.unwrap().starts_with("ed25519:mock:"));
}

#[test]
fn test_get_public_key_none_when_no_signing_key() {
    let config = test_config();
    let manager = AuthenticationManager::with_inmemory(config);
    assert!(manager.get_public_key().is_none());
}

#[test]
fn test_get_public_key_with_signing_key() {
    let config = test_config_with_signing_key();
    let manager = AuthenticationManager::with_inmemory(config);
    let public_key = manager.get_public_key();
    assert!(public_key.is_some());
    use base64::{engine::general_purpose, Engine as _};
    let pk_bytes = general_purpose::STANDARD
        .decode(public_key.unwrap())
        .expect("Valid base64");
    assert_eq!(pk_bytes.len(), 32);
}

#[test]
fn test_primal_type_config_toadstool_variant() {
    let toad_config = ToadStoolConfig {
        enabled: true,
        orchestrator: true,
        resources: None,
        runtime_engines: vec!["wgpu".to_string()],
        execution_environments: vec!["container".to_string()],
        substrates: vec!["linux".to_string()],
        config: HashMap::new(),
    };
    let primal = PrimalTypeConfig::ToadStool(toad_config);
    assert!(matches!(primal, PrimalTypeConfig::ToadStool(c) if c.enabled));
}
