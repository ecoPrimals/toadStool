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
    #[allow(dead_code)] // Stored for potential future use (defaults, reporting, etc.)
    pub(super) storage_tier: String,
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
                    "No storage service discovered: {}. Ensure a storage provider is running.",
                    e
                ))
            })?;

        Ok(Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            storage_tier: storage_tier.into(),
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
    #[allow(deprecated)]
    pub fn new(
        _endpoint: impl Into<String>,
        storage_tier: impl Into<String>,
        replication_enabled: bool,
        replication_factor: u32,
    ) -> Self {
        // LEGACY: Uses primal name for backward compatibility
        let socket_path = toadstool_common::primal_sockets::get_socket_path_for_service("nestgate");
        Self {
            rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient::new(socket_path),
            storage_tier: storage_tier.into(),
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
                .call("nestgate.health", serde_json::json!({}))
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
                    strategy: "async".to_string(),
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
                .call_typed::<VolumeInfo>("nestgate.provision_volume", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to provision volume {}: {}",
                        config_name, e
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
                    strategy: "sync".to_string(),
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
                .call_typed::<VolumeInfo>("nestgate.provision_persistent_volume", params)
                .await
                .map_err(|e| {
                    ToadStoolError::runtime(format!(
                        "Failed to provision persistent volume {}: {}",
                        config_name, e
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
        let volume_name = volume_name.to_string();
        let service_name_clone = service_name.to_string();
        let mount_path_str = mount_path.to_string();

        Box::pin(async move {
            let params = serde_json::json!({
                "volume_name": volume_name,
                "service_name": service_name_clone,
                "mount_path": mount_path_str,
            });

            let _: serde_json::Value = self
                .rpc_client
                .call("nestgate.mount_volume", params)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to mount volume: {e}")))?;

            tracing::info!(
                "Successfully mounted volume {} to {}",
                volume_name,
                service_name_clone
            );
            Ok(())
        })
    }

    fn unmount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let volume_name = volume_name.to_string();
        let service_name_clone = service_name.to_string();

        Box::pin(async move {
            let params = serde_json::json!({
                "volume_name": volume_name,
                "service_name": service_name_clone,
            });

            let _: serde_json::Value = self
                .rpc_client
                .call("nestgate.unmount_volume", params)
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to unmount volume: {e}")))?;

            tracing::info!(
                "Successfully unmounted volume {} from {}",
                volume_name,
                service_name_clone
            );
            Ok(())
        })
    }

    fn delete_volume(
        &self,
        volume_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        let volume_name = volume_name.to_string();

        Box::pin(async move {
            let params = serde_json::json!({"volume_name": volume_name});

            let _: serde_json::Value = self
                .rpc_client
                .call("nestgate.delete_volume", params)
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
        let volume_name = volume_name.to_string();

        Box::pin(async move {
            let params = serde_json::json!({"volume_name": volume_name});

            let status = self
                .rpc_client
                .call_typed::<VolumeStatus>("nestgate.get_volume_status", params)
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
                .call_typed::<Vec<VolumeInfo>>("nestgate.list_volumes", serde_json::json!({}))
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to list volumes: {e}")))?;

            Ok(volumes)
        })
    }
}
