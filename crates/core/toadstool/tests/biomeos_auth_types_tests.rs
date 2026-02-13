//! Comprehensive tests for BiomeOS authentication types

use std::collections::HashMap;
use std::time::Duration;
use toadstool::biomeos_integration::{
    AuthManagerConfig, AuthenticationToken, TokenPropagationRequest, TokenVerificationResponse,
    TokenVerificationStatus,
};

// ============================================================================
// AuthManagerConfig Tests (5 tests)
// ============================================================================

#[test]
fn test_auth_manager_config_creation() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:6000".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        signing_key_seed: None,
    };

    assert_eq!(config.beardog_endpoint, "http://localhost:6000");
    assert!(config.signature_validation);
}

#[test]
fn test_auth_manager_config_short_refresh() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:6000".to_string(),
        token_refresh_interval: Duration::from_secs(60),
        signature_validation: true,
        timestamp_window: Duration::from_secs(30),
        replay_protection: true,
        signing_key_seed: None,
    };

    assert_eq!(config.token_refresh_interval, Duration::from_secs(60));
}

#[test]
fn test_auth_manager_config_no_replay_protection() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:6000".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: false,
        signing_key_seed: None,
    };

    assert!(!config.replay_protection);
}

#[test]
fn test_auth_manager_config_no_signature_validation() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:6000".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: false,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        signing_key_seed: None,
    };

    assert!(!config.signature_validation);
}

#[test]
fn test_auth_manager_config_serialization() {
    let config = AuthManagerConfig {
        beardog_endpoint: "http://localhost:6000".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        signing_key_seed: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
}

// ============================================================================
// AuthenticationToken Tests (5 tests)
// ============================================================================

#[test]
fn test_authentication_token_creation() {
    let token = AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "encrypted".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string()],
        scope: vec!["read".to_string()],
        claims: HashMap::new(),
    };

    assert_eq!(token.issuer, "beardog");
}

#[test]
fn test_authentication_token_multiple_audiences() {
    let token = AuthenticationToken {
        id: "token-456".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string(), "songbird".to_string()],
        scope: vec!["read".to_string(), "write".to_string()],
        claims: HashMap::new(),
    };

    assert_eq!(token.audience.len(), 2);
}

#[test]
fn test_authentication_token_with_claims() {
    let mut claims = HashMap::new();
    claims.insert("user_id".to_string(), serde_json::json!("user-123"));

    let token = AuthenticationToken {
        id: "token-789".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["all".to_string()],
        scope: vec!["full".to_string()],
        claims,
    };

    assert_eq!(token.claims.len(), 1);
}

#[test]
fn test_authentication_token_expiry() {
    let token = AuthenticationToken {
        id: "token-future".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string()],
        scope: vec!["read".to_string()],
        claims: HashMap::new(),
    };

    assert!(token.expires_at > chrono::Utc::now());
}

#[test]
fn test_authentication_token_serialization() {
    let token = AuthenticationToken {
        id: "token-001".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string()],
        scope: vec!["read".to_string()],
        claims: HashMap::new(),
    };

    let json = serde_json::to_string(&token).unwrap();
    assert!(!json.is_empty());
}

// ============================================================================
// TokenPropagationRequest Tests (5 tests)
// ============================================================================

#[test]
fn test_token_propagation_request_creation() {
    let token = AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string()],
        scope: vec!["read".to_string()],
        claims: HashMap::new(),
    };

    let request = TokenPropagationRequest {
        token,
        source_primal: "toadstool".to_string(),
        target_primal: "nestgate".to_string(),
        timestamp: chrono::Utc::now(),
        signature: "sig".to_string(),
    };

    assert_eq!(request.source_primal, "toadstool");
}

#[test]
fn test_token_propagation_request_different_targets() {
    let token = AuthenticationToken {
        id: "token-456".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["all".to_string()],
        scope: vec!["full".to_string()],
        claims: HashMap::new(),
    };

    let request = TokenPropagationRequest {
        token,
        source_primal: "songbird".to_string(),
        target_primal: "squirrel".to_string(),
        timestamp: chrono::Utc::now(),
        signature: "sig".to_string(),
    };

    assert_eq!(request.target_primal, "squirrel");
}

