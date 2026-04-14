// SPDX-License-Identifier: AGPL-3.0-or-later


use super::*;

#[test]
fn test_encrypted_payload_size() {
    let payload = EncryptedPayload {
        ciphertext: vec![0u8; 100],
        auth_tag: Some(vec![0u8; 16]),
    };

    assert_eq!(payload.size(), 116);
}

#[test]
fn test_key_expiration() {
    let mut key = EncryptionKey::new(
        "test-key".to_string(),
        vec![1, 2, 3, 4, 5],
        "aes-256-gcm".to_string(),
        SecurityLevel::Standard,
    );
    assert!(!key.is_expired());
    assert!(key.is_valid());

    // Set expiration to past
    key.expires_at = Some(unix_timestamp_now() - 1000);
    assert!(key.is_expired());
    assert!(!key.is_valid());
}

#[test]
fn test_key_rotation_policy() {
    let policy = KeyRotationPolicy::default();

    // Should rotate when exceeding max uses
    assert!(policy.should_rotate(100_001, 0, 0));

    // Should rotate when exceeding max age
    assert!(policy.should_rotate(0, 86400 * 31, 0));

    // Should not rotate when within limits
    assert!(!policy.should_rotate(1000, 86400, 1024 * 1024));
}

#[test]
fn test_key_debug_redacts_material() {
    let key = EncryptionKey::new(
        "test".to_string(),
        vec![1, 2, 3, 4, 5],
        "test-alg".to_string(),
        SecurityLevel::Standard,
    );

    let debug_str = format!("{key:?}");
    assert!(debug_str.contains("[REDACTED]"));
    assert!(!debug_str.contains("1, 2, 3, 4, 5"));
}

#[test]
fn test_encrypted_payload_new() {
    let payload = EncryptedPayload::new(vec![1, 2, 3, 4, 5]);
    assert_eq!(payload.ciphertext.len(), 5);
    assert!(payload.auth_tag.is_none());
    assert_eq!(payload.size(), 5);
}

#[test]
fn test_encrypted_payload_with_auth_tag() {
    let payload =
        EncryptedPayload::new(vec![1, 2, 3]).with_auth_tag(vec![10, 11, 12, 13, 14, 15, 16]);
    assert!(payload.auth_tag.is_some());
    assert_eq!(payload.auth_tag.as_ref().unwrap().len(), 7);
    assert_eq!(payload.size(), 10);
}

#[test]
fn test_encrypted_payload_default() {
    let payload = EncryptedPayload::default();
    assert!(payload.ciphertext.is_empty());
    assert!(payload.auth_tag.is_none());
    assert_eq!(payload.size(), 0);
}

#[test]
fn test_encrypted_payload_serde_roundtrip() {
    let payload = EncryptedPayload::new(vec![1, 2, 3]).with_auth_tag(vec![4, 5, 6]);
    let json = serde_json::to_string(&payload).unwrap();
    let restored: EncryptedPayload = serde_json::from_str(&json).unwrap();
    assert_eq!(payload.ciphertext, restored.ciphertext);
    assert_eq!(payload.auth_tag, restored.auth_tag);
}

#[test]
fn test_encryption_metadata_default() {
    let meta = EncryptionMetadata::default();
    assert_eq!(meta.algorithm, "chacha20poly1305");
    assert!(meta.nonce.is_empty());
    assert!(meta.aad.is_none());
    assert!(meta.kdf_info.is_none());
}

#[test]
fn test_key_derivation_info_serde_roundtrip() {
    let kdf = KeyDerivationInfo {
        algorithm: "HKDF-SHA256".to_string(),
        salt: vec![1, 2, 3, 4, 5],
        iterations: Some(100_000),
        memory_kb: Some(64),
        parallelism: Some(4),
    };
    let json = serde_json::to_string(&kdf).unwrap();
    let restored: KeyDerivationInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(kdf.algorithm, restored.algorithm);
    assert_eq!(kdf.salt, restored.salt);
    assert_eq!(kdf.iterations, restored.iterations);
}

#[test]
fn test_encryption_key_new() {
    let key = EncryptionKey::new(
        "key-1".to_string(),
        vec![1, 2, 3, 4, 5, 6, 7, 8],
        "aes-256-gcm".to_string(),
        SecurityLevel::Standard,
    );
    assert_eq!(key.id, "key-1");
    assert_eq!(key.size(), 8);
    assert!(key.is_valid());
    assert!(!key.is_expired());
    assert!(key.expires_at.is_none());
}

#[test]
fn test_encryption_key_with_expiration() {
    let key = EncryptionKey::new(
        "key-1".to_string(),
        vec![1, 2, 3],
        "test".to_string(),
        SecurityLevel::Standard,
    )
    .with_expiration(unix_timestamp_now() + 3600);
    assert!(key.expires_at.is_some());
    assert!(!key.is_expired());
}

#[test]
fn test_encryption_key_empty_invalid() {
    let key = EncryptionKey::new(
        "key-1".to_string(),
        vec![],
        "test".to_string(),
        SecurityLevel::Standard,
    );
    assert!(!key.is_valid());
}

#[test]
fn test_key_rotation_policy_max_data_bytes() {
    let policy = KeyRotationPolicy::default();
    assert!(policy.should_rotate(0, 0, 1024 * 1024 * 1024 * 101));
    assert!(!policy.should_rotate(0, 0, 1024 * 1024));
}

#[test]
fn test_key_rotation_policy_serde_roundtrip() {
    let policy = KeyRotationPolicy {
        max_uses: Some(50_000),
        max_age_seconds: Some(86400),
        max_data_bytes: Some(1024 * 1024),
        auto_retire: false,
    };
    let json = serde_json::to_string(&policy).unwrap();
    let restored: KeyRotationPolicy = serde_json::from_str(&json).unwrap();
    assert_eq!(policy.max_uses, restored.max_uses);
    assert_eq!(policy.auto_retire, restored.auto_retire);
}

#[test]
fn test_encryption_key_serde_roundtrip() {
    let key = EncryptionKey::new(
        "test-id".to_string(),
        vec![1, 2, 3, 4, 5],
        "chacha20".to_string(),
        SecurityLevel::Standard,
    );
    let json = serde_json::to_string(&key).unwrap();
    let restored: EncryptionKey = serde_json::from_str(&json).unwrap();
    assert_eq!(key.id, restored.id);
    assert_eq!(key.key_material, restored.key_material);
    assert_eq!(key.algorithm, restored.algorithm);
}
