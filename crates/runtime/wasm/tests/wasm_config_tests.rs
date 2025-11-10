//! Comprehensive tests for WASM runtime configuration

use toadstool_runtime_wasm::*;

// ============================================================================
// WasmRuntimeConfig Tests
// ============================================================================

#[test]
fn test_wasm_runtime_config_default() {
    let config = WasmRuntimeConfig::default();

    assert!(config.cache_enabled);
    assert_eq!(config.max_cache_size_mb, 512);
    assert_eq!(config.cache_ttl_hours, 24);
    assert_eq!(config.max_memory_mb, 128);
    assert_eq!(config.max_pages, 2048);
    assert_eq!(config.execution_timeout_ms, 30000);
    assert_eq!(config.module_load_timeout_ms, 10000);
    assert_eq!(config.fuel_limit, Some(1_000_000));
}

#[test]
fn test_wasm_runtime_config_cache_settings() {
    let config = WasmRuntimeConfig::default();

    assert!(config.cache_enabled);
    assert_eq!(config.max_cache_size_mb, 512);
    assert_eq!(config.cache_ttl_hours, 24);
}

#[test]
fn test_wasm_runtime_config_memory_limits() {
    let config = WasmRuntimeConfig::default();

    assert_eq!(config.max_memory_mb, 128);
    assert_eq!(config.max_pages, 2048);
}

#[test]
fn test_wasm_runtime_config_timeout_settings() {
    let config = WasmRuntimeConfig::default();

    assert_eq!(config.execution_timeout_ms, 30000); // 30 seconds
    assert_eq!(config.module_load_timeout_ms, 10000); // 10 seconds
}

#[test]
fn test_wasm_runtime_config_fuel_limit() {
    let config = WasmRuntimeConfig::default();

    assert_eq!(config.fuel_limit, Some(1_000_000));
}

#[test]
fn test_wasm_runtime_config_security_level() {
    let config = WasmRuntimeConfig::default();

    assert!(matches!(config.security_level, SecurityLevel::Strict));
}

#[test]
fn test_wasm_runtime_config_clone() {
    let config1 = WasmRuntimeConfig::default();
    let config2 = config1.clone();

    assert_eq!(config1.cache_enabled, config2.cache_enabled);
    assert_eq!(config1.max_cache_size_mb, config2.max_cache_size_mb);
    assert_eq!(config1.max_memory_mb, config2.max_memory_mb);
}

// ============================================================================
// SecurityLevel Tests
// ============================================================================

#[test]
fn test_security_level_none() {
    let level = SecurityLevel::None;
    assert!(matches!(level, SecurityLevel::None));
}

#[test]
fn test_security_level_basic() {
    let level = SecurityLevel::Basic;
    assert!(matches!(level, SecurityLevel::Basic));
}

#[test]
fn test_security_level_strict() {
    let level = SecurityLevel::Strict;
    assert!(matches!(level, SecurityLevel::Strict));
}

#[test]
fn test_security_level_maximum() {
    let level = SecurityLevel::Maximum;
    assert!(matches!(level, SecurityLevel::Maximum));
}

#[test]
fn test_security_level_clone() {
    let level1 = SecurityLevel::Strict;
    let level2 = level1.clone();

    match (level1, level2) {
        (SecurityLevel::Strict, SecurityLevel::Strict) => {
            // Clone successful
        }
        _ => panic!("Clone failed"),
    }
}

#[test]
fn test_all_security_levels() {
    let levels = [
        SecurityLevel::None,
        SecurityLevel::Basic,
        SecurityLevel::Strict,
        SecurityLevel::Maximum,
    ];

    assert_eq!(levels.len(), 4);
}
