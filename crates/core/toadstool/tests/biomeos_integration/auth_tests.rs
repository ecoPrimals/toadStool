// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for BiomeOS authentication integration types

use std::collections::HashMap;
use std::time::{Duration, SystemTime};
use toadstool::biomeos_integration::*;

// ============================================================================
// AuthManagerConfig Tests
// ============================================================================

#[test]
fn test_auth_manager_config_creation() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        ..Default::default()
    };
    
    assert_eq!(config.security_endpoint, "http://localhost:8080");
    assert_eq!(config.token_refresh_interval, Duration::from_secs(300));
    assert!(config.signature_validation);
    assert_eq!(config.timestamp_window, Duration::from_secs(60));
    assert!(config.replay_protection);
}

#[test]
fn test_auth_manager_config_clone() {
    let config1 = AuthManagerConfig {
        security_endpoint: "http://beardog:8080".to_string(),
        token_refresh_interval: Duration::from_secs(600),
        signature_validation: false,
        timestamp_window: Duration::from_secs(120),
        replay_protection: false,
        ..Default::default()
    };
    
    let config2 = config1.clone();
    
    assert_eq!(config1.security_endpoint, config2.security_endpoint);
    assert_eq!(config1.token_refresh_interval, config2.token_refresh_interval);
}

#[test]
fn test_auth_manager_config_serialization() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        ..Default::default()
    };
    
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("security_endpoint"));
}

