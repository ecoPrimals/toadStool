// SPDX-License-Identifier: AGPL-3.0-or-later
//! `IntelligentCache` and the rest of `performance_hardening` are
//! re-exported only under the `hardening` feature. Without a matching
//! gate this file did not compile, so none of its tests ran.
#![cfg(feature = "hardening")]

#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Comprehensive tests for performance hardening module
//!
//! Tests for optimized resource monitoring, memory pools, intelligent caching,
//! async batching, and performance hardening manager.

use std::sync::Arc;
use std::time::Duration;
use toadstool::performance_hardening::*;
use toadstool_common::config_bases::ConnectionPoolConfig;
// Removed: use toadstool::resources::RuntimeMetrics;

// ============================================================================
// PerformanceHardeningConfig Tests
// ============================================================================

#[test]
fn test_performance_hardening_config_default() {
    let config = PerformanceHardeningConfig::default();
    assert!(config.enable_optimized_monitoring);
    assert!(config.enable_memory_pools);
    assert!(config.enable_caching);
    assert!(config.enable_async_optimization);
    assert!(config.enable_connection_pooling);
}

#[test]
fn test_performance_hardening_config_custom() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: false,
        enable_memory_pools: true,
        enable_caching: true,
        enable_async_optimization: false,
        enable_connection_pooling: true,
        monitoring_config: OptimizedMonitoringConfig::default(),
        memory_pool_config: MemoryPoolConfig::default(),
        caching_config: CachingConfig::default(),
        async_config: AsyncOptimizationConfig::default(),
        connection_pool_config: PerformanceConnectionPoolConfig::default(),
    };
    assert!(!config.enable_optimized_monitoring);
    assert!(config.enable_memory_pools);
}

#[test]
fn test_performance_hardening_config_all_disabled() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: false,
        enable_memory_pools: false,
        enable_caching: false,
        enable_async_optimization: false,
        enable_connection_pooling: false,
        monitoring_config: OptimizedMonitoringConfig::default(),
        memory_pool_config: MemoryPoolConfig::default(),
        caching_config: CachingConfig::default(),
        async_config: AsyncOptimizationConfig::default(),
        connection_pool_config: PerformanceConnectionPoolConfig::default(),
    };
    assert!(!config.enable_optimized_monitoring);
    assert!(!config.enable_memory_pools);
    assert!(!config.enable_caching);
}

#[test]
fn test_performance_hardening_config_clone() {
    let config = PerformanceHardeningConfig::default();
    let cloned = config.clone();
    assert_eq!(
        config.enable_optimized_monitoring,
        cloned.enable_optimized_monitoring
    );
}

#[test]
fn test_performance_hardening_config_serialization() {
    let config = PerformanceHardeningConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: PerformanceHardeningConfig =
        serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(
        config.enable_optimized_monitoring,
        deserialized.enable_optimized_monitoring
    );
}

// ============================================================================
// OptimizedMonitoringConfig Tests
// ============================================================================

#[test]
fn test_optimized_monitoring_config_default() {
    let config = OptimizedMonitoringConfig::default();
    assert_eq!(config.base_sampling_interval, Duration::from_millis(100));
    assert!(config.adaptive_sampling);
    assert_eq!(config.high_load_multiplier, 0.5);
    assert_eq!(config.low_load_multiplier, 2.0);
    assert_eq!(config.batch_size, 10);
}

#[test]
fn test_optimized_monitoring_config_custom() {
    let config = OptimizedMonitoringConfig {
        base_sampling_interval: Duration::from_millis(200),
        adaptive_sampling: false,
        high_load_multiplier: 0.25,
        low_load_multiplier: 3.0,
        batch_size: 20,
        aggregation_window: Duration::from_mins(2),
    };
    assert_eq!(config.base_sampling_interval, Duration::from_millis(200));
    assert!(!config.adaptive_sampling);
}

#[test]
fn test_optimized_monitoring_config_clone() {
    let config = OptimizedMonitoringConfig::default();
    let cloned = config.clone();
    assert_eq!(config.batch_size, cloned.batch_size);
}

#[test]
fn test_optimized_monitoring_config_serialization() {
    let config = OptimizedMonitoringConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: OptimizedMonitoringConfig =
        serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(config.batch_size, deserialized.batch_size);
}

// ============================================================================
// MemoryPoolConfig Tests
// ============================================================================

#[test]
fn test_memory_pool_config_default() {
    let config = MemoryPoolConfig::default();
    assert_eq!(config.initial_size, 100);
    assert_eq!(config.max_size, 1000);
    assert_eq!(config.growth_factor, 1.5);
    assert_eq!(config.shrink_threshold, 0.3);
}

