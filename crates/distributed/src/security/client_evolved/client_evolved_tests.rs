// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[tokio::test]
async fn test_security_client_creation() {
    let client = SecurityClient::new();
    let provider_lock = client.provider.read().await;
    assert!(provider_lock.is_none());
}

#[test]
fn test_error_messages() {
    let err = SecurityClientError::NoProvider;
    assert!(err.to_string().contains("No security provider found"));

    let err = SecurityClientError::EncryptionFailed("test error".into());
    assert!(err.to_string().contains("test error"));
}

#[tokio::test]
async fn test_rediscover() {
    let client = SecurityClient::new();
    // Should fail since no Coordination is running
    let result = client.rediscover().await;
    assert!(result.is_err());
}

#[test]
fn test_security_client_default() {
    let client = SecurityClient::default();
    assert!(std::mem::size_of_val(&client) > 0);
}

#[tokio::test]
async fn test_provider_info_none_when_no_provider() {
    let client = SecurityClient::new();
    let info = client.provider_info().await;
    assert!(info.is_none());
}

#[tokio::test]
async fn test_is_available_fails_without_provider() {
    let client = SecurityClient::new();
    let available = client.is_available().await;
    assert!(!available);
}

#[test]
fn test_encryption_request_construction() {
    let req = EncryptionRequest {
        data: vec![1, 2, 3],
        algorithm: "AES-256-GCM".to_string(),
        key_id: Some("key-1".to_string()),
    };
    assert_eq!(req.data.len(), 3);
    assert_eq!(req.algorithm, "AES-256-GCM");
    assert_eq!(req.key_id.as_deref(), Some("key-1"));
}

#[test]
fn test_decryption_request_construction() {
    let req = DecryptionRequest {
        encrypted_data: vec![0xaa, 0xbb],
        key_id: "key-xyz".to_string(),
    };
    assert_eq!(req.encrypted_data.len(), 2);
    assert_eq!(req.key_id, "key-xyz");
}

#[test]
fn test_signature_request_construction() {
    let req = SignatureRequest {
        data: vec![1, 2, 3, 4, 5],
        algorithm: "ECDSA".to_string(),
        key_id: None,
    };
    assert_eq!(req.data.len(), 5);
    assert_eq!(req.algorithm, "ECDSA");
}

#[test]
fn test_verification_request_construction() {
    let req = VerificationRequest {
        data: vec![1, 2, 3],
        signature: vec![0x11, 0x22],
        key_id: "sig-key".to_string(),
    };
    assert_eq!(req.signature.len(), 2);
    assert_eq!(req.key_id, "sig-key");
}

#[test]
fn test_token_validation_request_construction() {
    let req = TokenValidationRequest {
        token: "jwt-token-123".to_string(),
    };
    assert_eq!(req.token, "jwt-token-123");
}

#[test]
fn test_encryption_response_serde() {
    let resp = EncryptionResponse {
        encrypted_data: vec![0xde, 0xad, 0xbe, 0xef],
        key_id: "k1".to_string(),
        algorithm: "AES".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: EncryptionResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.key_id, resp.key_id);
    assert_eq!(parsed.encrypted_data, resp.encrypted_data);
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
fn test_verification_response_construction() {
    let resp = VerificationResponse {
        valid: true,
        reason: None,
    };
    assert!(resp.valid);
    assert!(resp.reason.is_none());
}

#[test]
fn test_token_validation_response_construction() {
    let resp = TokenValidationResponse {
        valid: true,
        user_id: Some("user-42".to_string()),
        scopes: vec!["read".to_string(), "write".to_string()],
        expires_at: Some(1234567890),
    };
    assert!(resp.valid);
    assert_eq!(resp.user_id.as_deref(), Some("user-42"));
    assert_eq!(resp.scopes.len(), 2);
}

#[test]
fn test_security_client_error_all_variants() {
    let _ = SecurityClientError::NoProvider;
    let _ = SecurityClientError::EncryptionFailed("e".into());
    let _ = SecurityClientError::DecryptionFailed("e".into());
    let _ = SecurityClientError::SignatureFailed("e".into());
    let _ = SecurityClientError::VerificationFailed("e".into());
    let _ = SecurityClientError::KeyManagementFailed("e".into());
    let _ = SecurityClientError::ValidationFailed("e".into());
    let err = SecurityClientError::Json(
        serde_json::from_str::<serde_json::Value>("invalid").unwrap_err(),
    );
    assert!(err.to_string().contains("expected"));
}

#[tokio::test]
async fn test_encrypt_fails_without_provider() {
    let client = SecurityClient::new();
    let req = EncryptionRequest {
        data: vec![1, 2, 3],
        algorithm: "AES-256-GCM".to_string(),
        key_id: None,
    };
    let result = client.encrypt(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_decrypt_fails_without_provider() {
    let client = SecurityClient::new();
    let req = DecryptionRequest {
        encrypted_data: vec![0xaa, 0xbb],
        key_id: "key-1".to_string(),
    };
    let result = client.decrypt(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_sign_fails_without_provider() {
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
async fn test_verify_fails_without_provider() {
    let client = SecurityClient::new();
    let req = VerificationRequest {
        data: vec![1, 2, 3],
        signature: vec![0x11, 0x22],
        key_id: "key-1".to_string(),
    };
    let result = client.verify(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_validate_token_fails_without_provider() {
    let client = SecurityClient::new();
    let req = TokenValidationRequest {
        token: "jwt-token".to_string(),
    };
    let result = client.validate_token(req).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_key_fails_without_provider() {
    let client = SecurityClient::new();
    let result = client.generate_key("AES-256".to_string()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_key_fails_without_provider() {
    let client = SecurityClient::new();
    let result = client.delete_key("key-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_keys_fails_without_provider() {
    let client = SecurityClient::new();
    let result = client.list_keys().await;
    assert!(result.is_err());
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
fn test_capability_error_conversion() {
    use toadstool_common::capability_provider::CapabilityError;
    use toadstool_common::primal_identity::{Capability, CryptoCapability};
    let cap_err =
        CapabilityError::NoProviderFound(Capability::Crypto(CryptoCapability::Encryption));
    let sec_err: SecurityClientError = cap_err.into();
    assert!(matches!(
        sec_err,
        SecurityClientError::Capability(CapabilityError::NoProviderFound(_))
    ));
}
