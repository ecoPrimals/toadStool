// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for Storage configuration

use std::path::PathBuf;
use std::time::Duration;
use toadstool_integration_storage::*;

// ============================================================================
// StorageConfig Tests
// ============================================================================

#[test]
fn test_storage_ext_config_default() {
    let config = StorageConfig::default();

    assert!(config.endpoint.starts_with("http://"));
    assert_eq!(config.max_retries, 3);
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert!(config.cache.is_some());
}

#[test]
fn test_storage_ext_config_custom_endpoint() {
    let config = StorageConfig {
        endpoint: "http://custom-nest:9000".to_string(),
        ..Default::default()
    };

    assert_eq!(config.endpoint, "http://custom-nest:9000");
}

#[test]
fn test_storage_ext_config_custom_timeout() {
    let config = StorageConfig {
        timeout: Duration::from_secs(60),
        ..Default::default()
    };

    assert_eq!(config.timeout, Duration::from_secs(60));
}

#[test]
fn test_storage_ext_config_custom_retries() {
    let config = StorageConfig {
        max_retries: 5,
        ..Default::default()
    };

    assert_eq!(config.max_retries, 5);
}

#[test]
fn test_storage_ext_config_with_auth() {
    let config = StorageConfig {
        auth: Some("Bearer token123".to_string()),
        ..Default::default()
    };

    assert!(config.auth.is_some());
    assert_eq!(config.auth.unwrap(), "Bearer token123");
}

#[test]
fn test_storage_ext_config_without_auth() {
    let config = StorageConfig::default();

    assert!(config.auth.is_none());
}

#[test]
fn test_storage_ext_config_clone() {
    let config = StorageConfig::default();
    let cloned = config.clone();

    assert_eq!(config.endpoint, cloned.endpoint);
    assert_eq!(config.max_retries, cloned.max_retries);
}

#[test]
fn test_storage_ext_config_debug() {
    let config = StorageConfig::default();
    let debug_str = format!("{config:?}");

    assert!(debug_str.contains("StorageConfig"));
    assert!(debug_str.contains("endpoint"));
}

#[test]
fn test_storage_ext_config_serialization() {
    let config = StorageConfig::default();
    let json = serde_json::to_string(&config).expect("Failed to serialize");

    assert!(json.contains("endpoint"));
    assert!(json.contains("timeout"));
}

#[test]
fn test_storage_ext_config_deserialization() {
    let json = r#"{
        "endpoint": "http://test:8080",
        "timeout": {"secs": 30, "nanos": 0},
        "max_retries": 3,
        "auth": null,
        "cache": null
    }"#;

    let config: StorageConfig = serde_json::from_str(json).expect("Failed to deserialize");
    assert_eq!(config.endpoint, "http://test:8080");
    assert_eq!(config.max_retries, 3);
}

// ============================================================================
// CacheConfig Tests
// ============================================================================

#[test]
fn test_cache_config_default() {
    let cache = CacheConfig::default();

    assert!(cache.enabled);
    assert_eq!(cache.max_size, 1024 * 1024 * 1024); // 1GB
    assert_eq!(cache.ttl, Duration::from_secs(3600)); // 1 hour
    assert!(cache.cache_dir.is_none());
}

#[test]
fn test_cache_config_disabled() {
    let cache = CacheConfig {
        enabled: false,
        ..Default::default()
    };

    assert!(!cache.enabled);
}

#[test]
fn test_cache_config_custom_size() {
    let cache = CacheConfig {
        max_size: 2 * 1024 * 1024 * 1024, // 2GB
        ..Default::default()
    };

    assert_eq!(cache.max_size, 2 * 1024 * 1024 * 1024);
}

#[test]
fn test_cache_config_custom_ttl() {
    let cache = CacheConfig {
        ttl: Duration::from_secs(7200), // 2 hours
        ..Default::default()
    };

    assert_eq!(cache.ttl, Duration::from_secs(7200));
}

#[test]
fn test_cache_config_with_directory() {
    let cache = CacheConfig {
        cache_dir: Some(PathBuf::from("/tmp/storage-cache")),
        ..Default::default()
    };

    assert!(cache.cache_dir.is_some());
    assert_eq!(
        cache.cache_dir.unwrap(),
        PathBuf::from("/tmp/storage-cache")
    );
}