#[test]
fn test_memory_pool_config_custom() {
    let config = MemoryPoolConfig {
        initial_size: 50,
        max_size: 500,
        growth_factor: 2.0,
        shrink_threshold: 0.2,
        cleanup_interval: Duration::from_secs(30),
    };
    assert_eq!(config.initial_size, 50);
    assert_eq!(config.max_size, 500);
}

#[test]
fn test_memory_pool_config_clone() {
    let config = MemoryPoolConfig::default();
    let cloned = config.clone();
    assert_eq!(config.initial_size, cloned.initial_size);
}

#[test]
fn test_memory_pool_config_serialization() {
    let config = MemoryPoolConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: MemoryPoolConfig = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(config.initial_size, deserialized.initial_size);
}

// ============================================================================
// CachingConfig Tests
// ============================================================================

#[test]
fn test_caching_config_default() {
    let config = CachingConfig::default();
    assert_eq!(config.max_size, 1000);
    assert_eq!(config.default_ttl, Duration::from_mins(5));
    assert_eq!(config.hit_rate_threshold, 0.8);
}

#[test]
fn test_caching_config_custom() {
    let config = CachingConfig {
        max_size: 2000,
        default_ttl: Duration::from_mins(10),
        cleanup_interval: Duration::from_mins(2),
        hit_rate_threshold: 0.9,
    };
    assert_eq!(config.max_size, 2000);
    assert_eq!(config.hit_rate_threshold, 0.9);
}

#[test]
fn test_caching_config_clone() {
    let config = CachingConfig::default();
    let cloned = config.clone();
    assert_eq!(config.max_size, cloned.max_size);
}

#[test]
fn test_caching_config_serialization() {
    let config = CachingConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: CachingConfig = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(config.max_size, deserialized.max_size);
}

// ============================================================================
// AsyncOptimizationConfig Tests
// ============================================================================

#[test]
fn test_async_optimization_config_default() {
    let config = AsyncOptimizationConfig::default();
    assert_eq!(config.batch_size, 50);
    assert_eq!(config.batch_timeout, Duration::from_millis(100));
    assert_eq!(config.concurrency_limit, 100);
    assert_eq!(config.queue_size_limit, 1000);
}

#[test]
fn test_async_optimization_config_custom() {
    let config = AsyncOptimizationConfig {
        batch_size: 100,
        batch_timeout: Duration::from_millis(200),
        concurrency_limit: 200,
        queue_size_limit: 2000,
    };
    assert_eq!(config.batch_size, 100);
    assert_eq!(config.concurrency_limit, 200);
}

#[test]
fn test_async_optimization_config_clone() {
    let config = AsyncOptimizationConfig::default();
    let cloned = config.clone();
    assert_eq!(config.batch_size, cloned.batch_size);
}

#[test]
fn test_async_optimization_config_serialization() {
    let config = AsyncOptimizationConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: AsyncOptimizationConfig =
        serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(config.batch_size, deserialized.batch_size);
}

// ============================================================================
// ConnectionPoolConfig Tests
// ============================================================================

#[test]
fn test_connection_pool_config_default() {
    let config = ConnectionPoolConfig::default();
    assert!(config.enabled);
    assert_eq!(config.max_connections_per_host, 100); // Default is 100
    assert_eq!(config.max_idle_connections, 10); // Default is 10
    assert!(config.idle_timeout > Duration::ZERO);
}

#[test]
fn test_connection_pool_config_custom() {
    let config = ConnectionPoolConfig {
        enabled: true,
        max_connections_per_host: 20,
        max_idle_connections: 100,
        idle_timeout: Duration::from_mins(10),
        connection_lifetime: Duration::from_hours(1),
    };
    assert_eq!(config.max_connections_per_host, 20);
    assert_eq!(config.max_idle_connections, 100);
}

#[test]
fn test_connection_pool_config_clone() {
    let config = ConnectionPoolConfig::default();
    let cloned = config.clone();
    assert_eq!(config.enabled, cloned.enabled);
    assert_eq!(
        config.max_connections_per_host,
        cloned.max_connections_per_host
    );
}

#[test]
fn test_connection_pool_config_serialization() {
    let config = ConnectionPoolConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: ConnectionPoolConfig =
        serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(config.enabled, deserialized.enabled);
    assert_eq!(
        config.max_connections_per_host,
        deserialized.max_connections_per_host
    );
}

// ============================================================================
// OptimizedResourceMonitor Tests
// ============================================================================

#[test]
fn test_optimized_resource_monitor_new() {
    let config = OptimizedMonitoringConfig::default();
    let _monitor = OptimizedResourceMonitor::new(config);
    // Should create without panicking
}

