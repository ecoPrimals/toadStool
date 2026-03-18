// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::pedantic, unused_variables)]
#![allow(
    clippy::cast_precision_loss,
    clippy::float_cmp,
    clippy::unreadable_literal,
    clippy::no_effect_underscore_binding,
    clippy::similar_names,
    clippy::default_trait_access,
    clippy::items_after_statements,
    clippy::unused_async
)]
//! Comprehensive coverage tests for crypto adapter
//! Target: exercise all branches including error paths and edge cases.

use std::path::Path;

use toadstool_cli::ecosystem::adapters::{AdapterFactory, CryptoAdapter};
use toadstool_cli::ecosystem::capabilities::{CapabilityId, StandardCapability};

// ─── Constructor and factory ───────────────────────────────────────────────

#[test]
fn crypto_adapter_new_from_factory() {
    let factory = AdapterFactory::new();
    let adapter = factory.crypto_adapter().expect("crypto adapter");
    let _ = CryptoAdapter::new(factory.universal_adapter());
}

#[test]
fn crypto_adapter_accepts_capability_id_directly() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let _cap: CapabilityId = StandardCapability::CryptoSignatureEd25519.into();
}

// ─── Async operations (no service - resolution/discovery error) ───────────────

#[tokio::test(flavor = "current_thread")]
async fn verify_signature_no_service_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto
        .verify_signature(
            StandardCapability::CryptoSignatureEd25519,
            b"pubkey",
            b"message",
            b"signature",
        )
        .await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("Failed to") || err.contains("resolve") || err.contains("discover"),
        "expected resolution error, got: {err}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn verify_signature_with_capability_id_string() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let cap_id = StandardCapability::CryptoSignatureEd25519.id();
    let result = crypto.verify_signature(cap_id, b"k", b"m", b"s").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn generate_keypair_no_service_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto
        .generate_keypair(StandardCapability::CryptoSignatureEd25519)
        .await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn encrypt_no_service_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto
        .encrypt(
            StandardCapability::CryptoEncryptionAes256,
            b"key",
            b"plaintext",
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn decrypt_no_service_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto
        .decrypt(
            StandardCapability::CryptoEncryptionAes256,
            b"key",
            b"ciphertext",
        )
        .await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn random_bytes_no_service_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto.random_bytes(32).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn random_bytes_zero_length_no_service_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto.random_bytes(0).await;
    assert!(result.is_err());
}

// ─── install_permissions: file and parse errors ────────────────────────────

#[tokio::test(flavor = "current_thread")]
async fn install_permissions_nonexistent_path_err() {
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let path = Path::new("/nonexistent/path/to/permissions.json");
    let result = crypto.install_permissions(path, true).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("read")
            || err.contains("Failed")
            || err.contains("No such")
            || err.contains("not found"),
        "expected file error, got: {err}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn install_permissions_invalid_json_parse_err() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("permissions.json");
    std::fs::write(&path, "{ invalid json }").expect("write");
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto.install_permissions(&path, true).await;
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("parse") || err.contains("JSON") || err.contains("Failed"),
        "expected parse error, got: {err}"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn install_permissions_empty_file_parse_err() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("empty.json");
    std::fs::write(&path, "").expect("write");
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto.install_permissions(&path, true).await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "current_thread")]
async fn install_permissions_valid_json_no_service_validate_err() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("permissions.json");
    let valid_json = r#"{"permissions": [], "version": 1}"#;
    std::fs::write(&path, valid_json).expect("write");
    let factory = AdapterFactory::new();
    let crypto = factory.crypto_adapter().expect("crypto adapter");
    let result = crypto.install_permissions(&path, true).await;
    assert!(result.is_err());
}

// ─── KeyPair struct coverage ──────────────────────────────────────────────

#[test]
fn keypair_debug_and_clone() {
    use toadstool_cli::ecosystem::adapters::crypto::KeyPair;
    let kp = KeyPair {
        public_key: vec![1, 2, 3],
        private_key: vec![4, 5, 6],
    };
    let cloned = kp.clone();
    assert_eq!(kp.public_key, cloned.public_key);
    assert_eq!(kp.private_key, cloned.private_key);
    let _ = format!("{:?}", kp);
}
