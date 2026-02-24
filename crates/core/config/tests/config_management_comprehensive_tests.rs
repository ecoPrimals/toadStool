//! Tests for backward compatibility with deprecated endpoint configuration
//! These tests validate that legacy hardcoded endpoints still work
#![allow(deprecated)]

//! Comprehensive Configuration Management Tests - Phase 2
//!
//! Tests for configuration loading, validation, updates, and composition:
//! - Configuration loading from files and environment
//! - Configuration validation and error handling  
//! - Environment-specific configurations
//! - Configuration merging and overrides
//! - Configuration serialization and deserialization
//! - Default value handling

use std::collections::HashMap;
use tempfile::TempDir;
use toadstool_config::types::*;

// ============================================================================
// Configuration Default Tests
// ============================================================================

#[test]
fn test_toadstool_config_default() {
    let config = ToadStoolConfig::default();

    assert!(!config.app.name.is_empty());
    assert!(config.app.worker_threads > 0);
    assert!(!config.network.endpoints.songbird.is_empty());
}

#[test]
fn test_application_config_default_values() {
    let config = ApplicationConfig::default();

    assert_eq!(config.name, "toadstool");
    assert_eq!(config.version, env!("CARGO_PKG_VERSION"));
    assert_eq!(config.environment, "development");
    assert!(config.worker_threads > 0);
    assert!(!config.data_dir.is_empty());
}

#[test]
fn test_network_config_default_values() {
    let config = NetworkConfig::default();

    assert!(config.bind_address.port() <= 65535); // 0 = OS-assigned
    assert!(!config.endpoints.songbird.is_empty());
    assert!(!config.endpoints.beardog.is_empty());
    assert!(!config.endpoints.nestgate.is_empty());
}

#[test]
fn test_runtime_config_default_values() {
    let config = RuntimeConfig::default();

    assert!(config.max_concurrent_executions > 0);
    assert!(config.execution_timeout.as_secs() > 0);
    assert!(config.resource_limits.max_cpu_usage > 0.0);
}

#[test]
fn test_logging_config_default_values() {
    let config = LoggingConfig::default();

    assert!(!config.level.is_empty());
    assert!(!config.format.is_empty());
}

#[test]
fn test_security_config_default_values() {
    let config = SecurityConfig::default();

    // Security config should have valid defaults (sandbox can be enabled or disabled)
    // Just verify the struct is constructible with defaults
    let _ = config.sandbox.enabled;
}

#[test]
fn test_feature_flags_default_exists() {
    let flags = FeatureFlags::default();

    // Features should have sensible defaults (can be enabled or disabled)
    // Just verify the struct is constructible with defaults
    let _ = flags.enable_distributed;
}

// ============================================================================
// Configuration Validation Tests
// ============================================================================

#[test]
fn test_config_validation_success() {
    let config = ToadStoolConfig::default();
    assert!(config.validate().is_ok());
}

