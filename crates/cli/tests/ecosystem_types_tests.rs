// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for CLI ecosystem types
//!
//! ⚠️ These tests verify backward compatibility with the deprecated `EcosystemService` enum.
//! The deprecated enum hardcodes service names, violating infant discovery principles.
//! Production code should use the capability-based `ServiceType` instead.

#![allow(deprecated)] // Testing backward compatibility with deprecated EcosystemService

use base64::Engine;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use toadstool_cli::ecosystem::*;
use uuid::Uuid;

// ============================================================================
// EcosystemService Tests
// ============================================================================

#[test]
fn test_ecosystem_service_discovery() {
    let service = EcosystemService::Discovery;
    assert!(matches!(service, EcosystemService::Discovery));
}

#[test]
fn test_ecosystem_service_crypto() {
    let service = EcosystemService::Crypto;
    assert!(matches!(service, EcosystemService::Crypto));
}

#[test]
fn test_ecosystem_service_storage() {
    let service = EcosystemService::Storage;
    assert!(matches!(service, EcosystemService::Storage));
}

#[test]
fn test_ecosystem_service_unknown() {
    let service = EcosystemService::Unknown("CustomService".to_string());

    if let EcosystemService::Unknown(name) = service {
        assert_eq!(name, "CustomService");
    } else {
        panic!("Expected Unknown variant");
    }
}

#[test]
fn test_ecosystem_service_clone() {
    let service = EcosystemService::Discovery;
    let cloned = service.clone();
    assert!(matches!(cloned, EcosystemService::Discovery));
}

// ============================================================================
// TrustLevel Tests (Ecosystem)
// ============================================================================

#[test]
fn test_trust_level_unknown_ecosystem() {
    let level = TrustLevel::Unknown;
    assert!(matches!(level, TrustLevel::Unknown));
}

#[test]
fn test_trust_level_discovered() {
    let level = TrustLevel::Discovered;
    assert!(matches!(level, TrustLevel::Discovered));
}

#[test]
fn test_trust_level_advertised() {
    let level = TrustLevel::Advertised;
    assert!(matches!(level, TrustLevel::Advertised));
}

#[test]
fn test_trust_level_verified_ecosystem() {
    let level = TrustLevel::Verified;
    assert!(matches!(level, TrustLevel::Verified));
}

#[test]
fn test_trust_level_sovereign_ecosystem() {
    let level = TrustLevel::Sovereign;
    assert!(matches!(level, TrustLevel::Sovereign));
}

#[test]
fn test_trust_level_progression_ecosystem() {
    let levels = [
        TrustLevel::Unknown,
        TrustLevel::Discovered,
        TrustLevel::Advertised,
        TrustLevel::Verified,
        TrustLevel::Sovereign,
    ];

    assert_eq!(levels.len(), 5);
}

// ============================================================================
// ServiceType Tests
// ============================================================================

#[test]
fn test_service_type_discovery() {
    let service_type = ServiceType::Discovery;
    assert!(matches!(service_type, ServiceType::Discovery));
}

#[test]
fn test_service_type_crypto() {
    let service_type = ServiceType::Crypto;
    assert!(matches!(service_type, ServiceType::Crypto));
}

#[test]
fn test_service_type_storage() {
    let service_type = ServiceType::Storage;
    assert!(matches!(service_type, ServiceType::Storage));
}

#[test]
fn test_service_type_compute() {
    let service_type = ServiceType::Compute;
    assert!(matches!(service_type, ServiceType::Compute));
}

#[test]
fn test_service_type_clone() {
    let service_type = ServiceType::Discovery;
    let cloned = service_type;
    assert!(matches!(cloned, ServiceType::Discovery));
}

// ============================================================================
// ServiceEndpoint Tests
// ============================================================================

#[test]
fn test_service_endpoint_creation() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Discovery,
        address: "127.0.0.1:8080".parse().unwrap(),
        version: Arc::from("1.0.0"),
        capabilities: vec!["discovery".to_string(), "routing".to_string()],
        trust_level: TrustLevel::Verified,
    };

    assert_eq!(endpoint.capabilities.len(), 2);
    assert_eq!(endpoint.version, Arc::from("1.0.0"));
}

