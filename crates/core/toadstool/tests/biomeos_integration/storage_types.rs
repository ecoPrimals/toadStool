//! Comprehensive tests for BiomeOS storage integration types

use toadstool::biomeos_integration::*;

// ============================================================================
// StorageProvisioningConfig Tests
// ============================================================================

#[test]
fn test_storage_provisioning_config_creation() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.nestgate_endpoint, "http://localhost:9090");
    assert_eq!(config.storage_tier, "hot");
    assert!(config.backup_enabled);
    assert!(config.replication_enabled);
    assert_eq!(config.replication_factor, 3);
}

#[test]
fn test_storage_provisioning_config_clone() {
    let config1 = StorageProvisioningConfig {
        nestgate_endpoint: "http://nestgate:9090".to_string(),
        storage_tier: "warm".to_string(),
        backup_enabled: false,
        replication_enabled: true,
        replication_factor: 2,
    };

    let config2 = config1.clone();

    assert_eq!(config1.nestgate_endpoint, config2.nestgate_endpoint);
    assert_eq!(config1.storage_tier, config2.storage_tier);
    assert_eq!(config1.replication_factor, config2.replication_factor);
}

#[test]
fn test_storage_provisioning_config_serialization() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "cold".to_string(),
        backup_enabled: true,
        replication_enabled: false,
        replication_factor: 1,
    };

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("nestgate_endpoint"));
}

// ============================================================================
// VolumeStatus Tests (7 variants)
// ============================================================================

#[test]
fn test_volume_status_creating() {
    let status = VolumeStatus::Creating;
    assert_eq!(status, VolumeStatus::Creating);
}

#[test]
fn test_volume_status_available() {
    let status = VolumeStatus::Available;
    assert_eq!(status, VolumeStatus::Available);
}

#[test]
fn test_volume_status_attaching() {
    let status = VolumeStatus::Attaching;
    assert_eq!(status, VolumeStatus::Attaching);
}

#[test]
fn test_volume_status_in_use() {
    let status = VolumeStatus::InUse;
    assert_eq!(status, VolumeStatus::InUse);
}

#[test]
fn test_volume_status_detaching() {
    let status = VolumeStatus::Detaching;
    assert_eq!(status, VolumeStatus::Detaching);
}

#[test]
fn test_volume_status_deleting() {
    let status = VolumeStatus::Deleting;
    assert_eq!(status, VolumeStatus::Deleting);
}

#[test]
fn test_volume_status_error() {
    let status = VolumeStatus::Error("Provisioning failed".to_string());

    match status {
        VolumeStatus::Error(msg) => {
            assert_eq!(msg, "Provisioning failed");
        }
        _ => panic!("Expected Error variant"),
    }
}

#[test]
fn test_volume_status_clone() {
    let status1 = VolumeStatus::Available;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

#[test]
fn test_volume_status_serialization() {
    let status = VolumeStatus::InUse;
    let json = serde_json::to_string(&status).unwrap();
    assert!(!json.is_empty());
}

// ============================================================================
// Storage Tier Tests
// ============================================================================

#[test]
fn test_storage_tier_hot() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.storage_tier, "hot");
}

#[test]
fn test_storage_tier_warm() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "warm".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 2,
    };

    assert_eq!(config.storage_tier, "warm");
}

#[test]
fn test_storage_tier_cold() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "cold".to_string(),
        backup_enabled: true,
        replication_enabled: false,
        replication_factor: 1,
    };

    assert_eq!(config.storage_tier, "cold");
}

// ============================================================================
// Replication Factor Tests
// ============================================================================

#[test]
fn test_replication_factor_one() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: false,
        replication_enabled: false,
        replication_factor: 1,
    };

    assert_eq!(config.replication_factor, 1);
}

#[test]
fn test_replication_factor_three() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.replication_factor, 3);
}

#[test]
fn test_replication_factor_five() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 5,
    };

    assert_eq!(config.replication_factor, 5);
}

// ============================================================================
// Backup Configuration Tests
// ============================================================================

#[test]
fn test_backup_enabled() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert!(config.backup_enabled);
}

#[test]
fn test_backup_disabled() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: false,
        replication_enabled: false,
        replication_factor: 1,
    };

    assert!(!config.backup_enabled);
}

// ============================================================================
// Endpoint Configuration Tests
// ============================================================================

#[test]
fn test_nestgate_endpoint_localhost() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.nestgate_endpoint, "http://localhost:9090");
}

#[test]
fn test_nestgate_endpoint_custom() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "https://nestgate.biomeos.local:9443".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(
        config.nestgate_endpoint,
        "https://nestgate.biomeos.local:9443"
    );
}

// ============================================================================
// StorageProvisioningManager Tests
// ============================================================================

#[test]
fn test_storage_provisioning_manager_creation() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let _manager = StorageProvisioningManager::with_inmemory(config);
    // Creation should succeed with in-memory backend
}

#[test]
fn test_storage_provisioning_manager_with_minimal_config() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "cold".to_string(),
        backup_enabled: false,
        replication_enabled: false,
        replication_factor: 1,
    };

    let _manager = StorageProvisioningManager::with_inmemory(config);
    // Creation should succeed with in-memory backend
}

// ============================================================================
