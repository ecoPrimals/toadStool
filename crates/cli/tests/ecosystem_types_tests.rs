//! Comprehensive tests for CLI ecosystem types
//!
//! ⚠️ These tests verify backward compatibility with the deprecated `EcosystemService` enum.
//! The deprecated enum hardcodes service names, violating infant discovery principles.
//! Production code should use the capability-based `ServiceType` instead.

#![allow(deprecated)] // Testing backward compatibility with deprecated EcosystemService

use chrono::Utc;
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
fn test_ecosystem_service_songbird() {
    let service = EcosystemService::Songbird;
    assert!(matches!(service, EcosystemService::Songbird));
}

#[test]
fn test_ecosystem_service_beardog() {
    let service = EcosystemService::BearDog;
    assert!(matches!(service, EcosystemService::BearDog));
}

#[test]
fn test_ecosystem_service_nestgate() {
    let service = EcosystemService::NestGate;
    assert!(matches!(service, EcosystemService::NestGate));
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
    let service = EcosystemService::Songbird;
    let cloned = service.clone();
    assert!(matches!(cloned, EcosystemService::Songbird));
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
fn test_service_type_songbird() {
    let service_type = ServiceType::Songbird;
    assert!(matches!(service_type, ServiceType::Songbird));
}

#[test]
fn test_service_type_beardog() {
    let service_type = ServiceType::BearDog;
    assert!(matches!(service_type, ServiceType::BearDog));
}

#[test]
fn test_service_type_nestgate() {
    let service_type = ServiceType::NestGate;
    assert!(matches!(service_type, ServiceType::NestGate));
}

#[test]
fn test_service_type_toadstool() {
    let service_type = ServiceType::ToadStool;
    assert!(matches!(service_type, ServiceType::ToadStool));
}

#[test]
fn test_service_type_clone() {
    let service_type = ServiceType::Songbird;
    let cloned = service_type;
    assert!(matches!(cloned, ServiceType::Songbird));
}

// ============================================================================
// ServiceEndpoint Tests
// ============================================================================

#[test]
fn test_service_endpoint_creation() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Songbird,
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
        service_type: EcosystemService::BearDog,
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
        service_type: EcosystemService::Songbird,
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
        valid_until: Utc::now() + chrono::Duration::hours(24),
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
        valid_until: Utc::now() + chrono::Duration::days(7),
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
        valid_until: Utc::now(),
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
        service_type: ServiceType::Songbird,
        address: "10.0.0.1:8080".parse().unwrap(),
        trust_level: TrustLevel::Discovered,
        capabilities: HashMap::new(),
        last_seen: Utc::now(),
    };

    assert!(matches!(service.service_type, ServiceType::Songbird));
    assert!(matches!(service.trust_level, TrustLevel::Discovered));
}

#[test]
fn test_discovered_service_with_capabilities() {
    let mut capabilities = HashMap::new();
    capabilities.insert("version".to_string(), "1.0.0".to_string());
    capabilities.insert("protocol".to_string(), "http".to_string());

    let service = DiscoveredService {
        service_type: ServiceType::BearDog,
        address: "192.168.1.100:9000".parse().unwrap(),
        trust_level: TrustLevel::Verified,
        capabilities,
        last_seen: Utc::now(),
    };

    assert_eq!(service.capabilities.len(), 2);
    assert_eq!(service.capabilities.get("version").unwrap(), "1.0.0");
}

#[test]
fn test_discovered_service_clone() {
    let service = DiscoveredService {
        service_type: ServiceType::NestGate,
        address: "127.0.0.1:7000".parse().unwrap(),
        trust_level: TrustLevel::Sovereign,
        capabilities: HashMap::new(),
        last_seen: Utc::now(),
    };

    let cloned = service.clone();
    assert!(matches!(cloned.service_type, ServiceType::NestGate));
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
        timestamp: Utc::now(),
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
        timestamp: Utc::now(),
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
        EcosystemService::Songbird,
        EcosystemService::BearDog,
        EcosystemService::NestGate,
        EcosystemService::Unknown("Test".to_string()),
    ];

    assert_eq!(services.len(), 4);
}

#[test]
fn test_all_service_types() {
    let types = [
        ServiceType::Songbird,
        ServiceType::BearDog,
        ServiceType::NestGate,
        ServiceType::ToadStool,
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