#[test]
fn test_service_endpoint_clone() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Crypto,
        address: "192.168.1.1:9000".parse().unwrap(),
        version: Arc::from("2.0.0"),
        capabilities: vec![],
        trust_level: TrustLevel::Sovereign,
    };

    let cloned = endpoint.clone();
    assert_eq!(endpoint.version, cloned.version);
}

// ============================================================================
// DiscoveryResult Tests
// ============================================================================

#[test]
fn test_discovery_result_empty() {
    let result = DiscoveryResult {
        services: vec![],
        scan_duration: Duration::from_secs(5),
        total_discovered: 0,
        verified_count: 0,
    };

    assert_eq!(result.services.len(), 0);
    assert_eq!(result.total_discovered, 0);
}

#[test]
fn test_discovery_result_with_services() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Discovery,
        address: "127.0.0.1:8080".parse().unwrap(),
        version: Arc::from("1.0.0"),
        capabilities: vec![],
        trust_level: TrustLevel::Discovered,
    };

    let result = DiscoveryResult {
        services: vec![endpoint],
        scan_duration: Duration::from_secs(10),
        total_discovered: 1,
        verified_count: 0,
    };

    assert_eq!(result.services.len(), 1);
    assert_eq!(result.total_discovered, 1);
}

#[test]
fn test_discovery_result_verified_ratio() {
    let result = DiscoveryResult {
        services: vec![],
        scan_duration: Duration::from_secs(3),
        total_discovered: 10,
        verified_count: 7,
    };

    let ratio = result.verified_count as f64 / result.total_discovered as f64;
    assert!((ratio - 0.7).abs() < 0.01);
}

// ============================================================================
// BearDogPermission Tests
// ============================================================================

#[test]
fn test_beardog_permission_creation() {
    let permission = BearDogPermission {
        permission_id: Uuid::new_v4(),
        granted_to: "user@example.com".to_string(),
        capabilities: vec!["read".to_string(), "write".to_string()],
        valid_until: std::time::SystemTime::now() + std::time::Duration::from_secs(24 * 3600),
        signature: "sig123".to_string(),
    };

    assert_eq!(permission.granted_to, "user@example.com");
    assert_eq!(permission.capabilities.len(), 2);
}

#[test]
fn test_beardog_permission_with_multiple_capabilities() {
    let permission = BearDogPermission {
        permission_id: Uuid::new_v4(),
        granted_to: "admin@example.com".to_string(),
        capabilities: vec![
            "read".to_string(),
            "write".to_string(),
            "execute".to_string(),
            "admin".to_string(),
        ],
        valid_until: std::time::SystemTime::now() + std::time::Duration::from_secs(7 * 24 * 3600),
        signature: "admin-sig-456".to_string(),
    };

    assert_eq!(permission.capabilities.len(), 4);
    assert!(permission.capabilities.contains(&"admin".to_string()));
}

#[test]
fn test_beardog_permission_clone() {
    let permission = BearDogPermission {
        permission_id: Uuid::new_v4(),
        granted_to: "test@test.com".to_string(),
        capabilities: vec![],
        valid_until: std::time::SystemTime::now(),
        signature: "test-sig".to_string(),
    };

    let cloned = permission.clone();
    assert_eq!(permission.permission_id, cloned.permission_id);
}

// ============================================================================
// NestGateMount Tests
// ============================================================================

#[test]
fn test_nestgate_mount_creation() {
    let mount = NestGateMount {
        dataset_name: "my-dataset".to_string(),
        mount_point: PathBuf::from("/mnt/data"),
        endpoint: "http://nestgate:8080".to_string(),
        zfs_dataset: Some("tank/dataset".to_string()),
        access_mode: "read".to_string(),
        encryption_key: None,
    };

    assert_eq!(mount.dataset_name, "my-dataset");
    assert_eq!(mount.access_mode, "read");
}

#[test]
fn test_nestgate_mount_with_encryption() {
    let mount = NestGateMount {
        dataset_name: "secure-data".to_string(),
        mount_point: PathBuf::from("/mnt/secure"),
        endpoint: "https://nestgate:8443".to_string(),
        zfs_dataset: None,
        access_mode: "write".to_string(),
        encryption_key: Some("enc-key-123".to_string()),
    };

    assert!(mount.encryption_key.is_some());
    assert_eq!(mount.access_mode, "write");
}

