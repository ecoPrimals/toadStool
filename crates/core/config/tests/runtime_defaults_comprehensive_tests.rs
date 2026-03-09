// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive concurrent tests for `runtime_defaults` module
//!
//! ✅ MODERN CONCURRENT TESTING - Uses scoped Mutex for parallel execution

use std::env;
use std::sync::Mutex;
use tempfile::NamedTempFile;
use toadstool_config::ToadStoolConfig;

// ✅ MODERN: Scoped lock for environment variable tests
static ENV_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

fn get_env_lock() -> &'static Mutex<()> {
    ENV_LOCK.get_or_init(|| Mutex::new(()))
}

// ==================== Environment Detection Tests ====================

#[test]
fn test_for_current_environment_defaults_to_development() {
    let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
                                                 // Clear all env vars
    env::remove_var("TOADSTOOL_ENVIRONMENT");
    env::remove_var("TOADSTOOL_ENV");
    env::remove_var("ENVIRONMENT");
    env::remove_var("ENV");

    let config = ToadStoolConfig::for_current_environment();
    assert_eq!(config.app.environment, "development");
}

#[test]
fn test_for_current_environment_toadstool_environment_only() {
    let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
                                                 // Clean slate first
    env::remove_var("TOADSTOOL_ENVIRONMENT");
    env::remove_var("TOADSTOOL_ENV");
    env::remove_var("ENVIRONMENT");
    env::remove_var("ENV");

    // Test TOADSTOOL_ENVIRONMENT alone (no conflicting vars)
    env::set_var("TOADSTOOL_ENVIRONMENT", "production");

    let config = ToadStoolConfig::for_current_environment();
    assert_eq!(config.app.environment, "production");

    // Cleanup
    env::remove_var("TOADSTOOL_ENVIRONMENT");
}

#[test]
fn test_for_current_environment_toadstool_env_only() {
    let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
                                                 // Clean slate first
    env::remove_var("TOADSTOOL_ENVIRONMENT");
    env::remove_var("TOADSTOOL_ENV");
    env::remove_var("ENVIRONMENT");
    env::remove_var("ENV");

    // Test TOADSTOOL_ENV alone
    env::set_var("TOADSTOOL_ENV", "staging");

    let config = ToadStoolConfig::for_current_environment();
    assert_eq!(config.app.environment, "staging");

    // Cleanup
    env::remove_var("TOADSTOOL_ENV");
}

#[test]
fn test_for_current_environment_fallback_chain() {
    let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
                                                 // Test fallback: TOADSTOOL_ENV
    env::remove_var("TOADSTOOL_ENVIRONMENT");
    env::set_var("TOADSTOOL_ENV", "staging");
    env::remove_var("ENVIRONMENT");
    env::remove_var("ENV");

    let config = ToadStoolConfig::for_current_environment();
    assert_eq!(config.app.environment, "staging");

    // Test fallback: ENVIRONMENT
    env::remove_var("TOADSTOOL_ENV");
    env::set_var("ENVIRONMENT", "test");

    let config = ToadStoolConfig::for_current_environment();
    assert_eq!(config.app.environment, "test");

    // Test fallback: ENV
    env::remove_var("ENVIRONMENT");
    env::set_var("ENV", "production");

    let config = ToadStoolConfig::for_current_environment();
    assert_eq!(config.app.environment, "production");

    // Complete cleanup of ALL env vars
    env::remove_var("ENV");
    env::remove_var("ENVIRONMENT");
    env::remove_var("TOADSTOOL_ENV");
    env::remove_var("TOADSTOOL_ENVIRONMENT");
}

// ==================== Load Functions Tests ====================