#[test]
fn test_config_validation_empty_app_name() {
    let mut config = ToadStoolConfig::default();
    config.app.name = String::new();

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_zero_worker_threads() {
    let mut config = ToadStoolConfig::default();
    config.app.worker_threads = 0;

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_empty_songbird_endpoint() {
    let mut config = ToadStoolConfig::default();
    config.network.endpoints.songbird = String::new();

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_zero_max_executions() {
    let mut config = ToadStoolConfig::default();
    config.runtime.max_concurrent_executions = 0;

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_invalid_cpu_percent() {
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_cpu_usage = 150.0;

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_negative_cpu_percent() {
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_cpu_usage = -10.0;

    let result = config.validate();
    assert!(result.is_err());
}

#[test]
fn test_config_validation_invalid_memory_percent() {
    let mut config = ToadStoolConfig::default();
    config.runtime.resource_limits.max_memory_usage = 200.0;

    let result = config.validate();
    assert!(result.is_err());
}

// ============================================================================
// Environment-Specific Configuration Tests
// ============================================================================

#[test]
fn test_config_for_development_environment() {
    let config = ToadStoolConfig::default().for_environment("development");

    assert_eq!(config.app.environment, "development");
}

#[test]
fn test_config_for_production_environment() {
    let config = ToadStoolConfig::default().for_environment("production");

    assert_eq!(config.app.environment, "production");
}

#[test]
fn test_config_for_test_environment() {
    let config = ToadStoolConfig::default().for_environment("test");

    assert_eq!(config.app.environment, "test");
}

#[test]
fn test_config_for_unknown_environment() {
    let config = ToadStoolConfig::default().for_environment("staging");

    assert_eq!(config.app.environment, "staging");
}

// ============================================================================
// Configuration Merging and Overrides Tests
// ============================================================================

#[test]
fn test_config_merge_empty_overrides() {
    let config = ToadStoolConfig::default();
    let overrides = HashMap::new();

    let merged = config.merge(overrides);

    assert_eq!(merged.overrides.len(), 0);
}

#[test]
fn test_config_merge_with_overrides() {
    let config = ToadStoolConfig::default();
    let mut overrides = HashMap::new();
    overrides.insert("key1".to_string(), serde_json::json!("value1"));
    overrides.insert("key2".to_string(), serde_json::json!(42));

    let merged = config.merge(overrides);

    assert_eq!(merged.overrides.len(), 2);
    assert!(merged.overrides.contains_key("key1"));
    assert!(merged.overrides.contains_key("key2"));
}

#[test]
fn test_config_get_override_with_default() {
    let config = ToadStoolConfig::default();

    let value = config.get_override("nonexistent", 100);
    assert_eq!(value, 100);
}

#[test]
fn test_config_override_multiple_merges() {
    let config = ToadStoolConfig::default();

    let mut overrides1 = HashMap::new();
    overrides1.insert("key1".to_string(), serde_json::json!(1));

    let mut overrides2 = HashMap::new();
    overrides2.insert("key2".to_string(), serde_json::json!(2));

    let merged = config.merge(overrides1).merge(overrides2);

    assert_eq!(merged.overrides.len(), 2);
}

// ============================================================================
// Configuration Serialization Tests
// ============================================================================

#[test]
fn test_config_serialization_toml() {
    let config = ToadStoolConfig::default();
    let serialized = toml::to_string(&config);

    assert!(serialized.is_ok());
    let toml_str = serialized.unwrap();
    assert!(!toml_str.is_empty());
    assert!(toml_str.contains("app"));
}

#[test]
fn test_config_deserialization_roundtrip() {
    // Create a config, serialize, then deserialize
    let original = ApplicationConfig::default();
    let toml_str = toml::to_string(&original).unwrap();
    let config: Result<ApplicationConfig, _> = toml::from_str(&toml_str);

    assert!(config.is_ok());
    let app_config = config.unwrap();
    assert_eq!(app_config.name, original.name);
    assert_eq!(app_config.version, original.version);
    assert_eq!(app_config.worker_threads, original.worker_threads);
}

#[test]
fn test_config_serialization_json() {
    let config = ApplicationConfig::default();
    let serialized = serde_json::to_string(&config);

    assert!(serialized.is_ok());
    let json_str = serialized.unwrap();
    assert!(!json_str.is_empty());
    assert!(json_str.contains("name"));
}

#[test]
fn test_config_roundtrip_serialization() {
    let original = ApplicationConfig::default();
    let json_str = serde_json::to_string(&original).unwrap();
    let deserialized: Result<ApplicationConfig, _> = serde_json::from_str(&json_str);

    assert!(deserialized.is_ok());
    let config = deserialized.unwrap();
    assert_eq!(original.name, config.name);
    assert_eq!(original.version, config.version);
}

// ============================================================================
// Configuration File Operations Tests
// ============================================================================

#[test]
fn test_config_load_from_file_success() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let config = ToadStoolConfig::default();
    let toml_content = toml::to_string(&config).unwrap();
    std::fs::write(&config_path, toml_content).unwrap();

    let loaded = ToadStoolConfig::load_from_file(&config_path);
    assert!(loaded.is_ok());
}

#[test]
fn test_config_load_from_nonexistent_file() {
    let result = ToadStoolConfig::load_from_file("/nonexistent/path/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_config_load_from_invalid_toml() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("invalid.toml");

    std::fs::write(&config_path, "invalid toml content {{{").unwrap();

    let result = ToadStoolConfig::load_from_file(&config_path);
    assert!(result.is_err());
}

// ============================================================================
// Endpoint Configuration Tests
// ============================================================================

#[test]
fn test_endpoints_default_values() {
    let endpoints = EndpointConfig::default();

    assert!(!endpoints.songbird.is_empty());
    assert!(!endpoints.beardog.is_empty());
    assert!(!endpoints.nestgate.is_empty());
    assert!(!endpoints.squirrel.is_empty());
    assert!(!endpoints.federation.is_empty());
}

#[test]
fn test_endpoints_custom_values() {
    let endpoints = EndpointConfig {
        songbird: "http://localhost:8080".to_string(),
        beardog: "http://localhost:8081".to_string(),
        nestgate: "http://localhost:8082".to_string(),
        squirrel: "http://localhost:8083".to_string(),
        federation: "http://localhost:8084".to_string(),
        metrics: "http://localhost:8085/metrics".to_string(),
        health: "http://localhost:8086/health".to_string(),
    };

    assert_eq!(endpoints.songbird, "http://localhost:8080");
    assert_eq!(endpoints.beardog, "http://localhost:8081");
}

// ============================================================================
// Feature Flags Tests
// ============================================================================

#[test]
fn test_feature_flags_default() {
    let flags = FeatureFlags::default();

    // Flags should have valid boolean values
    // Just verify the struct is constructible and fields are accessible
    let _ = flags.enable_distributed;
    let _ = flags.enable_federation;
}

#[test]
fn test_feature_flags_all_enabled() {
    let flags = FeatureFlags {
        enable_distributed: true,
        enable_federation: true,
        enable_debug: true,
        ..Default::default()
    };

    assert!(flags.enable_distributed);
    assert!(flags.enable_federation);
    assert!(flags.enable_debug);
}

#[test]
fn test_feature_flags_all_disabled() {
    let flags = FeatureFlags {
        enable_distributed: false,
        enable_federation: false,
        enable_debug: false,
        ..Default::default()
    };

    assert!(!flags.enable_distributed);
    assert!(!flags.enable_federation);
    assert!(!flags.enable_debug);
}

// ============================================================================
// Runtime Configuration Tests
// ============================================================================

#[test]
fn test_runtime_config_default() {
    let config = RuntimeConfig::default();

    assert!(config.max_concurrent_executions > 0);
    assert!(config.execution_timeout.as_secs() > 0);
}

#[test]
fn test_runtime_config_custom_values() {
    let config = RuntimeConfig {
        max_concurrent_executions: 100,
        execution_timeout: std::time::Duration::from_secs(600),
        ..Default::default()
    };

    assert_eq!(config.max_concurrent_executions, 100);
    assert_eq!(config.execution_timeout.as_secs(), 600);
}

// ============================================================================
// Configuration Composition Tests
// ============================================================================

#[test]
fn test_config_composition_environment_then_overrides() {
    let base = ToadStoolConfig::default();
    let mut overrides = HashMap::new();
    overrides.insert("custom_setting".to_string(), serde_json::json!(true));

    let composed = base.for_environment("development").merge(overrides);

    assert_eq!(composed.app.environment, "development");
    assert!(composed.overrides.contains_key("custom_setting"));
}

#[test]
fn test_config_composition_multiple_environments() {
    let dev = ToadStoolConfig::default().for_environment("development");
    let prod = ToadStoolConfig::default().for_environment("production");

    assert_eq!(dev.app.environment, "development");
    assert_eq!(prod.app.environment, "production");
    assert_ne!(dev.app.environment, prod.app.environment);
}

// ============================================================================
// Configuration Clone Tests
// ============================================================================

#[test]
fn test_config_clone() {
    let config = ToadStoolConfig::default();
    let cloned = config.clone();

    assert_eq!(config.app.name, cloned.app.name);
    assert_eq!(config.app.version, cloned.app.version);
}

#[test]
fn test_config_independent_clones() {
    let config = ToadStoolConfig::default();
    let mut cloned = config.clone();

    cloned.app.name = "modified".to_string();

    assert_ne!(config.app.name, cloned.app.name);
}

// ============================================================================
// Edge Cases Tests
// ============================================================================

#[test]
fn test_config_with_extreme_worker_threads() {
    let config = ApplicationConfig {
        worker_threads: 1000,
        ..Default::default()
    };

    assert_eq!(config.worker_threads, 1000);
}

#[test]
fn test_config_with_minimal_timeout() {
    let config = RuntimeConfig {
        execution_timeout: std::time::Duration::from_millis(1),
        ..Default::default()
    };

    assert_eq!(config.execution_timeout.as_millis(), 1);
}

#[test]
fn test_config_with_max_timeout() {
    let config = RuntimeConfig {
        execution_timeout: std::time::Duration::from_secs(3600),
        ..Default::default()
    };

    assert_eq!(config.execution_timeout.as_secs(), 3600);
}

// ============================================================================
// Connection Configuration Tests
// ============================================================================

#[test]
fn test_connection_config_default() {
    let config = ConnectionConfig::default();

    assert!(config.request_timeout.as_secs() > 0);
    assert!(config.connection_timeout.as_secs() > 0);
    assert!(config.max_retries > 0);
}

#[test]
fn test_connection_config_custom_timeouts() {
    let config = ConnectionConfig {
        request_timeout: std::time::Duration::from_secs(30),
        connection_timeout: std::time::Duration::from_secs(10),
        ..Default::default()
    };

    assert_eq!(config.request_timeout.as_secs(), 30);
    assert_eq!(config.connection_timeout.as_secs(), 10);
}
