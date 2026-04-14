// SPDX-License-Identifier: AGPL-3.0-or-later


use super::*;

#[test]
fn test_storage_requirements() {
    let requirements = StorageRequirements {
        mount_point: PathBuf::from("/mnt/data"),
        capacity_gb: Some(100),
        access_mode: AccessMode::ReadWrite,
        encryption: true,
    };

    assert_eq!(requirements.capacity_gb, Some(100));
    assert!(requirements.encryption);
}

#[test]
fn test_storage_requirements_read_only() {
    let requirements = StorageRequirements {
        mount_point: PathBuf::from("/readonly"),
        capacity_gb: None,
        access_mode: AccessMode::ReadOnly,
        encryption: false,
    };
    assert!(matches!(requirements.access_mode, AccessMode::ReadOnly));
    assert!(requirements.capacity_gb.is_none());
}

#[test]
fn test_access_mode_variants() {
    let ro = AccessMode::ReadOnly;
    let rw = AccessMode::ReadWrite;
    assert!(matches!(ro, AccessMode::ReadOnly));
    assert!(matches!(rw, AccessMode::ReadWrite));
}

#[test]
fn test_mount_info_creation() {
    let info = MountInfo {
        mount_point: PathBuf::from("/mnt/zfs"),
        dataset_name: "pool/data".to_string(),
        endpoint: "tcp://storage:9000".to_string(),
        backend_type: "zfs".to_string(),
    };
    assert_eq!(info.dataset_name, "pool/data");
    assert_eq!(info.backend_type, "zfs");
}

#[test]
fn test_mount_info_clone() {
    let info = MountInfo {
        mount_point: PathBuf::from("/mnt"),
        dataset_name: "default".to_string(),
        endpoint: "local".to_string(),
        backend_type: "nfs".to_string(),
    };
    let cloned = info.clone();
    assert_eq!(info.dataset_name, cloned.dataset_name);
}

#[test]
fn test_object_storage_connection() {
    let conn = ObjectStorageConnection {
        bucket: "my-bucket".to_string(),
        endpoint: "https://s3.amazonaws.com".to_string(),
        region: "us-east-1".to_string(),
    };
    assert_eq!(conn.bucket, "my-bucket");
    assert_eq!(conn.region, "us-east-1");
}

#[test]
fn test_storage_adapter_new() {
    use crate::ecosystem::adapters::AdapterFactory;
    let factory = AdapterFactory::new();
    let adapter = factory.storage_adapter().unwrap();
    let _ = adapter;
}

#[tokio::test]
async fn test_mount_distributed_storage_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let storage = factory.storage_adapter().unwrap();

    let requirements = StorageRequirements {
        mount_point: PathBuf::from("/mnt/test"),
        capacity_gb: Some(100),
        access_mode: AccessMode::ReadWrite,
        encryption: false,
    };

    let result = storage.mount_distributed_storage(requirements).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connect_object_storage_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let storage = factory.storage_adapter().unwrap();

    let result = storage
        .connect_object_storage("my-bucket".to_string(), Some("us-east-1".to_string()))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_put_object_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let storage = factory.storage_adapter().unwrap();

    let result = storage
        .put_object("bucket", "key", bytes::Bytes::from("data"))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_object_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let storage = factory.storage_adapter().unwrap();

    let result = storage.get_object("bucket", "key").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_put_kv_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let storage = factory.storage_adapter().unwrap();

    let result = storage.put_kv("key", serde_json::json!("value")).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_kv_no_service_returns_err() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let storage = factory.storage_adapter().unwrap();

    let result = storage.get_kv("key").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_connect_object_storage_no_region() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let storage = factory.storage_adapter().unwrap();

    let result = storage
        .connect_object_storage("bucket".to_string(), None)
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_mount_read_only_access_mode() {
    use crate::ecosystem::adapters::AdapterFactory;

    let factory = AdapterFactory::new();
    let storage = factory.storage_adapter().unwrap();

    let requirements = StorageRequirements {
        mount_point: PathBuf::from("/readonly"),
        capacity_gb: None,
        access_mode: AccessMode::ReadOnly,
        encryption: true,
    };

    let result = storage.mount_distributed_storage(requirements).await;
    assert!(result.is_err());
}
