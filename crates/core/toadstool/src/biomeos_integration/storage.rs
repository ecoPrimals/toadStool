// SPDX-License-Identifier: AGPL-3.0-or-later
//! Storage provisioning and management via NestGate
//!
//! This module provides a high-level storage provisioning manager that uses
//! pluggable storage backends via dependency injection. No feature flags!

use std::sync::Arc;

use serde::{Deserialize, Serialize};

use super::storage_backend::{StorageBackend, VolumeStatus};
use super::types::{PersistentVolume, VolumeConfig, VolumeInfo};
use crate::ToadStoolResult;

/// Storage provisioning manager for NestGate integration
///
/// Uses dependency injection via the `StorageBackend` trait for flexibility.
/// No conditional compilation or feature flags - the backend determines behavior.
pub struct StorageProvisioningManager {
    /// Configuration
    config: StorageProvisioningConfig,
    /// Pluggable storage backend (NestGate, in-memory, etc.)
    backend: Arc<dyn StorageBackend>,
}

/// Configuration for storage provisioning manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProvisioningConfig {
    /// NestGate endpoint URL.
    ///
    /// **Deprecated** — leave empty (`String::new()`) to use capability-based socket
    /// discovery via `discover_storage_socket()`. Explicit endpoints are only supported
    /// by the legacy `with_nestgate()` constructor which is also deprecated.
    #[deprecated(
        since = "0.3.0",
        note = "Leave empty and use with_storage_service() for runtime discovery"
    )]
    pub nestgate_endpoint: String,
    /// Storage tier preference
    pub storage_tier: String,
    /// Enable backup
    pub backup_enabled: bool,
    /// Enable replication
    pub replication_enabled: bool,
    /// Replication factor
    pub replication_factor: u32,
}

#[allow(deprecated)]
impl Default for StorageProvisioningConfig {
    fn default() -> Self {
        Self {
            nestgate_endpoint: String::new(), // empty = use runtime discovery
            storage_tier: "hot".to_string(),
            backup_enabled: false,
            replication_enabled: false,
            replication_factor: 1,
        }
    }
}

impl StorageProvisioningManager {
    /// Create a new storage provisioning manager with custom backend
    #[must_use]
    pub fn new(config: StorageProvisioningConfig, backend: Arc<dyn StorageBackend>) -> Self {
        Self { config, backend }
    }

    /// Create a new manager with capability-based storage service discovery (RECOMMENDED)
    ///
    /// **Deep Debt Compliant**: Discovers storage service by capability, not name.
    ///
    /// # Errors
    ///
    /// Returns an error if storage service discovery fails or the backend cannot be initialized.
    pub async fn with_storage_service(config: StorageProvisioningConfig) -> ToadStoolResult<Self> {
        let backend = super::storage_backend::NestGateBackend::new_async(
            config.storage_tier.clone(),
            config.replication_enabled,
            config.replication_factor,
        )
        .await?;
        Ok(Self {
            config,
            backend: Arc::new(backend),
        })
    }

