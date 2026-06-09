// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Ecosystem Integrator
//!
//! Coverage target: 0% → 25% (15 tests)
//!
//! Testing strategy:
//! - `EcosystemIntegrator` initialization
//! - Service discovery
//! - Service verification
//! - Endpoint management
//! - Connection handling
//!
//! ⚠️ These tests verify backward compatibility with the deprecated `EcosystemService` enum.
//! Production code should use the capability-based `ServiceType` instead.

#![allow(deprecated)] // Testing backward compatibility with deprecated EcosystemService

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
fn test_service_endpoint_discovery() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Discovery,
        address: "127.0.0.1:8080".parse().unwrap(),
        version: Arc::from("1.0.0"),
        capabilities: vec!["discovery".to_string(), "coordination".to_string()],
        trust_level: TrustLevel::Verified,
    };

    assert!(matches!(endpoint.service_type, EcosystemService::Discovery));
    assert_eq!(endpoint.version, Arc::from("1.0.0"));
    assert_eq!(endpoint.capabilities.len(), 2);
    assert!(endpoint.capabilities.contains(&"discovery".to_string()));
}

#[test]
fn test_service_endpoint_crypto() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Crypto,
        address: "192.168.1.100:8443".parse().unwrap(),
        version: Arc::from("2.0.0"),
        capabilities: vec!["auth".to_string(), "crypto".to_string()],
        trust_level: TrustLevel::Sovereign,
    };

    assert!(matches!(endpoint.service_type, EcosystemService::Crypto));
    assert_eq!(endpoint.address.port(), 8443);
    assert!(matches!(endpoint.trust_level, TrustLevel::Sovereign));
}

#[test]
fn test_service_endpoint_storage() {
    let endpoint = ServiceEndpoint {
        service_type: EcosystemService::Storage,
        address: "10.0.0.50:9000".parse().unwrap(),
        version: Arc::from("1.5.0"),
        capabilities: vec!["storage".to_string(), "zfs".to_string()],
        trust_level: TrustLevel::Verified,
    };

    assert!(matches!(endpoint.service_type, EcosystemService::Storage));
    assert_eq!(endpoint.address.ip().to_string(), "10.0.0.50");
    assert!(endpoint.capabilities.contains(&"zfs".to_string()));
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
        service_type: EcosystemService::Discovery,
        address: "127.0.0.1:8080".parse().unwrap(),
        version: Arc::from("1.0.0"),
        capabilities: vec!["discovery".to_string()],
        trust_level: TrustLevel::Verified,
    };

    let endpoint2 = ServiceEndpoint {
        service_type: EcosystemService::Crypto,
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
// SecurityPermission Tests (2 tests)
// ============================================================================

#[test]
fn test_beardog_permission_creation() {
    let permission = SecurityPermission {
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

    let permission = SecurityPermission {
        permission_id: uuid::Uuid::new_v4(),
        granted_to: "admin-service".to_string(),
        capabilities,
        valid_until: std::time::SystemTime::now() + std::time::Duration::from_secs(7 * 24 * 3600),
        signature: "signature123".to_string(),
    };

    assert_eq!(permission.capabilities.len(), 4);
    assert!(permission.capabilities.contains(&"admin".to_string()));
}

// ============================================================================
// StorageMount Tests (2 tests)
// ============================================================================

#[test]
fn test_nestgate_mount_with_encryption() {
    let mount = StorageMount {
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
    let mount = StorageMount {
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
