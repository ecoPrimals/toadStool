//! Storage configuration types for BiomeOS integration
//!
//! This module contains types for storage provisioning, volumes,
//! and NestGate integration.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Storage configuration for the biome
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BiomeStorage {
    /// Enable `NestGate` integration
    pub nestgate_integration: bool,
    /// Global storage settings
    pub global_settings: HashMap<String, serde_json::Value>,
    /// Storage classes
    pub storage_classes: Vec<StorageClass>,
    /// Persistent volumes
    pub persistent_volumes: Vec<PersistentVolume>,
}

/// Storage class definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageClass {
    /// Storage class name
    pub name: String,
    /// Provisioner
    pub provisioner: String,
    /// Parameters
    pub parameters: HashMap<String, String>,
    /// Reclaim policy
    pub reclaim_policy: String,
}

/// Persistent volume definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentVolume {
    /// Volume name
    pub name: String,
    /// Capacity
    pub capacity: String,
    /// Access modes
    pub access_modes: Vec<String>,
    /// Storage class
    pub storage_class: String,
    /// Host path (for local storage)
    pub host_path: Option<PathBuf>,
}

/// Storage provisioning request to `NestGate`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageProvisioningRequest {
    /// Volume name
    pub volume_name: String,
    /// Volume size
    pub size: String,
    /// Storage class
    pub storage_class: Option<String>,
    /// Access modes
    pub access_modes: Vec<String>,
    /// Backup policy
    pub backup_policy: Option<String>,
    /// Replication settings
    pub replication: Option<ReplicationSettings>,
}

/// Replication settings for storage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplicationSettings {
    /// Enable replication
    pub enabled: bool,
    /// Replication factor
    pub factor: u32,
    /// Replication strategy
    pub strategy: String,
}

/// Volume information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeInfo {
    /// Volume name
    pub name: String,
    /// Volume ID
    pub id: String,
    /// Size
    pub size: String,
    /// Storage class
    pub storage_class: String,
    /// Status
    pub status: String,
    /// Created timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Volume mount information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeMountInfo {
    /// Volume name
    pub volume_name: String,
    /// Mount path in container
    pub mount_path: String,
    /// Read-only mount
    pub read_only: bool,
}

/// Volume mount specification for services
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VolumeMountSpec {
    /// Volume name to mount
    pub name: String,
    /// Mount path in container
    pub mount_path: String,
    /// Sub-path within volume
    pub sub_path: Option<String>,
    /// Read-only mount
    pub read_only: bool,
}
