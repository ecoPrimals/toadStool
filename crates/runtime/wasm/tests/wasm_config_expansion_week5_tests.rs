//! WASM Runtime Configuration Tests - Week 5
//! Comprehensive tests for WebAssembly runtime configuration

use toadstool::RuntimeEngine;
use toadstool_runtime_wasm::{CacheMetrics, WasmRuntimeConfig, WasmRuntimeEngine};

// ============================================================================
// WasmRuntimeConfig Tests
// ============================================================================

#[test]
fn test_wasm_config_default() {
    let config = WasmRuntimeConfig::default();

    assert_eq!(config.max_memory_mb, 128);
    assert_eq!(config.execution_timeout_ms, 30_000);
    assert!(config.cache.enabled);
    assert_eq!(config.max_pages, 2048);
}

#[test]
fn test_wasm_config_clone() {
    let config = WasmRuntimeConfig::default();
    let cloned = config.clone();

    assert_eq!(cloned.max_memory_mb, config.max_memory_mb);
    assert_eq!(cloned.execution_timeout_ms, config.execution_timeout_ms);
    assert_eq!(cloned.cache.enabled, config.cache.enabled);
}

#[test]
fn test_wasm_config_debug() {
    let config = WasmRuntimeConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("WasmRuntimeConfig"));
    assert!(debug_str.contains("max_memory_mb"));
}

#[test]
fn test_wasm_config_custom_memory_limit() {
    let config = WasmRuntimeConfig {
        max_memory_mb: 256,
        ..Default::default()
    };

    assert_eq!(config.max_memory_mb, 256);
}

#[test]
fn test_wasm_config_custom_timeout() {
    let config = WasmRuntimeConfig {
        execution_timeout_ms: 60_000,
        ..Default::default()
    };

    assert_eq!(config.execution_timeout_ms, 60_000);
}

#[test]
fn test_wasm_config_disable_caching() {
    let mut config = WasmRuntimeConfig::default();
    config.cache.enabled = false;

    assert!(!config.cache.enabled);
}

#[test]
fn test_wasm_config_max_pages() {
    let config = WasmRuntimeConfig {
        max_pages: 4096,
        ..Default::default()
    };

    assert_eq!(config.max_pages, 4096);
}

#[test]
fn test_wasm_config_zero_memory_limit() {
    let config = WasmRuntimeConfig {
        max_memory_mb: 0,
        ..Default::default()
    };

    // Zero memory should be caught during initialization
    assert_eq!(config.max_memory_mb, 0);
}

#[test]
fn test_wasm_config_zero_timeout() {
    let config = WasmRuntimeConfig {
        execution_timeout_ms: 0,
        ..Default::default()
    };

    // Zero timeout should be caught during initialization
    assert_eq!(config.execution_timeout_ms, 0);
}

#[test]
fn test_wasm_config_large_memory_limit() {
    let config = WasmRuntimeConfig {
        max_memory_mb: 2048,
        ..Default::default()
    };

    assert_eq!(config.max_memory_mb, 2048);
}

// ============================================================================
// CacheMetrics Tests
// ============================================================================

#[test]
fn test_cache_metrics_default() {
    let metrics = CacheMetrics::default();

    assert_eq!(metrics.total_modules, 0);
    assert_eq!(metrics.total_size_bytes, 0);
    assert_eq!(metrics.average_module_size, 0);
    assert_eq!(metrics.cache_hit_rate, 0.0);
    assert_eq!(metrics.memory_usage_bytes, 0);
}

#[test]
fn test_cache_metrics_clone() {
    let metrics = CacheMetrics::default();
    let cloned = metrics.clone();

    assert_eq!(cloned.total_modules, metrics.total_modules);
    assert_eq!(cloned.cache_hit_rate, metrics.cache_hit_rate);
}

#[test]
fn test_cache_metrics_debug() {
    let metrics = CacheMetrics::default();
    let debug_str = format!("{:?}", metrics);

    assert!(debug_str.contains("CacheMetrics"));
}

#[test]
fn test_cache_metrics_display() {
    let metrics = CacheMetrics {
        total_modules: 10,
        total_size_bytes: 1024,
        average_module_size: 102,
        cache_hit_rate: 0.85,
        memory_usage_bytes: 2048,
    };

    let display_str = format!("{}", metrics);
    assert!(display_str.contains("10"));
    assert!(display_str.contains("1024"));
    assert!(display_str.contains("85"));
}

#[test]
fn test_cache_metrics_with_data() {
    let metrics = CacheMetrics {
        total_modules: 5,
        total_size_bytes: 5000,
        average_module_size: 1000,
        ..Default::default()
    };

    assert_eq!(metrics.total_modules, 5);
    assert_eq!(metrics.average_module_size, 1000);
}

