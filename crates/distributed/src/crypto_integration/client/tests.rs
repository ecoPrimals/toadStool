// SPDX-License-Identifier: AGPL-3.0-only
//! Tests for configuration, [`ServiceLocation`](crate::crypto_integration::ServiceLocation), and request/response types used by the crypto client.

use crate::crypto_integration::types::{
    CryptoRequest, CryptoResponse, KeyManagementRequest, KeyManagementResponse,
};
use crate::crypto_integration::{CryptoServiceConfig, ServiceLocation};

#[test]
fn test_service_location_types() {
    assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
    assert_ne!(ServiceLocation::Local, ServiceLocation::Network);
}

#[test]
fn test_crypto_request_serialization() {
    use crate::crypto_integration::types::{CryptoOperation, EncryptionAlgorithm, SecurityLevel};
    let req = CryptoRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CryptoOperation::Encrypt,
        data: vec![1, 2, 3, 4, 5],
        key_id: Some("key-1".to_string()),
        algorithm: Some(EncryptionAlgorithm::Aes256Gcm),
        security_level: SecurityLevel::High,
        metadata: serde_json::json!({"test": true}),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("operation").is_some());
    assert!(json.get("data").is_some());
}

#[test]
fn test_key_management_request_construction() {
    use crate::crypto_integration::types::{KeyOperation, KeyType};
    let req = KeyManagementRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: KeyOperation::Generate {
            key_type: KeyType::Symmetric { bits: 256 },
        },
        metadata: serde_json::Value::Null,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("operation").is_some());
}

#[test]
fn test_crypto_request_construction_encrypt() {
    use crate::crypto_integration::types::{CryptoOperation, EncryptionAlgorithm, SecurityLevel};
    let req = CryptoRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CryptoOperation::Encrypt,
        data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
        key_id: Some("enc-key".to_string()),
        algorithm: Some(EncryptionAlgorithm::Aes256Gcm),
        security_level: SecurityLevel::High,
        metadata: serde_json::json!({"nonce": "test"}),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["key_id"].as_str(), Some("enc-key"));
}

#[test]
fn test_crypto_request_construction_decrypt() {
    use crate::crypto_integration::types::{CryptoOperation, EncryptionAlgorithm, SecurityLevel};
    let req = CryptoRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: CryptoOperation::Decrypt,
        data: vec![0xAA, 0xBB, 0xCC],
        key_id: None,
        algorithm: Some(EncryptionAlgorithm::ChaCha20Poly1305),
        security_level: SecurityLevel::Standard,
        metadata: serde_json::Value::Null,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert!(json.get("data").is_some());
}

#[test]
fn test_crypto_response_parsing_success() {
    let resp = CryptoResponse {
        request_id: uuid::Uuid::new_v4(),
        data: vec![0x11, 0x22, 0x33, 0x44],
        key_id: "key-123".to_string(),
        algorithm: "aes-256-gcm".to_string(),
        metadata: serde_json::json!({"iv": "abc"}),
    };
    let json = serde_json::to_value(&resp).unwrap();
    let parsed: CryptoResponse = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.key_id, "key-123");
    assert_eq!(parsed.data.len(), 4);
}

#[test]
fn test_key_management_response_parsing() {
    let resp = KeyManagementResponse {
        request_id: uuid::Uuid::new_v4(),
        key_id: "gen-key-1".to_string(),
        success: true,
        metadata: serde_json::json!({"algorithm": "aes-256-gcm"}),
    };
    let json = serde_json::to_value(&resp).unwrap();
    let parsed: KeyManagementResponse = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.key_id, "gen-key-1");
    assert!(parsed.success);
}

#[test]
fn test_crypto_config_default_required_capabilities() {
    let config = CryptoServiceConfig::default();
    assert!(!config.required_capabilities.is_empty());
    assert!(config.auto_discover);
    assert_eq!(config.discovery_timeout_ms, 5000);
}

#[test]
fn test_crypto_config_preferred_location_variants() {
    let local = CryptoServiceConfig {
        preferred_location: ServiceLocation::Local,
        ..Default::default()
    };
    let network = CryptoServiceConfig {
        preferred_location: ServiceLocation::Network,
        ..Default::default()
    };
    let any = CryptoServiceConfig {
        preferred_location: ServiceLocation::Any,
        ..Default::default()
    };
    assert_eq!(local.preferred_location, ServiceLocation::Local);
    assert_eq!(network.preferred_location, ServiceLocation::Network);
    assert_eq!(any.preferred_location, ServiceLocation::Any);
}

#[test]
fn test_crypto_operation_variants_serde() {
    use crate::crypto_integration::types::CryptoOperation;
    let enc = CryptoOperation::Encrypt;
    let json = serde_json::to_value(&enc).unwrap();
    let _: CryptoOperation = serde_json::from_value(json).unwrap();
}

#[test]
fn test_encryption_algorithm_serde() {
    use crate::crypto_integration::types::EncryptionAlgorithm;
    let alg = EncryptionAlgorithm::Aes256Gcm;
    let json = serde_json::to_value(&alg).unwrap();
    let _: EncryptionAlgorithm = serde_json::from_value(json).unwrap();
}

#[test]
fn test_crypto_operation_all_variants_serde() {
    use crate::crypto_integration::types::{CryptoOperation, KeyType};
    let ops = [
        CryptoOperation::Encrypt,
        CryptoOperation::Decrypt,
        CryptoOperation::Sign,
        CryptoOperation::Verify,
        CryptoOperation::Hash,
        CryptoOperation::GenerateKey {
            key_type: KeyType::Symmetric { bits: 256 },
        },
        CryptoOperation::RotateKey {
            old_key_id: "old".to_string(),
        },
        CryptoOperation::ExportKey {
            key_id: "k1".to_string(),
        },
        CryptoOperation::ImportKey {
            key_data: vec![1, 2, 3],
        },
    ];
    for op in ops {
        let json = serde_json::to_value(&op).unwrap();
        let _: CryptoOperation = serde_json::from_value(json).unwrap();
    }
}

#[test]
fn test_key_type_serde() {
    use crate::crypto_integration::types::KeyType;
    let types = [
        KeyType::Symmetric { bits: 256 },
        KeyType::Asymmetric {
            algorithm: "RSA".to_string(),
            bits: 2048,
        },
        KeyType::Signing {
            algorithm: "Ed25519".to_string(),
        },
    ];
    for kt in types {
        let json = serde_json::to_value(&kt).unwrap();
        let _: KeyType = serde_json::from_value(json).unwrap();
    }
}

#[test]
fn test_security_level_serde() {
    use crate::crypto_integration::types::SecurityLevel;
    for level in [
        SecurityLevel::Standard,
        SecurityLevel::High,
        SecurityLevel::Maximum,
        SecurityLevel::QuantumResistant,
    ] {
        let json = serde_json::to_value(&level).unwrap();
        let _: SecurityLevel = serde_json::from_value(json).unwrap();
    }
}

#[test]
fn test_crypto_response_serde_roundtrip() {
    let resp = CryptoResponse {
        request_id: uuid::Uuid::new_v4(),
        data: vec![0xDE, 0xAD, 0xBE, 0xEF],
        key_id: "key-x".to_string(),
        algorithm: "aes-256-gcm".to_string(),
        metadata: serde_json::json!({"nonce": "abc"}),
    };
    let json = serde_json::to_value(&resp).unwrap();
    let parsed: CryptoResponse = serde_json::from_value(json).unwrap();
    assert_eq!(parsed.key_id, "key-x");
    assert_eq!(parsed.data.len(), 4);
}
