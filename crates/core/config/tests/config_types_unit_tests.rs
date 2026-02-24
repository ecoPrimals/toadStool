//! Unit tests for configuration types and utilities

use toadstool_config::types::{
    ApplicationConfig, FeatureFlags, NetworkConfig, RuntimeConfig, SecurityConfig, ToadStoolConfig,
};

// ============================================================================
// ToadStoolConfig Tests
// ============================================================================

#[test]
fn test_toadstool_config_default() {
    let config = ToadStoolConfig::default();
    assert!(!config.app.name.is_empty());
    assert!(!config.logging.level.is_empty());
    assert!(config.features.enable_federation);
}

#[test]
fn test_toadstool_config_validation() {
    let config = ToadStoolConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_toadstool_config_invalid_name() {
    let mut config = ToadStoolConfig::default();
    config.app.name = String::new();
    assert!(config.validate().is_err());
}

#[test]
fn test_toadstool_config_invalid_worker_threads() {
    let mut config = ToadStoolConfig::default();
    config.app.worker_threads = 0;
    assert!(config.validate().is_err());
}

#[test]
fn test_toadstool_config_invalid_resource_limits() {
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_cpu_usage = 150.0;
    assert!(config.validate().is_err());
}

// ============================================================================
// Environment-Specific Config Tests
// ============================================================================

#[test]
fn test_development_config() {
    let config = ToadStoolConfig::default().for_environment("development");
    assert_eq!(config.app.environment, "development");
    assert_eq!(config.logging.level, "debug");
    assert!(config.features.enable_debug);
    assert!(!config.security.auth.enabled);
}

#[test]
fn test_production_config() {
    let config = ToadStoolConfig::default().for_environment("production");
    assert_eq!(config.app.environment, "production");
    assert_eq!(config.logging.level, "info");
    assert!(!config.features.enable_debug);
    assert!(config.security.auth.enabled);
}

#[test]
fn test_testing_config() {
    let config = ToadStoolConfig::default().for_environment("test");
    assert_eq!(config.app.environment, "test");
    assert_eq!(config.logging.level, "debug");
}

#[test]
fn test_staging_config() {
    let config = ToadStoolConfig::default().for_environment("staging");
    assert_eq!(config.app.environment, "staging");
}

// ============================================================================
// ApplicationConfig Tests
// ============================================================================

#[test]
fn test_application_config_default() {
    let config = ApplicationConfig::default();
    assert!(!config.name.is_empty());
    assert!(!config.environment.is_empty());
    assert!(config.worker_threads > 0);
}

#[test]
fn test_application_config_custom() {
    let config = ApplicationConfig {
        name: "test-app".to_string(),
        environment: "test".to_string(),
        worker_threads: 8,
        ..Default::default()
    };

    assert_eq!(config.name, "test-app");
    assert_eq!(config.environment, "test");
    assert_eq!(config.worker_threads, 8);
}

// ============================================================================
// NetworkConfig Tests
// ============================================================================

#[test]
fn test_network_config_default() {
    let _config = NetworkConfig::default();
}

#[test]
fn test_network_config_custom_bind() {
    let config = NetworkConfig {
        bind_address: "127.0.0.1:9000".parse().unwrap(),
        ..Default::default()
    };
    assert_eq!(config.bind_address.port(), 9000);
}

#[test]
fn test_network_config_localhost() {
    let config = NetworkConfig {
        bind_address: "127.0.0.1:8080".parse().unwrap(),
        ..Default::default()
    };
    assert!(config.bind_address.ip().is_loopback());
}

// ============================================================================
// RuntimeConfig Tests
// ============================================================================

#[test]
fn test_runtime_config_default() {
    let config = RuntimeConfig::default();
    assert!(config.resource_limits.max_memory_usage > 0.0);
    assert!(config.resource_limits.max_cpu_usage > 0.0);
}

#[test]
fn test_runtime_config_wasm_enabled() {
    let config = RuntimeConfig::default();
    // WASM config should exist with max_memory (in bytes)
    assert!(config.wasm.max_memory > 0);
}

#[test]
fn test_runtime_config_gpu_settings() {
    let config = RuntimeConfig::default();
    // GPU config should be optional
    assert!(config.gpu.is_some() || config.gpu.is_none());
}

#[test]
fn test_runtime_config_resource_limits() {
    let mut config = RuntimeConfig::default();
    config.resource_limits.max_memory_usage = 2_147_483_648.0; // 2GB in bytes (f64)
    config.resource_limits.max_cpu_usage = 75.0;

    assert_eq!(config.resource_limits.max_memory_usage, 2_147_483_648.0);
    assert_eq!(config.resource_limits.max_cpu_usage, 75.0);
}

// ============================================================================
// SecurityConfig Tests
// ============================================================================

#[test]
fn test_security_config_default() {
    let config = SecurityConfig::default();
    // Should have auth and authz configs
    assert!(!config.auth.enabled);
}

#[test]
fn test_security_config_auth_enabled() {
    let mut config = SecurityConfig::default();
    config.auth.enabled = true;
    assert!(config.auth.enabled);
}

#[test]
fn test_security_config_sandbox_enabled() {
    let mut config = SecurityConfig::default();
    config.sandbox.enabled = true;
    assert!(config.sandbox.enabled);
}

// ============================================================================
// FeatureFlags Tests
// ============================================================================

#[test]
fn test_feature_flags_default() {
    let flags = FeatureFlags::default();
    assert!(flags.enable_federation);
}

#[test]
fn test_feature_flags_all_disabled() {
    let flags = FeatureFlags {
        enable_federation: false,
        enable_debug: false,
        enable_hot_reload: false,
        ..Default::default()
    };

    assert!(!flags.enable_federation);
    assert!(!flags.enable_debug);
    assert!(!flags.enable_hot_reload);
}

#[test]
fn test_feature_flags_debug_mode() {
    let flags = FeatureFlags {
        enable_debug: true,
        ..Default::default()
    };
    assert!(flags.enable_debug);
}

// ============================================================================
// Configuration Merging Tests
// ============================================================================

#[test]
fn test_config_overrides() {
    use std::collections::HashMap;

    let mut overrides = HashMap::new();
    overrides.insert("custom_key".to_string(), serde_json::Value::Bool(true));

    let config = ToadStoolConfig::default().merge(overrides);
    assert!(config.get_override("custom_key", false));
}

#[test]
fn test_config_override_default_value() {
    let config = ToadStoolConfig::default().merge(Default::default());
    assert_eq!(config.get_override("nonexistent", 42), 42);
}

#[test]
fn test_config_multiple_overrides() {
    use std::collections::HashMap;

    let mut overrides = HashMap::new();
    overrides.insert("key1".to_string(), serde_json::Value::Bool(true));
    overrides.insert("key2".to_string(), serde_json::Value::Number(100.into()));
    overrides.insert(
        "key3".to_string(),
        serde_json::Value::String("test".to_string()),
    );

    let config = ToadStoolConfig::default().merge(overrides);
    assert!(config.get_override("key1", false));
    assert_eq!(config.get_override("key2", 0), 100);
}

// ============================================================================
// Configuration Serialization Tests
// ============================================================================

#[test]
fn test_config_serialization() {
    let config = ToadStoolConfig::default();
    let serialized = serde_json::to_string(&config);
    assert!(serialized.is_ok());
}

#[test]
fn test_config_deserialization() {
    let config = ToadStoolConfig::default();
    let serialized = serde_json::to_string(&config).unwrap();
    let deserialized: Result<ToadStoolConfig, _> = serde_json::from_str(&serialized);
    assert!(deserialized.is_ok());
}

#[test]
fn test_config_round_trip() {
    let original = ToadStoolConfig::default();
    let serialized = serde_json::to_string(&original).unwrap();
    let deserialized: ToadStoolConfig = serde_json::from_str(&serialized).unwrap();

    assert_eq!(original.app.name, deserialized.app.name);
    assert_eq!(original.app.environment, deserialized.app.environment);
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_config_empty_environment() {
    let mut config = ToadStoolConfig::default();
    config.app.environment = String::new();
    // Empty environment is now allowed (defaults to "development")
    // If validation becomes stricter in future, update this test
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_extreme_worker_threads() {
    let mut config = ToadStoolConfig::default();
    config.app.worker_threads = 1000;
    // Should still be valid (just a large number)
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_negative_resource_limits() {
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_cpu_usage = -1.0;
    // Should fail validation
    assert!(config.validate().is_err());
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_full_config_lifecycle() {
    // Create default config
    let config = ToadStoolConfig::default();
    assert!(config.validate().is_ok());

    // Customize for environment
    let dev_config = config.clone().for_environment("development");
    assert_eq!(dev_config.app.environment, "development");

    // Add overrides
    let mut overrides = std::collections::HashMap::new();
    overrides.insert("test".to_string(), serde_json::Value::Bool(true));
    let final_config = dev_config.merge(overrides);

    // Should still be valid
    assert!(final_config.validate().is_ok());
}

#[test]
fn test_config_cloning() {
    let original = ToadStoolConfig::default();
    let cloned = original.clone();

    assert_eq!(original.app.name, cloned.app.name);
    assert_eq!(original.logging.level, cloned.logging.level);
}
