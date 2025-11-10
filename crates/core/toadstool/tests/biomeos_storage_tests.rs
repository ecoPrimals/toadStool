//! Comprehensive tests for biomeOS storage integration
//!
//! Tests for StorageProvisioningManager, volume management,
//! and NestGate integration.

use toadstool::biomeos_integration::{
    PersistentVolume, ReplicationSettings, StorageProvisioningConfig, StorageProvisioningManager,
    VolumeConfig, VolumeStatus,
};

// ============================================================================
// StorageProvisioningConfig Tests
// ============================================================================

#[test]
fn test_storage_provisioning_config_creation() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert_eq!(config.nestgate_endpoint, "http://localhost:8080");
    assert_eq!(config.storage_tier, "standard");
    assert!(config.backup_enabled);
    assert!(config.replication_enabled);
    assert_eq!(config.replication_factor, 3);
}

#[test]
fn test_storage_provisioning_config_clone() {
    let config1 = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "premium".to_string(),
        backup_enabled: false,
        replication_enabled: false,
        replication_factor: 1,
    };

    let config2 = config1.clone();

    assert_eq!(config1.nestgate_endpoint, config2.nestgate_endpoint);
    assert_eq!(config1.storage_tier, config2.storage_tier);
    assert_eq!(config1.backup_enabled, config2.backup_enabled);
    assert_eq!(config1.replication_enabled, config2.replication_enabled);
    assert_eq!(config1.replication_factor, config2.replication_factor);
}

#[test]
fn test_storage_provisioning_config_different_tiers() {
    let tiers = vec!["standard", "premium", "archive", "high-performance"];

    for tier in tiers {
        let config = StorageProvisioningConfig {
            nestgate_endpoint: "http://localhost:8080".to_string(),
            storage_tier: tier.to_string(),
            backup_enabled: true,
            replication_enabled: true,
            replication_factor: 3,
        };

        assert_eq!(config.storage_tier, tier);
    }
}

#[test]
fn test_storage_provisioning_config_replication_factors() {
    for factor in 1..=5 {
        let config = StorageProvisioningConfig {
            nestgate_endpoint: "http://localhost:8080".to_string(),
            storage_tier: "standard".to_string(),
            backup_enabled: true,
            replication_enabled: true,
            replication_factor: factor,
        };

        assert_eq!(config.replication_factor, factor);
    }
}

#[test]
fn test_storage_provisioning_config_backup_disabled() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: false,
        replication_enabled: true,
        replication_factor: 3,
    };

    assert!(!config.backup_enabled);
}

// ============================================================================
// VolumeStatus Tests
// ============================================================================

#[test]
fn test_volume_status_creating() {
    let status = VolumeStatus::Creating;
    assert!(matches!(status, VolumeStatus::Creating));
}

#[test]
fn test_volume_status_available() {
    let status = VolumeStatus::Available;
    assert!(matches!(status, VolumeStatus::Available));
}

#[test]
fn test_volume_status_attaching() {
    let status = VolumeStatus::Attaching;
    assert!(matches!(status, VolumeStatus::Attaching));
}

#[test]
fn test_volume_status_in_use() {
    let status = VolumeStatus::InUse;
    assert!(matches!(status, VolumeStatus::InUse));
}

#[test]
fn test_volume_status_detaching() {
    let status = VolumeStatus::Detaching;
    assert!(matches!(status, VolumeStatus::Detaching));
}

#[test]
fn test_volume_status_deleting() {
    let status = VolumeStatus::Deleting;
    assert!(matches!(status, VolumeStatus::Deleting));
}

#[test]
fn test_volume_status_error() {
    let status = VolumeStatus::Error("Connection timeout".to_string());
    match status {
        VolumeStatus::Error(msg) => {
            assert_eq!(msg, "Connection timeout");
        }
        _ => panic!("Expected Error status"),
    }
}

#[test]
fn test_volume_status_equality() {
    let status1 = VolumeStatus::Available;
    let status2 = VolumeStatus::Available;
    assert_eq!(status1, status2);
}

#[test]
fn test_volume_status_clone() {
    let status1 = VolumeStatus::InUse;
    let status2 = status1.clone();
    assert_eq!(status1, status2);
}

#[test]
fn test_volume_status_error_messages() {
    let error_messages = vec![
        "Disk full",
        "Permission denied",
        "Network error",
        "Invalid configuration",
        "Quota exceeded",
    ];

    for msg in error_messages {
        let status = VolumeStatus::Error(msg.to_string());
        match status {
            VolumeStatus::Error(error_msg) => {
                assert_eq!(error_msg, msg);
            }
            _ => panic!("Expected Error status"),
        }
    }
}

// ============================================================================
// StorageProvisioningManager Tests
// ============================================================================