// ============================================================================
// TokenVerificationStatus Tests (5 variants)
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
    let status = TokenVerificationStatus::Error("Signature mismatch".to_string());
    
    match status {
        TokenVerificationStatus::Error(msg) => {
            assert_eq!(msg, "Signature mismatch");
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_token_verification_status_clone() {
    let status1 = TokenVerificationStatus::Valid;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// TokenPropagationStatus Tests (4 variants)
// ============================================================================

#[test]
fn test_token_propagation_status_success() {
    let status = TokenPropagationStatus::Success;
    assert_eq!(status, TokenPropagationStatus::Success);
}

#[test]
fn test_token_propagation_status_failed() {
    let status = TokenPropagationStatus::Failed("Network timeout".to_string());
    
    match status {
        TokenPropagationStatus::Failed(msg) => {
            assert_eq!(msg, "Network timeout");
        }
        _ => panic!("Expected Failed variant"),
    }
}

#[test]
fn test_token_propagation_status_pending() {
    let status = TokenPropagationStatus::Pending;
    assert_eq!(status, TokenPropagationStatus::Pending);
}

#[test]
fn test_token_propagation_status_skipped() {
    let status = TokenPropagationStatus::Skipped("Primal offline".to_string());
    
    match status {
        TokenPropagationStatus::Skipped(reason) => {
            assert_eq!(reason, "Primal offline");
        }
        _ => panic!("Expected Skipped variant"),
    }
}

#[test]
fn test_token_propagation_status_clone() {
    let status1 = TokenPropagationStatus::Success;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

// ============================================================================
// AuthenticationToken Tests
// ============================================================================

#[test]
fn test_authentication_token_creation() {
    let token = AuthenticationToken {
        id: "token-123".to_string(),
        token_type: "Bearer".to_string(),
        token: "encrypted_token_value".to_string(),
        public_key: "ed25519_public_key".to_string(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        issued_at: SystemTime::now(),
        issuer: "beardog".to_string(),
        audience: vec!["toadstool".to_string(), "nestgate".to_string()],
        scope: vec!["read".to_string(), "write".to_string()],
        claims: std::collections::HashMap::new(),
    };
    
    assert_eq!(token.id, "token-123");
    assert_eq!(token.token_type, "Bearer");
    assert_eq!(token.issuer, "beardog");
    assert_eq!(token.audience.len(), 2);
    assert_eq!(token.scope.len(), 2);
}

#[test]
fn test_authentication_token_clone() {
    let token1 = AuthenticationToken {
        id: "token-456".to_string(),
        token_type: "Bearer".to_string(),
        token: "encrypted_token".to_string(),
        public_key: "public_key".to_string(),
        expires_at: SystemTime::now() + Duration::from_secs(3600),
        issued_at: SystemTime::now(),
        issuer: "beardog".to_string(),
        audience: vec!["squirrel".to_string()],
        scope: vec!["read".to_string()],
        claims: std::collections::HashMap::new(),
    };
    
    let token2 = token1.clone();
    
    assert_eq!(token1.id, token2.id);
    assert_eq!(token1.issuer, token2.issuer);
}

// ============================================================================
// PropagationResult Tests
// ============================================================================

#[test]
fn test_propagation_result_creation() {
    let mut results = std::collections::HashMap::new();
    results.insert("toadstool".to_string(), TokenPropagationStatus::Success);
    results.insert("nestgate".to_string(), TokenPropagationStatus::Success);
    results.insert("squirrel".to_string(), TokenPropagationStatus::Failed("timeout".to_string()));
    
    let propagation = PropagationResult {
        total_primals: 3,
        successful_propagations: 2,
        results,
        token_id: "token-789".to_string(),
        propagation_time: SystemTime::now(),
    };
    
    assert_eq!(propagation.total_primals, 3);
    assert_eq!(propagation.successful_propagations, 2);
    assert_eq!(propagation.results.len(), 3);
}

#[test]
fn test_propagation_result_all_success() {
    let mut results = std::collections::HashMap::new();
    results.insert("toadstool".to_string(), TokenPropagationStatus::Success);
    results.insert("nestgate".to_string(), TokenPropagationStatus::Success);
    results.insert("squirrel".to_string(), TokenPropagationStatus::Success);
    results.insert("songbird".to_string(), TokenPropagationStatus::Success);
    results.insert("biomeos".to_string(), TokenPropagationStatus::Success);
    
    let propagation = PropagationResult {
        total_primals: 5,
        successful_propagations: 5,
        results,
        token_id: "token-all-success".to_string(),
        propagation_time: SystemTime::now(),
    };
    
    assert_eq!(propagation.total_primals, 5);
    assert_eq!(propagation.successful_propagations, 5);
}

// ============================================================================
// VerificationResult Tests
// ============================================================================

#[test]
fn test_verification_result_creation() {
    let mut results = std::collections::HashMap::new();
    results.insert("toadstool".to_string(), TokenVerificationStatus::Valid);
    results.insert("nestgate".to_string(), TokenVerificationStatus::Valid);
    results.insert("squirrel".to_string(), TokenVerificationStatus::Expired);
    
    let verification = VerificationResult {
        total_primals: 3,
        valid_tokens: 2,
        results,
        verification_time: SystemTime::now(),
    };
    
    assert_eq!(verification.total_primals, 3);
    assert_eq!(verification.valid_tokens, 2);
    assert_eq!(verification.results.len(), 3);
}

#[test]
fn test_verification_result_all_valid() {
    let mut results = std::collections::HashMap::new();
    results.insert("toadstool".to_string(), TokenVerificationStatus::Valid);
    results.insert("nestgate".to_string(), TokenVerificationStatus::Valid);
    results.insert("squirrel".to_string(), TokenVerificationStatus::Valid);
    
    let verification = VerificationResult {
        total_primals: 3,
        valid_tokens: 3,
        results,
        verification_time: SystemTime::now(),
    };
    
    assert_eq!(verification.total_primals, 3);
    assert_eq!(verification.valid_tokens, 3);
}

// ============================================================================
// AuthenticationManager Tests
// ============================================================================

#[test]
fn test_authentication_manager_creation() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(60),
        replay_protection: true,
        ..Default::default()
    };
    
    let _manager = AuthenticationManager::with_inmemory(config);
    // Creation should succeed
}

#[test]
fn test_authentication_manager_without_validation() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(600),
        signature_validation: false,
        timestamp_window: Duration::from_secs(120),
        replay_protection: false,
        ..Default::default()
    };
    
    let _manager = AuthenticationManager::with_inmemory(config);
    // Creation should succeed
}

// ============================================================================
// Security Configuration Tests
// ============================================================================

#[test]
fn test_auth_config_short_refresh_interval() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(60),
        signature_validation: true,
        timestamp_window: Duration::from_secs(30),
        replay_protection: true,
        ..Default::default()
    };
    
    assert_eq!(config.token_refresh_interval, Duration::from_secs(60));
}

#[test]
fn test_auth_config_long_refresh_interval() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(3600),
        signature_validation: true,
        timestamp_window: Duration::from_secs(300),
        replay_protection: true,
        ..Default::default()
    };
    
    assert_eq!(config.token_refresh_interval, Duration::from_secs(3600));
}

#[test]
fn test_auth_config_wide_timestamp_window() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(300),
        replay_protection: true,
        ..Default::default()
    };
    
    assert_eq!(config.timestamp_window, Duration::from_secs(300));
}

#[test]
fn test_auth_config_narrow_timestamp_window() {
    let config = AuthManagerConfig {
        security_endpoint: "http://localhost:8080".to_string(),
        token_refresh_interval: Duration::from_secs(300),
        signature_validation: true,
        timestamp_window: Duration::from_secs(10),
        replay_protection: true,
        ..Default::default()
    };
    
    assert_eq!(config.timestamp_window, Duration::from_secs(10));
}