#[test]
fn test_cache_metrics_hit_rate_calculation() {
    let metrics = CacheMetrics {
        cache_hit_rate: 0.75,
        ..Default::default()
    };

    assert!((metrics.cache_hit_rate - 0.75).abs() < 0.001);
}

#[test]
fn test_cache_metrics_zero_hit_rate() {
    let metrics = CacheMetrics::default();

    assert_eq!(metrics.cache_hit_rate, 0.0);
}

#[test]
fn test_cache_metrics_perfect_hit_rate() {
    let metrics = CacheMetrics {
        cache_hit_rate: 1.0,
        ..Default::default()
    };

    assert_eq!(metrics.cache_hit_rate, 1.0);
}

// ============================================================================
// WasmRuntimeEngine Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_creation_default() {
    let config = WasmRuntimeConfig::default();
    let result = WasmRuntimeEngine::new(config);

    assert!(
        result.is_ok(),
        "Failed to create WASM engine: {:?}",
        result.err()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_creation_custom_config() {
    let config = WasmRuntimeConfig {
        max_memory_mb: 256,
        execution_timeout_ms: 60_000,
        ..Default::default()
    };

    let result = WasmRuntimeEngine::new(config);
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_get_capabilities() {
    let config = WasmRuntimeConfig::default();
    let engine = WasmRuntimeEngine::new(config).unwrap();

    let capabilities = engine.get_capabilities();
    assert!(!capabilities.supported_workloads.is_empty());
    assert!(!capabilities.supported_architectures.is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_supports_wasm_workload() {
    let config = WasmRuntimeConfig::default();
    let engine = WasmRuntimeEngine::new(config).unwrap();

    use toadstool::workload::WorkloadType;
    assert!(engine.supports_workload(&WorkloadType::Wasm));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_rejects_non_wasm_workload() {
    let config = WasmRuntimeConfig::default();
    let engine = WasmRuntimeEngine::new(config).unwrap();

    use toadstool::workload::WorkloadType;
    assert!(!engine.supports_workload(&WorkloadType::Container));
    assert!(!engine.supports_workload(&WorkloadType::Native));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_architectures() {
    let config = WasmRuntimeConfig::default();
    let engine = WasmRuntimeEngine::new(config).unwrap();

    let capabilities = engine.get_capabilities();
    assert!(capabilities
        .supported_architectures
        .contains(&"wasm32".to_string()));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_platform_features() {
    let config = WasmRuntimeConfig::default();
    let engine = WasmRuntimeEngine::new(config).unwrap();

    let capabilities = engine.get_capabilities();
    let has_wasi = capabilities.platform_features.get("wasi_support");
    assert!(has_wasi.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_with_disabled_caching() {
    let mut config = WasmRuntimeConfig::default();
    config.cache.enabled = false;

    let result = WasmRuntimeEngine::new(config);
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_with_fuel_limit() {
    let config = WasmRuntimeConfig {
        fuel_limit: Some(500_000),
        ..Default::default()
    };

    let result = WasmRuntimeEngine::new(config);
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_wasm_engine_version() {
    let config = WasmRuntimeConfig::default();
    let engine = WasmRuntimeEngine::new(config).unwrap();

    let capabilities = engine.get_capabilities();
    assert!(!capabilities.version.is_empty());
}

// ============================================================================
// Configuration Validation Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_config_validation_reasonable_limits() {
    let configs = vec![
        WasmRuntimeConfig {
            max_memory_mb: 64,
            execution_timeout_ms: 5_000,
            ..Default::default()
        },
        WasmRuntimeConfig {
            max_memory_mb: 512,
            execution_timeout_ms: 120_000,
            ..Default::default()
        },
    ];

    for config in configs {
        let result = WasmRuntimeEngine::new(config);
        assert!(result.is_ok(), "Valid config should be accepted");
    }
}

#[test]
fn test_config_boundary_values() {
    let configs = vec![
        (1, 1),          // Minimum
        (128, 30_000),   // Default
        (2048, 300_000), // Large
    ];

    for (mem, timeout) in configs {
        let config = WasmRuntimeConfig {
            max_memory_mb: mem,
            execution_timeout_ms: timeout,
            ..Default::default()
        };

        assert_eq!(config.max_memory_mb, mem);
        assert_eq!(config.execution_timeout_ms, timeout);
    }
}

#[test]
fn test_config_cache_settings() {
    let cache_sizes = [128, 256, 512, 1024];

    for size in cache_sizes {
        let mut config = WasmRuntimeConfig::default();
        config.cache.max_entries = size;

        assert_eq!(config.cache.max_entries, size);
    }
}
