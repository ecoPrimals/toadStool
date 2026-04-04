// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for `biomeos_integration/auth` module
//!
//! This test suite covers:
//! - `TokenVerificationStatus` enum and variants
//! - `TokenPropagationStatus` enum and variants
//! - `AuthManagerConfig` struct
//! - `AuthenticationToken` struct
//! - `TokenPropagationRequest` struct
//! - `TokenVerificationRequest` struct
//! - `TokenVerificationResponse` struct
//! - `PropagationResult` struct

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool::biomeos_integration::auth::*;

// ============================================================================
// TokenVerificationStatus Tests
// ============================================================================

#[test]
fn test_token_verification_status_valid() {
    let status = TokenVerificationStatus::Valid;

    assert!(matches!(status, TokenVerificationStatus::Valid));
    assert!(format!("{status:?}").contains("Valid"));
}

#[test]
fn test_token_verification_status_expired() {
    let status = TokenVerificationStatus::Expired;

    assert!(matches!(status, TokenVerificationStatus::Expired));
}

#[test]
fn test_token_verification_status_invalid() {
    let status = TokenVerificationStatus::Invalid;

    assert!(matches!(status, TokenVerificationStatus::Invalid));
}

#[test]
fn test_token_verification_status_not_found() {
    let status = TokenVerificationStatus::NotFound;

    assert!(matches!(status, TokenVerificationStatus::NotFound));
}

#[test]
fn test_token_verification_status_error() {
    let status = TokenVerificationStatus::Error("Signature mismatch".to_string());

    match status {
        TokenVerificationStatus::Error(msg) => {
            assert_eq!(msg, "Signature mismatch");
        }
        _ => panic!("Expected Error status"),
    }
}

#[test]
fn test_token_verification_status_clone() {
    let status1 = TokenVerificationStatus::Valid;
    let status2 = status1.clone();

    assert_eq!(status1, status2);
}

#[test]
fn test_token_verification_status_eq() {
    assert_eq!(
        TokenVerificationStatus::Valid,
        TokenVerificationStatus::Valid
    );
    assert_ne!(
        TokenVerificationStatus::Valid,
        TokenVerificationStatus::Expired
    );

    let error1 = TokenVerificationStatus::Error("err1".to_string());
    let error2 = TokenVerificationStatus::Error("err1".to_string());
    assert_eq!(error1, error2);
}

#[test]
fn test_token_verification_status_serialization() {
    let statuses = vec![
        TokenVerificationStatus::Valid,
        TokenVerificationStatus::Expired,
        TokenVerificationStatus::Invalid,
        TokenVerificationStatus::NotFound,
        TokenVerificationStatus::Error("test".to_string()),
    ];

    for status in statuses {
        let json = serde_json::to_string(&status);
        assert!(json.is_ok());
    }
}

// ============================================================================
// TokenPropagationStatus Tests
// ============================================================================

#[test]
fn test_token_propagation_status_success() {
    let status = TokenPropagationStatus::Success;

    assert!(matches!(status, TokenPropagationStatus::Success));
}

#[test]
fn test_token_propagation_status_failed() {
    let status = TokenPropagationStatus::Failed("Network error".to_string());

    match status {
        TokenPropagationStatus::Failed(msg) => {
            assert_eq!(msg, "Network error");
        }
        _ => panic!("Expected Failed status"),
    }
}

#[test]
fn test_token_propagation_status_pending() {
    let status = TokenPropagationStatus::Pending;

    assert!(matches!(status, TokenPropagationStatus::Pending));
}

#[test]
fn test_token_propagation_status_skipped() {
    let status = TokenPropagationStatus::Skipped("Already propagated".to_string());

    match status {
        TokenPropagationStatus::Skipped(msg) => {
            assert_eq!(msg, "Already propagated");
        }
        _ => panic!("Expected Skipped status"),
    }
}

#[test]
fn test_token_propagation_status_clone() {
    let status1 = TokenPropagationStatus::Success;
    let status2 = status1.clone();

    assert_eq!(status1, status2);
}

#[test]
fn test_token_propagation_status_eq() {
    assert_eq!(
        TokenPropagationStatus::Success,
        TokenPropagationStatus::Success
    );
    assert_ne!(
        TokenPropagationStatus::Success,
        TokenPropagationStatus::Pending
    );
}

