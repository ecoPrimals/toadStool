// SPDX-License-Identifier: AGPL-3.0-or-later
//! Serialization roundtrip tests for BearDog integration types

use crate::beardog_integration::types::{
    BearDogCapability, BearDogEndpoint, EncryptionOperation, EncryptionRequest, EncryptionResponse,
    KeyManagementRequest, KeyManagementResponse, KeyOperation, KeyOperationResult, SecurityLevel,
    SignatureRequest, VerificationRequest,
};
use crate::beardog_integration::ServiceLocation;

#[test]
fn test_encryption_request_serialization() {
    let request = EncryptionRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: EncryptionOperation::Encrypt,
        data: vec![1, 2, 3, 4, 5],
        key_id: Some("key-123".to_string()),
        algorithm: Some("aes-256-gcm".to_string()),
        security_level: SecurityLevel::Enhanced,
    };
    let json = serde_json::to_value(&request);
    assert!(json.is_ok());
    let parsed: Result<EncryptionRequest, _> = serde_json::from_value(json.unwrap());
    assert!(parsed.is_ok());
    let p = parsed.unwrap();
    assert_eq!(p.data, vec![1, 2, 3, 4, 5]);
    assert_eq!(p.key_id.as_deref(), Some("key-123"));
}

#[test]
fn test_encryption_response_serialization() {
    let response = EncryptionResponse {
        request_id: uuid::Uuid::new_v4(),
        data: vec![10, 20, 30],
        key_id: "key-456".to_string(),
        algorithm: "chacha20".to_string(),
        metadata: serde_json::json!({"nonce": "abc"}),
    };
    let json = serde_json::to_value(&response);
    assert!(json.is_ok());
    let parsed: Result<EncryptionResponse, _> = serde_json::from_value(json.unwrap());
    assert!(parsed.is_ok());
}

#[test]
fn test_signature_request_serialization() {
    let request = SignatureRequest {
        request_id: uuid::Uuid::new_v4(),
        data: vec![1, 2, 3],
        key_id: None,
        algorithm: Some("ed25519".to_string()),
    };
    assert!(serde_json::to_value(&request).is_ok());
}

#[test]
fn test_verification_request_serialization() {
    let request = VerificationRequest {
        request_id: uuid::Uuid::new_v4(),
        data: vec![1, 2, 3],
        signature: vec![4, 5, 6],
        public_key_id: "pub-key-1".to_string(),
    };
    assert!(serde_json::to_value(&request).is_ok());
}

#[test]
fn test_key_management_request_serialization() {
    let request = KeyManagementRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: KeyOperation::Generate,
        key_id: None,
        security_level: Some(SecurityLevel::Standard),
    };
    assert!(serde_json::to_value(&request).is_ok());
}

#[test]
fn test_key_operation_result_serialization() {
    let result = KeyOperationResult::Generated {
        key_id: "gen-key-1".to_string(),
        algorithm: "aes-256".to_string(),
    };
    assert!(serde_json::to_value(&result).is_ok());
}

#[test]
fn test_bear_dog_endpoint_serialization() {
    let endpoint = BearDogEndpoint {
        service_id: "beardog-1".to_string(),
        protocol: "http".to_string(),
        address: "127.0.0.1:8081".parse().unwrap(),
        api_version: "v1".to_string(),
        capabilities: vec![BearDogCapability::Encryption {
            algorithms: vec!["aes-256".to_string()],
        }],
        healthy: true,
        latency_ms: Some(5),
    };
    assert!(serde_json::to_value(&endpoint).is_ok());
}

#[test]
fn test_bear_dog_capability_variants() {
    let enc = BearDogCapability::Encryption {
        algorithms: vec!["aes".to_string()],
    };
    assert!(matches!(enc, BearDogCapability::Encryption { .. }));
    assert!(matches!(
        BearDogCapability::KeyManagement,
        BearDogCapability::KeyManagement
    ));
}

#[test]
fn test_security_level_ordering() {
    assert!(SecurityLevel::Standard < SecurityLevel::Enhanced);
    assert!(SecurityLevel::Enhanced < SecurityLevel::HardwareSecured);
}

#[test]
fn test_signature_response_serialization() {
    use crate::beardog_integration::types::SignatureResponse;
    let resp = SignatureResponse {
        request_id: uuid::Uuid::new_v4(),
        signature: vec![1, 2, 3, 4],
        key_id: "key-1".to_string(),
        algorithm: "ed25519".to_string(),
    };
    let json = serde_json::to_value(&resp);
    assert!(json.is_ok());
    let parsed: Result<SignatureResponse, _> = serde_json::from_value(json.unwrap());
    assert!(parsed.is_ok());
}

