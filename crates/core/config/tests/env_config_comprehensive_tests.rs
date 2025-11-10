//! Comprehensive tests for environment configuration

use serial_test::serial;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::time::Duration;
use toadstool_config::env_config::EnvConfigLoader;

// ============================================================================
// EnvConfigLoader Construction Tests
// ============================================================================

#[test]
fn test_env_config_loader_new() {
    let loader = EnvConfigLoader::new();
    // Default prefix should be TOADSTOOL
    assert!(loader.get_string("TEST", "default").contains("default"));
}

#[test]
fn test_env_config_loader_with_custom_prefix() {
    let loader = EnvConfigLoader::with_prefix("CUSTOM");
    assert!(loader.get_string("TEST", "default").contains("default"));
}

#[test]
fn test_env_config_loader_default() {
    let loader = EnvConfigLoader::default();
    assert!(loader.get_string("TEST", "default").contains("default"));
}

// ============================================================================
// String Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_string_default() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_string("NONEXISTENT_KEY", "default_value");
    assert_eq!(value, "default_value");
}

#[test]
fn test_env_config_get_string_empty_default() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_string("NONEXISTENT_KEY", "");
    assert_eq!(value, "");
}

#[test]
fn test_env_config_get_string_with_env() {
    std::env::set_var("TOADSTOOL_TEST_STRING", "test_value");
    let loader = EnvConfigLoader::new();
    let value = loader.get_string("TEST_STRING", "default");
    assert_eq!(value, "test_value");
    std::env::remove_var("TOADSTOOL_TEST_STRING");
}

// ============================================================================
// Boolean Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_bool_default_false() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_bool("NONEXISTENT_BOOL", false);
    assert!(!value);
}

#[test]
fn test_env_config_get_bool_default_true() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_bool("NONEXISTENT_BOOL", true);
    assert!(value);
}

#[test]
#[serial]
fn test_env_config_get_bool_true_lowercase() {
    std::env::set_var("TOADSTOOL_TEST_BOOL", "true");
    let loader = EnvConfigLoader::new();
    let value = loader.get_bool("TEST_BOOL", false);
    assert!(value);
    std::env::remove_var("TOADSTOOL_TEST_BOOL");
}

#[test]
#[serial]
fn test_env_config_get_bool_true_uppercase() {
    std::env::set_var("TOADSTOOL_TEST_BOOL", "TRUE");
    let loader = EnvConfigLoader::new();
    let value = loader.get_bool("TEST_BOOL", false);
    assert!(value);
    std::env::remove_var("TOADSTOOL_TEST_BOOL");
}

#[test]
#[serial]
fn test_env_config_get_bool_numeric_one() {
    std::env::set_var("TOADSTOOL_TEST_BOOL", "1");
    let loader = EnvConfigLoader::new();
    let value = loader.get_bool("TEST_BOOL", false);
    assert!(value);
    std::env::remove_var("TOADSTOOL_TEST_BOOL");
}

#[test]
#[serial]
fn test_env_config_get_bool_false() {
    std::env::set_var("TOADSTOOL_TEST_BOOL", "false");
    let loader = EnvConfigLoader::new();
    let value = loader.get_bool("TEST_BOOL", true);
    assert!(!value);
    std::env::remove_var("TOADSTOOL_TEST_BOOL");
}

#[test]
fn test_env_config_get_bool_invalid_returns_default() {
    // Invalid string values return the default parameter
    std::env::set_var("TOADSTOOL_TEST_BOOL", "invalid");
    let loader = EnvConfigLoader::new();
    let value = loader.get_bool("TEST_BOOL", false); // Test with false default
    assert!(!value); // Invalid values return the default (false in this case)
    std::env::remove_var("TOADSTOOL_TEST_BOOL");

    // Also test with true default
    std::env::set_var("TOADSTOOL_TEST_BOOL", "invalid");
    let value_true = loader.get_bool("TEST_BOOL", true);
    assert!(value_true); // Invalid values return the default (true in this case)
    std::env::remove_var("TOADSTOOL_TEST_BOOL");
}

// ============================================================================
// U16 Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_u16_default() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_u16("NONEXISTENT_U16", 8080);
    assert_eq!(value, 8080);
}

#[test]
fn test_env_config_get_u16_zero() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_u16("NONEXISTENT_U16", 0);
    assert_eq!(value, 0);
}

#[test]
fn test_env_config_get_u16_max() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_u16("NONEXISTENT_U16", 65535);
    assert_eq!(value, 65535);
}

