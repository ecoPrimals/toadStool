// SPDX-License-Identifier: AGPL-3.0-or-later

use crate::biomeos_integration::storage_backend::{StorageBackend, VolumeStatus};
use crate::biomeos_integration::types::{
    PersistentVolume, ReplicationSettings, StorageProvisioningRequest, VolumeConfig,
};
use std::path::PathBuf;
use std::sync::Arc;

use super::construct::SocketStorageBackend;

#[test]
fn test_replication_settings_serialization() {
    let settings = ReplicationSettings {
        enabled: true,
        factor: 5,
        strategy: Arc::from("sync"),
    };
    let json = serde_json::to_value(&settings).unwrap();
    assert_eq!(json["enabled"], true);
    assert_eq!(json["factor"], 5);
    assert_eq!(json["strategy"], "sync");
}

#[test]
#[expect(deprecated)]
fn test_storage_backend_impl_new_configuration() {
    let backend = SocketStorageBackend::new("http://ignored", "fast-tier", true, 3);
    assert_eq!(backend._storage_tier, "fast-tier");
    assert!(backend.replication_enabled);
    assert_eq!(backend.replication_factor, 3);
}

#[test]
#[expect(deprecated)]
fn test_storage_backend_impl_new_storage_tier_into() {
    let tier = String::from("ssd-tier");
    let backend = SocketStorageBackend::new("x", tier, false, 1);
    assert_eq!(backend._storage_tier, "ssd-tier");
    assert!(!backend.replication_enabled);
}

#[test]
#[expect(deprecated)]
fn test_storage_backend_impl_new_replication_disabled() {
    let backend = SocketStorageBackend::new("", "cold", false, 0);
    assert!(!backend.replication_enabled);
    assert_eq!(backend.replication_factor, 0);
}

#[tokio::test]
#[expect(deprecated)]
async fn test_nestgate_initialize_fails_without_service() {
    let backend = SocketStorageBackend::new("", "test", false, 1);
    let result = backend.initialize().await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.to_string().contains("Failed to connect") || err.to_string().contains("storage"));
}

#[test]
fn test_storage_provisioning_request_serialization() {
    let req = StorageProvisioningRequest {
        volume_name: Arc::from("test-vol"),
        size: Arc::from("100Gi"),
        storage_class: Some(Arc::from("fast")),
        access_modes: vec![Arc::from("ReadWriteOnce")],
        backup_policy: Some(Arc::from("daily")),
        replication: Some(ReplicationSettings {
            enabled: true,
            factor: 3,
            strategy: Arc::from("async"),
        }),
    };
    let json = serde_json::to_value(&req).expect("serialize");
    assert_eq!(json["volume_name"], "test-vol");
    assert_eq!(json["size"], "100Gi");
    assert_eq!(json["replication"]["factor"], 3);
}

