//! In-memory storage backend for testing and lightweight deployments

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;

use super::super::types::{PersistentVolume, VolumeConfig, VolumeInfo};
use super::VolumeStatus;
use crate::{ToadStoolError, ToadStoolResult};

use super::StorageBackend;

/// In-memory storage backend for testing without external dependencies
///
/// This is a proper implementation, not a mock. It maintains state
/// and implements the full backend interface for testing and lightweight
/// in-process deployments.
pub struct InMemoryBackend {
    pub(super) volumes: Arc<Mutex<HashMap<String, VolumeInfo>>>,
    pub(super) storage_tier: String,
}

impl InMemoryBackend {
    /// Create a new in-memory backend
    #[must_use]
    pub fn new(storage_tier: impl Into<String>) -> Self {
        Self {
            volumes: Arc::new(Mutex::new(HashMap::new())),
            storage_tier: storage_tier.into(),
        }
    }
}

impl StorageBackend for InMemoryBackend {
    fn provision_volume(
        &self,
        config: &VolumeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>> {
        let volumes = Arc::clone(&self.volumes);
        let storage_tier = self.storage_tier.clone();
        let config_name = config.name.clone();
        let config_size = config.size.clone();
        let config_storage_class = config.storage_class.clone();

        Box::pin(async move {
            let volume_info = VolumeInfo {
                name: config_name.clone(),
                id: format!("test-{}", config_name),
                size: config_size,
                storage_class: config_storage_class.unwrap_or(storage_tier),
                status: "Available".to_string(),
                created_at: SystemTime::now(),
            };

            let mut vols = volumes.lock().await;
            vols.insert(config_name.clone(), volume_info.clone());

            tracing::debug!("Provisioned test volume: {}", config_name);
            Ok(volume_info)
        })
    }

    fn provision_persistent_volume(
        &self,
        config: &PersistentVolume,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>> {
        let volumes = Arc::clone(&self.volumes);
        let config_name = config.name.clone();
        let config_capacity = config.capacity.clone();
        let config_storage_class = config.storage_class.clone();

        Box::pin(async move {
            let volume_info = VolumeInfo {
                name: config_name.clone(),
                id: format!("test-pv-{}", config_name),
                size: config_capacity,
                storage_class: config_storage_class,
                status: "Available".to_string(),
                created_at: SystemTime::now(),
            };

            let mut vols = volumes.lock().await;
            vols.insert(config_name.clone(), volume_info.clone());

            tracing::debug!("Provisioned test persistent volume: {}", config_name);
            Ok(volume_info)
        })
    }

    fn mount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
        _mount_path: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let volumes = Arc::clone(&self.volumes);
        let volume_name = volume_name.to_string();
        let service_name = service_name.to_string();

        Box::pin(async move {
            let vols = volumes.lock().await;
            if !vols.contains_key(&volume_name) {
                return Err(ToadStoolError::not_found(format!(
                    "Volume {} not found",
                    volume_name
                )));
            }

            tracing::debug!(
                "Mounted test volume {} to service {}",
                volume_name,
                service_name
            );
            Ok(())
        })
    }

    fn unmount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let volumes = Arc::clone(&self.volumes);
        let volume_name = volume_name.to_string();
        let service_name = service_name.to_string();

        Box::pin(async move {
            let vols = volumes.lock().await;
            if !vols.contains_key(&volume_name) {
                return Err(ToadStoolError::not_found(format!(
                    "Volume {} not found",
                    volume_name
                )));
            }

            tracing::debug!(
                "Unmounted test volume {} from service {}",
                volume_name,
                service_name
            );
            Ok(())
        })
    }

    fn delete_volume(
        &self,
        volume_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let volumes = Arc::clone(&self.volumes);
        let volume_name = volume_name.to_string();

        Box::pin(async move {
            let mut vols = volumes.lock().await;
            vols.remove(&volume_name).ok_or_else(|| {
                ToadStoolError::not_found(format!("Volume {} not found", volume_name))
            })?;

            tracing::debug!("Deleted test volume: {}", volume_name);
            Ok(())
        })
    }

    fn get_volume_status(
        &self,
        volume_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeStatus>> + Send + '_>> {
        let volumes = Arc::clone(&self.volumes);
        let volume_name = volume_name.to_string();

        Box::pin(async move {
            let vols = volumes.lock().await;
            vols.get(&volume_name)
                .map(|_| VolumeStatus::Available)
                .ok_or_else(|| {
                    ToadStoolError::not_found(format!("Volume {} not found", volume_name))
                })
        })
    }

    fn list_volumes(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<VolumeInfo>>> + Send + '_>> {
        let volumes = Arc::clone(&self.volumes);

        Box::pin(async move {
            let vols: tokio::sync::MutexGuard<HashMap<String, VolumeInfo>> = volumes.lock().await;
            Ok(vols.values().cloned().collect())
        })
    }
}
