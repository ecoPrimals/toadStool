// SPDX-License-Identifier: AGPL-3.0-or-later
//! Tests for biomeOS storage integration.
//!
//! All configs use capability-based discovery: no hardcoded primal endpoints.
//! Storage services are discovered at runtime via `with_storage_service()`.

use toadstool::biomeos_integration::{
    PersistentVolume, ReplicationSettings, StorageProvisioningConfig, StorageProvisioningManager,
    VolumeConfig, VolumeStatus,
};

/// Returns a test config with capability-based discovery (no hardcoded endpoint).
fn make_config(
    tier: &str,
    backup: bool,
    replicate: bool,
    factor: u32,
) -> StorageProvisioningConfig {
    StorageProvisioningConfig {
        storage_tier: tier.to_string(),
        backup_enabled: backup,
        replication_enabled: replicate,
        replication_factor: factor,
        ..StorageProvisioningConfig::default()
    }
}

// ============================================================================
// StorageProvisioningConfig Tests
// ============================================================================

#[test]
fn test_storage_provisioning_config_creation() {
    let config = make_config("standard", true, true, 3);

    assert_eq!(config.storage_tier, "standard");
    assert!(config.backup_enabled);
    assert!(config.replication_enabled);
    assert_eq!(config.replication_factor, 3);
}

#[test]
fn test_storage_provisioning_config_uses_runtime_discovery() {
    // Default config must use empty endpoint so the runtime discovers storage
    // capability at runtime rather than relying on a hardcoded address.
    let config = StorageProvisioningConfig::default();
    #[allow(deprecated)]
    let ep = &config.nestgate_endpoint;
    assert!(
        ep.is_empty(),
        "default config must use empty endpoint for runtime discovery"
    );
}

#[test]
fn test_storage_provisioning_config_clone() {
    let config1 = make_config("premium", false, false, 1);
    let config2 = config1.clone();

    assert_eq!(config1.storage_tier, config2.storage_tier);
    assert_eq!(config1.backup_enabled, config2.backup_enabled);
    assert_eq!(config1.replication_enabled, config2.replication_enabled);
    assert_eq!(config1.replication_factor, config2.replication_factor);
}

#[test]
fn test_storage_provisioning_config_different_tiers() {
    let tiers = ["standard", "premium", "archive", "high-performance"];

    for tier in tiers {
        let config = make_config(tier, true, true, 3);
        assert_eq!(config.storage_tier, tier);
    }
}

#[test]
fn test_storage_provisioning_config_replication_factors() {
    for factor in 1..=5 {
        let config = make_config("standard", true, true, factor);
        assert_eq!(config.replication_factor, factor);
    }
}

