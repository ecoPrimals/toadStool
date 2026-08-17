// SPDX-License-Identifier: AGPL-3.0-or-later
//! In-memory storage backend for testing and lightweight deployments

use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::SystemTime;

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
    fn provision_volume<'a>(
        &'a self,
        config: &'a VolumeConfig,
    ) -> impl std::future::Future<Output = ToadStoolResult<VolumeInfo>> + Send + 'a {
        let volumes = Arc::clone(&self.volumes);
        let storage_tier = self.storage_tier.clone();
        let config_name = config.name.clone();
        let config_size = config.size.clone();
        let config_storage_class = config.storage_class.clone();

        async move {
            let volume_info = VolumeInfo {
                name: config_name.clone(),
                id: format!("test-{config_name}"),
                size: config_size,
                storage_class: config_storage_class.unwrap_or(storage_tier),
                status: "Available".to_string(),
                created_at: SystemTime::now(),
            };

            volumes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(config_name, volume_info.clone());

            tracing::debug!("Provisioned test volume: {}", volume_info.name);
            Ok(volume_info)
        }
    }

    fn provision_persistent_volume<'a>(
        &'a self,
        config: &'a PersistentVolume,
    ) -> impl std::future::Future<Output = ToadStoolResult<VolumeInfo>> + Send + 'a {
        let volumes = Arc::clone(&self.volumes);
        let config_name = config.name.clone();
        let config_capacity = config.capacity.clone();
        let config_storage_class = config.storage_class.clone();

        async move {
            let volume_info = VolumeInfo {
                name: config_name.clone(),
                id: format!("test-pv-{config_name}"),
                size: config_capacity,
                storage_class: config_storage_class,
                status: "Available".to_string(),
                created_at: SystemTime::now(),
            };

            volumes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .insert(config_name, volume_info.clone());

            tracing::debug!("Provisioned test persistent volume: {}", volume_info.name);
            Ok(volume_info)
        }
    }

    fn mount_volume<'a>(
        &'a self,
        volume_name: &'a str,
        service_name: &'a str,
        _mount_path: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        let volumes = Arc::clone(&self.volumes);
        let volume_name = volume_name.to_string();
        let service_name = service_name.to_string();

        async move {
            if !volumes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains_key(&volume_name)
            {
                return Err(ToadStoolError::not_found(format!(
                    "Volume {volume_name} not found"
                )));
            }

            tracing::debug!(
                "Mounted test volume {} to service {}",
                volume_name,
                service_name
            );
            Ok(())
        }
    }

    fn unmount_volume<'a>(
        &'a self,
        volume_name: &'a str,
        service_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        let volumes = Arc::clone(&self.volumes);
        let volume_name = volume_name.to_string();
        let service_name = service_name.to_string();

        async move {
            if !volumes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .contains_key(&volume_name)
            {
                return Err(ToadStoolError::not_found(format!(
                    "Volume {volume_name} not found"
                )));
            }

            tracing::debug!(
                "Unmounted test volume {} from service {}",
                volume_name,
                service_name
            );
            Ok(())
        }
    }

    fn delete_volume<'a>(
        &'a self,
        volume_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        let volumes = Arc::clone(&self.volumes);
        let volume_name = volume_name.to_string();

        async move {
            volumes
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .remove(&volume_name)
                .ok_or_else(|| {
                    ToadStoolError::not_found(format!("Volume {volume_name} not found"))
                })?;

            tracing::debug!("Deleted test volume: {}", volume_name);
            Ok(())
        }
    }

    fn get_volume_status<'a>(
        &'a self,
        volume_name: &'a str,
    ) -> impl std::future::Future<Output = ToadStoolResult<VolumeStatus>> + Send + 'a {
        let volumes = Arc::clone(&self.volumes);
        let volume_name = volume_name.to_string();

        async move {
            let vols = volumes.lock().unwrap_or_else(|e| e.into_inner());
            vols.get(&volume_name)
                .map(|_| VolumeStatus::Available)
                .ok_or_else(|| ToadStoolError::not_found(format!("Volume {volume_name} not found")))
        }
    }

    fn list_volumes(&self) -> impl Future<Output = ToadStoolResult<Vec<VolumeInfo>>> + Send + '_ {
        let volumes = Arc::clone(&self.volumes);

        async move {
            let vols = volumes.lock().unwrap_or_else(|e| e.into_inner());
            Ok(vols.values().cloned().collect())
        }
    }
}