#[test]
fn test_token_propagation_status_serialization() {
    let statuses = vec![
        TokenPropagationStatus::Success,
        TokenPropagationStatus::Failed("error".to_string()),
        TokenPropagationStatus::Pending,
        TokenPropagationStatus::Skipped("reason".to_string()),
    ];

    for status in statuses {
        let json = serde_json::to_string(&status);
        assert!(json.is_ok());
    }
}

// ============================================================================
// AuthManagerConfig Tests
// ============================================================================

#[test]
fn test_auth_manager_config_creation() {
    let config = AuthManagerConfig {
        security_endpoint: "https://beardog.example.com".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        signing_key_seed: None,
        ..Default::default()
    };

    assert_eq!(config.security_endpoint, "https://beardog.example.com");
    assert_eq!(config.token_refresh_interval, Duration::from_secs(300));
    assert!(config.signature_validation);
    assert_eq!(config.timestamp_window, Duration::from_secs(60));
    assert!(config.replay_protection);
}

#[test]
fn test_auth_manager_config_no_validation() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(600),
        signature_validation: false,
        timestamp_window: Duration::from_secs(300),
        replay_protection: false,
        signing_key_seed: None,
        ..Default::default()
    };

    assert!(!config.signature_validation);
    assert!(!config.replay_protection);
}

#[test]
fn test_auth_manager_config_clone() {
    let config1 = AuthManagerConfig {
        security_endpoint: "https://beardog.example.com".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        signing_key_seed: None,
        ..Default::default()
    };
    let config2 = config1.clone();

    assert_eq!(config1.security_endpoint, config2.security_endpoint);
    assert_eq!(
        config1.token_refresh_interval,
        config2.token_refresh_interval
    );
}

#[test]
fn test_auth_manager_config_serialization() {
    let config = AuthManagerConfig {
        security_endpoint: "https://beardog.example.com".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        signing_key_seed: None,
        ..Default::default()
    };

    let json = serde_json::to_string(&config);
    assert!(json.is_ok());
}

// ============================================================================
// AuthenticationToken Tests
// ============================================================================

#[test]
fn test_authentication_token_creation() {
    let now = SystemTime::now();
    let mut claims = HashMap::new();
    claims.insert("user_id".to_string(), serde_json::json!("123"));

    let token = AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "encrypted-token-data".to_string(),
        public_key: "public-key-data".to_string(),
        expires_at: now + Duration::from_secs(3600),
        issued_at: now,
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string(), "songbird".to_string()],
        scope: vec!["read".to_string(), "write".to_string()],
        claims,
    };

    assert_eq!(token.id, "token-123");
    assert_eq!(token.token_type, "Bearer");
    assert_eq!(token.issuer, "beardog");
    assert_eq!(token.audience.len(), 2);
    assert_eq!(token.scope.len(), 2);
}

#[test]
fn test_authentication_token_ed25519() {
    let now = SystemTime::now();

    let token = AuthenticationToken {
        id: "token-456".to_string(),
        token_type: "Ed25519".to_string(),
        token: "ed25519-signature".to_string(),
        public_key: "ed25519-public-key".to_string(),
        expires_at: now + Duration::from_secs(86400),
        issued_at: now,
        issuer: "beardog".to_string(),
        audience: vec!["nestgate".to_string()],
        scope: vec!["storage".to_string()],
        claims: HashMap::new(),
    };

    assert_eq!(token.token_type, "Ed25519");
    assert_eq!(token.audience.len(), 1);
}

#[test]
fn test_authentication_token_clone() {
    let now = SystemTime::now();

    let token1 = AuthenticationToken {
        id: "token-789".to_string(),
        token_type: "Bearer".to_string(),
        token: "token-data".to_string(),
        public_key: "public-key".to_string(),
        expires_at: now + Duration::from_secs(3600),
        issued_at: now,
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string()],
        scope: vec!["read".to_string()],
        claims: HashMap::new(),
    };
    let token2 = token1.clone();

    assert_eq!(token1.id, token2.id);
    assert_eq!(token1.token_type, token2.token_type);
}

