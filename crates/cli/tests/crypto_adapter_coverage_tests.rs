// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for `CryptoAdapter` (ecosystem/adapters/crypto.rs) - coverage target 90%
//!
//! Tests `verify_signature`, `generate_keypair`, encrypt, decrypt, `random_bytes`,
//! `install_permissions`, and `KeyPair` struct. Uses `AdapterFactory` - no live services.

use std::path::Path;
use toadstool_cli::ecosystem::adapters::AdapterFactory;
use toadstool_cli::ecosystem::capabilities::StandardCapability;

// ============================================================================
// CryptoAdapter construction
// ============================================================================

#[test]
fn test_crypto_adapter_factory_creates() {
    let factory = AdapterFactory::new();
    let adapter = factory.crypto_adapter();
    assert!(adapter.is_ok());
}

// ============================================================================
// verify_signature - error paths (no service)
// ============================================================================

#[tokio::test]
async fn test_verify_signature_no_service_returns_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .verify_signature(
            StandardCapability::CryptoSignatureEd25519,
            b"pubkey",
            b"message",
            b"signature",
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_verify_signature_with_capability_string() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .verify_signature("ed25519", b"key", b"msg", b"sig")
        .await;

    assert!(result.is_err());
}

// ============================================================================
// generate_keypair - error paths
// ============================================================================

#[tokio::test]
async fn test_generate_keypair_no_service_returns_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .generate_keypair(StandardCapability::CryptoSignatureEd25519)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_keypair_ecdsa_capability() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .generate_keypair(StandardCapability::CryptoSignatureEcdsa)
        .await;

    assert!(result.is_err());
}

// ============================================================================
// encrypt - error paths
// ============================================================================

#[tokio::test]
async fn test_encrypt_no_service_returns_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .encrypt(
            StandardCapability::CryptoEncryptionAes256,
            b"key12345678901234567890123456789012",
            b"plaintext",
        )
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_encrypt_empty_plaintext() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .encrypt(StandardCapability::CryptoEncryptionAes256, b"key", b"")
        .await;

    assert!(result.is_err());
}

// ============================================================================
// decrypt - error paths
// ============================================================================

#[tokio::test]
async fn test_decrypt_no_service_returns_err() {
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

// ============================================================================
// random_bytes - error paths
// ============================================================================

#[tokio::test]
async fn test_random_bytes_no_service_returns_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto.random_bytes(32).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_random_bytes_zero_length() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto.random_bytes(0).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_random_bytes_large_length() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto.random_bytes(1024).await;

    assert!(result.is_err());
}

// ============================================================================
// install_permissions - error paths
// ============================================================================

#[tokio::test]
async fn test_install_permissions_file_not_found() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .install_permissions(Path::new("/nonexistent/path/permissions.json"), true)
        .await;

    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("read") || err_str.contains("Failed") || err_str.contains("No such"),
        "Expected file read error: {err_str}"
    );
}

#[tokio::test]
async fn test_install_permissions_validate_only_nonexistent() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .install_permissions(Path::new("/tmp/nonexistent-permissions-xyz.json"), true)
        .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_install_permissions_install_mode_nonexistent() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();

    let result = crypto
        .install_permissions(Path::new("/tmp/nonexistent-permissions-xyz.json"), false)
        .await;

    assert!(result.is_err());
}

// ============================================================================
// KeyPair struct (from crypto adapter)
// ============================================================================

#[test]
fn test_keypair_struct_creation() {
    let keypair = toadstool_cli::ecosystem::adapters::crypto::KeyPair {
        public_key: vec![1, 2, 3, 4, 5],
        private_key: vec![6, 7, 8, 9, 10],
    };
    assert_eq!(keypair.public_key.len(), 5);
    assert_eq!(keypair.private_key.len(), 5);
}

#[test]
fn test_keypair_clone() {
    let keypair = toadstool_cli::ecosystem::adapters::crypto::KeyPair {
        public_key: vec![1, 2, 3],
        private_key: vec![4, 5, 6],
    };
    let cloned = keypair.clone();
    assert_eq!(keypair.public_key, cloned.public_key);
    assert_eq!(keypair.private_key, cloned.private_key);
}

// ─── Additional coverage: error paths, capability variants ───

#[tokio::test]
async fn test_install_permissions_invalid_json() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("invalid.json");
    std::fs::write(&path, "{ invalid json }").expect("write");

    let result = crypto.install_permissions(&path, true).await;
    assert!(result.is_err());
    let err_str = result.unwrap_err().to_string();
    assert!(
        err_str.contains("parse") || err_str.contains("JSON") || err_str.contains("Failed"),
        "expected parse/JSON error: {err_str}"
    );
}

#[tokio::test]
async fn test_install_permissions_empty_file() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("empty.json");
    std::fs::write(&path, "").expect("write");

    let result = crypto.install_permissions(&path, true).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_install_permissions_malformed_json() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("malformed.json");
    std::fs::write(&path, "not json at all").expect("write");

    let result = crypto.install_permissions(&path, true).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_verify_signature_rsa_capability() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();
    let result = crypto
        .verify_signature(
            toadstool_cli::ecosystem::capabilities::StandardCapability::CryptoSignatureRsa,
            b"pubkey",
            b"msg",
            b"sig",
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_generate_keypair_rsa_capability() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();
    let result = crypto
        .generate_keypair(
            toadstool_cli::ecosystem::capabilities::StandardCapability::CryptoSignatureRsa,
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_encrypt_chacha20_capability() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();
    let result = crypto
        .encrypt(
            toadstool_cli::ecosystem::capabilities::StandardCapability::CryptoEncryptionChaCha20,
            b"key12345678901234567890123456789012",
            b"data",
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_decrypt_chacha20_capability() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().unwrap();
    let result = crypto
        .decrypt(
            toadstool_cli::ecosystem::capabilities::StandardCapability::CryptoEncryptionChaCha20,
            b"key",
            b"cipher",
        )
        .await;
    assert!(result.is_err());
}

#[test]
fn test_keypair_debug() {
    let kp = toadstool_cli::ecosystem::adapters::crypto::KeyPair {
        public_key: vec![1],
        private_key: vec![2],
    };
    let s = format!("{kp:?}");
    assert!(s.contains("KeyPair") || s.contains("public_key"));
}

#[tokio::test]
async fn test_crypto_adapter_new_via_factory() {
    let factory = AdapterFactory::new();
    let adapter = factory.crypto_adapter().unwrap();
    let _ = adapter; // CryptoAdapter::new(universal) exercised via factory
}
