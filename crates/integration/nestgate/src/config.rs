//! Configuration structures for `NestGate` integration

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

use crate::types::{CompressionType, EncryptionType, StorageTier};

/// Storage client configuration
///
/// **Evolution**: Now supports capability-based discovery via `NestGateClient::discover()`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NestGateConfig {
    /// Storage server endpoint
    pub endpoint: String,

    /// Request timeout duration
    pub timeout: Duration,

    /// Number of retry attempts
    pub max_retries: u32,

    /// Authentication configuration
    pub auth: Option<String>,

    /// Cache configuration
    pub cache: Option<CacheConfig>,
}

impl Default for NestGateConfig {
    fn default() -> Self {
        // Use environment-aware configuration for endpoint
        let port: u16 = std::env::var("TOADSTOOL_STORAGE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .or_else(|| {
                std::env::var("TOADSTOOL_NESTGATE_PORT")
                    .ok()
                    .and_then(|p| p.parse().ok())
            })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nestgate_config_default() {
        let config = NestGateConfig::default();
        assert!(!config.endpoint.is_empty());
        assert_eq!(config.timeout, Duration::from_secs(30));
        assert_eq!(config.max_retries, 3);
        assert!(config.auth.is_none());
        assert!(config.cache.is_some());
    }

    #[test]
    fn test_cache_config_default() {
        let config = CacheConfig::default();
        assert!(config.enabled);
        assert!(config.cache_dir.is_none());
        assert_eq!(config.max_size, 1024 * 1024 * 1024); // 1GB
        assert_eq!(config.ttl, Duration::from_secs(3600));
    }

    #[test]
    fn test_storage_preferences_default() {
        let prefs = StoragePreferences::default();
        assert!(matches!(prefs.storage_tier, StorageTier::Standard));
        assert_eq!(prefs.replication_factor, 3);
        assert!(matches!(prefs.compression, CompressionType::Auto));
        assert!(matches!(prefs.encryption, EncryptionType::Default));
        assert_eq!(prefs.regions.len(), 0);
    }

    #[test]
    fn test_nestgate_config_custom() {
        let config = NestGateConfig {
            endpoint: "https://nestgate.example.com:8080".to_string(),
            timeout: Duration::from_secs(60),
            max_retries: 5,
            auth: Some("bearer-token-123".to_string()),
            cache: None,
        };

        assert_eq!(config.endpoint, "https://nestgate.example.com:8080");
        assert_eq!(config.timeout, Duration::from_secs(60));
        assert_eq!(config.max_retries, 5);
        assert_eq!(config.auth, Some("bearer-token-123".to_string()));
        assert!(config.cache.is_none());
    }

    #[test]
    fn test_cache_config_custom() {
        let config = CacheConfig {
            enabled: false,
            cache_dir: Some(PathBuf::from("/tmp/nestgate-cache")),
            max_size: 512 * 1024 * 1024, // 512MB
            ttl: Duration::from_secs(7200),
        };

        assert!(!config.enabled);
        assert_eq!(config.cache_dir, Some(PathBuf::from("/tmp/nestgate-cache")));
        assert_eq!(config.max_size, 512 * 1024 * 1024);
        assert_eq!(config.ttl, Duration::from_secs(7200));
    }

    #[test]
    fn test_storage_preferences_custom() {
        let prefs = StoragePreferences {
            storage_tier: StorageTier::Hot,
            replication_factor: 5,
            compression: CompressionType::Lz4,
            encryption: EncryptionType::Aes256,
            regions: vec!["us-west".to_string(), "us-east".to_string()],
        };

        assert!(matches!(prefs.storage_tier, StorageTier::Hot));
        assert_eq!(prefs.replication_factor, 5);
        assert!(matches!(prefs.compression, CompressionType::Lz4));
        assert!(matches!(prefs.encryption, EncryptionType::Aes256));
        assert_eq!(prefs.regions.len(), 2);
    }

    #[test]
    fn test_nestgate_config_clone() {
        let config1 = NestGateConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.endpoint, config2.endpoint);
        assert_eq!(config1.timeout, config2.timeout);
        assert_eq!(config1.max_retries, config2.max_retries);
    }

    #[test]
    fn test_cache_config_clone() {
        let config1 = CacheConfig::default();
        let config2 = config1.clone();

        assert_eq!(config1.enabled, config2.enabled);
        assert_eq!(config1.max_size, config2.max_size);
        assert_eq!(config1.ttl, config2.ttl);
    }

    #[test]
    fn test_storage_preferences_clone() {
        let prefs1 = StoragePreferences::default();
        let prefs2 = prefs1.clone();

        assert_eq!(prefs1.replication_factor, prefs2.replication_factor);
        assert_eq!(prefs1.regions.len(), prefs2.regions.len());
    }

    #[test]
    fn test_nestgate_config_serialization() {
        let config = NestGateConfig {
            endpoint: "http://localhost:8000".to_string(),
            timeout: Duration::from_secs(30),
            max_retries: 3,
            auth: None,
            cache: Some(CacheConfig::default()),
        };

        // Test serialization
        let json = serde_json::to_string(&config).expect("Failed to serialize");
        assert!(!json.is_empty());

        // Test deserialization
        let deserialized: NestGateConfig = serde_json::from_str(&json).expect("Failed to deserialize");
        assert_eq!(deserialized.endpoint, config.endpoint);
        assert_eq!(deserialized.max_retries, config.max_retries);
    }

    #[test]
    fn test_cache_config_serialization() {
        let config = CacheConfig::default();

        let json = serde_json::to_string(&config).expect("Failed to serialize");
        let deserialized: CacheConfig = serde_json::from_str(&json).expect("Failed to deserialize");

        assert_eq!(deserialized.enabled, config.enabled);
        assert_eq!(deserialized.max_size, config.max_size);
    }

    #[test]
    fn test_storage_preferences_with_regions() {
        let prefs = StoragePreferences {
            storage_tier: StorageTier::Standard,
            replication_factor: 3,
            compression: CompressionType::Auto,
            encryption: EncryptionType::Default,
            regions: vec!["eu-west".to_string(), "ap-south".to_string(), "us-central".to_string()],
        };

        assert_eq!(prefs.regions.len(), 3);
        assert_eq!(prefs.regions[0], "eu-west");
        assert_eq!(prefs.regions[1], "ap-south");
        assert_eq!(prefs.regions[2], "us-central");
    }
}