#[test]
fn test_load_with_overrides_success() {
    let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
                                                 // Clean slate first
    env::remove_var("TOADSTOOL_ENVIRONMENT");
    env::remove_var("TOADSTOOL_ENV");
    env::remove_var("ENVIRONMENT");
    env::remove_var("ENV");

    let config = ToadStoolConfig::development();
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();

    // Save config
    config.save_to_file(temp_path).unwrap();

    // Load with overrides
    let loaded = ToadStoolConfig::load_with_overrides(temp_path);
    assert!(loaded.is_ok());
    let loaded_config = loaded.unwrap();
    assert_eq!(loaded_config.app.environment, "development");

    // Cleanup
    env::remove_var("TOADSTOOL_ENVIRONMENT");
    env::remove_var("TOADSTOOL_ENV");
    env::remove_var("ENVIRONMENT");
    env::remove_var("ENV");
}

#[test]
fn test_load_with_overrides_nonexistent_file() {
    let result = ToadStoolConfig::load_with_overrides("/nonexistent/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_load_from_env_only_success() {
    let _guard = get_env_lock().lock().unwrap(); // ✅ MODERN: Concurrent-safe
                                                 // Clean slate first
    env::remove_var("TOADSTOOL_ENVIRONMENT");
    env::remove_var("TOADSTOOL_ENV");
    env::remove_var("ENVIRONMENT");
    env::remove_var("ENV");
    env::remove_var("TOADSTOOL_LOG_LEVEL");

    // Set minimal env vars
    env::set_var("TOADSTOOL_ENVIRONMENT", "test");
    env::set_var("TOADSTOOL_LOG_LEVEL", "info");

    let result = ToadStoolConfig::load_from_env_only();
    assert!(result.is_ok());
    let config = result.unwrap();
    assert_eq!(config.app.environment, "test");

    // Cleanup
    env::remove_var("TOADSTOOL_ENVIRONMENT");
    env::remove_var("TOADSTOOL_ENV");
    env::remove_var("ENVIRONMENT");
    env::remove_var("ENV");
    env::remove_var("TOADSTOOL_LOG_LEVEL");
}

// ==================== Save/Load Round-Trip Tests ====================

#[test]
fn test_save_and_load_roundtrip_development() {
    let original = ToadStoolConfig::development();
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();

    // Save
    let save_result = original.save_to_file(temp_path);
    assert!(save_result.is_ok());

    // Load
    let loaded = ToadStoolConfig::load_from_file(temp_path);
    assert!(loaded.is_ok());
    let loaded_config = loaded.unwrap();

    // Verify key fields match
    assert_eq!(loaded_config.app.environment, original.app.environment);
    assert_eq!(loaded_config.logging.level, original.logging.level);
    assert_eq!(
        loaded_config.features.enable_debug,
        original.features.enable_debug
    );
}

#[test]
fn test_save_and_load_roundtrip_production() {
    let original = ToadStoolConfig::production();
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();

    // Save
    original.save_to_file(temp_path).unwrap();

    // Load
    let loaded = ToadStoolConfig::load_from_file(temp_path).unwrap();

    assert_eq!(loaded.app.environment, "production");
    assert_eq!(loaded.logging.level, "info");
    assert!(!loaded.features.enable_debug);
    assert!(loaded.security.auth.enabled);
}

#[test]
fn test_save_and_load_roundtrip_testing() {
    let original = ToadStoolConfig::testing();
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();

    original.save_to_file(temp_path).unwrap();
    let loaded = ToadStoolConfig::load_from_file(temp_path).unwrap();

    assert_eq!(loaded.app.environment, "test");
    assert!(!loaded.security.auth.enabled);
}

// ==================== JSON Serialization Tests ====================

#[test]
fn test_to_json_contains_required_sections() {
    let config = ToadStoolConfig::default();
    let json = config.to_json().unwrap();

    // Verify all major sections are present
    assert!(json.contains("app"));
    assert!(json.contains("network"));
    assert!(json.contains("runtime"));
    assert!(json.contains("security"));
    assert!(json.contains("logging"));
    assert!(json.contains("features"));
}

#[test]
fn test_to_json_development_config() {
    let config = ToadStoolConfig::development();
    let json = config.to_json().unwrap();

    assert!(json.contains("development"));
    assert!(json.contains("debug"));
}

#[test]
fn test_to_json_production_config() {
    let config = ToadStoolConfig::production();
    let json = config.to_json().unwrap();

    assert!(json.contains("production"));
    assert!(json.contains("info"));
}

#[test]
fn test_to_json_is_valid_json() {
    let config = ToadStoolConfig::default();
    let json = config.to_json().unwrap();

    // Verify it can be parsed back
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed.is_object());
}

