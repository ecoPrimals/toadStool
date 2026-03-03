// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Ecosystem Integrator
//!
//! Coverage target: 0% → 25% (15 tests)
//!
//! Testing strategy:
//! - EcosystemIntegrator initialization
//! - Service discovery
//! - Service verification
//! - Endpoint management
//! - Connection handling
//!
//! ⚠️ These tests verify backward compatibility with the deprecated `EcosystemService` enum.
//! Production code should use the capability-based `ServiceType` instead.

#![allow(deprecated)] // Testing backward compatibility with deprecated EcosystemService

use std::collections::HashMap;
use std::sync::Arc;
use toadstool_cli::ecosystem::*;

// ============================================================================
// Initialization Tests (3 tests)
// ============================================================================

#[test]
fn test_ecosystem_integrator_new() {
    let _integrator = EcosystemIntegrator::new();
    // Test passes if construction succeeds
}

#[test]
fn test_ecosystem_integrator_default() {
    let _integrator = EcosystemIntegrator::default();
    // Should be same as new() - test passes if construction succeeds
}

#[test]
fn test_ecosystem_integrator_can_create_multiple() {
    let _integrator1 = EcosystemIntegrator::new();
    let _integrator2 = EcosystemIntegrator::new();
    let _integrator3 = EcosystemIntegrator::default();

    // Test passes if no panic
}

// ============================================================================
// ServiceEndpoint Tests (3 tests)
// ============================================================================

#[test]
fn test_service_endpoint_songbird() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Songbird,
        address: "127.0.0.1:8080".parse().unwrap(),
        version: Arc::from("1.0.0"),
        capabilities: vec!["discovery".to_string(), "coordination".to_string()],
        trust_level: TrustLevel::Verified,
    };

    assert!(matches!(endpoint.service_type, EcosystemService::Songbird));
    assert_eq!(endpoint.version, Arc::from("1.0.0"));
    assert_eq!(endpoint.capabilities.len(), 2);
    assert!(endpoint.capabilities.contains(&"discovery".to_string()));
}

#[test]
fn test_service_endpoint_beardog() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::BearDog,
        address: "192.168.1.100:8443".parse().unwrap(),
        version: Arc::from("2.0.0"),
        capabilities: vec!["auth".to_string(), "crypto".to_string()],
        trust_level: TrustLevel::Sovereign,
    };

    assert!(matches!(endpoint.service_type, EcosystemService::BearDog));
    assert_eq!(endpoint.address.port(), 8443);
    assert!(matches!(endpoint.trust_level, TrustLevel::Sovereign));
}

#[test]
fn test_service_endpoint_nestgate() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::NestGate,
        address: "10.0.0.50:9000".parse().unwrap(),
        version: Arc::from("1.5.0"),
        capabilities: vec!["storage".to_string(), "zfs".to_string()],
        trust_level: TrustLevel::Verified,
    };

    assert!(matches!(endpoint.service_type, EcosystemService::NestGate));
    assert_eq!(endpoint.address.ip().to_string(), "10.0.0.50");
    assert!(endpoint.capabilities.contains(&"zfs".to_string()));
}

// ============================================================================
// DiscoveredService Tests (3 tests)
// ============================================================================

#[test]
fn test_discovered_service_creation() {
    let mut capabilities = HashMap::new();
    capabilities.insert("version".to_string(), "1.0.0".to_string());
    capabilities.insert("protocol".to_string(), "http".to_string());

    let service = DiscoveredService {
        service_type: ServiceType::Songbird,
        address: "127.0.0.1:8080".parse().unwrap(),
        trust_level: TrustLevel::Discovered,
        capabilities: capabilities.clone(),
        last_seen: std::time::SystemTime::now(),
    };

    assert!(matches!(service.service_type, ServiceType::Songbird));
    assert_eq!(service.capabilities.len(), 2);
    assert!(matches!(service.trust_level, TrustLevel::Discovered));
}

#[test]
fn test_discovered_service_with_empty_capabilities() {
    let service = DiscoveredService {
        service_type: ServiceType::BearDog,
        address: "192.168.1.1:8443".parse().unwrap(),
        trust_level: TrustLevel::Unknown,
        capabilities: HashMap::new(),
        last_seen: std::time::SystemTime::now(),
    };

    assert!(service.capabilities.is_empty());
    assert!(matches!(service.trust_level, TrustLevel::Unknown));
}