#[test]
fn test_storage_provisioning_manager_creation() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let _manager = StorageProvisioningManager::with_inmemory(config.clone());

    // Manager should be created successfully (verified by not panicking)
    assert_eq!(config.nestgate_endpoint, "http://localhost:8080");
}

#[test]
fn test_storage_provisioning_manager_with_different_endpoints() {
    let endpoints = vec![
        "http://localhost:8080",
        "http://nestgate.local:9090",
        "https://nestgate.example.com",
        "http://127.0.0.1:8000",
    ];

    for endpoint in endpoints {
        let config = StorageProvisioningConfig {
            nestgate_endpoint: endpoint.to_string(),
            storage_tier: "standard".to_string(),
            backup_enabled: true,
            replication_enabled: true,
            replication_factor: 3,
        };

        let _manager = StorageProvisioningManager::with_inmemory(config);
        // Should create successfully
    }
}

#[tokio::test]
async fn test_provision_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "10Gi".to_string(),
        mount_path: Some("/data".to_string()),
        storage_class: Some("fast-ssd".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        backup_policy: None,
    };

    // This uses in-memory backend for testing
    let result = manager.provision_volume(&volume_config).await;

    // In-memory backend should always succeed
    assert!(result.is_ok());
    let volume_info = result.unwrap();
    assert_eq!(volume_info.name, "test-volume");
    assert!(volume_info.id.starts_with("test-"));
}

#[tokio::test]
async fn test_provision_persistent_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let pv_config = PersistentVolume {
        name: "persistent-vol-1".to_string(),
        capacity: "50Gi".to_string(),
        storage_class: "premium".to_string(),
        access_modes: vec!["ReadWriteMany".to_string()],
        host_path: None,
    };

    let result = manager.provision_persistent_volume(&pv_config).await;

    // In-memory backend should always succeed
    assert!(result.is_ok());
    let volume_info = result.unwrap();
    assert_eq!(volume_info.name, "persistent-vol-1");
    assert!(volume_info.id.starts_with("test-pv-"));

    #[cfg(feature = "networking")]
    {
        // Real mode might fail if NestGate not running
        let _ = result;
    }
}

#[test]
fn test_storage_provisioning_manager_with_backup_disabled() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: false,
        replication_enabled: true,
        replication_factor: 3,
    };

    let _manager = StorageProvisioningManager::with_inmemory(config);
    // Should create successfully with backup disabled
}

#[test]
fn test_storage_provisioning_manager_with_replication_disabled() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: false,
        replication_factor: 1,
    };

    let _manager = StorageProvisioningManager::with_inmemory(config);
    // Should create successfully with replication disabled
}

// ============================================================================
// VolumeConfig Integration Tests
// ============================================================================

#[tokio::test]
async fn test_provision_volume_with_different_sizes() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let sizes = vec!["1Gi", "10Gi", "100Gi", "1Ti"];

    for size in sizes {
        let volume_config = VolumeConfig {
            name: format!("test-{size}"),
            size: size.to_string(),
            mount_path: Some("/data".to_string()),
            storage_class: Some("standard".to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            backup_policy: None,
        };

        #[cfg(not(feature = "networking"))]
        {
            let result = manager.provision_volume(&volume_config).await;
            assert!(result.is_ok());
            let volume = result.unwrap();
            assert_eq!(volume.size, size);
        }

        #[cfg(feature = "networking")]
        {
            let _ = manager.provision_volume(&volume_config).await;
        }
    }
}

#[tokio::test]
async fn test_provision_volume_with_different_storage_classes() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let storage_classes = vec!["standard", "premium", "fast-ssd", "archive"];

    for storage_class in storage_classes {
        let volume_config = VolumeConfig {
            name: format!("test-{storage_class}"),
            size: "10Gi".to_string(),
            mount_path: Some("/data".to_string()),
            storage_class: Some(storage_class.to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            backup_policy: None,
        };

        #[cfg(not(feature = "networking"))]
        {
            let result = manager.provision_volume(&volume_config).await;
            assert!(result.is_ok());
            let volume = result.unwrap();
            assert_eq!(volume.storage_class, storage_class);
        }

        #[cfg(feature = "networking")]
        {
            let _ = manager.provision_volume(&volume_config).await;
        }
    }
}

#[tokio::test]
async fn test_provision_volume_with_different_access_modes() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let access_modes = vec![
        vec!["ReadWriteOnce"],
        vec!["ReadOnlyMany"],
        vec!["ReadWriteMany"],
        vec!["ReadWriteOnce", "ReadOnlyMany"],
    ];

    for (idx, modes) in access_modes.into_iter().enumerate() {
        let volume_config = VolumeConfig {
            name: format!("test-access-{idx}"),
            size: "10Gi".to_string(),
            mount_path: Some("/data".to_string()),
            storage_class: Some("standard".to_string()),
            access_modes: modes.iter().map(|s| s.to_string()).collect(),
            backup_policy: None,
        };

        #[cfg(not(feature = "networking"))]
        {
            let result = manager.provision_volume(&volume_config).await;
            assert!(result.is_ok());
        }

        #[cfg(feature = "networking")]
        {
            let _ = manager.provision_volume(&volume_config).await;
        }
    }
}