#[test]
fn test_nestgate_mount_admin_mode() {
    let mount = NestGateMount {
        dataset_name: "admin-pool".to_string(),
        mount_point: PathBuf::from("/admin"),
        endpoint: "http://localhost:9000".to_string(),
        zfs_dataset: Some("admin/pool".to_string()),
        access_mode: "admin".to_string(),
        encryption_key: None,
    };

    assert_eq!(mount.access_mode, "admin");
}

// ============================================================================
// DiscoveredService Tests
// ============================================================================

#[test]
fn test_discovered_service_creation() {
    let service = DiscoveredService {
        service_type: ServiceType::Discovery,
        address: "10.0.0.1:8080".parse().unwrap(),
        trust_level: TrustLevel::Discovered,
        capabilities: HashMap::new(),
        last_seen: std::time::SystemTime::now(),
    };

    assert!(matches!(service.service_type, ServiceType::Discovery));
    assert!(matches!(service.trust_level, TrustLevel::Discovered));
}

#[test]
fn test_discovered_service_with_capabilities() {
    let mut capabilities = HashMap::new();
    capabilities.insert("version".to_string(), "1.0.0".to_string());
    capabilities.insert("protocol".to_string(), "http".to_string());

    let service = DiscoveredService {
        service_type: ServiceType::Crypto,
        address: "192.168.1.100:9000".parse().unwrap(),
        trust_level: TrustLevel::Verified,
        capabilities,
        last_seen: std::time::SystemTime::now(),
    };

    assert_eq!(service.capabilities.len(), 2);
    assert_eq!(service.capabilities.get("version").unwrap(), "1.0.0");
}

#[test]
fn test_discovered_service_clone() {
    let service = DiscoveredService {
        service_type: ServiceType::Storage,
        address: "127.0.0.1:7000".parse().unwrap(),
        trust_level: TrustLevel::Sovereign,
        capabilities: HashMap::new(),
        last_seen: std::time::SystemTime::now(),
    };

    let cloned = service.clone();
    assert!(matches!(cloned.service_type, ServiceType::Storage));
}

// ============================================================================
// ServiceSignature Tests
// ============================================================================

#[test]
fn test_service_signature_creation() {
    let signature = ServiceSignature {
        algorithm: "Ed25519".to_string(),
        signature: "base64-signature".to_string(),
        public_key: "public-key-123".to_string(),
        timestamp: std::time::SystemTime::now(),
        nonce: "nonce-456".to_string(),
    };

    assert_eq!(signature.algorithm, "Ed25519");
    assert!(!signature.signature.is_empty());
}

