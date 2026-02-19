//! Storage provisioning manager tests
//!
//! Tests ToadStool's local storage management capabilities (in-memory fallback).
//! NestGate integration tests belong in NestGate's integration test suite.

use toadstool::biomeos_integration::{StorageProvisioningConfig, StorageProvisioningManager};

// ============================================================================
// Manager Creation Tests (ToadStool's local capabilities)
// ============================================================================

#[test]
fn test_storage_provisioning_manager_new() {
    let config = StorageProvisioningConfig {
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
        ..StorageProvisioningConfig::default()
    };

    let _manager = StorageProvisioningManager::with_inmemory(config.clone());
    // Should create without panicking
}

#[test]
fn test_storage_provisioning_manager_with_hot_tier() {
    let config = StorageProvisioningConfig {
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
        ..StorageProvisioningConfig::default()
    };

    let _manager = StorageProvisioningManager::with_inmemory(config.clone());
    // Verified by successful creation
}

#[test]
fn test_storage_provisioning_manager_with_cold_tier() {
    let config = StorageProvisioningConfig {
        storage_tier: "cold".to_string(),
        backup_enabled: false,
        replication_enabled: false,
        replication_factor: 1,
        ..StorageProvisioningConfig::default()
    };

    let _manager = StorageProvisioningManager::with_inmemory(config.clone());
    // Verified by successful creation
}

#[test]
fn test_storage_provisioning_manager_replication_disabled() {
    let config = StorageProvisioningConfig {
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: false,
        replication_factor: 1,
        ..StorageProvisioningConfig::default()
    };

    let manager = StorageProvisioningManager::with_inmemory(config.clone());
    assert!(!manager.config().replication_enabled);
    assert_eq!(manager.config().replication_factor, 1);
}

// ============================================================================
// Basic Volume Provisioning Tests (Local fallback capabilities)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[cfg(not(feature = "networking"))] // Only run in mock mode
async fn test_provision_volume_basic() {
    let config = StorageProvisioningConfig {
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 2,
        ..StorageProvisioningConfig::default()
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // Test basic provisioning works with in-memory fallback
    let volume_config = toadstool::biomeos_integration::VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: Some("fast-ssd".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: Some("/data".to_string()),
        backup_policy: Some("daily".to_string()),
    };

    let result = manager.provision_volume(&volume_config).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[cfg(not(feature = "networking"))]
async fn test_list_volumes_basic() {
    let config = StorageProvisioningConfig {
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 2,
        ..StorageProvisioningConfig::default()
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // Test listing volumes (should be empty initially)
    let result = manager.list_volumes().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
#[cfg(not(feature = "networking"))]
async fn test_delete_volume_basic() {
    let config = StorageProvisioningConfig {
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 2,
        ..StorageProvisioningConfig::default()
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // Test deletion capability exists (in-memory backend)
    let result = manager.delete_volume("test-volume").await;
    // May fail (volume doesn't exist) but call should work
    let _ = result;
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_manager_with_backup_disabled() {
    let config = StorageProvisioningConfig {
        storage_tier: "standard".to_string(),
        backup_enabled: false,
        replication_enabled: true,
        replication_factor: 2,
        ..StorageProvisioningConfig::default()
    };

    let manager = StorageProvisioningManager::with_inmemory(config.clone());
    assert!(!manager.config().backup_enabled);
}

#[test]
fn test_manager_with_replication_disabled() {
    let config = StorageProvisioningConfig {
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: false,
        replication_factor: 1,
        ..StorageProvisioningConfig::default()
    };

    let manager = StorageProvisioningManager::with_inmemory(config.clone());
    assert!(!manager.config().replication_enabled);
    assert_eq!(manager.config().replication_factor, 1);
}

#[test]
fn test_manager_with_high_replication() {
    let config = StorageProvisioningConfig {
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 5,
        ..StorageProvisioningConfig::default()
    };

    let manager = StorageProvisioningManager::with_inmemory(config.clone());
    assert_eq!(manager.config().replication_factor, 5);
}

// Note: Mount, unmount, and volume status tests are NestGate's responsibility.
// Those belong in NestGate's integration test suite with a real NestGate instance.
// ToadStool's job is to correctly call NestGate, not simulate storage operations.