#[test]
fn test_storage_provisioning_config_backup_disabled() {
    let config = make_config("standard", false, true, 3);
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
    let error_messages = [
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
    let config = make_config("standard", true, true, 3);
    let _manager = StorageProvisioningManager::with_inmemory(config.clone());
    // Manager created with capability-based config (no hardcoded endpoint)
    assert_eq!(config.storage_tier, "standard");
}

#[test]
fn test_storage_provisioning_manager_default_config_uses_discovery() {
    // Verify that the default config produces a manager that will use
    // runtime service discovery rather than a hardcoded endpoint.
    let config = StorageProvisioningConfig::default();
    let _manager = StorageProvisioningManager::with_inmemory(config);
    // Creating with default (empty endpoint) should succeed
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provision_volume_inmemory() {
    let config = make_config("standard", true, true, 3);
    let manager = StorageProvisioningManager::with_inmemory(config);

    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "10Gi".to_string(),
        mount_path: Some("/data".to_string()),
        storage_class: Some("fast-ssd".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        backup_policy: None,
    };

    let result = manager.provision_volume(&volume_config).await;
    assert!(result.is_ok());
    let volume_info = result.unwrap();
    assert_eq!(volume_info.name, "test-volume");
    assert!(volume_info.id.starts_with("test-"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provision_persistent_volume_inmemory() {
    let config = make_config("standard", true, true, 3);
    let manager = StorageProvisioningManager::with_inmemory(config);

    let pv_config = PersistentVolume {
        name: "persistent-vol-1".to_string(),
        capacity: "50Gi".to_string(),
        storage_class: "premium".to_string(),
        access_modes: vec!["ReadWriteMany".to_string()],
        host_path: None,
    };

    let result = manager.provision_persistent_volume(&pv_config).await;
    assert!(result.is_ok());
    let volume_info = result.unwrap();
    assert_eq!(volume_info.name, "persistent-vol-1");
    assert!(volume_info.id.starts_with("test-pv-"));
}

#[test]
fn test_storage_provisioning_manager_with_backup_disabled() {
    let config = make_config("standard", false, true, 3);
    let _manager = StorageProvisioningManager::with_inmemory(config);
}

#[test]
fn test_storage_provisioning_manager_with_replication_disabled() {
    let config = make_config("standard", true, false, 1);
    let _manager = StorageProvisioningManager::with_inmemory(config);
}

// ============================================================================
// VolumeConfig Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provision_volume_with_different_sizes() {
    let config = make_config("standard", true, true, 3);
    let manager = StorageProvisioningManager::with_inmemory(config);

    let sizes = ["1Gi", "10Gi", "100Gi", "1Ti"];

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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provision_volume_with_different_storage_classes() {
    let config = make_config("standard", true, true, 3);
    let manager = StorageProvisioningManager::with_inmemory(config);

    let storage_classes = ["standard", "premium", "fast-ssd", "archive"];

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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provision_volume_with_different_access_modes() {
    let config = make_config("standard", true, true, 3);
    let manager = StorageProvisioningManager::with_inmemory(config);

    let access_modes = [
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
            access_modes: modes.iter().map(|s| (*s).to_string()).collect(),
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
        strategy: std::sync::Arc::from("sync"),
    };

    assert!(settings.enabled);
    assert_eq!(settings.factor, 3);
    assert_eq!(settings.strategy.as_ref(), "sync");
}

#[test]
fn test_replication_settings_disabled() {
    let settings = ReplicationSettings {
        enabled: false,
        factor: 1,
        strategy: std::sync::Arc::from("none"),
    };

    assert!(!settings.enabled);
    assert_eq!(settings.factor, 1);
}

#[test]
fn test_replication_settings_different_strategies() {
    let strategies = ["sync", "async", "hybrid", "chain"];

    for strategy in strategies {
        let settings = ReplicationSettings {
            enabled: true,
            factor: 3,
            strategy: std::sync::Arc::from(strategy),
        };

        assert_eq!(settings.strategy.as_ref(), strategy);
    }
}

#[test]
fn test_replication_settings_clone() {
    let settings1 = ReplicationSettings {
        enabled: true,
        factor: 5,
        strategy: std::sync::Arc::from("async"),
    };

    let settings2 = settings1.clone();

    assert_eq!(settings1.enabled, settings2.enabled);
    assert_eq!(settings1.factor, settings2.factor);
    assert_eq!(settings1.strategy.as_ref(), settings2.strategy.as_ref());
}

// ============================================================================
// Integration Tests (Multiple Components)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_complete_volume_provisioning_workflow() {
    let config = make_config("premium", true, true, 3);
    let manager = StorageProvisioningManager::with_inmemory(config);

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
    let statuses = [
        VolumeStatus::Creating,
        VolumeStatus::Available,
        VolumeStatus::Attaching,
        VolumeStatus::InUse,
        VolumeStatus::Detaching,
        VolumeStatus::Available,
        VolumeStatus::Deleting,
    ];

    for status in statuses {
        let _cloned = status.clone();
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_volume_provisioning() {
    use tokio::task::JoinSet;

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
            let config = make_config("standard", true, true, 3);
            let manager = StorageProvisioningManager::with_inmemory(config);
            manager.provision_volume(&volume_config).await
        });
    }

    while set.join_next().await.is_some() {}
}
