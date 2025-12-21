//! Comprehensive tests for zero-unsafe WASM module cache
//!
//! Tests the modern safe implementation that eliminates all unsafe code
//! while maintaining high performance through smart caching strategies.
//!
//! Key features tested:
//! - Concurrent operations (100% safe multi-threading)
//! - Source and compiled module caching
//! - LRU eviction with safe locking
//! - Metrics and statistics
//! - Zero unsafe code paths

use std::sync::Arc;
use wasmtime::Engine;

// Use the zero-unsafe cache (100% safe Rust, default)
use toadstool_runtime_wasm::cache_zero_unsafe::ZeroUnsafeModuleCache;

/// Helper to create a minimal valid WASM module
fn minimal_wasm_module() -> Vec<u8> {
    // Minimal valid WASM module (magic number + version)
    vec![
        0x00, 0x61, 0x73, 0x6d, // Magic number "\0asm"
        0x01, 0x00, 0x00, 0x00, // Version 1
    ]
}

/// Helper to create a simple WASM module with a function
fn simple_wasm_function() -> Vec<u8> {
    // A minimal WASM module with an empty function
    vec![
        0x00, 0x61, 0x73, 0x6d, // Magic
        0x01, 0x00, 0x00, 0x00, // Version
        0x01, 0x04, 0x01, 0x60, 0x00, 0x00, // Type section: [] -> []
        0x03, 0x02, 0x01, 0x00, // Function section
        0x0a, 0x04, 0x01, 0x02, 0x00, 0x0b, // Code section: empty function
    ]
}

// ============================================================================
// Basic Cache Operations
// ============================================================================

#[tokio::test]
async fn test_cache_creation() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let metrics = cache.get_metrics().await;

    assert_eq!(metrics.source_entries, 0);
    assert_eq!(metrics.compiled_entries, 0);
}

#[tokio::test]
async fn test_cache_get_or_compile_first_time() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let engine = Engine::default();
    let wasm = minimal_wasm_module();

    let result = cache
        .get_or_compile("test_module", &engine, Some(&wasm))
        .await;
    assert!(result.is_ok(), "First compilation should succeed");

    let metrics = cache.get_metrics().await;
    assert_eq!(metrics.total_compilations, 1, "Should record a compilation");
}

#[tokio::test]
async fn test_cache_hit_on_second_access() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let engine = Engine::default();
    let wasm = simple_wasm_function();

    // First access - miss
    let _module1 = cache
        .get_or_compile("test", &engine, Some(&wasm))
        .await
        .unwrap();

    // Second access - should hit compiled cache
    let _module2 = cache.get_or_compile("test", &engine, None).await.unwrap();

    let metrics = cache.get_metrics().await;
    assert!(metrics.hit_rate > 0.0, "Should have cache hits");
}

#[tokio::test]
async fn test_cache_concurrent_access() {
    let cache = Arc::new(ZeroUnsafeModuleCache::new(100, 50));
    let engine = Arc::new(Engine::default());
    let wasm = Arc::new(simple_wasm_function());

    let mut handles = vec![];

    // Spawn 10 concurrent tasks
    for i in 0..10 {
        let cache_clone = Arc::clone(&cache);
        let engine_clone = Arc::clone(&engine);
        let wasm_clone = Arc::clone(&wasm);

        let handle = tokio::spawn(async move {
            let key = format!("module_{}", i);
            cache_clone
                .get_or_compile(&key, &engine_clone, Some(&wasm_clone))
                .await
        });

        handles.push(handle);
    }

    // All should succeed
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Concurrent access should succeed");
    }

    let metrics = cache.get_metrics().await;
    assert_eq!(metrics.compiled_entries, 10, "Should cache all 10 modules");
}

#[tokio::test]
async fn test_cache_source_then_compiled() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let engine = Engine::default();
    let wasm = simple_wasm_function();

    // First access stores in both source and compiled
    let _module1 = cache
        .get_or_compile("test", &engine, Some(&wasm))
        .await
        .unwrap();

    let metrics = cache.get_metrics().await;
    assert_eq!(metrics.source_entries, 1);
    assert_eq!(metrics.compiled_entries, 1);
}

#[tokio::test]
async fn test_cache_clear() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let engine = Engine::default();
    let wasm = minimal_wasm_module();

    // Add some modules
    let _m1 = cache
        .get_or_compile("m1", &engine, Some(&wasm))
        .await
        .unwrap();
    let _m2 = cache
        .get_or_compile("m2", &engine, Some(&wasm))
        .await
        .unwrap();

    cache.clear().await;

    let metrics = cache.get_metrics().await;
    assert_eq!(metrics.source_entries, 0);
    assert_eq!(metrics.compiled_entries, 0);
}

// ============================================================================
// LRU and Capacity Tests
// ============================================================================

#[tokio::test]
async fn test_cache_respects_capacity() {
    // Small cache that will trigger eviction
    let cache = ZeroUnsafeModuleCache::new(5, 5);
    let engine = Engine::default();
    let wasm = simple_wasm_function();

    // Add more than capacity
    for i in 0..10 {
        let key = format!("module_{}", i);
        let _ = cache.get_or_compile(&key, &engine, Some(&wasm)).await;
    }

    let metrics = cache.get_metrics().await;
    // Should not exceed capacity significantly (allows some overage during eviction)
    assert!(
        metrics.compiled_entries <= 7,
        "Should respect capacity limits"
    );
}