#[test]
fn test_optimized_resource_monitor_with_custom_config() {
    let config = OptimizedMonitoringConfig {
        base_sampling_interval: Duration::from_millis(50),
        adaptive_sampling: true,
        high_load_multiplier: 0.5,
        low_load_multiplier: 2.0,
        batch_size: 20,
        aggregation_window: Duration::from_secs(30),
    };
    let _monitor = OptimizedResourceMonitor::new(config);
    // Should create without panicking
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_optimized_resource_monitor_get_sampling_interval() {
    let config = OptimizedMonitoringConfig::default();
    let monitor = OptimizedResourceMonitor::new(config.clone());

    let interval = monitor.get_sampling_interval().await;
    assert_eq!(interval, config.base_sampling_interval);
}

// ============================================================================
// MemoryPool Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_new() {
    let config = MemoryPoolConfig::default();
    let pool: MemoryPool<Vec<u8>> = MemoryPool::new(config, || Vec::with_capacity(1024));

    let stats = pool.get_stats().await;
    assert_eq!(stats.current_size, 100); // initial_size
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_get_object() {
    let config = MemoryPoolConfig {
        initial_size: 5,
        max_size: 10,
        growth_factor: 1.5,
        shrink_threshold: 0.3,
        cleanup_interval: Duration::from_mins(1),
    };
    let pool: MemoryPool<Vec<u8>> = MemoryPool::new(config, || Vec::with_capacity(1024));

    let obj = pool.get().await;
    assert!(obj.get().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_get_multiple_objects() {
    let config = MemoryPoolConfig {
        initial_size: 3,
        max_size: 10,
        growth_factor: 1.5,
        shrink_threshold: 0.3,
        cleanup_interval: Duration::from_mins(1),
    };
    let pool: Arc<MemoryPool<Vec<u8>>> =
        Arc::new(MemoryPool::new(config, || Vec::with_capacity(1024)));

    let obj1 = pool.get().await;
    let obj2 = pool.get().await;
    let obj3 = pool.get().await;

    assert!(obj1.get().is_some());
    assert!(obj2.get().is_some());
    assert!(obj3.get().is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_stats() {
    let config = MemoryPoolConfig {
        initial_size: 10,
        max_size: 100,
        growth_factor: 1.5,
        shrink_threshold: 0.3,
        cleanup_interval: Duration::from_mins(1),
    };
    let pool: MemoryPool<Vec<u8>> = MemoryPool::new(config, || Vec::with_capacity(1024));

    let stats = pool.get_stats().await;
    assert_eq!(stats.current_size, 10);
    assert_eq!(stats.total_allocations, 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_pooled_object_get() {
    let config = MemoryPoolConfig::default();
    let pool: MemoryPool<Vec<u8>> = MemoryPool::new(config, || Vec::with_capacity(1024));

    let obj = pool.get().await;
    let vec_ref = obj.get();
    assert!(vec_ref.is_some());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_pooled_object_get_mut() {
    let config = MemoryPoolConfig::default();
    let pool: MemoryPool<Vec<u8>> = MemoryPool::new(config, || Vec::with_capacity(1024));

    let mut obj = pool.get().await;
    if let Some(vec) = obj.get_mut() {
        vec.push(42);
        assert_eq!(vec.len(), 1);
    }
}

// ============================================================================
// IntelligentCache Tests
// ============================================================================

#[test]
fn test_intelligent_cache_new() {
    let config = CachingConfig::default();
    let _cache: IntelligentCache<String, i32> = IntelligentCache::new(config);
    // Should create without panicking
}

#[test]
fn test_intelligent_cache_with_custom_config() {
    let config = CachingConfig {
        max_size: 500,
        default_ttl: Duration::from_mins(10),
        cleanup_interval: Duration::from_secs(30),
        hit_rate_threshold: 0.9,
    };
    let _cache: IntelligentCache<String, i32> = IntelligentCache::new(config);
    // Should create without panicking
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_cache_put_and_get() {
    let config = CachingConfig::default();
    let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

    let key = "test-key".to_string();
    let value = 42;

    cache.put(key.clone(), value).await.expect("Should insert");
    let retrieved = cache.get(&key).await;
    assert_eq!(retrieved, Some(value));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_cache_get_nonexistent() {
    let config = CachingConfig::default();
    let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

    let result = cache.get(&"nonexistent".to_string()).await;
    assert_eq!(result, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_cache_put_multiple() {
    let config = CachingConfig::default();
    let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

    for i in 0..10 {
        let key = format!("key-{i}");
        cache.put(key, i).await.expect("Should insert");
    }

    let value = cache.get(&"key-5".to_string()).await;
    assert_eq!(value, Some(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_cache_put_with_ttl() {
    let config = CachingConfig::default();
    let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

    let key = "ttl-key".to_string();
    let value = 100;

    cache
        .put_with_ttl(key.clone(), value, Duration::from_secs(1000))
        .await
        .expect("Should insert");

    let retrieved = cache.get(&key).await;
    assert_eq!(retrieved, Some(value));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_cache_get_stats() {
    let config = CachingConfig::default();
    let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

    // Initially empty
    let stats = cache.get_stats().await;
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);

    // Add some data
    cache
        .put("key1".to_string(), 1)
        .await
        .expect("Should insert");

    // Get existing key (hit)
    cache.get(&"key1".to_string()).await;
    let stats = cache.get_stats().await;
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 0);

    // Get nonexistent key (miss)
    cache.get(&"key2".to_string()).await;
    let stats = cache.get_stats().await;
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_intelligent_cache_lru_eviction() {
    let config = CachingConfig {
        max_size: 3,
        default_ttl: Duration::from_mins(5),
        cleanup_interval: Duration::from_mins(1),
        hit_rate_threshold: 0.8,
    };
    let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

    // Fill cache to capacity
    cache
        .put("key1".to_string(), 1)
        .await
        .expect("Should insert");
    cache
        .put("key2".to_string(), 2)
        .await
        .expect("Should insert");
    cache
        .put("key3".to_string(), 3)
        .await
        .expect("Should insert");

    // Add one more, should evict least recently used
    cache
        .put("key4".to_string(), 4)
        .await
        .expect("Should insert");

    // key1 should be evicted
    let result = cache.get(&"key1".to_string()).await;
    assert_eq!(result, None);
}

// ============================================================================
// PerformanceHardeningManager Tests
// ============================================================================

#[test]
fn test_performance_hardening_manager_new() {
    let config = PerformanceHardeningConfig::default();
    let _manager = PerformanceHardeningManager::new(config);
    // Should create without panicking
}

#[test]
fn test_performance_hardening_manager_with_custom_config() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: true,
        enable_memory_pools: false,
        enable_caching: true,
        enable_async_optimization: false,
        enable_connection_pooling: true,
        monitoring_config: OptimizedMonitoringConfig::default(),
        memory_pool_config: MemoryPoolConfig::default(),
        caching_config: CachingConfig::default(),
        async_config: AsyncOptimizationConfig::default(),
        connection_pool_config: PerformanceConnectionPoolConfig::default(),
    };
    let _manager = PerformanceHardeningManager::new(config);
    // Should create without panicking
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_manager_initialize() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);

    let result = manager.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_manager_get_resource_monitor() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);

    let monitor = manager.get_resource_monitor();
    // Should return a monitor
    let interval = monitor.get_sampling_interval().await;
    assert!(interval.as_millis() > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_manager_create_memory_pool() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);

    let result = manager
        .create_memory_pool::<Vec<u8>, _>("test-pool", || Vec::with_capacity(1024))
        .await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_manager_create_memory_pool_disabled() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: true,
        enable_memory_pools: false, // Disabled
        enable_caching: true,
        enable_async_optimization: true,
        enable_connection_pooling: true,
        monitoring_config: OptimizedMonitoringConfig::default(),
        memory_pool_config: MemoryPoolConfig::default(),
        caching_config: CachingConfig::default(),
        async_config: AsyncOptimizationConfig::default(),
        connection_pool_config: PerformanceConnectionPoolConfig::default(),
    };
    let manager = PerformanceHardeningManager::new(config);

    let result = manager
        .create_memory_pool::<Vec<u8>, _>("test-pool", || Vec::with_capacity(1024))
        .await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_manager_create_cache() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);

    let result = manager.create_cache::<String, i32>("test-cache").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_manager_create_cache_disabled() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: true,
        enable_memory_pools: true,
        enable_caching: false, // Disabled
        enable_async_optimization: true,
        enable_connection_pooling: true,
        monitoring_config: OptimizedMonitoringConfig::default(),
        memory_pool_config: MemoryPoolConfig::default(),
        caching_config: CachingConfig::default(),
        async_config: AsyncOptimizationConfig::default(),
        connection_pool_config: PerformanceConnectionPoolConfig::default(),
    };
    let manager = PerformanceHardeningManager::new(config);

    let result = manager.create_cache::<String, i32>("test-cache").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_manager_use_created_cache() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);

    let cache = manager
        .create_cache::<String, i32>("test-cache")
        .await
        .expect("Should create cache");

    // Use the cache
    cache
        .put("key".to_string(), 42)
        .await
        .expect("Should insert");
    let value = cache.get(&"key".to_string()).await;
    assert_eq!(value, Some(42));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_manager_use_created_pool() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);

    let pool = manager
        .create_memory_pool::<Vec<u8>, _>("test-pool", || Vec::with_capacity(1024))
        .await
        .expect("Should create pool");

    // Use the pool
    let obj = pool.get().await;
    assert!(obj.get().is_some());
}
