// SPDX-License-Identifier: AGPL-3.0-or-later
//! Storage adapter - capability-based storage operations
//!
//! This adapter replaces the hardcoded NestGate integration with a generic
//! storage adapter that works with ANY service providing storage capabilities.
//!
//! # Migration from NestGate
//! ```rust,ignore
//! // ❌ OLD: Hardcoded NestGate (services/nestgate.rs)
//! use crate::ecosystem::services::nestgate;
//! let mount = nestgate::connect_storage(&addr, &mount_point, dataset).await?;
//!
//! // ✅ NEW: Capability-based (adapters/storage.rs)
//! use crate::ecosystem::adapters::StorageAdapter;
//! use crate::ecosystem::capabilities::StandardCapability;
//! let mount = storage.mount_distributed_storage(requirements).await?;
//! ```

use crate::{CliContextExt, Result};
use bytes::Bytes;
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;

use super::universal::{Request, UniversalServiceAdapter};
use crate::ecosystem::capabilities::StandardCapability;

/// Storage adapter - provides storage operations via capability discovery
///
/// This adapter discovers and invokes storage services without knowing their identity.
/// Services could be NestGate, Ceph, GlusterFS, S3, or custom implementations.
pub struct StorageAdapter {
    /// Universal service adapter for invoking capabilities
    universal: Arc<UniversalServiceAdapter>,
}

impl StorageAdapter {
    /// Create a new storage adapter
    pub fn new(universal: Arc<UniversalServiceAdapter>) -> Self {
        Self { universal }
    }

