//! Storage backend traits and implementations for BiomeOS integration
//!
//! This module defines the trait interface for storage backends and provides
//! production and test implementations using proper dependency injection.
//!
//! ## Backends
//!
//! | Backend | Module | Purpose |
//! |---------|--------|---------|
//! | [`NestGateBackend`] | `nestgate` | Production — communicates via Unix socket JSON-RPC |
//! | [`InMemoryBackend`] | `inmemory` | Testing & lightweight — pure in-process HashMap |
//!
//! ## Design Pattern: Dependency Injection
//!
//! ```ignore
//! // Production
//! let backend: Arc<dyn StorageBackend> = Arc::new(
//!     NestGateBackend::new_async("fast", true, 3).await?
//! );
//!
//! // Testing
//! let backend: Arc<dyn StorageBackend> = Arc::new(InMemoryBackend::new("test"));
//!
//! // Same API, different implementation!
//! let volume = backend.provision_volume(&config).await?;
//! ```

use std::future::Future;
use std::pin::Pin;

use super::types::{PersistentVolume, VolumeConfig, VolumeInfo};
use crate::ToadStoolResult;

mod inmemory;
mod nestgate;

pub use inmemory::InMemoryBackend;
pub use nestgate::NestGateBackend;

#[cfg(test)]
mod tests;

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
/// # Trait Invariants
///
/// 1. **Idempotency**: Calling the same operation twice should be safe
/// 2. **Error Handling**: Return specific errors, not panics
/// 3. **Thread Safety**: All methods must be safe to call concurrently
/// 4. **Resource Cleanup**: Deleted volumes must release storage
/// 5. **Status Accuracy**: `get_volume_status()` must return current state
pub trait StorageBackend: Send + Sync {
    /// Initialize the storage backend and test connectivity.
    ///
    /// Default implementation is a no-op (suitable for in-memory backends).
    fn initialize(&self) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>> {
        Box::pin(async { Ok(()) })
    }

    /// Provision a new volume from configuration.
    fn provision_volume(
        &self,
        config: &VolumeConfig,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>>;

    /// Provision a persistent volume with lifecycle guarantees.
    fn provision_persistent_volume(
        &self,
        config: &PersistentVolume,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeInfo>> + Send + '_>>;

    /// Mount a volume to a service at the specified path.
    fn mount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
        mount_path: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Unmount a volume from a service.
    fn unmount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Delete a volume and free its storage.
    fn delete_volume(
        &self,
        volume_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<()>> + Send + '_>>;

    /// Get the current status of a volume.
    fn get_volume_status(
        &self,
        volume_name: &str,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<VolumeStatus>> + Send + '_>>;

    /// List all volumes managed by this backend.
    fn list_volumes(
        &self,
    ) -> Pin<Box<dyn Future<Output = ToadStoolResult<Vec<VolumeInfo>>> + Send + '_>>;
}
