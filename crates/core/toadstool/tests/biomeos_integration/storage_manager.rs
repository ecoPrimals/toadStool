// SPDX-License-Identifier: AGPL-3.0-or-later
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
// Storage Async Method Tests (Mock Mode)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provision_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: Some("fast-ssd".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: Some("/mnt/data".to_string()),
        backup_policy: Some("daily".to_string()),
    };

    let result = manager.provision_volume(&volume_config).await;

    // In-memory backend returns test VolumeInfo
    assert!(result.is_ok());
    let volume_info = result.unwrap();
    assert_eq!(volume_info.name, "test-volume");
    assert!(volume_info.id.starts_with("test-"));
    assert_eq!(volume_info.status, "Available");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_provision_persistent_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let pv_config = PersistentVolume {
        name: "test-pv".to_string(),
        capacity: "500Gi".to_string(),
        storage_class: "premium-ssd".to_string(),
        access_modes: vec!["ReadWriteMany".to_string()],
        host_path: None,
    };

    let result = manager.provision_persistent_volume(&pv_config).await;

    // In-memory backend returns test VolumeInfo
    assert!(result.is_ok());
    let volume_info = result.unwrap();
    assert_eq!(volume_info.name, "test-pv");
    assert!(volume_info.id.starts_with("test-pv-"));
    assert_eq!(volume_info.storage_class, "premium-ssd");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mount_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // First provision the volume so it exists
    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    manager.provision_volume(&volume_config).await.unwrap();

    let result = manager
        .mount_volume("test-volume", "test-service", "/mnt/data")
        .await;

    // In-memory backend succeeds when volume exists
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_unmount_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // First provision the volume so it exists
    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    manager.provision_volume(&volume_config).await.unwrap();

    let result = manager.unmount_volume("test-volume", "test-service").await;

    // In-memory backend succeeds when volume exists
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_delete_volume_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // First provision the volume so it exists
    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    manager.provision_volume(&volume_config).await.unwrap();

    let result = manager.delete_volume("test-volume").await;

    // In-memory backend succeeds when volume exists
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_volume_status_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // First provision the volume so it exists
    let volume_config = VolumeConfig {
        name: "test-volume".to_string(),
        size: "100Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    manager.provision_volume(&volume_config).await.unwrap();

    let result = manager.get_volume_status("test-volume").await;

    // In-memory backend returns Available status when volume exists
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), VolumeStatus::Available);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_list_volumes_mock() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let result = manager.list_volumes().await;

    // In-memory backend returns empty list initially
    assert!(result.is_ok());
    assert_eq!(result.unwrap().len(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_volume_lifecycle_provision_mount_unmount_delete() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // 1. Provision volume
    let volume_config = VolumeConfig {
        name: "lifecycle-test".to_string(),
        size: "10Gi".to_string(),
        storage_class: Some("standard".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: Some("/data".to_string()),
        backup_policy: None,
    };

    let provision_result = manager.provision_volume(&volume_config).await;
    assert!(provision_result.is_ok());

    // 2. Mount volume
    let mount_result = manager
        .mount_volume("lifecycle-test", "test-service", "/data")
        .await;
    assert!(mount_result.is_ok());

    // 3. Check status
    let status_result = manager.get_volume_status("lifecycle-test").await;
    assert!(status_result.is_ok());

    // 4. Unmount volume
    let unmount_result = manager
        .unmount_volume("lifecycle-test", "test-service")
        .await;
    assert!(unmount_result.is_ok());

    // 5. Delete volume
    let delete_result = manager.delete_volume("lifecycle-test").await;
    assert!(delete_result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_volumes_provisioning() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "warm".to_string(),
        backup_enabled: false,
        replication_enabled: true,
        replication_factor: 2,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    // Provision multiple volumes
    for i in 1..=5 {
        let volume_config = VolumeConfig {
            name: format!("volume-{}", i),
            size: format!("{}Gi", i * 10),
            storage_class: Some("standard".to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: Some(format!("/mnt/vol{}", i)),
            backup_policy: None,
        };

        let result = manager.provision_volume(&volume_config).await;
        assert!(result.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_replication_enabled_provisioning() {
    let config = StorageProvisioningConfig {
        nestgate_endpoint: "http://localhost:9090".to_string(),
        storage_tier: "hot".to_string(),
        backup_enabled: true,
        replication_enabled: true,
        replication_factor: 3,
    };

    let manager = StorageProvisioningManager::with_inmemory(config);

    let volume_config = VolumeConfig {
        name: "replicated-volume".to_string(),
        size: "200Gi".to_string(),
        storage_class: Some("replicated-ssd".to_string()),
        access_modes: vec!["ReadWriteMany".to_string()],
        mount_path: Some("/mnt/replicated".to_string()),
        backup_policy: Some("hourly".to_string()),
    };

    let result = manager.provision_volume(&volume_config).await;
    assert!(result.is_ok());
}

// ============================================================================