#[test]
fn test_permission_response_serialization() {
    use crate::beardog_integration::types::PermissionResponse;
    let resp = PermissionResponse {
        request_id: uuid::Uuid::new_v4(),
        permission_id: uuid::Uuid::new_v4(),
        proof: vec![5, 6, 7],
        metadata: serde_json::json!({"scope": "read"}),
    };
    assert!(serde_json::to_value(&resp).is_ok());
}

#[test]
fn test_validation_response_serialization() {
    use crate::beardog_integration::types::ValidationResponse;
    let resp = ValidationResponse {
        request_id: uuid::Uuid::new_v4(),
        valid: true,
        details: Some("ok".to_string()),
    };
    let json = serde_json::to_value(&resp);
    assert!(json.is_ok());
    let parsed: Result<ValidationResponse, _> = serde_json::from_value(json.unwrap());
    assert!(parsed.unwrap().valid);
}

#[test]
fn test_revocation_request_serialization() {
    use crate::beardog_integration::types::RevocationRequest;
    let req = RevocationRequest {
        reason: "expired".to_string(),
    };
    assert!(serde_json::to_value(&req).is_ok());
}

#[test]
fn test_key_management_response_serialization() {
    let resp = KeyManagementResponse {
        request_id: uuid::Uuid::new_v4(),
        result: KeyOperationResult::Deleted {
            key_id: "del-key".to_string(),
        },
    };
    assert!(serde_json::to_value(&resp).is_ok());
}

#[test]
fn test_key_operation_result_all_variants_serialization() {
    let deleted = KeyOperationResult::Deleted {
        key_id: "k1".to_string(),
    };
    assert!(serde_json::to_value(&deleted).is_ok());

    let listed = KeyOperationResult::Listed {
        keys: vec!["k1".to_string(), "k2".to_string()],
    };
    assert!(serde_json::to_value(&listed).is_ok());

    let err = KeyOperationResult::Error {
        message: "failed".to_string(),
    };
    assert!(serde_json::to_value(&err).is_ok());

    let retrieved = KeyOperationResult::Retrieved {
        key_id: "k1".to_string(),
        key_material: vec![0, 1, 2],
        algorithm: "aes".to_string(),
    };
    let json = serde_json::to_value(&retrieved).unwrap();
    let back: KeyOperationResult = serde_json::from_value(json).unwrap();
    assert!(matches!(back, KeyOperationResult::Retrieved { .. }));
}