#[test]
fn test_token_propagation_request_with_signature() {
    let token = AuthenticationToken {
        id: "token-789".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["all".to_string()],
        scope: vec!["full".to_string()],
        claims: HashMap::new(),
    };

    let request = TokenPropagationRequest {
        token,
        source_primal: "a".to_string(),
        target_primal: "b".to_string(),
        timestamp: chrono::Utc::now(),
        signature: "ed25519-signature-value".to_string(),
    };

    assert!(request.signature.starts_with("ed25519"));
}

#[test]
fn test_token_propagation_request_timestamp() {
    let token = AuthenticationToken {
        id: "token-001".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["all".to_string()],
        scope: vec!["full".to_string()],
        claims: HashMap::new(),
    };

    let now = chrono::Utc::now();
    let request = TokenPropagationRequest {
        token,
        source_primal: "a".to_string(),
        target_primal: "b".to_string(),
        timestamp: now,
        signature: "sig".to_string(),
    };

    assert!(request.timestamp <= chrono::Utc::now());
}

#[test]
fn test_token_propagation_request_serialization() {
    let token = AuthenticationToken {
        id: "token-002".to_string(),
        token_type: "Bearer".to_string(),
        token: "value".to_string(),
        public_key: "key".to_string(),
        expires_at: chrono::Utc::now() + chrono::Duration::hours(1),
        issued_at: chrono::Utc::now(),
        issuer: "beardog".to_string(),
        audience: vec!["all".to_string()],
        scope: vec!["full".to_string()],
        claims: HashMap::new(),
    };

    let request = TokenPropagationRequest {
        token,
        source_primal: "source".to_string(),
        target_primal: "target".to_string(),
        timestamp: chrono::Utc::now(),
        signature: "sig".to_string(),
    };

    let json = serde_json::to_string(&request).unwrap();
    assert!(!json.is_empty());
}

// ============================================================================
// TokenVerificationStatus Tests (5 tests)
// ============================================================================

#[test]
fn test_token_verification_status_valid() {
    let status = TokenVerificationStatus::Valid;
    assert_eq!(status, TokenVerificationStatus::Valid);
}

#[test]
fn test_token_verification_status_expired() {
    let status = TokenVerificationStatus::Expired;
    assert_eq!(status, TokenVerificationStatus::Expired);
}

#[test]
fn test_token_verification_status_invalid() {
    let status = TokenVerificationStatus::Invalid;
    assert_eq!(status, TokenVerificationStatus::Invalid);
}

#[test]
fn test_token_verification_status_not_found() {
    let status = TokenVerificationStatus::NotFound;
    assert_eq!(status, TokenVerificationStatus::NotFound);
}

#[test]
fn test_token_verification_status_error() {
    let status = TokenVerificationStatus::Error("Connection failed".to_string());
    match status {
        TokenVerificationStatus::Error(msg) => assert_eq!(msg, "Connection failed"),
        _ => panic!("Expected Error status"),
    }
}

// ============================================================================
// TokenVerificationResponse Tests (3 tests)
// ============================================================================

#[test]
fn test_token_verification_response_valid() {
    let response = TokenVerificationResponse {
        status: TokenVerificationStatus::Valid,
        expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        details: None,
    };

    assert_eq!(response.status, TokenVerificationStatus::Valid);
}

#[test]
fn test_token_verification_response_invalid() {
    let response = TokenVerificationResponse {
        status: TokenVerificationStatus::Invalid,
        expires_at: None,
        details: Some("Signature mismatch".to_string()),
    };

    assert_eq!(response.status, TokenVerificationStatus::Invalid);
}

#[test]
fn test_token_verification_response_serialization() {
    let response = TokenVerificationResponse {
        status: TokenVerificationStatus::Valid,
        expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
        details: None,
    };

    let json = serde_json::to_string(&response).unwrap();
    assert!(!json.is_empty());
}
