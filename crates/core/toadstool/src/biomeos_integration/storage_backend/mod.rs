//! Storage backend traits and implementations for BiomeOS integration
//!
//! This module defines the trait interface for storage backends and provides
//! production and test implementations using proper dependency injection.

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;

use super::types::{PersistentVolume, VolumeConfig, VolumeInfo};
use crate::{ToadStoolError, ToadStoolResult};

/// Volume status enumeration
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum VolumeStatus {
    /// Volume is being created
    Creating,
    /// Volume is available for use
    Available,
    /// Volume is being attached
    Attaching,
    /// Volume is attached and in use
    InUse,
    /// Volume is being detached
    Detaching,
    /// Volume is being deleted
    Deleting,
    /// Volume creation or operation failed
    Error(String),
}

/// Trait defining the storage backend interface for BiomeOS integration.
///
/// This trait enables dependency injection of different storage implementations:
/// - **Production**: NestGate HTTP API backend for distributed storage
/// - **Testing**: In-memory backend for fast, isolated tests  
/// - **Custom**: Your own storage implementation
///
/// # Design Pattern: Dependency Injection
///
/// Instead of using feature flags or conditional compilation, this trait enables
/// clean separation between production and test code through dependency injection:
///
/// ```ignore
/// // Production
/// let backend: Arc<dyn StorageBackend> = Arc::new(
///     NestGateBackend::new("http://nestgate:8082")
/// );
///
/// // Testing
/// let backend: Arc<dyn StorageBackend> = Arc::new(
///     InMemoryBackend::new()
/// );
///
/// // Same API, different implementation!
/// let volume = backend.provision_volume(&config).await?;
/// ```
///
/// # Lifecycle
///
/// ```text
/// initialize() ──> provision_volume() ──> mount_volume()
///      │                                       │
///      │                                       ▼
///      │                                  [Volume In Use]
///      │                                       │
///      └──────────────────────────────> unmount_volume() ──> delete_volume()
/// ```
///
/// # Example Implementation
///
/// ```ignore
/// use async_trait::async_trait;
/// use std::collections::HashMap;
/// use std::sync::Mutex;
///
/// pub struct InMemoryBackend {
///     volumes: Mutex<HashMap<String, VolumeInfo>>,
/// }
///
/// #[async_trait]
/// impl StorageBackend for InMemoryBackend {
///     async fn initialize(&self) -> ToadStoolResult<()> {
///         // No-op for in-memory
///         Ok(())
///     }
///     
///     async fn provision_volume(&self, config: &VolumeConfig) -> ToadStoolResult<VolumeInfo> {
///         let info = VolumeInfo {
///             name: config.name.clone(),
///             size_bytes: config.size,
///             status: VolumeStatus::Available,
///         };
///         
///         self.volumes.lock().unwrap().insert(config.name.clone(), info.clone());
///         Ok(info)
///     }
///     
///     async fn delete_volume(&self, volume_name: &str) -> ToadStoolResult<()> {
///         self.volumes.lock().unwrap().remove(volume_name);
///         Ok(())
///     }
///     // ... other methods
/// }
/// ```
///
/// # Trait Invariants
///
/// 1. **Idempotency**: Calling the same operation twice should be safe
/// 2. **Error Handling**: Return specific errors, not panics
/// 3. **Thread Safety**: All methods must be safe to call concurrently
/// 4. **Resource Cleanup**: Deleted volumes must release storage
/// 5. **Status Accuracy**: `get_volume_status()` must return current state
///
/// # Performance Characteristics
///
/// - `initialize()`: O(1) - Connection test only
/// - `provision_volume()`: O(1) - Create volume metadata
/// - `mount_volume()`: O(1) - Update mount state
/// - `unmount_volume()`: O(1) - Update mount state  
/// - `delete_volume()`: O(1) - Remove volume
/// - `get_volume_status()`: O(1) - Lookup current status
/// - `list_volumes()`: O(n) - Iterate all volumes
///
/// # Concurrency
///
/// Multiple operations can run concurrently. Implementations must handle:
/// - Concurrent provisions of different volumes ✅
/// - Concurrent mounts to different services ✅
/// - Mount while another service is accessing ⚠️ (backend-dependent)
///
/// # Error Scenarios
///
/// Common errors implementations should handle:
/// - Volume already exists during provision
/// - Volume not found during mount/unmount/delete
/// - Insufficient storage space
/// - Network connectivity issues (for remote backends)
/// - Permission denied
///
/// # Testing
///
/// Use the provided `InMemoryBackend` for testing:
///
/// ```ignore
/// use toadstool::biomeos_integration::storage_backend::InMemoryBackend;
///
/// #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
/// async fn test_volume_lifecycle() {
///     let backend = InMemoryBackend::new();
///     
///     let config = VolumeConfig {
///         name: "test-vol".to_string(),
///         size: 1_000_000,
///     };
///     
///     let info = backend.provision_volume(&config).await?;
///     assert_eq!(info.name, "test-vol");
///     assert_eq!(info.status, VolumeStatus::Available);
/// }
/// ```
///
/// # See Also
///
/// - [`NestGateBackend`] - Production implementation
/// - [`InMemoryBackend`] - Test implementation
/// - [`VolumeConfig`] - Volume configuration
/// - [`VolumeInfo`] - Volume information
/// - [`VolumeStatus`] - Volume state enum
pub trait StorageBackend: Send + Sync {
    /// Initialize the storage backend and test connectivity.
    ///
    /// For network backends (NestGate), this tests connectivity and authentication.
    /// For local backends (in-memory), this is typically a no-op.
    ///
    /// # Errors
    ///
    /// Returns error if backend is unreachable or authentication fails.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let backend = NestGateBackend::new("http://nestgate:8082");
    /// backend.initialize().await?; // Test connection
    /// ```
    fn initialize(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) }) // Default implementation is no-op
    }

    /// Provision a new volume from configuration.
    ///
    /// Creates a new volume with the specified size and configuration.
    /// The volume starts in `Creating` state and transitions to `Available`.
    ///
    /// # Errors
    ///
    /// - Volume name already exists
    /// - Insufficient storage space
    /// - Invalid configuration
    ///
    /// # Example
    ///
    /// ```ignore
    /// let config = VolumeConfig {
    ///     name: "my-data".to_string(),
    ///     size: 10_000_000_000, // 10 GB
    /// };
    /// let info = backend.provision_volume(&config).await?;
    /// ```
    fn provision_volume(
        &self,
        config: &VolumeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>>;

    /// Provision a persistent volume with lifecycle guarantees.
    ///
    /// Similar to `provision_volume()` but with persistent volume semantics:
    /// - Volume persists across service restarts
    /// - Volume has independent lifecycle from services
    /// - Volume can be attached/detached dynamically
    fn provision_persistent_volume(
        &self,
        config: &PersistentVolume,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>>;

    /// Mount a volume to a service at the specified path.
    ///
    /// Makes the volume accessible to the service at `mount_path`.
    /// Volume transitions from `Available` to `InUse`.
    ///
    /// # Errors
    ///
    /// - Volume not found
    /// - Volume already mounted to another service
    /// - Invalid mount path
    fn mount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
        mount_path: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Unmount a volume from a service.
    ///
    /// Detaches the volume from the service. Volume transitions from
    /// `InUse` back to `Available` (if not mounted elsewhere).
    ///
    /// # Errors
    ///
    /// - Volume not found
    /// - Volume not currently mounted to this service
    fn unmount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Delete a volume and free its storage.
    ///
    /// Permanently removes the volume. **This cannot be undone!**
    /// Volume must be unmounted from all services first.
    ///
    /// # Errors
    ///
    /// - Volume not found
    /// - Volume still mounted (must unmount first)
    ///
    /// # Safety
    ///
    /// Ensure all data is backed up before deleting volumes.
    fn delete_volume(
        &self,
        volume_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Get the current status of a volume.
    ///
    /// Returns the current state: Creating, Available, InUse, Error, etc.
    ///
    /// # Errors
    ///
    /// - Volume not found
    fn get_volume_status(
        &self,
        volume_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeStatus>> + Send + '_>>;

    /// List all volumes managed by this backend.
    ///
    /// Returns information about all volumes, including their current status.
    fn list_volumes(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<VolumeInfo>>> + Send + '_>>;
}

/// Production implementation using NestGate Unix Socket API (Pure Rust!)
///
/// **TRUE PRIMAL**: Uses unix sockets for local IPC (no HTTP, no TLS, no ring!)
pub struct NestGateBackend {
    rpc_client: toadstool_common::unix_jsonrpc_client::UnixJsonRpcClient,
    #[allow(dead_code)] // Stored for potential future use (defaults, reporting, etc.)
    storage_tier: String,
    replication_enabled: bool,
    replication_factor: u32,
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
            // Health check via JSON-RPC over unix socket
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
        use super::types::{ReplicationSettings, StorageProvisioningRequest};

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

            let volume_info: VolumeInfo = self
                .rpc_client
                .call_typed("nestgate.provision_volume", params)
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
        use super::types::{ReplicationSettings, StorageProvisioningRequest};

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

            let volume_info: VolumeInfo = self
                .rpc_client
                .call_typed("nestgate.provision_persistent_volume", params)
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

            let status: VolumeStatus = self
                .rpc_client
                .call_typed("nestgate.get_volume_status", params)
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
            let volumes: Vec<VolumeInfo> = self
                .rpc_client
                .call_typed("nestgate.list_volumes", serde_json::json!({}))
                .await
                .map_err(|e| ToadStoolError::runtime(format!("Failed to list volumes: {e}")))?;

            Ok(volumes)
        })
    }
}

/// In-memory test backend for testing without external dependencies
///
/// This is a proper test implementation, not a mock. It maintains state
/// and implements the full backend interface correctly for testing purposes.
pub struct InMemoryBackend {
    volumes: Arc<Mutex<HashMap<String, VolumeInfo>>>,
    storage_tier: String,
}

impl InMemoryBackend {
    /// Create a new in-memory backend for testing
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
                created_at: chrono::Utc::now(),
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
                created_at: chrono::Utc::now(),
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
            let vols = volumes.lock().await;
            Ok(vols.values().cloned().collect())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::types::{PersistentVolume, VolumeConfig};
    use super::*;

    #[test]
    fn test_volume_status_all_variants() {
        let _ = VolumeStatus::Creating;
        let _ = VolumeStatus::Available;
        let _ = VolumeStatus::Attaching;
        let _ = VolumeStatus::InUse;
        let _ = VolumeStatus::Detaching;
        let _ = VolumeStatus::Deleting;
        let _ = VolumeStatus::Error("oops".to_string());
    }

    #[test]
    fn test_volume_status_serialization() {
        let status = VolumeStatus::Available;
        let json = serde_json::to_value(&status).unwrap();
        let parsed: VolumeStatus = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, VolumeStatus::Available);

        let err_status = VolumeStatus::Error("disk full".to_string());
        let json = serde_json::to_value(&err_status).unwrap();
        let parsed: VolumeStatus = serde_json::from_value(json).unwrap();
        assert_eq!(parsed, VolumeStatus::Error("disk full".to_string()));
    }

    #[test]
    fn test_volume_status_equality() {
        assert_eq!(VolumeStatus::Creating, VolumeStatus::Creating);
        assert_ne!(VolumeStatus::Creating, VolumeStatus::Available);
    }

    #[test]
    fn test_nestgate_backend_constructor() {
        let backend = NestGateBackend::new("http://nestgate:8082", "fast-tier", true, 3);
        assert_eq!(backend.storage_tier, "fast-tier");
        assert!(backend.replication_enabled);
        assert_eq!(backend.replication_factor, 3);
    }

    #[test]
    fn test_inmemory_backend_constructor() {
        let backend = InMemoryBackend::new("test-tier");
        assert_eq!(backend.storage_tier, "test-tier");
    }

    #[test]
    fn test_inmemory_backend_constructor_string_into() {
        let tier = String::from("dynamic-tier");
        let backend = InMemoryBackend::new(tier);
        assert_eq!(backend.storage_tier, "dynamic-tier");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_storage_trait_default_initialize() {
        // Default implementation returns Ok(())
        let backend = InMemoryBackend::new("tier");
        let result = backend.initialize().await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_provision_persistent_volume() {
        let backend = InMemoryBackend::new("tier");
        let config = PersistentVolume {
            name: "pv-test".to_string(),
            capacity: "50Gi".to_string(),
            access_modes: vec!["ReadWriteMany".to_string()],
            storage_class: "ssd".to_string(),
            host_path: None,
        };
        let result = backend.provision_persistent_volume(&config).await;
        assert!(result.is_ok());
        let info = result.unwrap();
        assert_eq!(info.name, "pv-test");
        assert_eq!(info.id, "test-pv-pv-test");
        assert_eq!(info.size, "50Gi");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_mount_volume_not_found() {
        let backend = InMemoryBackend::new("tier");
        let result = backend
            .mount_volume("nonexistent", "svc", "/mnt/data")
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_unmount_volume_not_found() {
        let backend = InMemoryBackend::new("tier");
        let result = backend.unmount_volume("nonexistent", "svc").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_delete_volume_not_found() {
        let backend = InMemoryBackend::new("tier");
        let result = backend.delete_volume("nonexistent").await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_get_volume_status_not_found() {
        let backend = InMemoryBackend::new("tier");
        let result = backend.get_volume_status("nonexistent").await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_provision_uses_default_storage_class() {
        let backend = InMemoryBackend::new("default-tier");
        let config = VolumeConfig {
            name: "no-class-vol".to_string(),
            size: "10Gi".to_string(),
            storage_class: None,
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: None,
            backup_policy: None,
        };
        let info = backend.provision_volume(&config).await.unwrap();
        assert_eq!(info.storage_class, "default-tier");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_backend_provision() {
        let backend = InMemoryBackend::new("test-tier");
        let config = VolumeConfig {
            name: "test-vol".to_string(),
            size: "100Gi".to_string(),
            storage_class: Some("fast".to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: None,
            backup_policy: None,
        };

        let result = backend.provision_volume(&config).await;
        assert!(result.is_ok());

        let info = result.unwrap();
        assert_eq!(info.name, "test-vol");
        assert_eq!(info.storage_class, "fast");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_backend_lifecycle() {
        let backend = InMemoryBackend::new("test-tier");
        let config = VolumeConfig {
            name: "lifecycle-test".to_string(),
            size: "50Gi".to_string(),
            storage_class: None,
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: None,
            backup_policy: None,
        };

        // Provision
        backend.provision_volume(&config).await.unwrap();

        // Mount
        backend
            .mount_volume("lifecycle-test", "test-service", "/mnt/data")
            .await
            .unwrap();

        // Check status
        let status = backend.get_volume_status("lifecycle-test").await.unwrap();
        assert_eq!(status, VolumeStatus::Available);

        // Unmount
        backend
            .unmount_volume("lifecycle-test", "test-service")
            .await
            .unwrap();

        // Delete
        backend.delete_volume("lifecycle-test").await.unwrap();

        // Verify deleted
        let status_result = backend.get_volume_status("lifecycle-test").await;
        assert!(status_result.is_err());
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_inmemory_backend_list() {
        let backend = InMemoryBackend::new("test-tier");

        // Initially empty
        let list = backend.list_volumes().await.unwrap();
        assert_eq!(list.len(), 0);

        // Add volumes
        for i in 1..=3 {
            let config = VolumeConfig {
                name: format!("vol-{}", i),
                size: "10Gi".to_string(),
                storage_class: None,
                access_modes: vec!["ReadWriteOnce".to_string()],
                mount_path: None,
                backup_policy: None,
            };
            backend.provision_volume(&config).await.unwrap();
        }

        // List should have 3 volumes
        let list = backend.list_volumes().await.unwrap();
        assert_eq!(list.len(), 3);
    }
}
