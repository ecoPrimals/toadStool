// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests extracted from auth_backend.rs (S336).

use std::collections::HashMap;
use std::time::{Duration, SystemTime};

use toadstool_common::constants::ecosystem::capabilities;
use toadstool_common::constants::primal_identity::PRIMAL_NAME;

use super::auth_backend::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_auth_backend_request() {
    let backend = InMemoryAuthBackend::new();
    let request = TokenRequest {
        requesting_primal: "toadstool".to_string(),
        scope: vec!["cross-primal".to_string()],
        audience: vec![capabilities::COORDINATION.to_string()],
        timestamp: SystemTime::now(),
    };

    let result = backend.request_token(&request).await;
    assert!(result.is_ok());

    let token = result.unwrap();
    assert_eq!(token.issuer, capabilities::CRYPTO);
    assert!(token.expires_at > SystemTime::now());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_auth_backend_refresh() {
    let backend = InMemoryAuthBackend::new();
    let request = TokenRefreshRequest {
        requesting_primal: "toadstool".to_string(),
        timestamp: SystemTime::now(),
    };

    let result = backend.refresh_token(&request).await;
    assert!(result.is_ok());

    let token = result.unwrap();
    assert!(token.id.contains("refreshed"));
    assert_eq!(token.issuer, capabilities::CRYPTO);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_token_validation() {
    use toadstool_common::interned_strings::capabilities;
    let backend = InMemoryAuthBackend::new();
    let request = TokenRequest {
        requesting_primal: "toadstool".to_string(),
        scope: vec!["cross-primal".to_string()],
        audience: vec![capabilities::COORDINATION.to_string()],
        timestamp: SystemTime::now(),
    };

    let token = backend.request_token(&request).await.unwrap();
    let validation_result = backend.validate_token(&token);
    assert!(validation_result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_expired_token_validation() {
    let backend = InMemoryAuthBackend::new();
    let mut token = AuthenticationToken {
        id: "test-expired".to_string(),
        token_type: "Bearer".to_string(),
        token: "test-value".to_string(),
        public_key: "test-key".to_string(),
        expires_at: SystemTime::now() - Duration::from_secs(3600), // Expired!
        issued_at: SystemTime::now() - Duration::from_secs(7200),
        issuer: capabilities::CRYPTO.to_string(),
        audience: vec![PRIMAL_NAME.to_string()],
        scope: vec!["test".to_string()],
        claims: HashMap::new(),
    };

    let result = backend.validate_token(&token);
    assert!(result.is_err());

    // Fix expiration
    token.expires_at = SystemTime::now() + Duration::from_secs(3600);
    let result = backend.validate_token(&token);
    assert!(result.is_ok());
}