#[test]
fn test_cache_config_clone() {
    let cache = CacheConfig::default();
    let cloned = cache.clone();

    assert_eq!(cache.enabled, cloned.enabled);
    assert_eq!(cache.max_size, cloned.max_size);
}

#[test]
fn test_cache_config_debug() {
    let cache = CacheConfig::default();
    let debug_str = format!("{cache:?}");

    assert!(debug_str.contains("CacheConfig"));
    assert!(debug_str.contains("enabled"));
}

#[test]
fn test_cache_config_serialization() {
    let cache = CacheConfig::default();
    let json = serde_json::to_string(&cache).expect("Failed to serialize");

    assert!(json.contains("enabled"));
    assert!(json.contains("max_size"));
}

#[test]
fn test_cache_config_very_small_cache() {
    let cache = CacheConfig {
        max_size: 1024, // 1KB
        ..Default::default()
    };

    assert_eq!(cache.max_size, 1024);
}

#[test]
fn test_cache_config_very_large_cache() {
    let cache = CacheConfig {
        max_size: 100 * 1024 * 1024 * 1024, // 100GB
        ..Default::default()
    };

    assert_eq!(cache.max_size, 100 * 1024 * 1024 * 1024);
}

// ============================================================================
// StoragePreferences Tests
// ============================================================================

#[test]
fn test_storage_preferences_default() {
    let prefs = StoragePreferences::default();

    assert!(matches!(prefs.storage_tier, StorageTier::Standard));
    assert_eq!(prefs.replication_factor, 3);
    assert!(matches!(prefs.compression, CompressionType::Auto));
    assert!(matches!(prefs.encryption, EncryptionType::Default));
    assert!(prefs.regions.is_empty());
}

#[test]
fn test_storage_preferences_hot_tier() {
    let prefs = StoragePreferences {
        storage_tier: StorageTier::Hot,
        ..Default::default()
    };

    assert!(matches!(prefs.storage_tier, StorageTier::Hot));
}

#[test]
fn test_storage_preferences_cold_tier() {
    let prefs = StoragePreferences {
        storage_tier: StorageTier::Cold,
        ..Default::default()
    };

    assert!(matches!(prefs.storage_tier, StorageTier::Cold));
}

#[test]
fn test_storage_preferences_archive_tier() {
    let prefs = StoragePreferences {
        storage_tier: StorageTier::Archive,
        ..Default::default()
    };

    assert!(matches!(prefs.storage_tier, StorageTier::Archive));
}

#[test]
fn test_storage_preferences_high_replication() {
    let prefs = StoragePreferences {
        replication_factor: 5,
        ..Default::default()
    };

    assert_eq!(prefs.replication_factor, 5);
}

#[test]
fn test_storage_preferences_no_replication() {
    let prefs = StoragePreferences {
        replication_factor: 1,
        ..Default::default()
    };

    assert_eq!(prefs.replication_factor, 1);
}

#[test]
fn test_storage_preferences_gzip_compression() {
    let prefs = StoragePreferences {
        compression: CompressionType::Gzip,
        ..Default::default()
    };

    assert!(matches!(prefs.compression, CompressionType::Gzip));
}

#[test]
fn test_storage_preferences_lz4_compression() {
    let prefs = StoragePreferences {
        compression: CompressionType::Lz4,
        ..Default::default()
    };

    assert!(matches!(prefs.compression, CompressionType::Lz4));
}

#[test]
fn test_storage_preferences_zstd_compression() {
    let prefs = StoragePreferences {
        compression: CompressionType::Zstd,
        ..Default::default()
    };

    assert!(matches!(prefs.compression, CompressionType::Zstd));
}

#[test]
fn test_storage_preferences_no_compression() {
    let prefs = StoragePreferences {
        compression: CompressionType::None,
        ..Default::default()
    };

    assert!(matches!(prefs.compression, CompressionType::None));
}

#[test]
fn test_storage_preferences_aes256_encryption() {
    let prefs = StoragePreferences {
        encryption: EncryptionType::Aes256,
        ..Default::default()
    };

    assert!(matches!(prefs.encryption, EncryptionType::Aes256));
}

#[test]
fn test_storage_preferences_no_encryption() {
    let prefs = StoragePreferences {
        encryption: EncryptionType::None,
        ..Default::default()
    };

    assert!(matches!(prefs.encryption, EncryptionType::None));
}