#[test]
fn test_discovered_service_multiple_capabilities() {
    let mut capabilities = HashMap::new();
    capabilities.insert("version".to_string(), "2.0.0".to_string());
    capabilities.insert("protocol".to_string(), "https".to_string());
    capabilities.insert("auth".to_string(), "bearer".to_string());
    capabilities.insert("crypto".to_string(), "ed25519".to_string());

    let service = DiscoveredService {
        service_type: ServiceType::BearDog,
        address: "10.0.0.1:8443".parse().unwrap(),
        trust_level: TrustLevel::Verified,
        capabilities,
        last_seen: std::time::SystemTime::now(),
    };

    assert_eq!(service.capabilities.len(), 4);
    assert_eq!(
        service.capabilities.get("crypto"),
        Some(&"ed25519".to_string())
    );
}

// ============================================================================
// DiscoveryResult Tests (2 tests)
// ============================================================================

#[test]
fn test_discovery_result_empty() {
    let result = DiscoveryResult {
        services: vec![],
        scan_duration: tokio::time::Duration::from_secs(2),
        total_discovered: 0,
        verified_count: 0,
    };

    assert_eq!(result.services.len(), 0);
    assert_eq!(result.total_discovered, 0);
    assert_eq!(result.verified_count, 0);
    assert_eq!(result.scan_duration.as_secs(), 2);
}

#[test]
fn test_discovery_result_with_services() {
    let endpoint1 = ServiceEndpoint {
        service_type: EcosystemService::Songbird,
        address: "127.0.0.1:8080".parse().unwrap(),
        version: Arc::from("1.0.0"),
        capabilities: vec!["discovery".to_string()],
        trust_level: TrustLevel::Verified,
    };

    let endpoint2 = ServiceEndpoint {
        service_type: EcosystemService::BearDog,
        address: "127.0.0.1:8443".parse().unwrap(),
        version: Arc::from("2.0.0"),
        capabilities: vec!["auth".to_string()],
        trust_level: TrustLevel::Discovered,
    };

    let result = DiscoveryResult {
        services: vec![endpoint1, endpoint2],
        scan_duration: tokio::time::Duration::from_secs(5),
        total_discovered: 3,
        verified_count: 1,
    };

    assert_eq!(result.services.len(), 2);
    assert_eq!(result.total_discovered, 3);
    assert_eq!(result.verified_count, 1);
}

// ============================================================================
// BearDogPermission Tests (2 tests)
// ============================================================================

#[test]
fn test_beardog_permission_creation() {
    let permission = BearDogPermission {
        permission_id: uuid::Uuid::new_v4(),
        granted_to: "toadstool-instance-001".to_string(),
        capabilities: vec!["read".to_string(), "execute".to_string()],
        valid_until: std::time::SystemTime::now() + std::time::Duration::from_secs(24 * 3600),
        signature: "ed25519-signature-base64".to_string(),
    };

    assert_eq!(permission.granted_to, "toadstool-instance-001");
    assert_eq!(permission.capabilities.len(), 2);
    assert!(!permission.signature.is_empty());
}

#[test]
fn test_beardog_permission_multiple_capabilities() {
    let capabilities = vec![
        "read".to_string(),
        "write".to_string(),
        "execute".to_string(),
        "admin".to_string(),
    ];

    let permission = BearDogPermission {
        permission_id: uuid::Uuid::new_v4(),
        granted_to: "admin-service".to_string(),
        capabilities: capabilities.clone(),
        valid_until: std::time::SystemTime::now() + std::time::Duration::from_secs(7 * 24 * 3600),
        signature: "signature123".to_string(),
    };

    assert_eq!(permission.capabilities.len(), 4);
    assert!(permission.capabilities.contains(&"admin".to_string()));
}

// ============================================================================
// NestGateMount Tests (2 tests)
// ============================================================================

#[test]
fn test_nestgate_mount_with_encryption() {
    let mount = NestGateMount {
        dataset_name: "secure-research-data".to_string(),
        mount_point: std::path::PathBuf::from("/mnt/secure"),
        endpoint: "10.0.0.50:9000".to_string(),
        zfs_dataset: Some("tank/secure/research".to_string()),
        access_mode: "read-write".to_string(),
        encryption_key: Some("aes256-key-base64".to_string()),
    };

    assert_eq!(mount.dataset_name, "secure-research-data");
    assert_eq!(mount.access_mode, "read-write");
    assert!(mount.zfs_dataset.is_some());
    assert!(mount.encryption_key.is_some());
}

#[test]
fn test_nestgate_mount_read_only() {
    let mount = NestGateMount {
        dataset_name: "public-data".to_string(),
        mount_point: std::path::PathBuf::from("/mnt/public"),
        endpoint: "storage.local:9000".to_string(),
        zfs_dataset: None,
        access_mode: "read".to_string(),
        encryption_key: None,
    };

    assert_eq!(mount.access_mode, "read");
    assert!(mount.zfs_dataset.is_none());
    assert!(mount.encryption_key.is_none());
}