#[test]
fn test_authentication_token_serialization() {
    let now = SystemTime::now();

    let token = AuthenticationToken {
        id: "token-serialize".to_string(),
        token_type: "Bearer".to_string(),
        token: "data".to_string(),
        public_key: "key".to_string(),
        expires_at: now + Duration::from_secs(3600),
        issued_at: now,
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string()],
        scope: vec!["read".to_string()],
        claims: HashMap::new(),
    };

    let json = serde_json::to_string(&token);
    assert!(json.is_ok());
}

// ============================================================================
// TokenPropagationRequest Tests
// ============================================================================

#[test]
fn test_token_propagation_request_creation() {
    let now = SystemTime::now();

    let token = AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "data".to_string(),
        public_key: "key".to_string(),
        expires_at: now + Duration::from_secs(3600),
        issued_at: now,
        issuer: "beardog".to_string(),
        audience: vec!["songbird".to_string()],
        scope: vec!["propagate".to_string()],
        claims: HashMap::new(),
    };

    let request = TokenPropagationRequest {
        token,
        source_primal: "toadstool".to_string(),
        target_primal: "songbird".to_string(),
        timestamp: now,
        signature: "signature-data".to_string(),
    };

    assert_eq!(request.source_primal, "toadstool");
    assert_eq!(request.target_primal, "songbird");
}

#[test]
fn test_token_propagation_request_clone() {
    let now = SystemTime::now();

    let token = AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "data".to_string(),
        public_key: "key".to_string(),
        expires_at: now + Duration::from_secs(3600),
        issued_at: now,
        issuer: "beardog".to_string(),
        audience: vec![],
        scope: vec![],
        claims: HashMap::new(),
    };

    let request1 = TokenPropagationRequest {
        token,
        source_primal: "toadstool".to_string(),
        target_primal: "songbird".to_string(),
        timestamp: now,
        signature: "sig".to_string(),
    };
    let request2 = request1.clone();

    assert_eq!(request1.source_primal, request2.source_primal);
}

#[test]
fn test_token_propagation_request_serialization() {
    let now = SystemTime::now();

    let token = AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "data".to_string(),
        public_key: "key".to_string(),
        expires_at: now + Duration::from_secs(3600),
        issued_at: now,
        issuer: "beardog".to_string(),
        audience: vec![],
        scope: vec![],
        claims: HashMap::new(),
    };

    let request = TokenPropagationRequest {
        token,
        source_primal: "toadstool".to_string(),
        target_primal: "songbird".to_string(),
        timestamp: now,
        signature: "sig".to_string(),
    };

    let json = serde_json::to_string(&request);
    assert!(json.is_ok());
}

// ============================================================================
// TokenVerificationRequest Tests
// ============================================================================

#[test]
fn test_token_verification_request_creation() {
    let now = SystemTime::now();

    let request = TokenVerificationRequest {
        primal_name: "toadstool".to_string(),
        timestamp: now,
        signature: "verification-signature".to_string(),
    };

    assert_eq!(request.primal_name, "toadstool");
}

#[test]
fn test_token_verification_request_clone() {
    let now = SystemTime::now();

    let request1 = TokenVerificationRequest {
        primal_name: "songbird".to_string(),
        timestamp: now,
        signature: "sig".to_string(),
    };
    let request2 = request1.clone();

    assert_eq!(request1.primal_name, request2.primal_name);
}

#[test]
fn test_token_verification_request_serialization() {
    let now = SystemTime::now();

    let request = TokenVerificationRequest {
        primal_name: "nestgate".to_string(),
        timestamp: now,
        signature: "sig".to_string(),
    };

    let json = serde_json::to_string(&request);
    assert!(json.is_ok());
}

// ============================================================================
// TokenVerificationResponse Tests
// ============================================================================

#[test]
fn test_token_verification_response_valid() {
    let now = SystemTime::now();

    let response = TokenVerificationResponse {
        status: TokenVerificationStatus::Valid,
        expires_at: Some(now + Duration::from_secs(3600)),
        details: Some("Token is valid and active".to_string()),
    };

    assert!(matches!(response.status, TokenVerificationStatus::Valid));
    assert!(response.expires_at.is_some());
    assert!(response.details.is_some());
}

#[test]
fn test_token_verification_response_expired() {
    let response = TokenVerificationResponse {
        status: TokenVerificationStatus::Expired,
        expires_at: None,
        details: Some("Token has expired".to_string()),
    };

    assert!(matches!(response.status, TokenVerificationStatus::Expired));
    assert!(response.expires_at.is_none());
}