#[test]
fn test_provision_request_from_volume_config_structure() {
    let config = VolumeConfig {
        name: "myvol".to_string(),
        size: "50Gi".to_string(),
        storage_class: Some("ssd".to_string()),
        access_modes: vec!["ReadWriteMany".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    let req = StorageProvisioningRequest {
        volume_name: Arc::from(config.name.as_str()),
        size: Arc::from(config.size.as_str()),
        storage_class: config.storage_class.as_deref().map(Arc::from),
        access_modes: config
            .access_modes
            .iter()
            .map(|s| Arc::from(s.as_str()))
            .collect(),
        backup_policy: config.backup_policy.as_deref().map(Arc::from),
        replication: None,
    };
    assert_eq!(req.volume_name.as_ref(), "myvol");
    assert_eq!(req.size.as_ref(), "50Gi");
    assert_eq!(req.storage_class.as_deref(), Some("ssd"));
}

#[test]
fn test_provision_request_from_persistent_volume_structure() {
    let pv = PersistentVolume {
        name: "pv-data".to_string(),
        capacity: "200Gi".to_string(),
        access_modes: vec!["ReadWriteOnce".to_string(), "ReadOnlyMany".to_string()],
        storage_class: "standard".to_string(),
        host_path: Some(PathBuf::from("/data")),
    };
    let req = StorageProvisioningRequest {
        volume_name: Arc::from(pv.name.as_str()),
        size: Arc::from(pv.capacity.as_str()),
        storage_class: Some(Arc::from(pv.storage_class.as_str())),
        access_modes: pv
            .access_modes
            .iter()
            .map(|s| Arc::from(s.as_str()))
            .collect(),
        backup_policy: None,
        replication: Some(ReplicationSettings {
            enabled: true,
            factor: 2,
            strategy: Arc::from("sync"),
        }),
    };
    assert_eq!(req.volume_name.as_ref(), "pv-data");
    assert_eq!(req.size.as_ref(), "200Gi");
    assert_eq!(req.storage_class.as_deref(), Some("standard"));
    assert_eq!(req.replication.as_ref().unwrap().strategy.as_ref(), "sync");
}

#[test]
fn test_volume_status_serialization_roundtrip() {
    let available = VolumeStatus::Available;
    let json = serde_json::to_string(&available).unwrap();
    let restored: VolumeStatus = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, VolumeStatus::Available);
    let err_status = VolumeStatus::Error("disk full".to_string());
    let json2 = serde_json::to_string(&err_status).unwrap();
    let restored2: VolumeStatus = serde_json::from_str(&json2).unwrap();
    assert_eq!(restored2, VolumeStatus::Error("disk full".to_string()));
}

#[test]
fn test_mount_volume_params_structure() {
    let params = serde_json::json!({
        "volume_name": "vol1",
        "service_name": "svc-a",
        "mount_path": "/mnt/data",
    });
    assert_eq!(params["volume_name"], "vol1");
    assert_eq!(params["mount_path"], "/mnt/data");
}

#[test]
fn test_unmount_volume_params_structure() {
    let params = serde_json::json!({
        "volume_name": "vol1",
        "service_name": "svc-a",
    });
    assert_eq!(params["volume_name"], "vol1");
    assert!(params.get("mount_path").is_none());
}

#[test]
fn test_delete_volume_params_structure() {
    let params = serde_json::json!({"volume_name": "vol-to-delete"});
    assert_eq!(params["volume_name"], "vol-to-delete");
}

#[tokio::test]
#[expect(deprecated)]
async fn test_provision_volume_fails_without_service() {
    let backend = SocketStorageBackend::new("", "tier", false, 1);
    let config = VolumeConfig {
        name: "test-vol".to_string(),
        size: "10Gi".to_string(),
        storage_class: None,
        access_modes: vec!["ReadWriteOnce".to_string()],
        mount_path: None,
        backup_policy: None,
    };
    let result = backend.provision_volume(&config).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("Failed to provision") || err.to_string().contains("test-vol")
    );
}

#[tokio::test]
#[expect(deprecated)]
async fn test_list_volumes_fails_without_service() {
    let backend = SocketStorageBackend::new("", "tier", false, 1);
    let result = backend.list_volumes().await;
    assert!(result.is_err());
}

#[tokio::test]
#[expect(deprecated)]
async fn test_provision_persistent_volume_fails_without_service() {
    let backend = SocketStorageBackend::new("", "tier", false, 1);
    let pv = PersistentVolume {
        name: "pv-test".to_string(),
        capacity: "10Gi".to_string(),
        access_modes: vec!["ReadWriteOnce".to_string()],
        storage_class: "standard".to_string(),
        host_path: None,
    };
    let result = backend.provision_persistent_volume(&pv).await;
    assert!(result.is_err());
}

#[tokio::test]
#[expect(deprecated)]
async fn test_mount_volume_fails_without_service() {
    let backend = SocketStorageBackend::new("", "tier", false, 1);
    let result = backend.mount_volume("vol1", "svc1", "/mnt/data").await;
    assert!(result.is_err());
}

#[tokio::test]
#[expect(deprecated)]
async fn test_unmount_volume_fails_without_service() {
    let backend = SocketStorageBackend::new("", "tier", false, 1);
    let result = backend.unmount_volume("vol1", "svc1").await;
    assert!(result.is_err());
}

#[tokio::test]
#[expect(deprecated)]
async fn test_delete_volume_fails_without_service() {
    let backend = SocketStorageBackend::new("", "tier", false, 1);
    let result = backend.delete_volume("vol1").await;
    assert!(result.is_err());
}

#[tokio::test]
#[expect(deprecated)]
async fn test_get_volume_status_fails_without_service() {
    let backend = SocketStorageBackend::new("", "tier", false, 1);
    let result = backend.get_volume_status("vol1").await;
    assert!(result.is_err());
}
