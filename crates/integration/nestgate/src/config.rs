//! Configuration structures for `NestGate` integration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use crate::types::{CompressionType, EncryptionType, StorageTier};

/// `NestGate` client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestGateConfig {
    /// `NestGate` server endpoint
    pub endpoint: String,
    /// Request timeout duration
    pub timeout: Duration,
    /// Number of retry attempts
    pub max_retries: u32,
    /// Authentication configuration
    pub auth: Option<String>, // Simplified auth config
    /// Cache configuration
    pub cache: Option<CacheConfig>,
}

impl Default for NestGateConfig {
    fn default() -> Self {
        // Use environment-aware configuration
        let port: u16 = std::env::var("TOADSTOOL_NESTGATE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or_else(|| {
                let config = toadstool_config::env_config::EnvironmentConfig::from_env();
                config.network.nestgate_port
            });
        let config = toadstool_config::env_config::EnvironmentConfig::from_env();
        let host = &config.network.bind_address;

        Self {
            endpoint: format!("http://{host}:{port}"),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            auth: None,
            cache: Some(CacheConfig::default()),
        }
    }
}

/// Cache configuration for `NestGate` operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheConfig {
    /// Enable local caching
    pub enabled: bool,

    /// Local cache directory
    pub cache_dir: Option<PathBuf>,

    /// Cache size limit in bytes
    pub max_size: u64,

    /// Cache TTL for artifacts
    pub ttl: Duration,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            cache_dir: None,
            max_size: 1024 * 1024 * 1024,   // 1GB
            ttl: Duration::from_secs(3600), // 1 hour
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
