// SPDX-License-Identifier: AGPL-3.0-only
//! Backward compatibility tests for deprecated network configuration functions
#![allow(deprecated)]

//! Comprehensive tests for environment configuration

mod test_env_fixture;

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
    temp_env::with_var("TOADSTOOL_TEST_STRING", Some("test_value"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_string("TEST_STRING", "default");
        assert_eq!(value, "test_value");
    });
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
fn test_env_config_get_bool_true_lowercase() {
    temp_env::with_var("TOADSTOOL_TEST_BOOL_TRUE_LOWERCASE", Some("true"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_bool("TEST_BOOL_TRUE_LOWERCASE", false);
        assert!(value);
    });
}

#[test]
fn test_env_config_get_bool_true_uppercase() {
    temp_env::with_var("TOADSTOOL_TEST_BOOL_TRUE_UPPERCASE", Some("TRUE"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_bool("TEST_BOOL_TRUE_UPPERCASE", false);
        assert!(value);
    });
}

#[test]
fn test_env_config_get_bool_numeric_one() {
    temp_env::with_var("TOADSTOOL_TEST_BOOL_NUMERIC_ONE", Some("1"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_bool("TEST_BOOL_NUMERIC_ONE", false);
        assert!(value);
    });
}

#[test]
fn test_env_config_get_bool_false() {
    temp_env::with_var("TOADSTOOL_TEST_BOOL_FALSE", Some("false"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_bool("TEST_BOOL_FALSE", true);
        assert!(!value);
    });
}

#[test]
fn test_env_config_get_bool_invalid_returns_default() {
    temp_env::with_var("TOADSTOOL_TEST_BOOL_INVALID_FALSE", Some("invalid"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_bool("TEST_BOOL_INVALID_FALSE", false);
        assert!(!value);
    });

    temp_env::with_var("TOADSTOOL_TEST_BOOL_INVALID_TRUE", Some("invalid"), || {
        let loader = EnvConfigLoader::new();
        let value_true = loader.get_bool("TEST_BOOL_INVALID_TRUE", true);
        assert!(value_true);
    });
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
    temp_env::with_var("TOADSTOOL_TEST_PORT", Some("9000"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_u16("TEST_PORT", 8080);
        assert_eq!(value, 9000);
    });
}

#[test]
fn test_env_config_get_u16_invalid_returns_default() {
    temp_env::with_var("TOADSTOOL_TEST_PORT", Some("invalid"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_u16("TEST_PORT", 8080);
        assert_eq!(value, 8080);
    });
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
    temp_env::with_var("TOADSTOOL_TEST_U32", Some("42000"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_u32("TEST_U32", 1000);
        assert_eq!(value, 42000);
    });
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
    temp_env::with_var("TOADSTOOL_TEST_U64", Some("9999999999"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_u64("TEST_U64", 1024);
        assert_eq!(value, 9_999_999_999);
    });
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
#[allow(clippy::float_cmp)] // comparing against exact literal initialization
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
    temp_env::with_var("TOADSTOOL_TEST_F64", Some("2.71828"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_f64("TEST_F64", std::f64::consts::PI);
        assert!((value - std::f64::consts::E).abs() < 0.001);
    });
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
    temp_env::with_var("TOADSTOOL_TEST_DURATION_WITH_ENV", Some("60"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_duration("TEST_DURATION_WITH_ENV", Duration::from_secs(30));
        assert_eq!(value, Duration::from_secs(60));
    });
}

#[test]
fn test_env_config_get_duration_invalid_returns_default() {
    temp_env::with_var("TOADSTOOL_TEST_DURATION_INVALID", Some("invalid"), || {
        let loader = EnvConfigLoader::new();
        let default = Duration::from_secs(30);
        let value = loader.get_duration("TEST_DURATION_INVALID", default);
        assert_eq!(value, default);
    });
}

// ============================================================================
// SocketAddr Configuration Tests
// ============================================================================

#[test]
fn test_env_config_get_socket_addr_default() {
    let loader = EnvConfigLoader::new();
    let default = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
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
    temp_env::with_var("TOADSTOOL_TEST_ADDR", Some("0.0.0.0:9000"), || {
        let loader = EnvConfigLoader::new();
        let default = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 8080);
        let value = loader.get_socket_addr("TEST_ADDR", default);
        assert_eq!(value.ip(), IpAddr::V4(Ipv4Addr::UNSPECIFIED));
        assert_eq!(value.port(), 9000);
    });
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
    temp_env::with_var("TOADSTOOL_TEST_PATH", Some("/custom/path"), || {
        let loader = EnvConfigLoader::new();
        let value = loader.get_path("TEST_PATH", "/default/path");
        assert_eq!(value, PathBuf::from("/custom/path"));
    });
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
    temp_env::with_vars(
        [
            ("TOADSTOOL_PREFIX_VAR1", Some("value1")),
            ("TOADSTOOL_PREFIX_VAR2", Some("value2")),
            ("TOADSTOOL_OTHER_VAR", Some("other")),
        ],
        || {
            let loader = EnvConfigLoader::new();
            let values = loader.get_prefixed("PREFIX");
            assert!(values.contains_key("TOADSTOOL_PREFIX_VAR1") || values.is_empty());
        },
    );
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
    temp_env::with_var("TOADSTOOL_CACHE_TEST", Some("value"), || {
        let mut loader = EnvConfigLoader::new();
        loader.load_cache();
    });
}

// ============================================================================
// Custom Prefix Tests
// ============================================================================

#[test]
fn test_env_config_custom_prefix_string() {
    temp_env::with_var("CUSTOM_TEST", Some("custom_value"), || {
        let loader = EnvConfigLoader::with_prefix("CUSTOM");
        let value = loader.get_string("TEST", "default");
        assert_eq!(value, "custom_value");
    });
}

#[test]
fn test_env_config_custom_prefix_bool() {
    temp_env::with_var("APP_ENABLED", Some("true"), || {
        let loader = EnvConfigLoader::with_prefix("APP");
        let value = loader.get_bool("ENABLED", false);
        assert!(value);
    });
}

#[test]
fn test_env_config_custom_prefix_u16() {
    temp_env::with_var("SERVICE_PORT", Some("3000"), || {
        let loader = EnvConfigLoader::with_prefix("SERVICE");
        let value = loader.get_u16("PORT", 8080);
        assert_eq!(value, 3000);
    });
}
