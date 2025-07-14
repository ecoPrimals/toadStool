//! Configuration structures for NestGate integration

use std::path::PathBuf;
use std::time::Duration;

use crate::types::{CompressionType, EncryptionType, StorageTier};

/// NestGate client configuration
#[derive(Debug, Clone)]
pub struct NestGateConfig {
    /// NestGate server endpoint
    pub endpoint: String,

    /// Authentication token
    pub auth_token: Option<String>,

    /// Request timeout
    pub timeout: Duration,

    /// Connection pool size
    pub max_connections: u32,

    /// Enable compression
    pub enable_compression: bool,

    /// Cache configuration
    pub cache_config: CacheConfig,

    /// Storage preferences
    pub storage_preferences: StoragePreferences,
}

impl Default for NestGateConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:4040".to_string(),
            auth_token: None,
            timeout: Duration::from_secs(30),
            max_connections: 10,
            enable_compression: true,
            cache_config: CacheConfig::default(),
            storage_preferences: StoragePreferences::default(),
        }
    }
}

/// Cache configuration for NestGate operations
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Enable local caching
    pub enabled: bool,

    /// Cache size limit in bytes
    pub max_size_bytes: u64,

    /// Cache TTL for artifacts
    pub artifact_ttl: Duration,

    /// Cache TTL for metadata
    pub metadata_ttl: Duration,

    /// Local cache directory
    pub cache_dir: Option<PathBuf>,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_size_bytes: 1024 * 1024 * 1024,      // 1GB
            artifact_ttl: Duration::from_secs(3600), // 1 hour
            metadata_ttl: Duration::from_secs(300),  // 5 minutes
            cache_dir: None,
        }
    }
}

/// Storage preferences for data placement
#[derive(Debug, Clone)]
pub struct StoragePreferences {
    /// Preferred storage tier
    pub storage_tier: StorageTier,

    /// Replication factor
    pub replication_factor: u32,

    /// Compression preference
    pub compression: CompressionType,

    /// Encryption requirement
    pub encryption: EncryptionType,

    /// Geographic preferences
    pub regions: Vec<String>,
}

impl Default for StoragePreferences {
    fn default() -> Self {
        Self {
            storage_tier: StorageTier::Standard,
            replication_factor: 3,
            compression: CompressionType::Auto,
            encryption: EncryptionType::Default,
            regions: Vec::new(),
        }
    }
}
