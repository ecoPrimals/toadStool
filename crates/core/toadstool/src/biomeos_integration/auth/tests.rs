// SPDX-License-Identifier: AGPL-3.0-or-later
//! Auth module tests - uses capability domains for issuer validation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use toadstool_common::constants::ecosystem::capabilities;
use toadstool_common::constants::primal_identity::audience;
use toadstool_common::constants::timeouts::{TIMESTAMP_VALIDATION_WINDOW, TOKEN_REFRESH_INTERVAL};

use super::*;
use crate::biomeos_integration::auth_backend::AuthBackendDispatch;
use crate::biomeos_integration::types::{CoordinationConfig, ToadStoolConfig};

fn test_config() -> AuthManagerConfig {
    AuthManagerConfig {
        security_endpoint: "http://localhost:9090".to_string(),
        token_refresh_interval: TOKEN_REFRESH_INTERVAL,
        signature_validation: true,
        timestamp_window: TIMESTAMP_VALIDATION_WINDOW,
        replay_protection: true,
        signing_key_seed: None,
        token_audience: vec![
            capabilities::COORDINATION.to_string(),
            capabilities::STORAGE.to_string(),
        ],
    }
}

fn test_config_with_signing_key() -> AuthManagerConfig {
    AuthManagerConfig {
        security_endpoint: "http://localhost:9090".to_string(),
        token_refresh_interval: TOKEN_REFRESH_INTERVAL,
        signature_validation: true,
        timestamp_window: TIMESTAMP_VALIDATION_WINDOW,
        replay_protection: true,
        signing_key_seed: Some("configured-but-unused-locally".to_string()),
        token_audience: vec![
            capabilities::COORDINATION.to_string(),
            capabilities::STORAGE.to_string(),
        ],
    }
}

fn sample_token() -> AuthenticationToken {
    let now = SystemTime::now();
    let expires = now + Duration::from_hours(1);
    AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "encrypted-value".to_string(),
        public_key: "pk-abc".to_string(),
        expires_at: expires,
        issued_at: now,
        issuer: capabilities::CRYPTO.to_string(),
        audience: vec![
            capabilities::COORDINATION.to_string(),
            audience::PLATFORM_AUDIENCE.to_string(),
        ],
        scope: vec!["cross-primal".to_string()],
        claims: HashMap::new(),
    }
}

#[test]
fn test_auth_manager_config_construction() {
    let config = test_config();
    assert_eq!(config.security_endpoint, "http://localhost:9090");
}

#[test]
fn test_authentication_token_construction() {
    let token = sample_token();
    assert_eq!(token.id, "token-123");
    assert_eq!(token.issuer, capabilities::CRYPTO);
}

#[test]
fn test_authentication_manager_new() {
    let config = test_config();
    let backend = crate::biomeos_integration::InMemoryAuthBackend::new();
    let _manager =
        AuthenticationManager::new(config, Arc::new(AuthBackendDispatch::InMemory(backend)));
}

#[tokio::test(flavor = "current_thread")]
async fn test_manager_with_inmemory_backend() {
    let config = test_config();
    let manager = AuthenticationManager::with_inmemory(config);
    let result = manager.get_current_token().await;
    assert!(result.is_ok());
    let token = result.unwrap();
    assert_eq!(token.issuer, capabilities::CRYPTO);
}

#[tokio::test(flavor = "current_thread")]
async fn test_sign_token_request_mock() {
    let config = test_config();
    let manager = AuthenticationManager::with_inmemory(config);
    let token = manager.get_current_token().await.expect("token");
    let signature = manager
        .sign_token_request(&token, capabilities::COORDINATION)
        .await;
    assert!(signature.is_ok());
    assert!(signature.unwrap().starts_with("ed25519:mock:"));
}