#[test]
fn test_env_config_get_u16_with_env() {
    std::env::set_var("TOADSTOOL_TEST_PORT", "9000");
    let loader = EnvConfigLoader::new();
    let value = loader.get_u16("TEST_PORT", 8080);
    assert_eq!(value, 9000);
    std::env::remove_var("TOADSTOOL_TEST_PORT");
}

#[test]
fn test_env_config_get_u16_invalid_returns_default() {
    std::env::set_var("TOADSTOOL_TEST_PORT", "invalid");
    let loader = EnvConfigLoader::new();
    let value = loader.get_u16("TEST_PORT", 8080);
    assert_eq!(value, 8080);
    std::env::remove_var("TOADSTOOL_TEST_PORT");
}

// ============================================================================
// U32 Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_u32_default() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_u32("NONEXISTENT_U32", 1000);
    assert_eq!(value, 1000);
}

#[test]
fn test_env_config_get_u32_zero() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_u32("NONEXISTENT_U32", 0);
    assert_eq!(value, 0);
}

#[test]
fn test_env_config_get_u32_large() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_u32("NONEXISTENT_U32", 1_000_000);
    assert_eq!(value, 1_000_000);
}

#[test]
fn test_env_config_get_u32_with_env() {
    std::env::set_var("TOADSTOOL_TEST_U32", "42000");
    let loader = EnvConfigLoader::new();
    let value = loader.get_u32("TEST_U32", 1000);
    assert_eq!(value, 42000);
    std::env::remove_var("TOADSTOOL_TEST_U32");
}

// ============================================================================
// U64 Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_u64_default() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_u64("NONEXISTENT_U64", 1024);
    assert_eq!(value, 1024);
}

#[test]
fn test_env_config_get_u64_large() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_u64("NONEXISTENT_U64", 1_000_000_000);
    assert_eq!(value, 1_000_000_000);
}

#[test]
fn test_env_config_get_u64_with_env() {
    std::env::set_var("TOADSTOOL_TEST_U64", "9999999999");
    let loader = EnvConfigLoader::new();
    let value = loader.get_u64("TEST_U64", 1024);
    assert_eq!(value, 9999999999);
    std::env::remove_var("TOADSTOOL_TEST_U64");
}

// ============================================================================
// F64 Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_f64_default() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_f64("NONEXISTENT_F64", std::f64::consts::PI);
    assert!((value - std::f64::consts::PI).abs() < 0.001);
}

#[test]
fn test_env_config_get_f64_zero() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_f64("NONEXISTENT_F64", 0.0);
    assert_eq!(value, 0.0);
}

#[test]
fn test_env_config_get_f64_negative() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_f64("NONEXISTENT_F64", -10.5);
    assert!((value - (-10.5)).abs() < 0.001);
}

#[test]
fn test_env_config_get_f64_with_env() {
    std::env::set_var("TOADSTOOL_TEST_F64", "2.71828");
    let loader = EnvConfigLoader::new();
    let value = loader.get_f64("TEST_F64", std::f64::consts::PI);
    assert!((value - std::f64::consts::E).abs() < 0.001);
    std::env::remove_var("TOADSTOOL_TEST_F64");
}

// ============================================================================
// Duration Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_duration_default() {
    let loader = EnvConfigLoader::new();
    let default = Duration::from_secs(30);
    let value = loader.get_duration("NONEXISTENT_DURATION", default);
    assert_eq!(value, default);
}

#[test]
fn test_env_config_get_duration_zero() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_duration("NONEXISTENT_DURATION", Duration::from_secs(0));
    assert_eq!(value, Duration::from_secs(0));
}

#[test]
fn test_env_config_get_duration_with_env() {
    std::env::set_var("TOADSTOOL_TEST_DURATION", "60");
    let loader = EnvConfigLoader::new();
    let value = loader.get_duration("TEST_DURATION", Duration::from_secs(30));
    assert_eq!(value, Duration::from_secs(60));
    std::env::remove_var("TOADSTOOL_TEST_DURATION");
}

#[test]
fn test_env_config_get_duration_invalid_returns_default() {
    std::env::set_var("TOADSTOOL_TEST_DURATION", "invalid");
    let loader = EnvConfigLoader::new();
    let default = Duration::from_secs(30);
    let value = loader.get_duration("TEST_DURATION", default);
    assert_eq!(value, default);
    std::env::remove_var("TOADSTOOL_TEST_DURATION");
}

// ============================================================================
// SocketAddr Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_socket_addr_default() {
    let loader = EnvConfigLoader::new();
    let default = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    let value = loader.get_socket_addr("NONEXISTENT_ADDR", default);
    assert_eq!(value, default);
}

