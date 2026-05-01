// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;
use crate::security_provider::provider::{EncryptionOptions, SigningOptions};
use crate::security_provider::types::{
    ExternalTarget, PermissionRequest, PermissionScope, ResourceLimits,
};
use std::time::{Duration, SystemTime};

fn make_permission_request() -> PermissionRequest {
    PermissionRequest {
        requester_id: "test-requester".to_string(),
        target: ExternalTarget::ExternalTool {
            tool_name: "test-tool".to_string(),
            api_endpoints: vec!["http://localhost".to_string()],
            feature_set: vec!["read".to_string()],
        },
        scope: PermissionScope {
            operations: vec!["read".to_string()],
            resource_limits: ResourceLimits::default(),
            geo_restrictions: vec![],
        },
        validity_duration: Duration::from_secs(3600),
        delegation_info: None,
    }
}

#[tokio::test]
async fn test_software_hsm_new() {
    let provider = SoftwareHsmProvider::new();
    let caps = provider.capabilities().await.unwrap();
    assert!(caps.contains(&SecurityCapability::SymmetricEncryption));
    assert!(caps.contains(&SecurityCapability::DigitalSignatures));
}

#[tokio::test]
async fn test_software_hsm_default() {
    let provider = SoftwareHsmProvider::default();
    assert!(provider.capabilities().await.is_ok());
}

#[tokio::test]
async fn test_software_hsm_metadata() {
    let provider = SoftwareHsmProvider::new();
    let meta = provider.metadata().await.unwrap();
    assert_eq!(meta.provider_type, "SoftwareHSM");
    assert_eq!(meta.provider_id, "software-hsm-local");
    assert!(meta.metadata.contains_key("algorithm_sym"));
    assert_eq!(meta.metadata.get("algorithm_sym").unwrap(), "AES-256-GCM");
}

#[tokio::test]
async fn test_software_hsm_encrypt_decrypt_roundtrip() {
    let provider = SoftwareHsmProvider::new();
    let data = b"secret message";

    let encrypted = provider.encrypt(data, None).await.unwrap();
    assert!(!encrypted.ciphertext.is_empty());
    assert!(encrypted.iv.is_some());
    assert!(encrypted.auth_tag.is_some());
    assert_eq!(encrypted.metadata.algorithm, "AES-256-GCM");
    assert_eq!(encrypted.metadata.key_id, "default");

    let decrypted = provider
        .decrypt(&encrypted.ciphertext, &encrypted.metadata)
        .await
        .unwrap();
    assert_eq!(decrypted.plaintext, data);
}

#[tokio::test]
async fn test_software_hsm_encrypt_with_custom_key_id() {
    let provider = SoftwareHsmProvider::new();
    let data = b"custom key data";
    let options = Some(EncryptionOptions {
        algorithm: None,
        key_id: Some("my-key".to_string()),
        aad: None,
    });

    let encrypted = provider.encrypt(data, options).await.unwrap();
    assert_eq!(encrypted.metadata.key_id, "my-key");

    let decrypted = provider
        .decrypt(&encrypted.ciphertext, &encrypted.metadata)
        .await
        .unwrap();
    assert_eq!(decrypted.plaintext, data);
}

#[tokio::test]
async fn test_software_hsm_decrypt_ciphertext_too_short() {
    let provider = SoftwareHsmProvider::new();
    let short_ct = vec![0u8; 10];
    let metadata = EncryptionMetadata {
        algorithm: "AES-256-GCM".to_string(),
        key_id: "default".to_string(),
        encrypted_at: SystemTime::now(),
    };

    let result = provider.decrypt(&short_ct, &metadata).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_software_hsm_decrypt_key_not_found() {
    let provider = SoftwareHsmProvider::new();
    let data = b"data";
    let encrypted = provider.encrypt(data, None).await.unwrap();

    let metadata = EncryptionMetadata {
        algorithm: "AES-256-GCM".to_string(),
        key_id: "nonexistent-key".to_string(),
        encrypted_at: SystemTime::now(),
    };

    let result = provider.decrypt(&encrypted.ciphertext, &metadata).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_software_hsm_sign_verify() {
    let provider = SoftwareHsmProvider::new();
    let data = b"data to sign";

    let sig_result = provider.sign(data, None).await.unwrap();
    assert_eq!(sig_result.algorithm, SignatureAlgorithm::Ed25519);
    assert_eq!(sig_result.key_id, "default");
    assert_eq!(sig_result.signature.len(), 64);

    let verify_result = provider
        .verify(data, &sig_result.signature, &sig_result.key_id)
        .await
        .unwrap();
    assert_eq!(verify_result, VerificationResult::Valid);
}

#[tokio::test]
async fn test_software_hsm_sign_with_custom_key() {
    let provider = SoftwareHsmProvider::new();
    let options = Some(SigningOptions {
        algorithm: None,
        key_id: Some("sign-key".to_string()),
    });

    let sig = provider.sign(b"data", options).await.unwrap();
    assert_eq!(sig.key_id, "sign-key");
}

#[tokio::test]
async fn test_software_hsm_verify_invalid_signature_length() {
    let provider = SoftwareHsmProvider::new();
    let short_sig = vec![0u8; 32];

    let result = provider.verify(b"data", &short_sig, "default").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_software_hsm_verify_invalid_signature() {
    let provider = SoftwareHsmProvider::new();
    let _ = provider.sign(b"data", None).await.unwrap();
    let wrong_sig = vec![0u8; 64];

    let result = provider
        .verify(b"data", &wrong_sig, "default")
        .await
        .unwrap();
    assert_eq!(result, VerificationResult::Invalid);
}

#[tokio::test]
async fn test_software_hsm_verify_key_not_found() {
    let provider = SoftwareHsmProvider::new();
    let sig = vec![0u8; 64];

    let result = provider.verify(b"data", &sig, "nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_software_hsm_create_permission() {
    let provider = SoftwareHsmProvider::new();
    let request = make_permission_request();

    let permission = provider.create_permission(request).await.unwrap();
    assert!(!permission.proof.signature.is_empty());
    assert_eq!(permission.holder_id, "test-requester");
    assert_eq!(permission.proof.algorithm, SignatureAlgorithm::Ed25519);
}

#[tokio::test]
async fn test_software_hsm_validate_permission() {
    let provider = SoftwareHsmProvider::new();
    let request = make_permission_request();
    let permission = provider.create_permission(request).await.unwrap();

    let result = provider.validate_permission(&permission).await.unwrap();
    assert_eq!(result, PermissionValidationResult::Valid);
}

#[tokio::test]
async fn test_software_hsm_validate_revoked_permission() {
    let provider = SoftwareHsmProvider::new();
    let request = make_permission_request();
    let permission = provider.create_permission(request).await.unwrap();

    provider
        .revoke_permission(&permission.permission_id, "test")
        .await
        .unwrap();

    let result = provider.validate_permission(&permission).await.unwrap();
    assert_eq!(result, PermissionValidationResult::Revoked);
}

#[tokio::test]
async fn test_software_hsm_revoke_permission() {
    let provider = SoftwareHsmProvider::new();
    let perm_id = uuid::Uuid::new_v4();

    let result = provider.revoke_permission(&perm_id, "reason").await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_software_hsm_health_check() {
    let provider = SoftwareHsmProvider::new();
    let health = provider.health_check().await.unwrap();
    assert_eq!(health, ProviderHealth::Healthy);
}
