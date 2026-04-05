// SPDX-License-Identifier: AGPL-3.0-or-later
//! Core types and data structures for `Storage` integration

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// `Storage` integration errors
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    /// Connection to storage service failed.
    #[error("Connection failed: {0}")]
    Connection(String),

    /// Authentication or authorization failed.
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Storage operation (store, retrieve, delete) failed.
    #[error("Storage operation failed: {0}")]
    Storage(String),

    /// Data pipeline execution or configuration error.
    #[error("Data pipeline error: {0}")]
    Pipeline(String),

    /// Artifact versioning or conflict error.
    #[error("Versioning error: {0}")]
    Versioning(String),

    /// Network or transport layer error.
    #[error("Network error: {0}")]
    Network(String),

    /// JSON or binary serialization/deserialization failed.
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error (file system, socket, etc.).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Internal or unexpected error.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type for storage-service operations.
pub type StorageServiceResult<T> = Result<T, StorageError>;

/// Legacy alias for [`StorageServiceResult`].
pub type NestGateResult<T> = StorageServiceResult<T>;

/// Storage tier options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageTier {
    /// High-performance storage
    Hot,
    /// Standard performance
    Standard,
    /// Infrequent access
    Cold,
    /// Archive storage
    Archive,
}

/// Compression options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// Automatic compression selection
    Auto,
    /// Gzip compression
    Gzip,
    /// LZ4 compression
    Lz4,
    /// Zstandard compression
    Zstd,
}

/// Encryption options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EncryptionType {
    /// No encryption
    None,
    /// Default ecosystem encryption
    Default,
    /// AES-256 encryption
    Aes256,
    /// Custom encryption
    Custom(String),
}

/// Artifact type classification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArtifactType {
    /// Execution input data
    ExecutionInput,
    /// Execution output data
    ExecutionOutput,
    /// Log files
    Logs,
    /// Binary executables
    Binary,
    /// Container images
    ContainerImage,
    /// WASM modules
    WasmModule,
    /// Data files
    DataFile,
    /// Configuration files
    Configuration,
    /// Model artifacts (ML models, etc.)
    Model,
    /// Custom artifact type
    Custom(String),
}

/// Artifact metadata structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactMetadata {
    /// Artifact identifier
    pub id: String,

    /// Artifact type
    pub artifact_type: ArtifactType,

    /// Content type/MIME type
    pub content_type: String,

    /// Size in bytes
    pub size_bytes: u64,

    /// Checksum (SHA-256)
    pub checksum: String,

    /// Creation timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub created_at: std::time::SystemTime,

    /// Last access timestamp
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub last_accessed: Option<std::time::SystemTime>,

    /// Tags for categorization
    pub tags: HashMap<String, String>,

    /// Associated execution ID
    pub execution_id: Option<Uuid>,

    /// Storage location information
    pub storage_info: StorageInfo,

    /// Version information
    pub version: Option<VersionInfo>,
}

/// Storage location and configuration information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageInfo {
    /// Storage node ID
    pub node_id: String,

    /// Physical storage path
    pub path: String,

    /// Storage tier used
    pub tier: StorageTier,

    /// Replication status
    pub replicated: bool,

    /// Compression applied
    pub compression: CompressionType,

    /// Encryption applied
    pub encryption: EncryptionType,
}

/// Version information for artifacts
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionInfo {
    /// Version number
    pub version: u64,

    /// Parent version (for branching)
    pub parent_version: Option<u64>,

    /// Version timestamp
    #[serde(with = "toadstool_common::system_time_serde")]
    pub timestamp: std::time::SystemTime,

    /// Version description
    pub description: Option<String>,

    /// Author information
    pub author: Option<String>,
}

/// Artifact filtering options
#[derive(Debug, Clone, Default, Serialize)]
pub struct ArtifactFilters {
    /// Filter by artifact type
    pub artifact_type: Option<ArtifactType>,

    /// Filter by execution ID
    pub execution_id: Option<Uuid>,

    /// Filter by creation date (artifacts created since this date)
    #[serde(with = "toadstool_common::system_time_serde::opt")]
    pub created_since: Option<std::time::SystemTime>,

    /// Filter by tags
    pub tags: HashMap<String, String>,
}

/// Cached artifact data
#[derive(Debug, Clone)]
pub struct CachedArtifact {
    /// Artifact metadata.
    pub metadata: ArtifactMetadata,
    /// Cached payload (if loaded into memory).
    pub data: Option<Vec<u8>>,
    /// When the artifact was cached.
    pub cached_at: std::time::SystemTime,
    /// Number of times the cache entry was accessed.
    pub access_count: u64,
}

/// Storage operation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageResult {
    /// Artifact ID
    pub id: Uuid,
    /// Storage operation status
    pub status: StorageStatus,
    /// Status message
    pub message: String,
}

/// Storage operation status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StorageStatus {
    /// Operation succeeded
    Success,
    /// Operation failed
    Failed,
    /// Operation in progress
    InProgress,
    /// Operation cancelled
    Cancelled,
}

/// Cache entry for artifacts
#[derive(Debug, Clone)]
pub struct CacheEntry {
    /// Artifact ID
    pub id: Uuid,
    /// Cached data
    pub data: Vec<u8>,
    /// Artifact metadata
    pub metadata: ArtifactMetadata,
    /// Cache timestamp
    pub cached_at: std::time::SystemTime,
}
