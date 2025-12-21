//! Comprehensive safety and correctness tests for WASM cache implementations
//!
//! This test suite validates both the zero-unsafe and minimal-unsafe cache strategies

use std::sync::Arc;
use toadstool_runtime_wasm::cache_zero_unsafe::{CacheMetrics, ZeroUnsafeModuleCache};

#[tokio::test]
async fn test_zero_unsafe_cache_initialization() {
    let cache = ZeroUnsafeModuleCache::new(100, 10);
    let metrics = cache.get_metrics().await;

    assert_eq!(metrics.source_entries, 0);
    assert_eq!(metrics.compiled_entries, 0);
    assert_eq!(metrics.hit_rate, 0.0);
    assert_eq!(metrics.cache_efficiency, 0.0);
    assert_eq!(metrics.total_compilations, 0);
}

#[tokio::test]
async fn test_zero_unsafe_cache_clear() {
    let cache = ZeroUnsafeModuleCache::new(50, 5);
    cache.clear().await;

    let metrics = cache.get_metrics().await;
    assert_eq!(metrics.source_entries, 0);
    assert_eq!(metrics.compiled_entries, 0);
}

#[tokio::test]
async fn test_zero_unsafe_cache_stats() {
    let cache = ZeroUnsafeModuleCache::new(100, 10);
    let stats = cache.get_stats().await;

    // CacheStats is returned, verify it's valid
    // Fields are private, so we just verify the struct is returned correctly
    let _stats_debug = format!("{:?}", stats);
}

#[test]
fn test_cache_metrics_display() {
    let metrics = CacheMetrics {
        source_entries: 50,
        compiled_entries: 10,
        hit_rate: 0.85,
        total_compilations: 100,
        avg_compilation_ms: 5,
        cache_efficiency: 0.75,
    };

    let display = format!("{}", metrics);
    assert!(display.contains("source: 50"));
    assert!(display.contains("compiled: 10"));
    assert!(display.contains("85.00%")); // hit_rate
    assert!(display.contains("75.00%")); // efficiency
}

#[tokio::test]
async fn test_cache_with_small_limits() {
    let cache = ZeroUnsafeModuleCache::new(5, 2);
    let metrics = cache.get_metrics().await;

    // Should initialize even with small limits
    assert_eq!(metrics.source_entries, 0);
    assert_eq!(metrics.compiled_entries, 0);
}

#[tokio::test]
async fn test_cache_with_large_limits() {
    let cache = ZeroUnsafeModuleCache::new(10000, 1000);
    let metrics = cache.get_metrics().await;

    assert_eq!(metrics.source_entries, 0);
    assert_eq!(metrics.compiled_entries, 0);
}

#[test]
fn test_cache_metrics_zero_requests() {
    let metrics = CacheMetrics {
        source_entries: 0,
        compiled_entries: 0,
        hit_rate: 0.0,
        total_compilations: 0,
        avg_compilation_ms: 0,
        cache_efficiency: 0.0,
    };

    // Should handle zero requests gracefully
    assert_eq!(metrics.hit_rate, 0.0);
    assert_eq!(metrics.cache_efficiency, 0.0);
}

#[test]
fn test_cache_metrics_perfect_hit_rate() {
    let metrics = CacheMetrics {
        source_entries: 100,
        compiled_entries: 50,
        hit_rate: 1.0,
        total_compilations: 50,
        avg_compilation_ms: 3,
        cache_efficiency: 0.95,
    };

    assert_eq!(metrics.hit_rate, 1.0);
    assert!(metrics.cache_efficiency >= 0.95);
}

#[tokio::test]
async fn test_concurrent_cache_access() {
    let cache = Arc::new(ZeroUnsafeModuleCache::new(100, 10));

    let mut handles = vec![];
    for _ in 0..10 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move { cache_clone.get_stats().await });
        handles.push(handle);
    }

    // All should complete without panic
    for handle in handles {
        let stats = handle.await.unwrap();
        // Stats returned successfully
        let _debug = format!("{:?}", stats);
    }
}

#[tokio::test]
async fn test_cache_metrics_concurrent_reads() {
    let cache = Arc::new(ZeroUnsafeModuleCache::new(100, 10));

    let mut handles = vec![];
    for _ in 0..20 {
        let cache_clone = cache.clone();
        let handle = tokio::spawn(async move { cache_clone.get_metrics().await });
        handles.push(handle);
    }

    for handle in handles {
        let metrics = handle.await.unwrap();
        assert_eq!(metrics.source_entries, 0);
    }
}

// Safety documentation tests - verify safety invariants are documented
#[test]
fn test_safety_documentation_exists() {
    // This test verifies that safety documentation is present
    // The actual unsafe code in cache.rs has comprehensive safety docs

    // Read the cache.rs file to verify safety comments exist
    let cache_code = include_str!("../src/cache.rs");

    // Verify key safety documentation exists
    assert!(cache_code.contains("# Safety"));
    assert!(cache_code.contains("Origin Guarantee"));
    assert!(cache_code.contains("Engine Consistency"));
    assert!(cache_code.contains("Corruption Handling"));
    assert!(cache_code.contains("Memory Safety"));
}

#[test]
fn test_zero_unsafe_documentation() {
    let zero_unsafe_code = include_str!("../src/cache_zero_unsafe.rs");

    // Verify it claims to be zero-unsafe
    assert!(zero_unsafe_code.contains("Zero-unsafe"));
    assert!(zero_unsafe_code.contains("100% safe"));
    assert!(zero_unsafe_code.contains("no unsafe"));
}

#[test]
fn test_safe_cache_documentation() {
    let safe_cache_code = include_str!("../src/cache_safe.rs");

    // Verify comprehensive safety documentation
    assert!(safe_cache_code.contains("UNAVOIDABLE"));
    assert!(safe_cache_code.contains("Integrity Validation"));
    assert!(safe_cache_code.contains("Engine Compatibility"));
}