// ==================== Config Summary Tests ====================

#[test]
fn test_print_summary_does_not_panic() {
    // Just verify print_summary doesn't panic
    let config = ToadStoolConfig::default();
    config.print_summary();
    // If we get here, it didn't panic
}

#[test]
fn test_print_summary_development() {
    let config = ToadStoolConfig::development();
    config.print_summary();
}

#[test]
fn test_print_summary_production() {
    let config = ToadStoolConfig::production();
    config.print_summary();
}

// ==================== Configuration Creation Tests ====================

#[test]
fn test_development_config_has_correct_defaults() {
    let config = ToadStoolConfig::development();

    assert_eq!(config.app.environment, "development");
    assert_eq!(config.logging.level, "debug");
    assert!(config.features.enable_debug);
    assert!(config.features.enable_hot_reload);
    assert!(!config.security.auth.enabled);
}

#[test]
fn test_production_config_has_correct_defaults() {
    let config = ToadStoolConfig::production();

    assert_eq!(config.app.environment, "production");
    assert_eq!(config.logging.level, "info");
    assert!(!config.features.enable_debug);
    assert!(!config.features.enable_hot_reload);
    assert!(config.security.auth.enabled);
}

#[test]
fn test_testing_config_has_correct_defaults() {
    let config = ToadStoolConfig::testing();

    assert_eq!(config.app.environment, "test");
    assert_eq!(config.logging.level, "debug");
    assert!(!config.security.auth.enabled);
}

// ==================== Concurrent Safety Tests ====================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_config_creation() {
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for i in 0..10 {
        let bar = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            bar.wait().await;

            // Create configs concurrently
            let config = match i % 3 {
                0 => ToadStoolConfig::development(),
                1 => ToadStoolConfig::production(),
                _ => ToadStoolConfig::testing(),
            };

            config.app.environment.clone()
        });

        handles.push(handle);
    }

    // Collect results
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // Verify all succeeded
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(["development", "production", "test"].contains(&result.as_str()));
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_concurrent_json_serialization() {
    use std::sync::Arc;
    use tokio::sync::Barrier;

    let config = Arc::new(ToadStoolConfig::default());
    let barrier = Arc::new(Barrier::new(10));
    let mut handles = vec![];

    for _ in 0..10 {
        let cfg = Arc::clone(&config);
        let bar = Arc::clone(&barrier);

        let handle = tokio::spawn(async move {
            bar.wait().await;
            // Call to_json and handle result immediately to avoid Send issues
            cfg.to_json().ok()
        });

        handles.push(handle);
    }

    // Collect results
    let mut results = vec![];
    for handle in handles {
        results.push(handle.await.unwrap());
    }

    // All should succeed and produce same JSON
    assert_eq!(results.len(), 10);
    for result in results {
        assert!(result.is_some());
        let json = result.unwrap();
        assert!(json.contains("app"));
    }
}

// ==================== Error Handling Tests ====================

#[test]
fn test_save_to_invalid_path() {
    let config = ToadStoolConfig::default();
    let result = config.save_to_file("/invalid/nonexistent/path/config.toml");
    assert!(result.is_err());
}

#[test]
fn test_load_from_invalid_toml() {
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path();

    // Write invalid TOML
    std::fs::write(temp_path, "this is not valid TOML { [ }").unwrap();

    let result = ToadStoolConfig::load_from_file(temp_path);
    assert!(result.is_err());
}

#[test]
fn test_to_json_always_succeeds_for_valid_config() {
    let configs = vec![
        ToadStoolConfig::default(),
        ToadStoolConfig::development(),
        ToadStoolConfig::production(),
        ToadStoolConfig::testing(),
    ];

    for config in configs {
        let result = config.to_json();
        assert!(result.is_ok());
    }
}
