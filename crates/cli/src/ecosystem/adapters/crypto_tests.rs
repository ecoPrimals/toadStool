// SPDX-License-Identifier: AGPL-3.0-or-later
use super::*;

#[test]
fn test_base64_roundtrip() {
    let data = b"hello world";
    let encoded = base64::encode(data);
    let decoded = base64::decode(&encoded).unwrap();
    assert_eq!(data, &decoded[..]);
}

#[test]
fn test_base64_empty() {
    let data: &[u8] = &[];
    let encoded = base64::encode(data);
    let decoded = base64::decode(&encoded).unwrap();
    assert!(decoded.is_empty());
}

#[test]
fn test_base64_binary_data() {
    let data: Vec<u8> = (0u8..=255).collect();
    let encoded = base64::encode(&data);
    let decoded = base64::decode(&encoded).unwrap();
    assert_eq!(data, decoded);
}

#[test]
fn test_keypair_struct() {
    let keypair = KeyPair {
        public_key: vec![1, 2, 3, 4, 5],
        private_key: vec![6, 7, 8, 9, 10],
    };
    assert_eq!(keypair.public_key.len(), 5);
    assert_eq!(keypair.private_key.len(), 5);
    assert_eq!(keypair.public_key[0], 1);
}

#[test]
fn test_keypair_clone() {
    let keypair = KeyPair {
        public_key: vec![1, 2, 3],
        private_key: vec![4, 5, 6],
    };
    let cloned = keypair.clone();
    assert_eq!(keypair.public_key, cloned.public_key);
    assert_eq!(keypair.private_key, cloned.private_key);
}

#[test]
fn test_crypto_adapter_new() {
    use crate::ecosystem::adapters::AdapterFactory;
    let factory = AdapterFactory::new();
    let adapter = factory.crypto_adapter().unwrap();
    // Adapter created successfully - no panic
    let _ = adapter;
}

#[tokio::test]
async fn test_verify_signature_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;
    use crate::ecosystem::capabilities::StandardCapability;

    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    // No crypto service running - should fail with resolution/discovery error
    let result = crypto
        .verify_signature(
            StandardCapability::CryptoSignatureEd25519,
            b"pubkey",
            b"message",
            b"signature",
        )
        .await;

    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("Failed to")
            || err_str.contains("resolve")
            || err_str.contains("discover"),
        "Expected resolution/discovery error, got: {}",
        err_str
    );
}

#[tokio::test]
async fn test_generate_keypair_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;
    use crate::ecosystem::capabilities::StandardCapability;

    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .generate_keypair(StandardCapability::CryptoSignatureEd25519)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_encrypt_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;
    use crate::ecosystem::capabilities::StandardCapability;

    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .encrypt(
            StandardCapability::CryptoEncryptionAes256,
            b"key",
            b"plaintext",
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_decrypt_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;
    use crate::ecosystem::capabilities::StandardCapability;

    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .decrypt(
            StandardCapability::CryptoEncryptionAes256,
            b"key",
            b"ciphertext",
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_random_bytes_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto.random_bytes(32).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_install_permissions_file_not_found() {
    use crate::ecosystem::adapters::AdapterFactory;
    use std::path::Path;

    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .install_permissions(Path::new("/nonexistent/path/permissions.json"), true)
        .await;

    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("read") || err_str.contains("Failed") || err_str.contains("No such"),
        "Expected file read error: {}",
        err_str
    );
}