#[test]
fn test_service_signature_with_different_algorithm() {
    let signature = ServiceSignature {
        algorithm: "RSA-2048".to_string(),
        signature: "rsa-sig".to_string(),
        public_key: "rsa-pubkey".to_string(),
        timestamp: std::time::SystemTime::now(),
        nonce: "random-nonce".to_string(),
    };

    assert_eq!(signature.algorithm, "RSA-2048");
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_all_ecosystem_services() {
    let services = [
        EcosystemService::Discovery,
        EcosystemService::Crypto,
        EcosystemService::Storage,
        EcosystemService::Unknown("Test".to_string()),
    ];

    assert_eq!(services.len(), 4);
}

#[test]
fn test_all_service_types() {
    let types = [
        ServiceType::Discovery,
        ServiceType::Crypto,
        ServiceType::Storage,
        ServiceType::Compute,
    ];

    assert_eq!(types.len(), 4);
}

#[test]
fn test_trust_level_hierarchy() {
    // Trust levels should progress from Unknown to Sovereign
    let levels = [
        TrustLevel::Unknown,
        TrustLevel::Discovered,
        TrustLevel::Advertised,
        TrustLevel::Verified,
        TrustLevel::Sovereign,
    ];

    assert_eq!(levels.len(), 5);
    assert!(matches!(levels[0], TrustLevel::Unknown));
    assert!(matches!(levels[4], TrustLevel::Sovereign));
}

// ============================================================================
// ServiceType::from_capability - all variants
// ============================================================================

#[test]
fn test_service_type_from_capability_discovery() {
    assert!(matches!(
        ServiceType::from_capability("discovery"),
        ServiceType::Discovery
    ));
    assert!(matches!(
        ServiceType::from_capability("orchestration"),
        ServiceType::Discovery
    ));
    assert!(matches!(
        ServiceType::from_capability("coordination"),
        ServiceType::Discovery
    ));
}

#[test]
fn test_service_type_from_capability_crypto() {
    assert!(matches!(
        ServiceType::from_capability("crypto"),
        ServiceType::Crypto
    ));
    assert!(matches!(
        ServiceType::from_capability("pki"),
        ServiceType::Crypto
    ));
    assert!(matches!(
        ServiceType::from_capability("security"),
        ServiceType::Crypto
    ));
}

#[test]
fn test_service_type_from_capability_storage() {
    assert!(matches!(
        ServiceType::from_capability("storage"),
        ServiceType::Storage
    ));
}

#[test]
fn test_service_type_from_capability_compute() {
    assert!(matches!(
        ServiceType::from_capability("compute"),
        ServiceType::Compute
    ));
    assert!(matches!(
        ServiceType::from_capability("compute:execution"),
        ServiceType::Compute
    ));
}

#[test]
fn test_service_type_from_capability_generic() {
    assert!(matches!(
        ServiceType::from_capability("unknown"),
        ServiceType::Generic
    ));
    assert!(matches!(
        ServiceType::from_capability("custom"),
        ServiceType::Generic
    ));
}

// ============================================================================
// ServiceType::from_name - legacy primal names
// ============================================================================

#[test]
fn test_service_type_from_name_songbird() {
    assert!(matches!(
        ServiceType::from_name("songbird"),
        ServiceType::Discovery
    ));
}

#[test]
fn test_service_type_from_name_beardog() {
    assert!(matches!(
        ServiceType::from_name("beardog"),
        ServiceType::Crypto
    ));
}

#[test]
fn test_service_type_from_name_nestgate() {
    assert!(matches!(
        ServiceType::from_name("nestgate"),
        ServiceType::Storage
    ));
}

#[test]
fn test_service_type_from_name_toadstool() {
    assert!(matches!(
        ServiceType::from_name("toadstool"),
        ServiceType::Compute
    ));
}

#[test]
fn test_service_type_from_name_squirrel() {
    assert!(matches!(
        ServiceType::from_name("squirrel"),
        ServiceType::Compute
    ));
}

#[test]
fn test_service_type_from_name_case_insensitive() {
    assert!(matches!(
        ServiceType::from_name("SONGBIRD"),
        ServiceType::Discovery
    ));
    assert!(matches!(
        ServiceType::from_name("BearDog"),
        ServiceType::Crypto
    ));
}

// ============================================================================
// CryptoVerificationContext - verify_service_signature paths
// ============================================================================

#[test]
fn test_verify_service_signature_response_too_old() {
    use std::time::Duration;

    let ctx = CryptoVerificationContext::new().with_trusted_key(
        "test",
        &base64::engine::general_purpose::STANDARD.encode([0u8; 32]),
    );
    let old_timestamp = std::time::SystemTime::now() - Duration::from_secs(600);
    let response = SignedServiceResponse {
        service_id: "svc-1".to_string(),
        service_type: "test".to_string(),
        status: "ok".to_string(),
        capabilities: vec![],
        timestamp: old_timestamp,
        signature: ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: base64::engine::general_purpose::STANDARD.encode([0u8; 64]),
            public_key: "key".to_string(),
            timestamp: old_timestamp,
            nonce: "nonce".to_string(),
        },
    };
    let result = ctx.verify_service_signature("test", &response);
    assert!(result.is_ok());
    assert!(!result.unwrap());
}

#[test]
fn test_verify_service_signature_invalid_base64_public_key() {
    let ctx = CryptoVerificationContext::new().with_trusted_key("test", "not-valid-base64!!!");
    let response = SignedServiceResponse {
        service_id: "svc-1".to_string(),
        service_type: "test".to_string(),
        status: "ok".to_string(),
        capabilities: vec![],
        timestamp: std::time::SystemTime::now(),
        signature: ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "sig".to_string(),
            public_key: "key".to_string(),
            timestamp: std::time::SystemTime::now(),
            nonce: "nonce".to_string(),
        },
    };
    let result = ctx.verify_service_signature("test", &response);
    assert!(result.is_err());
}

