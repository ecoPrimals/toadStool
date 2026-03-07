// SPDX-License-Identifier: AGPL-3.0-or-later
use super::super::types::{PersistentVolume, VolumeConfig};
use super::*;

#[test]
fn test_volume_status_all_variants() {
    let _ = VolumeStatus::Creating;
    let _ = VolumeStatus::Available;
    let _ = VolumeStatus::Attaching;
    let _ = VolumeStatus::InUse;
    let _ = VolumeStatus::Detaching;
    let _ = VolumeStatus::Deleting;
    let _ = VolumeStatus::Error("oops".to_string());
}

#[test]
fn test_volume_status_serialization() {
    let status = VolumeStatus::Available;
    let json = serde_json::to_value(&status).unwrap();
    let parsed: VolumeStatus = serde_json::from_value(json).unwrap();
    assert_eq!(parsed, VolumeStatus::Available);

    let err_status = VolumeStatus::Error("disk full".to_string());
    let json = serde_json::to_value(&err_status).unwrap();
    let parsed: VolumeStatus = serde_json::from_value(json).unwrap();
    assert_eq!(parsed, VolumeStatus::Error("disk full".to_string()));
}

#[test]
fn test_volume_status_equality() {
    assert_eq!(VolumeStatus::Creating, VolumeStatus::Creating);
    assert_ne!(VolumeStatus::Creating, VolumeStatus::Available);
}

#[test]
#[allow(deprecated)]
fn test_nestgate_backend_constructor() {
    let backend = NestGateBackend::new("", "fast-tier", true, 3);
    assert_eq!(backend._storage_tier, "fast-tier");
    assert!(backend.replication_enabled);
    assert_eq!(backend.replication_factor, 3);
}

#[test]
fn test_inmemory_backend_constructor() {
    let backend = InMemoryBackend::new("test-tier");
    assert_eq!(backend.storage_tier, "test-tier");
}

#[test]
fn test_inmemory_backend_constructor_string_into() {
    let tier = String::from("dynamic-tier");
    let backend = InMemoryBackend::new(tier);
    assert_eq!(backend.storage_tier, "dynamic-tier");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_storage_trait_default_initialize() {
    let backend = InMemoryBackend::new("tier");
    let result = backend.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_provision_persistent_volume() {
    let backend = InMemoryBackend::new("tier");
    let config = PersistentVolume {
        name: "pv-test".to_string(),
        capacity: "50Gi".to_string(),
        access_modes: vec!["ReadWriteMany".to_string()],
        storage_class: "ssd".to_string(),
        host_path: None,
    };
    let result = backend.provision_persistent_volume(&config).await;
    assert!(result.is_ok());
    let info = result.unwrap();
    assert_eq!(info.name, "pv-test");
    assert_eq!(info.id, "test-pv-pv-test");
    assert_eq!(info.size, "50Gi");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_mount_volume_not_found() {
    let backend = InMemoryBackend::new("tier");
    let result = backend
        .mount_volume("nonexistent", "svc", "/mnt/data")
        .await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_unmount_volume_not_found() {
    let backend = InMemoryBackend::new("tier");
    let result = backend.unmount_volume("nonexistent", "svc").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_delete_volume_not_found() {
    let backend = InMemoryBackend::new("tier");
    let result = backend.delete_volume("nonexistent").await;
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("not found"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_get_volume_status_not_found() {
    let backend = InMemoryBackend::new("tier");
    let result = backend.get_volume_status("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_provision_uses_default_storage_class() {
    let backend = InMemoryBackend::new("default-tier");
    let config = VolumeConfig {
        name: "no-class-vol".to_string(),
        size: "10Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    let info = backend.provision_volume(&config).await.unwrap();
    assert_eq!(info.storage_class, "default-tier");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_backend_provision() {
    let backend = InMemoryBackend::new("test-tier");
    let config = VolumeConfig {
        name: "test-vol".to_string(),
        size: "100Gi".to_string(),
        storage_class: Some("fast".to_string()),
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };

    let result = backend.provision_volume(&config).await;
    assert!(result.is_ok());

    let info = result.unwrap();
    assert_eq!(info.name, "test-vol");
    assert_eq!(info.storage_class, "fast");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_backend_lifecycle() {
    let backend = InMemoryBackend::new("test-tier");
    let config = VolumeConfig {
        name: "lifecycle-test".to_string(),
        size: "50Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };

    backend.provision_volume(&config).await.unwrap();
    backend
        .mount_volume("lifecycle-test", "test-service", "/mnt/data")
        .await
        .unwrap();

    let status = backend.get_volume_status("lifecycle-test").await.unwrap();
    assert_eq!(status, VolumeStatus::Available);

    backend
        .unmount_volume("lifecycle-test", "test-service")
        .await
        .unwrap();

    backend.delete_volume("lifecycle-test").await.unwrap();

    let status_result = backend.get_volume_status("lifecycle-test").await;
    assert!(status_result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_inmemory_backend_list() {
    let backend = InMemoryBackend::new("test-tier");

    let list = backend.list_volumes().await.unwrap();
    assert_eq!(list.len(), 0);

    for i in 1..=3 {
        let config = VolumeConfig {
            name: format!("vol-{i}"),
            size: "10Gi".to_string(),
            storage_class: None,
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: None,
            backup_policy: None,
        };
        backend.provision_volume(&config).await.unwrap();
    }

    let list = backend.list_volumes().await.unwrap();
    assert_eq!(list.len(), 3);
}