#[test]
fn test_storage_preferences_with_regions() {
    let prefs = StoragePreferences {
        regions: vec![
            "us-east-1".to_string(),
            "us-west-2".to_string(),
            "eu-west-1".to_string(),
        ],
        ..Default::default()
    };

    assert_eq!(prefs.regions.len(), 3);
    assert!(prefs.regions.contains(&"us-east-1".to_string()));
}

#[test]
fn test_storage_preferences_clone() {
    let prefs = StoragePreferences::default();
    let cloned = prefs.clone();

    assert_eq!(prefs.replication_factor, cloned.replication_factor);
}

#[test]
fn test_storage_preferences_debug() {
    let prefs = StoragePreferences::default();
    let debug_str = format!("{prefs:?}");

    assert!(debug_str.contains("StoragePreferences"));
    assert!(debug_str.contains("replication_factor"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_config_with_cache_enabled() {
    let config = StorageConfig {
        cache: Some(CacheConfig {
            enabled: true,
            cache_dir: Some(PathBuf::from("/tmp/cache")),
            max_size: 512 * 1024 * 1024, // 512MB
            ttl: Duration::from_secs(1800),
        }),
        ..Default::default()
    };

    assert!(config.cache.is_some());
    let cache = config.cache.unwrap();
    assert!(cache.enabled);
    assert_eq!(cache.max_size, 512 * 1024 * 1024);
}

#[test]
fn test_config_with_cache_disabled() {
    let config = StorageConfig {
        cache: None,
        ..Default::default()
    };

    assert!(config.cache.is_none());
}

#[test]
fn test_full_configuration_scenario() {
    let mut config = StorageConfig {
        endpoint: "http://production-storage:8080".to_string(),
        timeout: Duration::from_secs(120),
        max_retries: 5,
        ..Default::default()
    };
    config.auth = Some("Bearer production-token".to_string());
    config.cache = Some(CacheConfig {
        enabled: true,
        cache_dir: Some(PathBuf::from("/var/cache/storage")),
        max_size: 10 * 1024 * 1024 * 1024,  // 10GB
        ttl: Duration::from_secs(3600 * 4), // 4 hours
    });

    assert_eq!(config.endpoint, "http://production-storage:8080");
    assert_eq!(config.timeout, Duration::from_secs(120));
    assert_eq!(config.max_retries, 5);
    assert!(config.auth.is_some());
    assert!(config.cache.is_some());
}

#[test]
fn test_storage_preferences_complete_scenario() {
    let prefs = StoragePreferences {
        storage_tier: StorageTier::Hot,
        replication_factor: 5,
        compression: CompressionType::Zstd,
        encryption: EncryptionType::Aes256,
        regions: vec!["us-east-1".to_string(), "us-west-2".to_string()],
    };

    assert!(matches!(prefs.storage_tier, StorageTier::Hot));
    assert_eq!(prefs.replication_factor, 5);
    assert!(matches!(prefs.compression, CompressionType::Zstd));
    assert!(matches!(prefs.encryption, EncryptionType::Aes256));
    assert_eq!(prefs.regions.len(), 2);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_cache_config_zero_ttl() {
    let cache = CacheConfig {
        ttl: Duration::from_secs(0),
        ..Default::default()
    };

    assert_eq!(cache.ttl, Duration::from_secs(0));
}

#[test]
fn test_cache_config_very_long_ttl() {
    let cache = CacheConfig {
        ttl: Duration::from_secs(86400 * 365), // 1 year
        ..Default::default()
    };

    assert_eq!(cache.ttl, Duration::from_secs(86400 * 365));
}

#[test]
fn test_storage_preferences_many_regions() {
    let prefs = StoragePreferences {
        regions: (0..20).map(|i| format!("region-{i}")).collect(),
        ..Default::default()
    };

    assert_eq!(prefs.regions.len(), 20);
}

#[test]
fn test_config_extreme_timeout() {
    let config = StorageConfig {
        timeout: Duration::from_secs(3600), // 1 hour
        ..Default::default()
    };

    assert_eq!(config.timeout, Duration::from_secs(3600));
}

#[test]
fn test_config_zero_retries() {
    let config = StorageConfig {
        max_retries: 0,
        ..Default::default()
    };

    assert_eq!(config.max_retries, 0);
}

#[test]
fn test_config_many_retries() {
    let config = StorageConfig {
        max_retries: 100,
        ..Default::default()
    };

    assert_eq!(config.max_retries, 100);
}