    /// Mount distributed storage
    ///
    /// Discovers a distributed storage service and mounts it at the specified location.
    ///
    /// # Example
    /// ```ignore
    /// // Forward-looking example - API under development
    /// # use toadstool_cli::ecosystem::adapters::StorageAdapter;
    /// # async fn example(storage: StorageAdapter) -> anyhow::Result<()> {
    /// use std::path::PathBuf;
    ///
    /// let mount = storage.mount_distributed_storage(StorageRequirements {
    ///     mount_point: PathBuf::from("/mnt/data"),
    ///     capacity_gb: Some(100),
    ///     access_mode: AccessMode::ReadWrite,
    ///     encryption: true,
    /// }).await?;
    ///
    /// println!("Mounted at: {}", mount.mount_point.display());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn mount_distributed_storage(
        &self,
        requirements: StorageRequirements,
    ) -> Result<MountInfo> {
        let capability = StandardCapability::StorageDistributedFilesystem.id();

        let request = Request::new(
            "mount",
            json!({
                "mount_point": requirements.mount_point.to_string_lossy(),
                "capacity_gb": requirements.capacity_gb,
                "access_mode": match requirements.access_mode {
                    AccessMode::ReadOnly => "read-only",
                    AccessMode::ReadWrite => "read-write",
                },
                "encryption": requirements.encryption,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to mount distributed storage")?;

        let data = response.data()?;

        Ok(MountInfo {
            mount_point: requirements.mount_point,
            dataset_name: data
                .get("dataset")
                .and_then(|v| v.as_str())
                .unwrap_or("default")
                .to_string(),
            endpoint: data
                .get("endpoint")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            backend_type: data
                .get("backend_type")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string(),
        })
    }

    /// Connect to object storage (S3-compatible)
    pub async fn connect_object_storage(
        &self,
        bucket: String,
        region: Option<String>,
    ) -> Result<ObjectStorageConnection> {
        let capability = StandardCapability::StorageObjectS3.id();

        let request = Request::new(
            "connect",
            json!({
                "bucket": bucket,
                "region": region,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to connect to object storage")?;

        let data = response.data()?;

        Ok(ObjectStorageConnection {
            bucket: bucket.clone(),
            endpoint: data
                .get("endpoint")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string(),
            region: region.unwrap_or_default(),
        })
    }

    /// Store object in object storage
    pub async fn put_object(&self, bucket: &str, key: &str, data: impl Into<Bytes>) -> Result<()> {
        let data = data.into();
        let capability = StandardCapability::StorageObjectS3.id();

        let request = Request::new(
            "put",
            json!({
                "bucket": bucket,
                "key": key,
                "data": base64::encode(data.as_ref()),
            }),
        );

        self.universal
            .invoke(capability, request)
            .await
            .context("Failed to put object")?;

        Ok(())
    }

    /// Retrieve object from object storage
    pub async fn get_object(&self, bucket: &str, key: &str) -> Result<Bytes> {
        let capability = StandardCapability::StorageObjectS3.id();

        let request = Request::new(
            "get",
            json!({
                "bucket": bucket,
                "key": key,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to get object")?;

        let data = response.data()?;
        let bytes = data
            .get("data")
            .and_then(|v| v.as_str())
            .map(base64::decode)
            .transpose()?
            .ok_or_else(|| crate::CliError::Other("Missing data in response".to_string()))?;

        Ok(Bytes::from(bytes))
    }

    /// Store key-value pair
    pub async fn put_kv(&self, key: &str, value: serde_json::Value) -> Result<()> {
        let capability = StandardCapability::StorageKeyValue.id();

        let request = Request::new(
            "put",
            json!({
                "key": key,
                "value": value,
            }),
        );

        self.universal
            .invoke(capability, request)
            .await
            .context("Failed to put key-value")?;

        Ok(())
    }

    /// Retrieve key-value pair
    pub async fn get_kv(&self, key: &str) -> Result<serde_json::Value> {
        let capability = StandardCapability::StorageKeyValue.id();

        let request = Request::new(
            "get",
            json!({
                "key": key,
            }),
        );

        let response = self
            .universal
            .invoke(capability, request)
            .await
            .context("Failed to get key-value")?;

        let data = response.data()?;
        let value = data
            .get("value")
            .cloned()
            .ok_or_else(|| crate::CliError::Other("Missing value in response".to_string()))?;

        Ok(value)
    }
}

/// Storage requirements for mounting distributed storage
#[derive(Debug, Clone)]
pub struct StorageRequirements {
    pub mount_point: PathBuf,
    pub capacity_gb: Option<u64>,
    pub access_mode: AccessMode,
    pub encryption: bool,
}

/// Storage access mode
#[derive(Debug, Clone, Copy)]
pub enum AccessMode {
    ReadOnly,
    ReadWrite,
}

/// Mount information
#[derive(Debug, Clone)]
pub struct MountInfo {
    pub mount_point: PathBuf,
    pub dataset_name: String,
    pub endpoint: String,
    pub backend_type: String,
}

/// Object storage connection
#[derive(Debug, Clone)]
pub struct ObjectStorageConnection {
    pub bucket: String,
    pub endpoint: String,
    pub region: String,
}

mod base64 {
    use base64::{engine::general_purpose::STANDARD, Engine as _};

    pub fn encode(data: &[u8]) -> String {
        STANDARD.encode(data)
    }

    pub fn decode(data: &str) -> Result<Vec<u8>, ::base64::DecodeError> {
        STANDARD.decode(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_requirements() {
        let requirements = StorageRequirements {
            mount_point: PathBuf::from("/mnt/data"),
            capacity_gb: Some(100),
            access_mode: AccessMode::ReadWrite,
            encryption: true,
        };

        assert_eq!(requirements.capacity_gb, Some(100));
        assert!(requirements.encryption);
    }

    #[test]
    fn test_storage_requirements_read_only() {
        let requirements = StorageRequirements {
            mount_point: PathBuf::from("/readonly"),
            capacity_gb: None,
            access_mode: AccessMode::ReadOnly,
            encryption: false,
        };
        assert!(matches!(requirements.access_mode, AccessMode::ReadOnly));
        assert!(requirements.capacity_gb.is_none());
    }

    #[test]
    fn test_access_mode_variants() {
        let ro = AccessMode::ReadOnly;
        let rw = AccessMode::ReadWrite;
        assert!(matches!(ro, AccessMode::ReadOnly));
        assert!(matches!(rw, AccessMode::ReadWrite));
    }

    #[test]
    fn test_mount_info_creation() {
        let info = MountInfo {
            mount_point: PathBuf::from("/mnt/zfs"),
            dataset_name: "pool/data".to_string(),
            endpoint: "tcp://storage:9000".to_string(),
            backend_type: "zfs".to_string(),
        };
        assert_eq!(info.dataset_name, "pool/data");
        assert_eq!(info.backend_type, "zfs");
    }

    #[test]
    fn test_mount_info_clone() {
        let info = MountInfo {
            mount_point: PathBuf::from("/mnt"),
            dataset_name: "default".to_string(),
            endpoint: "local".to_string(),
            backend_type: "nfs".to_string(),
        };
        let cloned = info.clone();
        assert_eq!(info.dataset_name, cloned.dataset_name);
    }

    #[test]
    fn test_object_storage_connection() {
        let conn = ObjectStorageConnection {
            bucket: "my-bucket".to_string(),
            endpoint: "https://s3.amazonaws.com".to_string(),
            region: "us-east-1".to_string(),
        };
        assert_eq!(conn.bucket, "my-bucket");
        assert_eq!(conn.region, "us-east-1");
    }

    #[test]
    fn test_storage_adapter_new() {
        use crate::ecosystem::adapters::AdapterFactory;
        let factory = AdapterFactory::new();
        let adapter = factory.storage_adapter().unwrap();
        let _ = adapter;
    }

    #[tokio::test]
    async fn test_mount_distributed_storage_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter().unwrap();

        let requirements = StorageRequirements {
            mount_point: PathBuf::from("/mnt/test"),
            capacity_gb: Some(100),
            access_mode: AccessMode::ReadWrite,
            encryption: false,
        };

        let result = storage.mount_distributed_storage(requirements).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connect_object_storage_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter().unwrap();

        let result = storage
            .connect_object_storage("my-bucket".to_string(), Some("us-east-1".to_string()))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_put_object_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter().unwrap();

        let result = storage
            .put_object("bucket", "key", bytes::Bytes::from("data"))
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_object_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter().unwrap();

        let result = storage.get_object("bucket", "key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_put_kv_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter().unwrap();

        let result = storage.put_kv("key", serde_json::json!("value")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_get_kv_no_service_returns_err() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter().unwrap();

        let result = storage.get_kv("key").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_connect_object_storage_no_region() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter().unwrap();

        let result = storage
            .connect_object_storage("bucket".to_string(), None)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mount_read_only_access_mode() {
        use crate::ecosystem::adapters::AdapterFactory;

        let factory = AdapterFactory::new();
        let storage = factory.storage_adapter().unwrap();

        let requirements = StorageRequirements {
            mount_point: PathBuf::from("/readonly"),
            capacity_gb: None,
            access_mode: AccessMode::ReadOnly,
            encryption: true,
        };

        let result = storage.mount_distributed_storage(requirements).await;
        assert!(result.is_err());
    }
}