    /// Create a new manager with NestGate production backend
    ///
    /// **DEPRECATED**: Use `with_storage_service()` for capability-based discovery.
    #[must_use]
    #[deprecated(
        since = "0.3.0",
        note = "Use with_storage_service() for capability-based discovery"
    )]
    #[allow(deprecated)]
    pub fn with_nestgate(config: StorageProvisioningConfig) -> Self {
        let backend = super::storage_backend::NestGateBackend::new(
            config.nestgate_endpoint.clone(),
            config.storage_tier.clone(),
            config.replication_enabled,
            config.replication_factor,
        );
        Self {
            config,
            backend: Arc::new(backend),
        }
    }

    /// Create a new manager with in-memory test backend
    #[must_use]
    pub fn with_inmemory(config: StorageProvisioningConfig) -> Self {
        let backend = super::storage_backend::InMemoryBackend::new(config.storage_tier.clone());
        Self {
            config,
            backend: Arc::new(backend),
        }
    }

    /// Initialize connection to storage backend
    ///
    /// For NestGate backend, this tests connectivity.
    /// For in-memory backend, this is a no-op.
    ///
    /// # Errors
    ///
    /// Returns an error if the backend connection cannot be established.
    pub async fn initialize_nestgate_connection(&self) -> ToadStoolResult<()> {
        self.backend.initialize().await
    }

    /// Provision a volume from manifest configuration
    ///
    /// # Errors
    ///
    /// Returns an error if provisioning fails or configuration is invalid.
    pub async fn provision_volume(
        &self,
        volume_config: &VolumeConfig,
    ) -> ToadStoolResult<VolumeInfo> {
        self.backend.provision_volume(volume_config).await
    }

    /// Provision a persistent volume
    ///
    /// # Errors
    ///
    /// Returns an error if provisioning fails or configuration is invalid.
    pub async fn provision_persistent_volume(
        &self,
        pv_config: &PersistentVolume,
    ) -> ToadStoolResult<VolumeInfo> {
        self.backend.provision_persistent_volume(pv_config).await
    }

    /// Mount a volume to a service
    ///
    /// # Errors
    ///
    /// Returns an error if the volume or service does not exist or mount fails.
    pub async fn mount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
        mount_path: &str,
    ) -> ToadStoolResult<()> {
        self.backend
            .mount_volume(volume_name, service_name, mount_path)
            .await
    }

    /// Unmount a volume from a service
    ///
    /// # Errors
    ///
    /// Returns an error if unmount fails or the volume/service does not exist.
    pub async fn unmount_volume(
        &self,
        volume_name: &str,
        service_name: &str,
    ) -> ToadStoolResult<()> {
        self.backend.unmount_volume(volume_name, service_name).await
    }

    /// Delete a volume
    ///
    /// # Errors
    ///
    /// Returns an error if deletion fails or the volume does not exist.
    pub async fn delete_volume(&self, volume_name: &str) -> ToadStoolResult<()> {
        self.backend.delete_volume(volume_name).await
    }

    /// Get volume status
    ///
    /// # Errors
    ///
    /// Returns an error if the volume does not exist or status cannot be retrieved.
    pub async fn get_volume_status(&self, volume_name: &str) -> ToadStoolResult<VolumeStatus> {
        self.backend.get_volume_status(volume_name).await
    }

    /// List all volumes
    ///
    /// # Errors
    ///
    /// Returns an error if the backend cannot list volumes.
    pub async fn list_volumes(&self) -> ToadStoolResult<Vec<VolumeInfo>> {
        self.backend.list_volumes().await
    }

    /// Get reference to configuration
    #[must_use]
    pub const fn config(&self) -> &StorageProvisioningConfig {
        &self.config
    }
}

/// Helper function to parse size strings (e.g., "100Gi", "1TB")
#[allow(
    dead_code,
    reason = "reserved for volume manifest size parsing; used in tests"
)]
fn parse_size_string(size_str: &str) -> Option<u64> {
    size_str
        .strip_suffix("Gi")
        .and_then(|v| v.parse::<u64>().ok().map(|n| n * 1_073_741_824))
        .or_else(|| {
            size_str
                .strip_suffix("GB")
                .and_then(|v| v.parse::<u64>().ok().map(|n| n * 1_000_000_000))
        })
        .or_else(|| {
            size_str
                .strip_suffix("Mi")
                .and_then(|v| v.parse::<u64>().ok().map(|n| n * 1_048_576))
        })
        .or_else(|| {
            size_str
                .strip_suffix("MB")
                .and_then(|v| v.parse::<u64>().ok().map(|n| n * 1_000_000))
        })
        .or_else(|| {
            size_str
                .strip_suffix("TB")
                .and_then(|v| v.parse::<u64>().ok().map(|n| n * 1_000_000_000_000))
        })
        .or_else(|| size_str.parse::<u64>().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> StorageProvisioningConfig {
        StorageProvisioningConfig {
            storage_tier: "hot".to_string(),
            backup_enabled: true,
            replication_enabled: true,
            replication_factor: 3,
            ..StorageProvisioningConfig::default()
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_manager_with_inmemory_backend() {
        let config = test_config();
        let manager = StorageProvisioningManager::with_inmemory(config);

        let volume_config = VolumeConfig {
            name: "test-volume".to_string(),
            size: "100Gi".to_string(),
            storage_class: Some("fast-ssd".to_string()),
            access_modes: vec!["ReadWriteOnce".to_string()],
            mount_path: Some("/mnt/data".to_string()),
            backup_policy: Some("daily".to_string()),
        };

        let result = manager.provision_volume(&volume_config).await;
        assert!(result.is_ok());

        let volume_info = result.expect("Volume provision should succeed in test");
        assert_eq!(volume_info.name, "test-volume");
        assert!(volume_info.id.starts_with("test-"));
        assert_eq!(volume_info.status, "Available");
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_parse_size_string() {
        assert_eq!(parse_size_string("100Gi"), Some(107_374_182_400));
        assert_eq!(parse_size_string("1TB"), Some(1_000_000_000_000));
        assert_eq!(parse_size_string("500MB"), Some(500_000_000));
        assert_eq!(parse_size_string("256Mi"), Some(268_435_456));
        assert_eq!(parse_size_string("1000"), Some(1000));
        assert_eq!(parse_size_string("invalid"), None);
    }
}