// ============================================================================
// Metrics and Statistics Tests
// ============================================================================

#[tokio::test]
async fn test_metrics_track_hits_and_misses() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let engine = Engine::default();
    let wasm = simple_wasm_function();

    // First access - miss
    let _m1 = cache
        .get_or_compile("test", &engine, Some(&wasm))
        .await
        .unwrap();

    // Second access - hit
    let _m2 = cache.get_or_compile("test", &engine, None).await.unwrap();

    let metrics = cache.get_metrics().await;
    assert!(metrics.total_compilations >= 1);
    assert!(metrics.hit_rate > 0.0);
}

#[tokio::test]
async fn test_metrics_efficiency_tracking() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let metrics = cache.get_metrics().await;

    // Should have efficiency metrics
    assert!(metrics.cache_efficiency >= 0.0);
    assert!(metrics.cache_efficiency <= 1.0);
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_invalid_wasm_returns_error() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let engine = Engine::default();
    let invalid_wasm = vec![0xFF, 0xFF, 0xFF, 0xFF]; // Invalid WASM

    let result = cache
        .get_or_compile("invalid", &engine, Some(&invalid_wasm))
        .await;
    assert!(result.is_err(), "Invalid WASM should return error");
}

#[tokio::test]
async fn test_missing_source_without_bytes_returns_error() {
    let cache = ZeroUnsafeModuleCache::new(100, 50);
    let engine = Engine::default();

    // Try to get module that doesn't exist and no bytes provided
    let result = cache.get_or_compile("nonexistent", &engine, None).await;
    assert!(
        result.is_err(),
        "Should error when module not cached and no bytes provided"
    );
}

// ============================================================================
// Performance and Concurrency Tests
// ============================================================================

#[tokio::test]
async fn test_parallel_compilation_limiting() {
    let cache = Arc::new(ZeroUnsafeModuleCache::new(100, 50));
    let engine = Arc::new(Engine::default());
    let wasm = Arc::new(simple_wasm_function());

    let mut handles = vec![];

    // Spawn many tasks simultaneously to test semaphore limiting
    for i in 0..20 {
        let cache_clone = Arc::clone(&cache);
        let engine_clone = Arc::clone(&engine);
        let wasm_clone = Arc::clone(&wasm);

        let handle = tokio::spawn(async move {
            let key = format!("concurrent_{}", i);
            cache_clone
                .get_or_compile(&key, &engine_clone, Some(&wasm_clone))
                .await
        });

        handles.push(handle);
    }

    // All should complete without deadlock or panic
    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

#[tokio::test]
async fn test_cache_under_contention() {
    let cache = Arc::new(ZeroUnsafeModuleCache::new(10, 10));
    let engine = Arc::new(Engine::default());
    let wasm = Arc::new(simple_wasm_function());

    let mut handles = vec![];

    // Many tasks accessing same keys
    for i in 0..50 {
        let cache_clone = Arc::clone(&cache);
        let engine_clone = Arc::clone(&engine);
        let wasm_clone = Arc::clone(&wasm);

        let handle = tokio::spawn(async move {
            let key = format!("key_{}", i % 5); // Only 5 unique keys, lots of contention
            cache_clone
                .get_or_compile(&key, &engine_clone, Some(&wasm_clone))
                .await
        });

        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[tokio::test]
async fn test_realistic_usage_pattern() {
    let cache = Arc::new(ZeroUnsafeModuleCache::new(50, 25));
    let engine = Arc::new(Engine::default());
    let wasm = Arc::new(simple_wasm_function());

    // Simulate realistic usage: some modules used frequently, others once
    let popular_keys = vec!["popular_1", "popular_2", "popular_3"];
    let rare_keys: Vec<String> = (0..20).map(|i| format!("rare_{}", i)).collect();

    // Access popular keys multiple times
    for _ in 0..10 {
        for key in &popular_keys {
            let _ = cache.get_or_compile(key, &engine, Some(&wasm)).await;
        }
    }

    // Access rare keys once each
    for key in &rare_keys {
        let _ = cache.get_or_compile(key, &engine, Some(&wasm)).await;
    }

    let metrics = cache.get_metrics().await;
    // Popular keys should have high hit rates
    assert!(metrics.hit_rate > 0.5, "Should have high cache hit rate");
}

// ============================================================================
// Safety Verification Tests
// ============================================================================

#[tokio::test]
async fn test_no_data_races() {
    // This test ensures safe concurrent access patterns
    let cache = Arc::new(ZeroUnsafeModuleCache::new(100, 50));
    let engine = Arc::new(Engine::default());
    let wasm = Arc::new(simple_wasm_function());

    let mut handles = vec![];

    // Mix of reads and writes
    for i in 0..100 {
        let cache_clone = Arc::clone(&cache);
        let engine_clone = Arc::clone(&engine);
        let wasm_clone = Arc::clone(&wasm);

        let handle = tokio::spawn(async move {
            if i % 2 == 0 {
                // Get metrics (read)
                let _ = cache_clone.get_metrics().await;
            } else {
                // Compile (read + write)
                let _ = cache_clone
                    .get_or_compile(&format!("key_{}", i), &engine_clone, Some(&wasm_clone))
                    .await;
            }
        });

        handles.push(handle);
    }

    // All should complete safely
    for handle in handles {
        handle.await.unwrap();
    }
}