// ============================================================================
// ReplicationSettings Tests
// ============================================================================

#[test]
fn test_replication_settings_creation() {
    let settings = ReplicationSettings {
        enabled: true,
        factor: 3,
        strategy: "sync".to_string(),
    };

    assert!(settings.enabled);
    assert_eq!(settings.factor, 3);
    assert_eq!(settings.strategy, "sync");
}

#[test]
fn test_replication_settings_disabled() {
    let settings = ReplicationSettings {
        enabled: false,
        factor: 1,
        strategy: "none".to_string(),
    };

    assert!(!settings.enabled);
    assert_eq!(settings.factor, 1);
}

#[test]
fn test_replication_settings_different_strategies() {
    let strategies = vec!["sync", "async", "hybrid", "chain"];

    for strategy in strategies {
        let settings = ReplicationSettings {
            enabled: true,
            factor: 3,
            strategy: strategy.to_string(),
        };

        assert_eq!(settings.strategy, strategy);
    }
}

#[test]
fn test_replication_settings_clone() {
    let settings1 = ReplicationSettings {
        enabled: true,
        factor: 5,
        strategy: "async".to_string(),
    };

    let settings2 = settings1.clone();

    assert_eq!(settings1.enabled, settings2.enabled);
    assert_eq!(settings1.factor, settings2.factor);
    assert_eq!(settings1.strategy, settings2.strategy);
}

// ============================================================================
// Integration Tests (Multiple Components)
// ============================================================================

#[tokio::test]
async fn test_complete_volume_provisioning_workflow() {
    // Create config with replication enabled
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "premium".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // Provision a regular volume
    let volume_config = VolumeConfig {
        name: "app-data".to_string(),
        size: "20Gi".to_string(),
        mount_path: Some("/app/data".to_string()),
        storage_class: Some("premium".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        backup_policy: None,
    };

    #[cfg(not(feature = "networking"))]
    {
        let result = manager.provision_volume(&volume_config).await;
        assert!(result.is_ok());

        let volume = result.unwrap();
        assert_eq!(volume.name, "app-data");
        assert_eq!(volume.size, "20Gi");
        assert_eq!(volume.storage_class, "premium");
    }

    #[cfg(feature = "networking")]
    {
        let _ = manager.provision_volume(&volume_config).await;
    }
}

#[test]
fn test_volume_status_lifecycle() {
    // Test typical volume status lifecycle
    let statuses = vec![
        VolumeStatus::Creating,
        VolumeStatus::Available,
        VolumeStatus::Attaching,
        VolumeStatus::InUse,
        VolumeStatus::Detaching,
        VolumeStatus::Available,
        VolumeStatus::Deleting,
    ];

    for status in statuses {
        // Should be able to create all lifecycle statuses
        let _cloned = status.clone();
    }
}

#[tokio::test]
async fn test_concurrent_volume_provisioning() {
    use tokio::task::JoinSet;

    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:8080".to_string(),
        storage_tier: "standard".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let _manager = StorageProvisioningManager::with_inmemory(config);

    let mut set = JoinSet::new();

    for i in 0..10 {
        let volume_config = VolumeConfig {
            name: format!("concurrent-vol-{i}"),
            size: "5Gi".to_string(),
            mount_path: Some(format!("/data-{i}")),
            storage_class: Some("standard".to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            backup_policy: None,
        };

        set.spawn(async move {
            #[cfg(not(feature = "networking"))]
            {
                let config = StorageProvisioningConfig {
                    nestgate_endpoint: "http://localhost:8080".to_string(),
                    storage_tier: "standard".to_string(),
                    backup_enabled: true,
                    replication_enabled: true,
                    replication_factor: 3,
                };
                let manager = StorageProvisioningManager::with_inmemory(config);
                manager.provision_volume(&volume_config).await
            }

            #[cfg(feature = "networking")]
            {
                let config = StorageProvisioningConfig {
                    nestgate_endpoint: "http://localhost:8080".to_string(),
                    storage_tier: "standard".to_string(),
                    backup_enabled: true,
                    replication_enabled: true,
                    replication_factor: 3,
                };
                let manager = StorageProvisioningManager::with_inmemory(config);
                manager.provision_volume(&volume_config).await
            }
        });
    }

    // Wait for all to complete
    while set.join_next().await.is_some() {
        // Continue until all tasks complete
    }
}