#[test]
fn test_token_verification_response_clone() {
    let now = SystemTime::now();

    let response1 = TokenVerificationResponse {
        status: TokenVerificationStatus::Valid,
        expires_at: Some(now),
        details: None,
    };
    let response2 = response1.clone();

    assert_eq!(response1.status, response2.status);
}

#[test]
fn test_token_verification_response_serialization() {
    let response = TokenVerificationResponse {
        status: TokenVerificationStatus::Valid,
        expires_at: None,
        details: Some("Test".to_string()),
    };

    let json = serde_json::to_string(&response);
    assert!(json.is_ok());
}

// ============================================================================
// PropagationResult Tests
// ============================================================================

#[test]
fn test_propagation_result_creation() {
    let now = SystemTime::now();
    let mut results = HashMap::new();
    results.insert("songbird".to_string(), TokenPropagationStatus::Success);
    results.insert("nestgate".to_string(), TokenPropagationStatus::Success);

    let result = PropagationResult {
        total_primals: 2,
        successful_propagations: 2,
        results,
        token_id: "token-123".to_string(),
        propagation_time: now,
    };

    assert_eq!(result.total_primals, 2);
    assert_eq!(result.successful_propagations, 2);
    assert_eq!(result.token_id, "token-123");
}

#[test]
fn test_propagation_result_partial_success() {
    let now = SystemTime::now();
    let mut results = HashMap::new();
    results.insert("songbird".to_string(), TokenPropagationStatus::Success);
    results.insert(
        "nestgate".to_string(),
        TokenPropagationStatus::Failed("Network error".to_string()),
    );

    let result = PropagationResult {
        total_primals: 2,
        successful_propagations: 1,
        results,
        token_id: "token-456".to_string(),
        propagation_time: now,
    };

    assert_eq!(result.total_primals, 2);
    assert_eq!(result.successful_propagations, 1);
}

#[test]
fn test_propagation_result_clone() {
    let now = SystemTime::now();
    let mut status_map = HashMap::new();
    status_map.insert("songbird".to_string(), TokenPropagationStatus::Success);

    let original = PropagationResult {
        total_primals: 1,
        successful_propagations: 1,
        results: status_map,
        token_id: "token-789".to_string(),
        propagation_time: now,
    };
    let cloned = original.clone();

    assert_eq!(original.token_id, cloned.token_id);
    assert_eq!(original.total_primals, cloned.total_primals);
}

#[test]
fn test_propagation_result_serialization() {
    let now = SystemTime::now();
    let mut status_map = HashMap::new();
    status_map.insert("songbird".to_string(), TokenPropagationStatus::Success);

    let propagation_result = PropagationResult {
        total_primals: 1,
        successful_propagations: 1,
        results: status_map,
        token_id: "token-serialize".to_string(),
        propagation_time: now,
    };

    let json = serde_json::to_string(&propagation_result);
    assert!(json.is_ok());
}

// ============================================================================
// AuthenticationManager Tests
// ============================================================================

#[test]
fn test_authentication_manager_creation() {
    let config = AuthManagerConfig {
        security_endpoint: "https://beardog.example.com".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        signing_key_seed: None,
        ..Default::default()
    };

    let _manager = AuthenticationManager::with_inmemory(config);

    // Manager created successfully
}

#[test]
fn test_authentication_manager_no_validation() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(600),
        signature_validation: false,
        timestamp_window: Duration::from_secs(300),
        replay_protection: false,
        signing_key_seed: None,
        ..Default::default()
    };

    let _manager = AuthenticationManager::with_inmemory(config);
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_biomeos_auth_coverage_summary() {
    println!("=== BiomeOS Auth Test Coverage ===");
    println!("TokenVerificationStatus:     8 tests");
    println!("TokenPropagationStatus:      7 tests");
    println!("AuthManagerConfig:           4 tests");
    println!("AuthenticationToken:         4 tests");
    println!("TokenPropagationRequest:     3 tests");
    println!("TokenVerificationRequest:    3 tests");
    println!("TokenVerificationResponse:   4 tests");
    println!("PropagationResult:           4 tests");
    println!("AuthenticationManager:       2 tests");
    println!("Total:                       39 tests");
    println!("===================================");
}
