// SPDX-License-Identifier: AGPL-3.0-only
//! NestGate storage backend — production implementation over Unix sockets

use std::future::Future;
use std::pin::Pin;

use super::super::types::{PersistentVolume, VolumeConfig, VolumeInfo};
use super::VolumeStatus;
use crate::{ToadStoolError, ToadStoolResult};

use super::StorageBackend;

/// Production implementation using NestGate Unix Socket API (Pure Rust!)
///
/// **TRUE PRIMAL**: Uses unix sockets for local IPC (no HTTP, no TLS, no ring!)
pub struct NestGateBackend {
    pub(super) rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    pub(super) _storage_tier: String,
    pub(super) replication_enabled: bool,
    pub(super) replication_factor: u32,
}

impl NestGateBackend {
    /// Create storage backend with capability-based discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers storage service by capability, not name.
    /// Works with ANY service providing storage.object capability.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    ///
    /// # Errors
    ///
    /// Returns an error if capability discovery fails or no storage service can be found.
    pub async fn new_async(
        storage_tier: impl Into<String>,
        replication_enabled: bool,
        replication_factor: u32,
    ) -> ToadStoolResult<Self> {
        // CAPABILITY-BASED: Discover ANY storage service (not hardcoded "nestgate")
        let socket_path = toadstool_common::primal_sockets::discover_storage_socket()
            .await
            .map_err(|e| {
                ToadStoolError::configuration(format!(
                    "No storage service discovered: {e}. Ensure a storage provider is running."
                ))
            })?;

        Ok(Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            _storage_tier: storage_tier.into(),
            replication_enabled,
            replication_factor,
        })
    }

    /// Create a new storage backend with unix socket transport
    ///
    /// **DEPRECATED**: Use `new_async()` for capability-based discovery.
    ///
    /// **Pure Rust**: No HTTP client, uses unix sockets!
    #[must_use]
    #[deprecated(
        since = "0.3.0",
        note = "Use new_async() for capability-based discovery"
    )]
    pub fn new(
        _endpoint: impl Into<String>,
        storage_tier: impl Into<String>,
        replication_enabled: bool,
        replication_factor: u32,
    ) -> Self {
        let socket_path =
            toadstool_common::primal_sockets::get_socket_path_for_capability("storage");
        Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            _storage_tier: storage_tier.into(),
            replication_enabled,
            replication_factor,
        }
    }
}

