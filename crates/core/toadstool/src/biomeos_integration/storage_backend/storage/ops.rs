// SPDX-License-Identifier: AGPL-3.0-or-later
//! `StorageBackend` trait — lifecycle, provisioning, volume ops, metadata.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::biomeos_integration::storage_backend::{StorageBackend, VolumeStatus};
use crate::biomeos_integration::types::{
    PersistentVolume, ReplicationSettings, StorageProvisioningRequest, VolumeConfig, VolumeInfo,
};
use crate::{ToadStoolError, ToadStoolResult};

use super::construct::SocketStorageBackend;

impl StorageBackend for SocketStorageBackend {
    fn initialize(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            let _health: serde_json::Value = self
                .rpc_client
                .call("storage.health", serde_json::json!({}))
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to connect to storage service: {e}"))
                })?;

            tracing::info!("Successfully connected to storage service via unix socket");
            Ok(())
        })
    }

    fn provision_volume(
        &self,
        config: &VolumeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>> {
        let config_name: Arc<str> = Arc::from(config.name.as_str());
        let replication_enabled = self.replication_enabled;
        let replication_factor = self.replication_factor;

        let request = StorageProvisioningRequest {
            volume_name: Arc::from(config.name.as_str()),
            size: Arc::from(config.size.as_str()),
            storage_class: config.storage_class.as_deref().map(Arc::from),
            access_modes: config
                .access_modes
                .iter()
                .map(|s| Arc::from(s.as_str()))
                .collect(),
            backup_policy: config.backup_policy.as_deref().map(Arc::from),
            replication: if replication_enabled {
                Some(ReplicationSettings {
                    enabled: true,
                    factor: replication_factor,
                    strategy: Arc::from("async"),
                })
            } else {
                None
            },
        };

        Box::pin(async move {
            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::runtime(format!("Failed to serialize request: {e}"))
            })?;

            let volume_info = self
                .rpc_client
                .call_typed::<VolumeInfo>("storage.provision_volume", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to provision volume {config_name}: {e}"
                    ))
                })?;

            Ok(volume_info)
        })
    }

    fn provision_persistent_volume(
        &self,
        config: &PersistentVolume,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>> {
        let config_name: Arc<str> = Arc::from(config.name.as_str());
        let replication_enabled = self.replication_enabled;
        let replication_factor = self.replication_factor;

        let request = StorageProvisioningRequest {
            volume_name: Arc::from(config.name.as_str()),
            size: Arc::from(config.capacity.as_str()),
            storage_class: Some(Arc::from(config.storage_class.as_str())),
            access_modes: config
                .access_modes
                .iter()
                .map(|s| Arc::from(s.as_str()))
                .collect(),
            backup_policy: None,
            replication: if replication_enabled {
                Some(ReplicationSettings {
                    enabled: true,
                    factor: replication_factor,
                    strategy: Arc::from("sync"),
                })
            } else {
                None
            },
        };

        Box::pin(async move {
            let params = serde_json::to_value(&request).map_err(|e| {
                ToadStoolError::runtime(format!("Failed to serialize request: {e}"))
            })?;

            let volume_info = self
                .rpc_client
                .call_typed::<VolumeInfo>("storage.provision_persistent_volume", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to provision persistent volume {config_name}: {e}"
                    ))
                })?;

            Ok(volume_info)
        })
    }

    fn mount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
        mount_path: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let volume_name = volume_name.to_owned();
        let service_name = service_name.to_owned();
        let mount_path = mount_path.to_owned();

        Box::pin(async move {
            let params = serde_json::json!({
                "volume_name": volume_name,
                "service_name": service_name,
                "mount_path": mount_path,
            });

            let _: serde_json::Value =
                self.rpc_client
                    .call("storage.mount_volume", params)
                    .await
                    .map_err(|e| ToadStoolError::runtime(format!("Failed to mount volume: {e}")))?;

            tracing::info!(
                "Successfully mounted volume {} to {}",
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
        let volume_name = volume_name.to_owned();
        let service_name = service_name.to_owned();

        Box::pin(async move {
            let params = serde_json::json!({
                "volume_name": volume_name,
                "service_name": service_name,
            });

            let _: serde_json::Value = self
                .rpc_client
                .call("storage.unmount_volume", params)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to unmount volume: {e}")))?;

            tracing::info!(
                "Successfully unmounted volume {} from {}",
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
        let volume_name = volume_name.to_owned();

        Box::pin(async move {
            let params = serde_json::json!({"volume_name": volume_name});

            let _: serde_json::Value = self
                .rpc_client
                .call("storage.delete_volume", params)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to delete volume: {e}")))?;

            tracing::info!("Successfully deleted volume {}", volume_name);
            Ok(())
        })
    }

    fn get_volume_status(
        &self,
        volume_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeStatus>> + Send + '_>> {
        let volume_name = volume_name.to_owned();

        Box::pin(async move {
            let params = serde_json::json!({"volume_name": volume_name});

            let status = self
                .rpc_client
                .call_typed::<VolumeStatus>("storage.get_volume_status", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to get volume status: {e}"))
                })?;

            Ok(status)
        })
    }

    fn list_volumes(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<VolumeInfo>>> + Send + '_>> {
        Box::pin(async move {
            let volumes = self
                .rpc_client
                .call_typed::<Vec<VolumeInfo>>("storage.list_volumes", serde_json::json!({}))
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to list volumes: {e}")))?;

            Ok(volumes)
        })
    }
}