#[tokio::test(flavor = "current_thread")]
async fn test_get_public_key_from_backend() {
    let config = test_config();
    let manager = AuthenticationManager::with_inmemory(config);
    assert!(manager.get_public_key().await.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn test_get_public_key_delegates_to_backend() {
    let config = test_config_with_signing_key();
    let manager = AuthenticationManager::with_inmemory(config);
    let public_key = manager.get_public_key().await;
    assert!(public_key.is_some());
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

#[test]
fn primal_type_config_deserializes_legacy_songbird_manifest_key() {
    let json = r#"{"Songbird":{"enabled":true,"service_mesh":false,"config":{}}}"#;
    let parsed: PrimalTypeConfig = serde_json::from_str(json).expect("deserialize");
    assert!(matches!(parsed, PrimalTypeConfig::Coordination(ref c) if c.enabled));
}

#[test]
fn primal_type_config_serializes_canonical_coordination_tag() {
    let cfg = CoordinationConfig {
        enabled: false,
        service_mesh: true,
        port_range: None,
        load_balancing: None,
        health_checks: None,
        config: HashMap::new(),
    };
    let primal = PrimalTypeConfig::Coordination(cfg);
    let out = serde_json::to_string(&primal).expect("serialize");
    assert!(out.contains("\"Coordination\""));
    assert!(!out.contains("Songbird"));
}

// ── DEEP tests: signature bypass, expiry, public key, token audience ───

fn config_signature_validation_disabled() -> AuthManagerConfig {
    AuthManagerConfig {
        security_endpoint: "http://localhost:9090".to_string(),
        token_refresh_interval: TOKEN_REFRESH_INTERVAL,
        signature_validation: false,
        timestamp_window: TIMESTAMP_VALIDATION_WINDOW,
        replay_protection: true,
        signing_key_seed: None,
        token_audience: vec![capabilities::COORDINATION.to_string()],
    }
}

#[tokio::test(flavor = "current_thread")]
async fn test_sign_token_request_disabled_returns_signature_disabled() {
    let config = config_signature_validation_disabled();
    let manager = AuthenticationManager::with_inmemory(config);
    let token = manager.get_current_token().await.expect("token");
    let sig = manager
        .sign_token_request(&token, capabilities::STORAGE)
        .await
        .unwrap();
    assert_eq!(sig, "signature_disabled");
}

#[tokio::test(flavor = "current_thread")]
async fn test_sign_verification_request_disabled_returns_signature_disabled() {
    let config = config_signature_validation_disabled();
    let manager = AuthenticationManager::with_inmemory(config);
    let sig = manager
        .sign_verification_request(capabilities::COORDINATION)
        .await
        .unwrap();
    assert_eq!(sig, "signature_disabled");
}

#[tokio::test(flavor = "current_thread")]
async fn test_get_public_key_returns_backend_key() {
    let config = test_config();
    let manager = AuthenticationManager::with_inmemory(config);
    assert!(manager.get_public_key().await.is_some());
}

#[tokio::test(flavor = "current_thread")]
async fn test_sign_payload_delegates_to_backend() {
    let config = test_config();
    let manager = AuthenticationManager::with_inmemory(config);
    let token = manager.get_current_token().await.expect("token");
    let result = manager
        .sign_token_request(&token, capabilities::COORDINATION)
        .await;
    assert!(result.is_ok());
    assert!(result.unwrap().starts_with("ed25519:mock:"));
}

#[test]
fn test_default_token_audience_from_env() {
    temp_env::with_var("TOADSTOOL_AUTH_AUDIENCE", Some("primal-a,primal-b"), || {
        let config = AuthManagerConfig::default();
        assert_eq!(config.token_audience.len(), 2);
        assert!(config.token_audience.contains(&"primal-a".to_string()));
        assert!(config.token_audience.contains(&"primal-b".to_string()));
    });
}

#[test]
fn test_default_token_audience_self_and_platform_when_no_env() {
    temp_env::with_var("TOADSTOOL_AUTH_AUDIENCE", None::<&str>, || {
        let config = AuthManagerConfig::default();
        assert!(!config.token_audience.is_empty());
        assert!(
            config
                .token_audience
                .contains(&audience::PLATFORM_AUDIENCE.to_string())
        );
    });
}

#[tokio::test(flavor = "current_thread")]
async fn test_start_and_stop_token_refresh() {
    let mut config = test_config();
    config.token_refresh_interval = std::time::Duration::from_hours(1);
    let mut manager = AuthenticationManager::with_inmemory(config);
    let start_result = manager.start_token_refresh();
    assert!(start_result.is_ok());
    manager.stop_token_refresh();
}

#[test]
fn test_verification_result_construction() {
    let result = VerificationResult {
        total_primals: 2,
        valid_tokens: 1,
        results: HashMap::new(),
        verification_time: SystemTime::now(),
    };
    assert_eq!(result.total_primals, 2);
    assert_eq!(result.valid_tokens, 1);
}

#[test]
fn test_propagation_result_construction() {
    let result = PropagationResult {
        total_primals: 3,
        successful_propagations: 2,
        results: HashMap::new(),
        token_id: "tok-1".to_string(),
        propagation_time: SystemTime::now(),
    };
    assert_eq!(result.total_primals, 3);
    assert_eq!(result.successful_propagations, 2);
}

#[test]
fn test_token_propagation_status_variants() {
    use TokenPropagationStatus::*;
    let _ = Success;
    let _ = Failed("msg".into());
    let _ = Pending;
    let _ = Skipped("reason".into());
}

#[test]
fn test_token_verification_status_variants() {
    use super::tokens::TokenVerificationStatus;
    let _ = TokenVerificationStatus::Valid;
    let _ = TokenVerificationStatus::Expired;
    let _ = TokenVerificationStatus::Invalid;
    let _ = TokenVerificationStatus::NotFound;
    let _ = TokenVerificationStatus::Error("e".into());
}
