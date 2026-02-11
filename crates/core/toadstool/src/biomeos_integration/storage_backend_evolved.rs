// storage_backend_evolved.rs - Capability-based storage backend
//
// DEEP DEBT EVOLUTION: Discovers storage providers by capability, not by name.
// Doesn't know or care if it's "nestgate" - just asks "Who can store data?"
//
// Migration from: storage_backend.rs (hardcoded "nestgate")
// Evolution: Capability-based discovery, proper error handling, zero unwrap()

use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use toadstool_common::capability_provider::{CapabilityError, CapabilityProvider};
use toadstool_common::primal_identity::Capability;
use tokio::sync::RwLock;

/// Errors for storage backend
#[derive(Debug, thiserror::Error)]
pub enum StorageBackendError {
    #[error("Storage provider not found")]
    NoStorageProvider,

    #[error("Volume provisioning failed: {0}")]
    ProvisioningFailed(String),

    #[error("Mount operation failed: {0}")]
    MountFailed(String),

    #[error("Unmount operation failed: {0}")]
    UnmountFailed(String),

    #[error("Volume deletion failed: {0}")]
    DeletionFailed(String),

    #[error("Volume not found: {0}")]
    VolumeNotFound(String),

    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, StorageBackendError>;

/// Volume information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub mount_path: Option<PathBuf>,
    pub status: VolumeStatus,
    pub persistent: bool,
}

/// Volume status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VolumeStatus {
    Creating,
    Ready,
    Mounted,
    Unmounted,
    Deleting,
    Error,
}

/// Volume provisioning request
#[derive(Debug, Serialize)]
pub struct VolumeRequest {
    pub name: String,
    pub size_bytes: u64,
    pub persistent: bool,
    pub mount_path: Option<PathBuf>,
}

/// Storage backend with capability-based discovery
///
/// # Deep Debt Principles
///
/// 1. **Self-knowledge only**: Knows it needs storage
/// 2. **Runtime discovery**: Finds provider by capability
/// 3. **Proper errors**: No unwrap(), all errors handled
/// 4. **Agnostic**: Doesn't care which primal provides storage
pub struct StorageBackend {
    /// Storage provider (discovered at runtime)
    provider: Arc<RwLock<Option<CapabilityProvider>>>,
}

impl StorageBackend {
    /// Create new storage backend
    pub fn new() -> Self {
        Self {
            provider: Arc::new(RwLock::new(None)),
        }
    }

    /// Get or discover storage provider
    ///
    /// Discovers by capability: "Who can store data?"
    async fn get_provider(&self) -> Result<CapabilityProvider> {
        let mut provider_lock = self.provider.write().await;

        if provider_lock.is_none() {
            use toadstool_common::primal_identity::StorageCapability;
            let capability = Capability::Storage(StorageCapability::ObjectStorage);

            let discovered =
                CapabilityProvider::discover(capability)
                    .await
                    .map_err(|e| match e {
                        CapabilityError::NoProviderFound(_) => {
                            StorageBackendError::NoStorageProvider
                        }
                        other => StorageBackendError::Capability(other),
                    })?;

            *provider_lock = Some(discovered);
        }

        provider_lock
            .as_ref()
            .ok_or(StorageBackendError::NoStorageProvider)
            .cloned()
    }

    /// Provision a new volume
    ///
    /// # Deep Debt Evolution
    ///
    /// Before: `call_rpc("/primal/nestgate", "nestgate.provision_volume", ...)`
    /// After: `provider.call("storage.provision_volume", ...)`
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable or provisioning fails
    pub async fn provision_volume(&self, request: VolumeRequest) -> Result<VolumeInfo> {
        let provider = self.get_provider().await?;

        let params = json!({
            "name": request.name,
            "size_bytes": request.size_bytes,
            "persistent": request.persistent,
            "mount_path": request.mount_path,
        });

        let response = provider
            .call("storage.provision_volume", params)
            .await
            .map_err(|e| StorageBackendError::ProvisioningFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(StorageBackendError::Json)
    }

    /// Mount a volume
    ///
    /// # Errors
    ///
    /// Returns error if volume doesn't exist or mount fails
    pub async fn mount_volume(&self, volume_id: &str, mount_path: PathBuf) -> Result<VolumeInfo> {
        let provider = self.get_provider().await?;

        let params = json!({
            "volume_id": volume_id,
            "mount_path": mount_path,
        });

        let response = provider
            .call("storage.mount_volume", params)
            .await
            .map_err(|e| StorageBackendError::MountFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(StorageBackendError::Json)
    }

    /// Unmount a volume
    ///
    /// # Errors
    ///
    /// Returns error if volume not mounted or unmount fails
    pub async fn unmount_volume(&self, volume_id: &str) -> Result<VolumeInfo> {
        let provider = self.get_provider().await?;

        let params = json!({
            "volume_id": volume_id,
        });

        let response = provider
            .call("storage.unmount_volume", params)
            .await
            .map_err(|e| StorageBackendError::UnmountFailed(e.to_string()))?;

        serde_json::from_value(response).map_err(StorageBackendError::Json)
    }

    /// Delete a volume
    ///
    /// # Errors
    ///
    /// Returns error if volume doesn't exist or deletion fails
    pub async fn delete_volume(&self, volume_id: &str) -> Result<()> {
        let provider = self.get_provider().await?;

        let params = json!({
            "volume_id": volume_id,
        });

        provider
            .call("storage.delete_volume", params)
            .await
            .map_err(|e| StorageBackendError::DeletionFailed(e.to_string()))?;

        Ok(())
    }

    /// Get volume status
    ///
    /// # Errors
    ///
    /// Returns error if volume not found
    pub async fn get_volume_status(&self, volume_id: &str) -> Result<VolumeInfo> {
        let provider = self.get_provider().await?;

        let params = json!({
            "volume_id": volume_id,
        });

        let response = provider
            .call("storage.get_volume_status", params)
            .await
            .map_err(|_| StorageBackendError::VolumeNotFound(volume_id.to_string()))?;

        serde_json::from_value(response).map_err(StorageBackendError::Json)
    }

    /// List all volumes
    ///
    /// # Errors
    ///
    /// Returns error if provider unavailable
    pub async fn list_volumes(&self) -> Result<Vec<VolumeInfo>> {
        let provider = self.get_provider().await?;

        let response = provider
            .call("storage.list_volumes", json!({}))
            .await
            .map_err(|e| {
                StorageBackendError::Capability(CapabilityError::RpcFailed(e.to_string()))
            })?;

        let volumes = response["volumes"].as_array().ok_or_else(|| {
            StorageBackendError::Capability(CapabilityError::InvalidResponse(
                "No volumes array in response".into(),
            ))
        })?;

        volumes
            .iter()
            .map(|v| serde_json::from_value(v.clone()))
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(StorageBackendError::Json)
    }

    /// Check if storage provider is available
    pub async fn is_available(&self) -> bool {
        self.get_provider().await.is_ok()
    }

    /// Get provider info (for debugging only!)
    pub async fn provider_info(&self) -> Option<String> {
        let provider_lock = self.provider.read().await;
        provider_lock.as_ref().map(|p| p.service_name().to_string())
    }
}

impl Default for StorageBackend {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_storage_backend_creation() {
        let backend = StorageBackend::new();
        let provider_lock = backend.provider.read().await;
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
}