#[test]
fn test_verify_service_signature_invalid_base64_signature() {
    let ctx = CryptoVerificationContext::new().with_trusted_key(
        "test",
        &base64::engine::general_purpose::STANDARD.encode([0u8; 32]),
    );
    let response = SignedServiceResponse {
        service_id: "svc-1".to_string(),
        service_type: "test".to_string(),
        status: "ok".to_string(),
        capabilities: vec![],
        timestamp: std::time::SystemTime::now(),
        signature: ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "!!!invalid-base64!!!".to_string(),
            public_key: "key".to_string(),
            timestamp: std::time::SystemTime::now(),
            nonce: "nonce".to_string(),
        },
    };
    let result = ctx.verify_service_signature("test", &response);
    assert!(result.is_err());
}

#[test]
fn test_crypto_verification_context_default_with_env_vars() {
    temp_env::with_vars(
        [
            ("CRYPTO_PROVIDER_PUBLIC_KEY", Some("dGVzdA==")),
            ("STORAGE_PROVIDER_PUBLIC_KEY", None::<&str>),
            ("DISCOVERY_PROVIDER_PUBLIC_KEY", None::<&str>),
        ],
        || {
            let ctx = CryptoVerificationContext::default();
            assert!(ctx.trusted_public_keys.contains_key("crypto"));
            assert!(ctx.trusted_public_keys.contains_key("beardog"));
        },
    );
}

#[test]
fn test_service_type_to_capability_all() {
    assert_eq!(ServiceType::Discovery.to_capability(), "discovery");
    assert_eq!(ServiceType::Crypto.to_capability(), "crypto");
    assert_eq!(ServiceType::Storage.to_capability(), "storage");
    assert_eq!(ServiceType::Compute.to_capability(), "compute");
    assert_eq!(ServiceType::Generic.to_capability(), "generic");
}

// ============================================================================
// CryptoVerificationContext::with_trusted_key builder
// ============================================================================

#[test]
fn test_crypto_verification_context_with_trusted_key() {
    let ctx = CryptoVerificationContext::new()
        .with_trusted_key("myservice", "dGVzdA==")
        .with_trusted_key("other", "YWJj");
    assert!(ctx.trusted_public_keys.contains_key("myservice"));
    assert!(ctx.trusted_public_keys.contains_key("other"));
}

// ============================================================================
// SignedServiceResponse and create_canonical_message (via verify_service_signature)
// ============================================================================

#[test]
fn test_verify_service_signature_exercises_canonical_message() {
    let valid_32 = base64::engine::general_purpose::STANDARD.encode([1u8; 32]);
    let valid_64 = base64::engine::general_purpose::STANDARD.encode([0u8; 64]);
    let ctx = CryptoVerificationContext::new().with_trusted_key("test", &valid_32);
    let response = SignedServiceResponse {
        service_id: "svc-1".to_string(),
        service_type: "test".to_string(),
        status: "ok".to_string(),
        capabilities: vec!["read".to_string()],
        timestamp: std::time::SystemTime::now(),
        signature: ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: valid_64,
            public_key: "key".to_string(),
            timestamp: std::time::SystemTime::now(),
            nonce: "nonce".to_string(),
        },
    };
    let result = ctx.verify_service_signature("test", &response);
    assert!(result.is_ok());
}

#[test]
fn test_signed_service_response_serde_roundtrip() {
    let response = SignedServiceResponse {
        service_id: "svc-1".to_string(),
        service_type: "crypto".to_string(),
        status: "ok".to_string(),
        capabilities: vec!["encrypt".to_string()],
        timestamp: std::time::SystemTime::now(),
        signature: ServiceSignature {
            algorithm: "ed25519".to_string(),
            signature: "sig".to_string(),
            public_key: "pubkey".to_string(),
            timestamp: std::time::SystemTime::now(),
            nonce: "n".to_string(),
        },
    };
    let json = serde_json::to_string(&response).unwrap();
    let parsed: SignedServiceResponse = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.service_id, response.service_id);
}

#[test]
fn test_crypto_verification_context_new() {
    let ctx = CryptoVerificationContext::new();
    assert!(ctx.revoked_keys.is_empty());
    assert_eq!(ctx.max_age_minutes, 5);
}
