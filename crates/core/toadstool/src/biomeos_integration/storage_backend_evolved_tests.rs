// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::path::PathBuf;

#[tokio::test]
async fn test_storage_backend_creation() {
    let backend = StorageBackend::new();
    let provider_lock = backend.provider.read().unwrap_or_else(|e| e.into_inner());
    assert!(provider_lock.is_none());
}

#[test]
fn test_storage_backend_default() {
    let backend = StorageBackend::default();
    assert_eq!(
        std::mem::size_of_val(&backend),
        std::mem::size_of::<StorageBackend>()
    );
}

#[test]
fn test_volume_status_enum() {
    assert_eq!(VolumeStatus::Ready, VolumeStatus::Ready);
    assert_ne!(VolumeStatus::Ready, VolumeStatus::Creating);
}

#[test]
fn test_volume_status_all_variants() {
    let _ = VolumeStatus::Creating;
    let _ = VolumeStatus::Ready;
    let _ = VolumeStatus::Mounted;
    let _ = VolumeStatus::Unmounted;
    let _ = VolumeStatus::Deleting;
    let _ = VolumeStatus::Error;
}

#[test]
fn test_volume_status_serialization() {
    let status = VolumeStatus::Ready;
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json, "ready");
    let parsed: VolumeStatus = serde_json::from_value(json).unwrap();
    assert_eq!(parsed, VolumeStatus::Ready);
}

#[test]
fn test_volume_info_constructor_and_serialization() {
    let info = VolumeInfo {
        id: "vol-1".to_string(),
        name: "data-vol".to_string(),
        size_bytes: 1_000_000_000,
        mount_path: Some(PathBuf::from("/mnt/data")),
        status: VolumeStatus::Ready,
        persistent: true,
    };
    let json = serde_json::to_value(&info).unwrap();
    assert_eq!(json["id"], "vol-1");
    assert_eq!(json["size_bytes"], 1_000_000_000);
    assert_eq!(json["persistent"], true);
}

#[test]
fn test_volume_request_constructor_and_serialization() {
    let req = VolumeRequest {
        name: "req-vol".to_string(),
        size_bytes: 5_000_000_000,
        persistent: false,
        mount_path: None,
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["name"], "req-vol");
    assert_eq!(json["size_bytes"].as_u64().unwrap(), 5_000_000_000);
}

#[test]
fn test_error_messages() {
    let err = StorageBackendError::NoStorageProvider;
    assert!(err.to_string().contains("Storage provider not found"));

    let err = StorageBackendError::VolumeNotFound("test-vol".into());
    assert!(err.to_string().contains("test-vol"));

    let err = StorageBackendError::ProvisioningFailed("fail".into());
    assert!(err.to_string().contains("Volume provisioning failed"));

    let err = StorageBackendError::MountFailed("m".into());
    assert!(err.to_string().contains("Mount operation failed"));

    let err = StorageBackendError::UnmountFailed("u".into());
    assert!(err.to_string().contains("Unmount operation failed"));

    let err = StorageBackendError::DeletionFailed("d".into());
    assert!(err.to_string().contains("Volume deletion failed"));
}

#[test]
fn test_volume_info_clone() {
    let info = VolumeInfo {
        id: "x".to_string(),
        name: "n".to_string(),
        size_bytes: 100,
        mount_path: None,
        status: VolumeStatus::Creating,
        persistent: false,
    };
    let cloned = info.clone();
    assert_eq!(cloned.id, info.id);
}

// ── DEEP tests: error paths, no-provider, configuration ────────────

#[tokio::test]
async fn test_storage_is_available_returns_false_when_no_provider() {
    let backend = StorageBackend::new();
    let available = backend.is_available().await;
    assert!(!available);
}

#[tokio::test]
async fn test_storage_provider_info_returns_none_when_no_provider() {
    let backend = StorageBackend::new();
    let info = backend.provider_info().await;
    assert!(info.is_none());
}

#[tokio::test]
async fn test_provision_volume_returns_error_without_provider() {
    let backend = StorageBackend::new();
    let req = VolumeRequest {
        name: "vol".to_string(),
        size_bytes: 1_000_000,
        persistent: false,
        mount_path: None,
    };
    let result = backend.provision_volume(req).await;
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(
            err,
            StorageBackendError::NoStorageProvider
                | StorageBackendError::Capability(_)
                | StorageBackendError::ProvisioningFailed(_)
        ),
        "expected provider/provisioning error, got {err:?}"
    );
}

#[tokio::test]
async fn test_mount_volume_returns_error_without_provider() {
    let backend = StorageBackend::new();
    let result = backend
        .mount_volume("vol-1", PathBuf::from("/mnt/data"))
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_unmount_volume_returns_no_provider_error() {
    let backend = StorageBackend::new();
    let result = backend.unmount_volume("vol-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_delete_volume_returns_no_provider_error() {
    let backend = StorageBackend::new();
    let result = backend.delete_volume("vol-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_get_volume_status_returns_error_without_provider() {
    let backend = StorageBackend::new();
    let result = backend.get_volume_status("vol-1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_list_volumes_returns_no_provider_error() {
    let backend = StorageBackend::new();
    let result = backend.list_volumes().await;
    assert!(result.is_err());
}

#[test]
fn test_storage_backend_error_capability_conversion() {
    use toadstool_common::capability_provider::CapabilityError;
    use toadstool_common::primal_identity::StorageCapability;
    let cap_err =
        CapabilityError::NoProviderFound(Capability::Storage(StorageCapability::ObjectStorage));
    let storage_err: StorageBackendError = cap_err.into();
    assert!(
        matches!(storage_err, StorageBackendError::Capability(_)),
        "expected Capability variant"
    );
}

#[test]
fn test_storage_backend_error_json_conversion() {
    let json_err = serde_json::from_str::<VolumeInfo>("{]").unwrap_err();
    let storage_err: StorageBackendError = json_err.into();
    assert!(!storage_err.to_string().is_empty());
}

#[test]
fn test_volume_status_serde_all_variants() {
    for s in [
        "creating",
        "ready",
        "mounted",
        "unmounted",
        "deleting",
        "error",
    ] {
        let parsed: VolumeStatus = serde_json::from_str(&format!("\"{s}\"")).unwrap();
        let _ = serde_json::to_string(&parsed).unwrap();
    }
}

#[test]
fn test_volume_request_with_mount_path() {
    let req = VolumeRequest {
        name: "data".to_string(),
        size_bytes: 10_000_000,
        persistent: true,
        mount_path: Some(PathBuf::from("/mnt/persistent")),
    };
    let json = serde_json::to_value(&req).unwrap();
    assert_eq!(json["mount_path"].as_str().unwrap(), "/mnt/persistent");
}