#[test]
fn test_encryption_operation_serde_roundtrip() {
    for op in [EncryptionOperation::Encrypt, EncryptionOperation::Decrypt] {
        let json = serde_json::to_string(&op).unwrap();
        let _: EncryptionOperation = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_bear_dog_endpoint_serde_roundtrip() {
    let endpoint = BearDogEndpoint {
        service_id: "ep-1".to_string(),
        protocol: "unix".to_string(),
        address: "127.0.0.1:9000".parse().unwrap(),
        api_version: "v2".to_string(),
        capabilities: vec![
            BearDogCapability::Encryption {
                algorithms: vec!["aes-256-gcm".to_string()],
            },
            BearDogCapability::KeyManagement,
        ],
        healthy: false,
        latency_ms: None,
    };
    let json = serde_json::to_string(&endpoint).unwrap();
    let parsed: BearDogEndpoint = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.service_id, endpoint.service_id);
    assert_eq!(parsed.capabilities.len(), 2);
}

#[test]
fn test_bear_dog_capability_all_variants_serde() {
    let custom = BearDogCapability::Custom("my-cap".to_string());
    let json = serde_json::to_value(&custom).unwrap();
    let back: BearDogCapability = serde_json::from_value(json).unwrap();
    assert!(matches!(back, BearDogCapability::Custom(s) if s == "my-cap"));

    for cap in [
        BearDogCapability::KeyManagement,
        BearDogCapability::HardwareSecurity,
        BearDogCapability::SecureStorage,
        BearDogCapability::GeneticEntropy,
    ] {
        let json = serde_json::to_value(&cap).unwrap();
        let _: BearDogCapability = serde_json::from_value(json).unwrap();
    }
}

#[test]
fn test_security_level_serde_roundtrip() {
    for level in [
        SecurityLevel::Standard,
        SecurityLevel::Enhanced,
        SecurityLevel::HardwareSecured,
    ] {
        let json = serde_json::to_string(&level).unwrap();
        let _: SecurityLevel = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_key_operation_variants_serde() {
    for op in [
        KeyOperation::Generate,
        KeyOperation::Get,
        KeyOperation::Delete,
        KeyOperation::List,
    ] {
        let json = serde_json::to_value(&op).unwrap();
        let _: KeyOperation = serde_json::from_value(json).unwrap();
    }
}

#[test]
fn test_encryption_request_roundtrip() {
    let request = EncryptionRequest {
        request_id: uuid::Uuid::new_v4(),
        operation: EncryptionOperation::Decrypt,
        data: vec![0xFF, 0xFE],
        key_id: None,
        algorithm: Some("chacha20".to_string()),
        security_level: SecurityLevel::HardwareSecured,
    };
    let json = serde_json::to_string(&request).unwrap();
    let parsed: EncryptionRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.data, request.data);
    assert_eq!(parsed.security_level, SecurityLevel::HardwareSecured);
}

#[test]
fn test_key_management_response_generated_roundtrip() {
    let resp = KeyManagementResponse {
        request_id: uuid::Uuid::new_v4(),
        result: KeyOperationResult::Generated {
            key_id: "gen-123".to_string(),
            algorithm: "aes-256-gcm".to_string(),
        },
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: KeyManagementResponse = serde_json::from_str(&json).unwrap();
    match &parsed.result {
        KeyOperationResult::Generated { key_id, algorithm } => {
            assert_eq!(key_id, "gen-123");
            assert_eq!(algorithm, "aes-256-gcm");
        }
        _ => panic!("expected Generated"),
    }
}

#[test]
fn test_key_management_response_parsing_error() {
    let resp = KeyManagementResponse {
        request_id: uuid::Uuid::new_v4(),
        result: KeyOperationResult::Error {
            message: "key not found".to_string(),
        },
    };
    let json = serde_json::to_value(&resp).unwrap();
    let parsed: KeyManagementResponse = serde_json::from_value(json).unwrap();
    match &parsed.result {
        KeyOperationResult::Error { message } => assert_eq!(message, "key not found"),
        _ => panic!("expected Error"),
    }
}

#[test]
fn test_encryption_response_full_roundtrip_with_metadata() {
    let resp = EncryptionResponse {
        request_id: uuid::Uuid::new_v4(),
        data: vec![0x01, 0x02, 0x03, 0x04, 0x05],
        key_id: "key-789".to_string(),
        algorithm: "aes-256-gcm".to_string(),
        metadata: serde_json::json!({"nonce": "abc123", "tag": "xyz"}),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: EncryptionResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.data, resp.data);
    assert_eq!(parsed.metadata["nonce"], "abc123");
}

#[test]
fn test_signature_response_roundtrip() {
    use crate::beardog_integration::types::SignatureResponse;
    let resp = SignatureResponse {
        request_id: uuid::Uuid::new_v4(),
        signature: vec![0xDE, 0xAD],
        key_id: "sig-key".to_string(),
        algorithm: "ed25519".to_string(),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: SignatureResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.signature, resp.signature);
}

#[test]
fn test_revocation_request_roundtrip() {
    use crate::beardog_integration::types::RevocationRequest;
    let req = RevocationRequest {
        reason: "user request".to_string(),
    };
    let json = serde_json::to_string(&req).unwrap();
    let parsed: RevocationRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.reason, req.reason);
}

#[test]
fn test_permission_response_roundtrip() {
    use crate::beardog_integration::types::PermissionResponse;
    let resp = PermissionResponse {
        request_id: uuid::Uuid::new_v4(),
        permission_id: uuid::Uuid::new_v4(),
        proof: vec![1, 2, 3, 4, 5],
        metadata: serde_json::json!({"scope": "read"}),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: PermissionResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.proof, resp.proof);
}

#[test]
fn test_verification_response_roundtrip() {
    use crate::beardog_integration::types::VerificationResponse;
    let resp = VerificationResponse {
        request_id: uuid::Uuid::new_v4(),
        valid: true,
        details: Some("signature valid".to_string()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: VerificationResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.valid, resp.valid);
    assert_eq!(parsed.details, resp.details);
}

#[test]
fn test_validation_response_valid_false() {
    use crate::beardog_integration::types::ValidationResponse;
    let resp = ValidationResponse {
        request_id: uuid::Uuid::new_v4(),
        valid: false,
        details: Some("expired".to_string()),
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: ValidationResponse = serde_json::from_str(&json).unwrap();
    assert!(!parsed.valid);
}

#[test]
fn test_validation_response_details_none() {
    use crate::beardog_integration::types::ValidationResponse;
    let resp = ValidationResponse {
        request_id: uuid::Uuid::new_v4(),
        valid: true,
        details: None,
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: ValidationResponse = serde_json::from_str(&json).unwrap();
    assert!(parsed.valid);
    assert!(parsed.details.is_none());
}

#[test]
fn test_key_operation_result_deleted_serde_roundtrip() {
    let deleted = KeyOperationResult::Deleted {
        key_id: "k-deleted".to_string(),
    };
    let json = serde_json::to_value(&deleted).unwrap();
    let back: KeyOperationResult = serde_json::from_value(json).unwrap();
    assert!(matches!(back, KeyOperationResult::Deleted { key_id } if key_id == "k-deleted"));
}

#[test]
fn test_key_operation_result_listed_serde() {
    let listed = KeyOperationResult::Listed {
        keys: vec!["k1".to_string(), "k2".to_string()],
    };
    let json = serde_json::to_value(&listed).unwrap();
    let back: KeyOperationResult = serde_json::from_value(json).unwrap();
    if let KeyOperationResult::Listed { keys } = back {
        assert_eq!(keys.len(), 2);
    } else {
        panic!("expected Listed");
    }
}

#[test]
fn test_bear_dog_endpoint_debug_clone() {
    let ep = BearDogEndpoint {
        service_id: "ep-1".to_string(),
        protocol: "unix".to_string(),
        address: "127.0.0.1:9000".parse().unwrap(),
        api_version: "v1".to_string(),
        capabilities: vec![BearDogCapability::KeyManagement],
        healthy: true,
        latency_ms: Some(1),
    };
    let ep2 = ep.clone();
    assert_eq!(ep.service_id, ep2.service_id);
    assert!(format!("{ep:?}").contains("ep-1"));
}

#[test]
fn test_bear_dog_config_debug_and_fields() {
    let config = crate::beardog_integration::BearDogConfig {
        auto_discover: true,
        discovery_timeout_ms: 3000,
        preferred_location: ServiceLocation::Any,
        fallback_enabled: true,
    };
    let dbg = format!("{config:?}");
    assert!(!dbg.is_empty());
    assert_eq!(config.discovery_timeout_ms, 3000);
}

#[test]
fn test_service_location_all_variants_eq() {
    assert_eq!(ServiceLocation::Local, ServiceLocation::Local);
    assert_eq!(ServiceLocation::Network, ServiceLocation::Network);
    assert_eq!(ServiceLocation::Any, ServiceLocation::Any);
    assert_ne!(ServiceLocation::Local, ServiceLocation::Network);
}

#[test]
fn test_encryption_operation_all_variants() {
    let enc = EncryptionOperation::Encrypt;
    let dec = EncryptionOperation::Decrypt;
    assert_eq!(enc, EncryptionOperation::Encrypt);
    assert_ne!(enc, dec);
}

#[test]
fn test_key_management_response_deleted_roundtrip() {
    let resp = KeyManagementResponse {
        request_id: uuid::Uuid::new_v4(),
        result: KeyOperationResult::Deleted {
            key_id: "del-123".to_string(),
        },
    };
    let json = serde_json::to_string(&resp).unwrap();
    let parsed: KeyManagementResponse = serde_json::from_str(&json).unwrap();
    assert!(matches!(parsed.result, KeyOperationResult::Deleted { .. }));
}

#[test]
fn test_toadstool_error_not_found_display() {
    let err = toadstool_common::ToadStoolError::not_found("No BearDog endpoints");
    assert!(err.to_string().to_lowercase().contains("bear"));
}

#[test]
fn test_toadstool_error_configuration_display() {
    let err = toadstool_common::ToadStoolError::configuration("Config invalid");
    assert!(err.to_string().to_lowercase().contains("config"));
}

#[test]
fn test_toadstool_error_runtime_display() {
    let err = toadstool_common::ToadStoolError::runtime("Runtime failure");
    assert!(!err.to_string().is_empty());
}

#[test]
fn test_toadstool_error_network_display() {
    let err = toadstool_common::ToadStoolError::network("Network error");
    assert!(!err.to_string().is_empty());
}

#[test]
fn test_toadstool_error_security_display() {
    let err = toadstool_common::ToadStoolError::security("Security violation");
    assert!(!err.to_string().is_empty());
}
