//! Core types and data structures for NestGate integration

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// NestGate integration errors
#[derive(Debug, thiserror::Error)]
pub enum NestGateError {
    #[error("Connection failed: {0}")]
    Connection(String),

    #[error("Authentication failed: {0}")]
    Authentication(String),

    #[error("Storage operation failed: {0}")]
    Storage(String),

    #[error("Data pipeline error: {0}")]
    Pipeline(String),

    #[error("Versioning error: {0}")]
    Versioning(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Internal error: {0}")]
    Internal(String),
}

pub type NestGateResult<T> = Result<T, NestGateError>;

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
    pub created_at: DateTime<Utc>,

    /// Last access timestamp
    pub last_accessed: Option<DateTime<Utc>>,

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
    pub timestamp: DateTime<Utc>,

    /// Version description
    pub description: Option<String>,

    /// Author information
    pub author: Option<String>,
}

/// Artifact filtering options
#[derive(Debug, Clone, Default)]
pub struct ArtifactFilters {
    /// Filter by artifact type
    pub artifact_type: Option<ArtifactType>,

    /// Filter by execution ID
    pub execution_id: Option<Uuid>,

    /// Filter by creation date (artifacts created since this date)
    pub created_since: Option<DateTime<Utc>>,

    /// Filter by tags
    pub tags: HashMap<String, String>,
}

/// Cached artifact data
#[derive(Debug, Clone)]
pub struct CachedArtifact {
    pub metadata: ArtifactMetadata,
    pub data: Option<Vec<u8>>,
    pub cached_at: DateTime<Utc>,
    pub access_count: u64,
}
