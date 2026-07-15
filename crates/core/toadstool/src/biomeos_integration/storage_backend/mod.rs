// SPDX-License-Identifier: AGPL-3.0-or-later
//! Storage backend traits and implementations for BiomeOS integration
//!
//! This module defines the trait interface for storage backends and provides
//! production and test implementations using proper dependency injection.
//!
//! ## Backends
//!
//! | Backend | Module | Purpose |
//! |---------|--------|---------|
//! | [`SocketStorageBackend`] | `storage` | Production — communicates via Unix socket JSON-RPC |
//! | [`InMemoryBackend`] | `inmemory` | Testing & lightweight — pure in-process `HashMap` |
//!
//! ## Design Pattern: Dependency Injection
//!
//! ```ignore
//! // Production
//! let backend: Arc<StorageBackendDispatch> = Arc::new(
//!     StorageBackendDispatch::Socket(
//!         SocketStorageBackend::new_async("fast", true, 3).await?
//!     )
//! );
//!
//! // Testing
//! let backend: Arc<StorageBackendDispatch> = Arc::new(
//!     StorageBackendDispatch::InMemory(InMemoryBackend::new("test"))
//! );
//!
//! // Same API, different implementation!
//! let volume = backend.provision_volume(&config).await?;
//! ```

use std::future::Future;

use super::types::{PersistentVolume, VolumeConfig, VolumeInfo};
#[cfg(not(unix))]
use crate::ToadStoolError;
use crate::ToadStoolResult;

mod inmemory;
#[cfg(unix)]
mod storage;

pub use inmemory::InMemoryBackend;
#[cfg(unix)]
pub use storage::SocketStorageBackend;

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
    fn initialize(&self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async { Ok(()) }
    }

    /// Provision a new volume from configuration.
    fn provision_volume<'a>(
        &'a self,
        config: &'a VolumeConfig,
    ) -> impl Future<Output = ToadStoolResult<VolumeInfo>> + Send + 'a;

    /// Provision a persistent volume with lifecycle guarantees.
    fn provision_persistent_volume<'a>(
        &'a self,
        config: &'a PersistentVolume,
    ) -> impl Future<Output = ToadStoolResult<VolumeInfo>> + Send + 'a;

    /// Mount a volume to a service at the specified path.
    fn mount_volume<'a>(
        &'a self,
        volume_name: &'a str,
        service_name: &'a str,
        mount_path: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Unmount a volume from a service.
    fn unmount_volume<'a>(
        &'a self,
        volume_name: &'a str,
        service_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Delete a volume and free its storage.
    fn delete_volume<'a>(
        &'a self,
        volume_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a;

    /// Get the current status of a volume.
    fn get_volume_status<'a>(
        &'a self,
        volume_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<VolumeStatus>> + Send + 'a;

    /// List all volumes managed by this backend.
    fn list_volumes(&self) -> impl Future<Output = ToadStoolResult<Vec<VolumeInfo>>> + Send + '_;
}

/// Unix IPC backends are only available on Unix platforms.
#[cfg(not(unix))]
fn unix_storage_backend_unavailable() -> ToadStoolError {
    ToadStoolError::configuration("Unix socket storage backends are unavailable on this platform")
}

/// Dispatch enum for storage backends (replaces `Arc<dyn StorageBackend>`).
pub enum StorageBackendDispatch {
    /// Production backend — Unix socket JSON-RPC to the storage service.
    #[cfg(unix)]
    Socket(SocketStorageBackend),
    /// In-memory backend for tests and lightweight in-process use.
    #[cfg(any(test, feature = "test-mocks"))]
    InMemory(InMemoryBackend),
    /// Unix IPC unavailable on this platform.
    #[cfg(not(unix))]
    UnixUnavailable,
}

#[cfg_attr(not(unix), allow(unused_variables))]
impl StorageBackend for StorageBackendDispatch {
    fn initialize(&self) -> impl Future<Output = ToadStoolResult<()>> + Send + '_ {
        async move {
            match self {
                #[cfg(unix)]
                Self::Socket(b) => b.initialize().await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.initialize().await,
                #[cfg(not(unix))]
                Self::UnixUnavailable => Err(unix_storage_backend_unavailable()),
            }
        }
    }

    fn provision_volume<'a>(
        &'a self,
        config: &'a VolumeConfig,
    ) -> impl Future<Output = ToadStoolResult<VolumeInfo>> + Send + 'a {
        async move {
            match self {
                #[cfg(unix)]
                Self::Socket(b) => b.provision_volume(config).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.provision_volume(config).await,
                #[cfg(not(unix))]
                Self::UnixUnavailable => Err(unix_storage_backend_unavailable()),
            }
        }
    }

    fn provision_persistent_volume<'a>(
        &'a self,
        config: &'a PersistentVolume,
    ) -> impl Future<Output = ToadStoolResult<VolumeInfo>> + Send + 'a {
        async move {
            match self {
                #[cfg(unix)]
                Self::Socket(b) => b.provision_persistent_volume(config).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.provision_persistent_volume(config).await,
                #[cfg(not(unix))]
                Self::UnixUnavailable => Err(unix_storage_backend_unavailable()),
            }
        }
    }

    fn mount_volume<'a>(
        &'a self,
        volume_name: &'a str,
        service_name: &'a str,
        mount_path: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(unix)]
                Self::Socket(b) => b.mount_volume(volume_name, service_name, mount_path).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.mount_volume(volume_name, service_name, mount_path).await,
                #[cfg(not(unix))]
                Self::UnixUnavailable => Err(unix_storage_backend_unavailable()),
            }
        }
    }

    fn unmount_volume<'a>(
        &'a self,
        volume_name: &'a str,
        service_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(unix)]
                Self::Socket(b) => b.unmount_volume(volume_name, service_name).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.unmount_volume(volume_name, service_name).await,
                #[cfg(not(unix))]
                Self::UnixUnavailable => Err(unix_storage_backend_unavailable()),
            }
        }
    }

    fn delete_volume<'a>(
        &'a self,
        volume_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<()>> + Send + 'a {
        async move {
            match self {
                #[cfg(unix)]
                Self::Socket(b) => b.delete_volume(volume_name).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.delete_volume(volume_name).await,
                #[cfg(not(unix))]
                Self::UnixUnavailable => Err(unix_storage_backend_unavailable()),
            }
        }
    }

    fn get_volume_status<'a>(
        &'a self,
        volume_name: &'a str,
    ) -> impl Future<Output = ToadStoolResult<VolumeStatus>> + Send + 'a {
        async move {
            match self {
                #[cfg(unix)]
                Self::Socket(b) => b.get_volume_status(volume_name).await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.get_volume_status(volume_name).await,
                #[cfg(not(unix))]
                Self::UnixUnavailable => Err(unix_storage_backend_unavailable()),
            }
        }
    }

    fn list_volumes(&self) -> impl Future<Output = ToadStoolResult<Vec<VolumeInfo>>> + Send + '_ {
        async move {
            match self {
                #[cfg(unix)]
                Self::Socket(b) => b.list_volumes().await,
                #[cfg(any(test, feature = "test-mocks"))]
                Self::InMemory(b) => b.list_volumes().await,
                #[cfg(not(unix))]
                Self::UnixUnavailable => Err(unix_storage_backend_unavailable()),
            }
        }
    }
}