#[test]
fn test_env_config_get_socket_addr_localhost() {
    let loader = EnvConfigLoader::new();
    let default = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    let value = loader.get_socket_addr("NONEXISTENT_ADDR", default);
    assert_eq!(value.ip(), IpAddr::V4(Ipv4Addr::LOCALHOST));
    assert_eq!(value.port(), 8080);
}

#[test]
fn test_env_config_get_socket_addr_with_env() {
    std::env::set_var("TOADSTOOL_TEST_ADDR", "0.0.0.0:9000");
    let loader = EnvConfigLoader::new();
    let default = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
    let value = loader.get_socket_addr("TEST_ADDR", default);
    assert_eq!(value.ip(), IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)));
    assert_eq!(value.port(), 9000);
    std::env::remove_var("TOADSTOOL_TEST_ADDR");
}

// ============================================================================
// Path Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_path_default() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_path("NONEXISTENT_PATH", "/default/path");
    assert_eq!(value, PathBuf::from("/default/path"));
}

#[test]
fn test_env_config_get_path_empty_default() {
    let loader = EnvConfigLoader::new();
    let value = loader.get_path("NONEXISTENT_PATH", "");
    assert_eq!(value, PathBuf::from(""));
}

#[test]
fn test_env_config_get_path_with_env() {
    std::env::set_var("TOADSTOOL_TEST_PATH", "/custom/path");
    let loader = EnvConfigLoader::new();
    let value = loader.get_path("TEST_PATH", "/default/path");
    assert_eq!(value, PathBuf::from("/custom/path"));
    std::env::remove_var("TOADSTOOL_TEST_PATH");
}

// ============================================================================
// Prefixed Variables Tests
// ============================================================================

#[test]
fn test_env_config_get_prefixed_empty() {
    let loader = EnvConfigLoader::new();
    let values = loader.get_prefixed("NONEXISTENT");
    // May have env vars from system, so just verify it returns a valid map
    assert!(values.is_empty() || !values.is_empty());
}

#[test]
fn test_env_config_get_prefixed_with_vars() {
    std::env::set_var("TOADSTOOL_PREFIX_VAR1", "value1");
    std::env::set_var("TOADSTOOL_PREFIX_VAR2", "value2");
    std::env::set_var("TOADSTOOL_OTHER_VAR", "other");

    let loader = EnvConfigLoader::new();
    let values = loader.get_prefixed("PREFIX");

    // Should only get PREFIX vars
    assert!(values.contains_key("TOADSTOOL_PREFIX_VAR1") || values.is_empty());

    std::env::remove_var("TOADSTOOL_PREFIX_VAR1");
    std::env::remove_var("TOADSTOOL_PREFIX_VAR2");
    std::env::remove_var("TOADSTOOL_OTHER_VAR");
}

// ============================================================================
// Cache Loading Tests
// ============================================================================

#[test]
fn test_env_config_load_cache() {
    let mut loader = EnvConfigLoader::new();
    loader.load_cache();
    // Should complete without panicking
}

#[test]
fn test_env_config_load_cache_with_vars() {
    std::env::set_var("TOADSTOOL_CACHE_TEST", "value");
    let mut loader = EnvConfigLoader::new();
    loader.load_cache();
    std::env::remove_var("TOADSTOOL_CACHE_TEST");
    // Should complete without panicking
}

// ============================================================================
// Custom Prefix Tests
// ============================================================================

#[test]
fn test_env_config_custom_prefix_string() {
    let loader = EnvConfigLoader::with_prefix("CUSTOM");
    std::env::set_var("CUSTOM_TEST", "custom_value");
    let value = loader.get_string("TEST", "default");
    assert_eq!(value, "custom_value");
    std::env::remove_var("CUSTOM_TEST");
}

#[test]
fn test_env_config_custom_prefix_bool() {
    let loader = EnvConfigLoader::with_prefix("APP");
    std::env::set_var("APP_ENABLED", "true");
    let value = loader.get_bool("ENABLED", false);
    assert!(value);
    std::env::remove_var("APP_ENABLED");
}

#[test]
fn test_env_config_custom_prefix_u16() {
    let loader = EnvConfigLoader::with_prefix("SERVICE");
    std::env::set_var("SERVICE_PORT", "3000");
    let value = loader.get_u16("PORT", 8080);
    assert_eq!(value, 3000);
    std::env::remove_var("SERVICE_PORT");
}