impl StorageBackend for NestGateBackend {
    fn initialize(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async move {
            let _health: serde_json::Value = self
                .rpc_client
                .call("storage.health", serde_json::json!({}))
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!("Failed to connect to NestGate: {e}"))
                })?;

            tracing::info!("Successfully connected to NestGate via unix socket");
            Ok(())
        })
    }

    fn provision_volume(
        &self,
        config: &VolumeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>> {
        use super::super::types::{ReplicationSettings, StorageProvisioningRequest};

        let config_name = config.name.clone();
        let replication_enabled = self.replication_enabled;
        let replication_factor = self.replication_factor;

        let request = StorageProvisioningRequest {
            volume_name: config.name.clone(),
            size: config.size.clone(),
            storage_class: config.storage_class.clone(),
            access_modes: config.access_modes.clone(),
            backup_policy: config.backup_policy.clone(),
            replication: if replication_enabled {
                Some(ReplicationSettings {
                    enabled: true,
                    factor: replication_factor,
                    strategy: "async".to_owned(),
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
        use super::super::types::{ReplicationSettings, StorageProvisioningRequest};

        let config_name = config.name.clone();
        let replication_enabled = self.replication_enabled;
        let replication_factor = self.replication_factor;

        let request = StorageProvisioningRequest {
            volume_name: config.name.clone(),
            size: config.capacity.clone(),
            storage_class: Some(config.storage_class.clone()),
            access_modes: config.access_modes.clone(),
            backup_policy: None,
            replication: if replication_enabled {
                Some(ReplicationSettings {
                    enabled: true,
                    factor: replication_factor,
                    strategy: "sync".to_owned(),
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

#[cfg(test)]
mod tests {
    use super::super::super::types::{
        PersistentVolume, ReplicationSettings, StorageProvisioningRequest, VolumeConfig,
    };
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_replication_settings_serialization() {
        let settings = ReplicationSettings {
            enabled: true,
            factor: 5,
            strategy: "sync".to_string(),
        };
        let json = serde_json::to_value(&settings).unwrap();
        assert_eq!(json["enabled"], true);
        assert_eq!(json["factor"], 5);
        assert_eq!(json["strategy"], "sync");
    }

    #[test]
    #[allow(deprecated)]
    fn test_nestgate_backend_new_configuration() {
        let backend = NestGateBackend::new("http://ignored", "fast-tier", true, 3);
        assert_eq!(backend._storage_tier, "fast-tier");
        assert!(backend.replication_enabled);
        assert_eq!(backend.replication_factor, 3);
    }

    #[test]
    #[allow(deprecated)]
    fn test_nestgate_backend_new_storage_tier_into() {
        let tier = String::from("ssd-tier");
        let backend = NestGateBackend::new("x", tier, false, 1);
        assert_eq!(backend._storage_tier, "ssd-tier");
        assert!(!backend.replication_enabled);
    }

    #[test]
    #[allow(deprecated)]
    fn test_nestgate_backend_new_replication_disabled() {
        let backend = NestGateBackend::new("", "cold", false, 0);
        assert!(!backend.replication_enabled);
        assert_eq!(backend.replication_factor, 0);
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_nestgate_initialize_fails_without_service() {
        let backend = NestGateBackend::new("", "test", false, 1);
        let result = backend.initialize().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("Failed to connect") || err.to_string().contains("NestGate")
        );
    }

    #[test]
    fn test_storage_provisioning_request_serialization() {
        let req = StorageProvisioningRequest {
            volume_name: "test-vol".to_string(),
            size: "100Gi".to_string(),
            storage_class: Some("fast".to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            backup_policy: Some("daily".to_string()),
            replication: Some(ReplicationSettings {
                enabled: true,
                factor: 3,
                strategy: "async".to_string(),
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
            volume_name: config.name.clone(),
            size: config.size.clone(),
            storage_class: config.storage_class.clone(),
            access_modes: config.access_modes.clone(),
            backup_policy: config.backup_policy.clone(),
            replication: None,
        };
        assert_eq!(req.volume_name, "myvol");
        assert_eq!(req.size, "50Gi");
        assert_eq!(req.storage_class, Some("ssd".to_string()));
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
            volume_name: pv.name.clone(),
            size: pv.capacity.clone(),
            storage_class: Some(pv.storage_class.clone()),
            access_modes: pv.access_modes.clone(),
            backup_policy: None,
            replication: Some(ReplicationSettings {
                enabled: true,
                factor: 2,
                strategy: "sync".to_string(),
            }),
        };
        assert_eq!(req.volume_name, "pv-data");
        assert_eq!(req.size, "200Gi");
        assert_eq!(req.storage_class, Some("standard".to_string()));
        assert_eq!(req.replication.unwrap().strategy, "sync");
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
    #[allow(deprecated)]
    async fn test_provision_volume_fails_without_service() {
        let backend = NestGateBackend::new("", "tier", false, 1);
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
    #[allow(deprecated)]
    async fn test_list_volumes_fails_without_service() {
        let backend = NestGateBackend::new("", "tier", false, 1);
        let result = backend.list_volumes().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_provision_persistent_volume_fails_without_service() {
        let backend = NestGateBackend::new("", "tier", false, 1);
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
    #[allow(deprecated)]
    async fn test_mount_volume_fails_without_service() {
        let backend = NestGateBackend::new("", "tier", false, 1);
        let result = backend.mount_volume("vol1", "svc1", "/mnt/data").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_unmount_volume_fails_without_service() {
        let backend = NestGateBackend::new("", "tier", false, 1);
        let result = backend.unmount_volume("vol1", "svc1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_delete_volume_fails_without_service() {
        let backend = NestGateBackend::new("", "tier", false, 1);
        let result = backend.delete_volume("vol1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[allow(deprecated)]
    async fn test_get_volume_status_fails_without_service() {
        let backend = NestGateBackend::new("", "tier", false, 1);
        let result = backend.get_volume_status("vol1").await;
        assert!(result.is_err());
    }
}
