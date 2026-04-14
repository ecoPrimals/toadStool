// SPDX-License-Identifier: AGPL-3.0-or-later
// storage_backend_evolved.rs - Capability-based storage backend
//
// DEEP DEBT EVOLUTION: Discovers storage providers by capability, not by name.
// Doesn't know or care about service names - just asks "Who can store data?"
//
// Migration from: storage_backend.rs (which hardcoded service names)
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
    /// No storage provider discovered
    #[error("Storage provider not found")]
    NoStorageProvider,

    /// Volume provisioning RPC failed
    #[error("Volume provisioning failed: {0}")]
    ProvisioningFailed(String),

    /// Mount operation failed
    #[error("Mount operation failed: {0}")]
    MountFailed(String),

    /// Unmount operation failed
    #[error("Unmount operation failed: {0}")]
    UnmountFailed(String),

    /// Volume deletion failed
    #[error("Volume deletion failed: {0}")]
    DeletionFailed(String),

    /// Volume not found
    #[error("Volume not found: {0}")]
    VolumeNotFound(String),

    /// Capability discovery or RPC error
    #[error("Capability error: {0}")]
    Capability(#[from] CapabilityError),

    /// JSON serialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// Result type for storage backend operations
pub type Result<T> = std::result::Result<T, StorageBackendError>;

/// Volume information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VolumeInfo {
    /// Volume identifier
    pub id: String,
    /// Volume name
    pub name: String,
    /// Size in bytes
    pub size_bytes: u64,
    /// Current mount path if mounted
    pub mount_path: Option<PathBuf>,
    /// Volume lifecycle status
    pub status: VolumeStatus,
    /// Whether volume persists across restarts
    pub persistent: bool,
}

/// Volume status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VolumeStatus {
    /// Volume is being created
    Creating,
    /// Volume is ready for use
    Ready,
    /// Volume is mounted
    Mounted,
    /// Volume is unmounted
    Unmounted,
    /// Volume is being deleted
    Deleting,
    /// Volume is in error state
    Error,
}

/// Volume provisioning request
#[derive(Debug, Serialize)]
pub struct VolumeRequest {
    /// Volume name
    pub name: String,
    /// Requested size in bytes
    pub size_bytes: u64,
    /// Whether volume should persist
    pub persistent: bool,
    /// Optional mount path hint
    pub mount_path: Option<PathBuf>,
}

/// Storage backend with capability-based discovery
///
/// # Deep Debt Principles
///
/// 1. **Self-knowledge only**: Knows it needs storage
/// 2. **Runtime discovery**: Finds provider by capability
/// 3. **Proper errors**: No `unwrap()`, all errors handled
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
#[path = "storage_backend_evolved_tests.rs"]
mod tests;
