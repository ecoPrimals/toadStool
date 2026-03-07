// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for SecurityClient (beardog_integration/client_evolved.rs) - coverage target 90%
//!
//! Tests encrypt, decrypt, sign, verify, validate_token, generate_key, delete_key,
//! list_keys, is_available, provider_info, rediscover. No live security provider.

#![allow(deprecated)] // Testing client_evolved for coverage

use toadstool_distributed::beardog_integration::client_evolved::{
    DecryptionRequest, DecryptionResponse, EncryptionRequest, EncryptionResponse, SecurityClient,
    SecurityClientError, SignatureRequest, SignatureResponse, TokenValidationRequest,
    TokenValidationResponse, VerificationRequest, VerificationResponse,
};

// ============================================================================
// SecurityClient construction
// ============================================================================

#[test]
fn test_security_client_new() {
    let client = SecurityClient::new();
    let _ = client;
}

#[test]
fn test_security_client_default() {
    let client = SecurityClient::default();
    let _ = client;
}

// ============================================================================
// Error variants
// ============================================================================

#[test]
fn test_security_client_error_no_provider() {
    let err = SecurityClientError::NoProvider;
    assert!(err.to_string().contains("No security provider"));
}

#[test]
fn test_security_client_error_encryption_failed() {
    let err = SecurityClientError::EncryptionFailed("test".into());
    assert!(err.to_string().contains("test"));
}

#[test]
fn test_security_client_error_decryption_failed() {
    let err = SecurityClientError::DecryptionFailed("err".into());
    assert!(err.to_string().contains("err"));
}

#[test]
fn test_security_client_error_signature_failed() {
    let err = SecurityClientError::SignatureFailed("sig".into());
    assert!(err.to_string().contains("sig"));
}

#[test]
fn test_security_client_error_verification_failed() {
    let err = SecurityClientError::VerificationFailed("ver".into());
    assert!(err.to_string().contains("ver"));
}

#[test]
fn test_security_client_error_key_management_failed() {
    let err = SecurityClientError::KeyManagementFailed("key".into());
    assert!(err.to_string().contains("key"));
}

#[test]
fn test_security_client_error_validation_failed() {
    let err = SecurityClientError::ValidationFailed("val".into());
    assert!(err.to_string().contains("val"));
}

// ============================================================================
// Request/Response structs - serde
// ============================================================================

#[test]
fn test_encryption_request_serde() {
    let req = EncryptionRequest {
        data: vec![1, 2, 3],
        algorithm: "AES-256".to_string(),
        key_id: Some("k1".to_string()),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: EncryptionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.algorithm, req.algorithm);
}

#[test]
fn test_encryption_response_serde() {
    let resp = EncryptionResponse {
        encrypted_data: vec![0xaa, 0xbb],
        key_id: "k1".to_string(),
        algorithm: "AES".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: EncryptionResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.key_id, resp.key_id);
}

#[test]
fn test_decryption_request_serde() {
    let req = DecryptionRequest {
        encrypted_data: vec![1, 2, 3],
        key_id: "key".to_string(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: DecryptionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.key_id, req.key_id);
}

#[test]
fn test_decryption_response_serde() {
    let resp = DecryptionResponse {
        data: vec![1, 2, 3],
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: DecryptionResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.data, resp.data);
}

#[test]
fn test_signature_request_serde() {
    let req = SignatureRequest {
        data: vec![1, 2, 3],
        algorithm: "ECDSA".to_string(),
        key_id: None,
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: SignatureRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.algorithm, req.algorithm);
}

#[test]
fn test_signature_response_serde() {
    let resp = SignatureResponse {
        signature: vec![0xde, 0xad],
        key_id: "k1".to_string(),
        algorithm: "ECDSA".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SignatureResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.key_id, resp.key_id);
}

#[test]
fn test_verification_request_serde() {
    let req = VerificationRequest {
        data: vec![1],
        signature: vec![2],
        key_id: "k".to_string(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: VerificationRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.key_id, req.key_id);
}

#[test]
fn test_verification_response_serde() {
    let resp = VerificationResponse {
        valid: true,
        reason: Some("ok".to_string()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: VerificationResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.valid, resp.valid);
}

#[test]
fn test_token_validation_request_serde() {
    let req = TokenValidationRequest {
        token: "jwt".to_string(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: TokenValidationRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.token, req.token);
}

#[test]
fn test_token_validation_response_serde() {
    let resp = TokenValidationResponse {
        valid: true,
        user_id: Some("u1".to_string()),
        scopes: vec!["read".to_string()],
        expires_at: Some(12345),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: TokenValidationResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.valid, resp.valid);
}

// ============================================================================
// Async operations - no provider (error path)
// ============================================================================

#[tokio::test]
async fn test_encrypt_no_provider() {
    let client = SecurityClient::new();
    let req = EncryptionRequest {
        data: vec![1, 2, 3],
        algorithm: "AES-256".to_string(),
        key_id: None,
    };
    let result = client.encrypt(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_decrypt_no_provider() {
    let client = SecurityClient::new();
    let req = DecryptionRequest {
        encrypted_data: vec![0xaa, 0xbb],
        key_id: "key".to_string(),
    };
    let result = client.decrypt(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sign_no_provider() {
    let client = SecurityClient::new();
    let req = SignatureRequest {
        data: vec![1, 2, 3],
        algorithm: "ECDSA".to_string(),
        key_id: None,
    };
    let result = client.sign(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_verify_no_provider() {
    let client = SecurityClient::new();
    let req = VerificationRequest {
        data: vec![1],
        signature: vec![2],
        key_id: "k".to_string(),
    };
    let result = client.verify(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_token_no_provider() {
    let client = SecurityClient::new();
    let req = TokenValidationRequest {
        token: "jwt".to_string(),
    };
    let result = client.validate_token(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_key_no_provider() {
    let client = SecurityClient::new();
    let result = client.generate_key("AES-256".to_string()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_key_no_provider() {
    let client = SecurityClient::new();
    let result = client.delete_key("key-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_keys_no_provider() {
    let client = SecurityClient::new();
    let result = client.list_keys().await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_is_available_no_provider() {
    let client = SecurityClient::new();
    let available = client.is_available().await;
    assert!(!available);
}

#[tokio::test]
async fn test_provider_info_no_provider() {
    let client = SecurityClient::new();
    let info = client.provider_info().await;
    assert!(info.is_none());
}

#[tokio::test]
async fn test_rediscover_no_provider() {
    let client = SecurityClient::new();
    let result = client.rediscover().await;
    assert!(result.is_err());
}
