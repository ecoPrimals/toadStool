// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive coverage tests for `performance_hardening` module
//!
//! Exercises all struct constructors, Default impls, and public methods
//! across types, async_ops, caching, memory, monitoring, and mod.

#![allow(clippy::pedantic)]

use std::sync::Arc;
use std::time::Duration;
use toadstool::performance_hardening::*;
use toadstool::resources::{
    CpuMetrics, MemoryMetrics, NetworkMetrics, RuntimeMetrics, StorageMetrics, TimingMetrics,
};

// ============================================================================
// Helper: create_test_metrics
// ============================================================================

fn create_test_metrics(cpu_percent: f64, memory_percent: f64) -> RuntimeMetrics {
    RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: cpu_percent,
            cores_used: cpu_percent / 100.0 * 4.0,
            cpu_time_seconds: 1.0,
        },
        memory: MemoryMetrics {
            usage_percent: memory_percent,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "test fixture; 8GB max fits u64"
            )]
            used_bytes: (memory_percent / 100.0 * 8_000_000_000.0) as u64,
            #[expect(
                clippy::cast_possible_truncation,
                reason = "test fixture; 8GB max fits u64"
            )]
            peak_bytes: (memory_percent / 100.0 * 8_000_000_000.0) as u64,
        },
        storage: StorageMetrics::default(),
        network: NetworkMetrics::default(),
        gpu: None,
        timing: TimingMetrics {
            start_time: std::time::SystemTime::now(),
            end_time: None,
            duration: Duration::ZERO,
        },
    }
}

// ============================================================================
// types.rs — Struct constructors and Default impls
// ============================================================================

#[test]
fn types_performance_hardening_config_default() {
    let config = PerformanceHardeningConfig::default();
    assert!(config.enable_optimized_monitoring);
    assert!(config.enable_memory_pools);
    assert!(config.enable_caching);
    assert!(config.enable_async_optimization);
    assert!(config.enable_connection_pooling);
}

#[test]
fn types_optimized_monitoring_config_default() {
    let config = OptimizedMonitoringConfig::default();
    assert_eq!(config.base_sampling_interval, Duration::from_millis(100));
    assert!(config.adaptive_sampling);
    assert!((config.high_load_multiplier - 0.5).abs() < 1e-10);
    assert!((config.low_load_multiplier - 2.0).abs() < 1e-10);
    assert_eq!(config.batch_size, 10);
}

#[test]
fn types_memory_pool_config_default() {
    let config = MemoryPoolConfig::default();
    assert_eq!(config.initial_size, 100);
    assert_eq!(config.max_size, 1000);
    assert!((config.growth_factor - 1.5).abs() < 1e-10);
    assert!((config.shrink_threshold - 0.3).abs() < 1e-10);
}

#[test]
fn types_caching_config_default() {
    let config = CachingConfig::default();
    assert_eq!(config.max_size, 1000);
    assert_eq!(config.cleanup_interval, Duration::from_secs(60));
    assert!((config.hit_rate_threshold - 0.8).abs() < 1e-10);
}

#[test]
fn types_async_optimization_config_default() {
    let config = AsyncOptimizationConfig::default();
    assert_eq!(config.batch_size, 50);
    assert_eq!(config.batch_timeout, Duration::from_millis(100));
    assert_eq!(config.concurrency_limit, 100);
    assert_eq!(config.queue_size_limit, 1000);
}

#[test]
fn types_performance_connection_pool_config_default() {
    let config = PerformanceConnectionPoolConfig::default();
    assert_eq!(config.initial_size, 10);
    assert_eq!(config.max_size, 100);
    assert_eq!(config.connection_timeout, Duration::from_secs(30));
    assert_eq!(config.idle_timeout, Duration::from_secs(300));
    assert_eq!(config.health_check_interval, Duration::from_secs(60));
}

#[test]
fn types_aggregated_metrics_construction() {
    let m = AggregatedMetrics {
        cpu_usage: 75.5,
        memory_usage: 1_000_000,
        active_connections: 5,
        request_rate: 10.0,
        avg_response_time: 50.0,
    };
    assert!((m.cpu_usage - 75.5).abs() < 1e-10);
    assert_eq!(m.memory_usage, 1_000_000);
    assert_eq!(m.active_connections, 5);
}

#[test]
fn types_pool_stats_construction() {
    let s = PoolStats {
        current_size: 10,
        in_use: 3,
        available: 7,
        total_allocations: 100,
        total_deallocations: 97,
        hit_rate: 0.9,
    };
    assert_eq!(s.in_use + s.available, s.current_size);
}

#[test]
fn types_cache_stats_construction() {
    let s = CacheStats {
        current_size: 5,
        hits: 10,
        misses: 2,
        hit_rate: 0.833,
        evictions: 1,
    };
    assert_eq!(s.current_size, 5);
    assert_eq!(s.hits, 10);
    assert_eq!(s.evictions, 1);
}

// ============================================================================
// async_ops.rs — AsyncBatcher
// ============================================================================

#[tokio::test]
async fn async_ops_batcher_new_and_submit() {
    let config = AsyncOptimizationConfig {
        batch_size: 1,
        batch_timeout: Duration::from_millis(100),
        ..Default::default()
    };
    let batcher = AsyncBatcher::new(config, |v: Vec<i32>| {
        Box::pin(async move { v.into_iter().map(|x| x * 2).collect() })
    });
    let result = batcher.submit(21).await.unwrap();
    assert_eq!(result, 42);
}

#[tokio::test]
async fn async_ops_batcher_batch_of_two() {
    let config = AsyncOptimizationConfig {
        batch_size: 2,
        batch_timeout: Duration::from_millis(100),
        ..Default::default()
    };
    let batcher = AsyncBatcher::new(config, |v: Vec<String>| {
        Box::pin(async move { v.into_iter().map(|s| s.to_uppercase()).collect() })
    });
    let (r1, r2) = tokio::join!(
        batcher.submit("hello".to_string()),
        batcher.submit("world".to_string())
    );
    assert_eq!(r1.unwrap(), "HELLO");
    assert_eq!(r2.unwrap(), "WORLD");
}

#[tokio::test]
async fn async_ops_batcher_start_batch_task() {
    let config = AsyncOptimizationConfig {
        batch_size: 2,
        batch_timeout: Duration::from_millis(50),
        ..Default::default()
    };
    let batcher = Arc::new(AsyncBatcher::new(config, |v: Vec<i32>| {
        Box::pin(async move { v.into_iter().map(|x| x + 1).collect() })
    }));
    batcher.start_batch_task().await;
    let r = batcher.submit(10).await.unwrap();
    assert_eq!(r, 11);
}

// ============================================================================
// caching.rs — IntelligentCache
// ============================================================================

#[tokio::test]
async fn caching_new_and_get_stats() {
    let config = CachingConfig::default();
    let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);
    let stats = cache.get_stats().await;
    assert_eq!(stats.current_size, 0);
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
}

#[tokio::test]
async fn caching_put_and_get() {
    let config = CachingConfig::default();
    let cache = IntelligentCache::new(config);
    let _ = cache.put("key1".to_string(), 42).await;
    assert_eq!(cache.get(&"key1".to_string()).await, Some(42));
}

#[tokio::test]
async fn caching_put_with_ttl() {
    let config = CachingConfig::default();
    let cache = IntelligentCache::new(config);
    let _ = cache
        .put_with_ttl("key".to_string(), 100, Duration::from_secs(60))
        .await;
    assert_eq!(cache.get(&"key".to_string()).await, Some(100));
}

#[tokio::test]
async fn caching_evict_lru_on_size_limit() {
    let config = CachingConfig {
        max_size: 3,
        ..Default::default()
    };
    let cache = IntelligentCache::new(config);
    let _ = cache.put("k1".to_string(), 1).await;
    let _ = cache.put("k2".to_string(), 2).await;
    let _ = cache.put("k3".to_string(), 3).await;
    let _ = cache.put("k4".to_string(), 4).await;
    let stats = cache.get_stats().await;
    assert_eq!(stats.evictions, 1);
    assert_eq!(stats.current_size, 3);
    assert_eq!(cache.get(&"k1".to_string()).await, None);
    assert_eq!(cache.get(&"k4".to_string()).await, Some(4));
}

#[tokio::test]
async fn caching_expired_entry_returns_none() {
    let config = CachingConfig::default();
    let cache = IntelligentCache::new(config);
    let _ = cache
        .put_with_ttl("key".to_string(), 42, Duration::from_nanos(1))
        .await;
    assert_eq!(cache.get(&"key".to_string()).await, None);
}

#[tokio::test]
async fn caching_start_cleanup_task() {
    let config = CachingConfig::default();
    let cache = IntelligentCache::new(config);
    cache.start_cleanup_task().await;
    let _ = cache.put("key".to_string(), 1).await;
    assert_eq!(cache.get(&"key".to_string()).await, Some(1));
}

// ============================================================================
// memory.rs — MemoryPool and PooledObject
// ============================================================================

#[tokio::test]
async fn memory_pool_new_and_get_stats() {
    let config = MemoryPoolConfig::default();
    let pool = MemoryPool::new(config, || String::from("test"));
    let stats = pool.get_stats().await;
    assert_eq!(stats.total_allocations, 0);
    assert_eq!(stats.available, 100);
}

#[tokio::test]
async fn memory_pool_get_and_release() {
    let config = MemoryPoolConfig {
        initial_size: 2,
        max_size: 10,
        ..Default::default()
    };
    let pool = MemoryPool::new(config, || String::from("obj"));
    let obj = pool.get().await;
    assert_eq!(obj.get().map(|s| s.as_str()), Some("obj"));
    drop(obj);
    let stats = pool.get_stats().await;
    assert_eq!(stats.total_deallocations, 1);
}

#[tokio::test]
async fn memory_pooled_object_get_mut() {
    let config = MemoryPoolConfig {
        initial_size: 1,
        max_size: 10,
        ..Default::default()
    };
    let pool = MemoryPool::new(config, || vec![1, 2, 3]);
    let mut obj = pool.get().await;
    if let Some(v) = obj.get_mut() {
        v.push(4);
        assert_eq!(v, &[1, 2, 3, 4]);
    }
}

#[tokio::test]
async fn memory_pool_exhaustion_then_reuse() {
    let config = MemoryPoolConfig {
        initial_size: 2,
        max_size: 5,
        ..Default::default()
    };
    let pool = MemoryPool::new(config, || 42i32);
    let mut objs = Vec::new();
    for _ in 0..5 {
        objs.push(pool.get().await);
    }
    drop(objs);
    let obj = pool.get().await;
    assert_eq!(obj.get(), Some(&42));
}

// ============================================================================
// monitoring.rs — OptimizedResourceMonitor
// ============================================================================

#[tokio::test]
async fn monitoring_new_and_get_sampling_interval() {
    let config = OptimizedMonitoringConfig::default();
    let monitor = OptimizedResourceMonitor::new(config.clone());
    let interval = monitor.get_sampling_interval().await;
    assert_eq!(interval, config.base_sampling_interval);
}

#[tokio::test]
async fn monitoring_add_sample_and_get_aggregated() {
    let config = OptimizedMonitoringConfig::default();
    let monitor = OptimizedResourceMonitor::new(config);
    let metrics = create_test_metrics(50.0, 60.0);
    monitor.add_sample("workload-1", metrics).await;
    let agg = monitor.get_aggregated_metrics("workload-1").await;
    assert!(agg.is_some());
    let a = agg.unwrap();
    assert!((a.cpu_usage - 50.0).abs() < 1e-10);
}

#[tokio::test]
async fn monitoring_adaptive_sampling_high_load() {
    let config = OptimizedMonitoringConfig {
        adaptive_sampling: true,
        base_sampling_interval: Duration::from_millis(100),
        high_load_multiplier: 0.5,
        ..Default::default()
    };
    let monitor = OptimizedResourceMonitor::new(config);
    let metrics = create_test_metrics(90.0, 90.0);
    monitor.add_sample("high", metrics).await;
    let interval = monitor.get_sampling_interval().await;
    assert_eq!(interval, Duration::from_millis(50));
}

#[tokio::test]
async fn monitoring_adaptive_sampling_low_load() {
    let config = OptimizedMonitoringConfig {
        adaptive_sampling: true,
        base_sampling_interval: Duration::from_millis(100),
        low_load_multiplier: 2.0,
        ..Default::default()
    };
    let monitor = OptimizedResourceMonitor::new(config);
    let metrics = create_test_metrics(10.0, 10.0);
    monitor.add_sample("low", metrics).await;
    let interval = monitor.get_sampling_interval().await;
    assert_eq!(interval, Duration::from_millis(200));
}

#[tokio::test]
async fn monitoring_get_aggregated_missing_workload() {
    let config = OptimizedMonitoringConfig::default();
    let monitor = OptimizedResourceMonitor::new(config);
    assert!(
        monitor
            .get_aggregated_metrics("nonexistent")
            .await
            .is_none()
    );
}

#[tokio::test]
async fn monitoring_buffer_trimming_on_overflow() {
    let config = OptimizedMonitoringConfig {
        batch_size: 5,
        ..Default::default()
    };
    let monitor = OptimizedResourceMonitor::new(config);
    for i in 0..15 {
        let m = create_test_metrics(i as f64, i as f64);
        monitor.add_sample("w", m).await;
    }
    let agg = monitor.get_aggregated_metrics("w").await;
    assert!(agg.is_some());
}

// ============================================================================
// mod.rs — PerformanceHardeningManager
// ============================================================================

#[tokio::test]
async fn manager_new() {
    let config = PerformanceHardeningConfig::default();
    let _manager = PerformanceHardeningManager::new(config);
}

#[tokio::test]
async fn manager_get_resource_monitor() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);
    let monitor = manager.get_resource_monitor();
    let metrics = create_test_metrics(50.0, 60.0);
    monitor.add_sample("test", metrics).await;
}

#[tokio::test]
async fn manager_initialize() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);
    let result = manager.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn manager_create_memory_pool() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);
    let pool = manager
        .create_memory_pool("test-pool", || String::from("test"))
        .await
        .unwrap();
    let obj = pool.get().await;
    assert_eq!(obj.get().map(|s| s.as_str()), Some("test"));
}

#[tokio::test]
async fn manager_create_cache() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);
    let cache: Arc<IntelligentCache<String, i32>> =
        manager.create_cache("test-cache").await.unwrap();
    let _ = cache.put("k".to_string(), 42).await;
    assert_eq!(cache.get(&"k".to_string()).await, Some(42));
}

#[tokio::test]
async fn manager_create_memory_pool_disabled() {
    let config = PerformanceHardeningConfig {
        enable_memory_pools: false,
        ..Default::default()
    };
    let manager = PerformanceHardeningManager::new(config);
    let result = manager.create_memory_pool("test", || 0).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn manager_create_cache_disabled() {
    let config = PerformanceHardeningConfig {
        enable_caching: false,
        ..Default::default()
    };
    let manager = PerformanceHardeningManager::new(config);
    let result: Result<Arc<IntelligentCache<String, i32>>, _> = manager.create_cache("test").await;
    assert!(result.is_err());
}

// ============================================================================
// Additional coverage: Caching edge cases
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn caching_concurrent_access() {
    let config = CachingConfig {
        max_size: 100,
        ..Default::default()
    };
    let cache = Arc::new(IntelligentCache::new(config));
    cache.start_cleanup_task().await;

    let mut handles = Vec::new();
    for i in 0..20 {
        let c = Arc::clone(&cache);
        handles.push(tokio::spawn(async move {
            let _ = c.put(format!("key-{i}"), i).await;
            c.get(&format!("key-{i}")).await
        }));
    }
    for (i, h) in handles.into_iter().enumerate() {
        let v = h.await.unwrap();
        assert_eq!(v, Some(i));
    }
}

#[tokio::test]
async fn caching_large_number_of_entries() {
    let config = CachingConfig {
        max_size: 50,
        ..Default::default()
    };
    let cache = IntelligentCache::new(config);
    for i in 0..100 {
        let _ = cache.put(format!("k{i}"), i).await;
    }
    let stats = cache.get_stats().await;
    assert_eq!(stats.current_size, 50);
    assert!(stats.evictions >= 50);
    assert_eq!(cache.get(&"k0".to_string()).await, None);
    assert_eq!(cache.get(&"k99".to_string()).await, Some(99));
}

#[tokio::test]
async fn caching_expired_entries_cleanup_task() {
    let config = CachingConfig {
        max_size: 100,
        cleanup_interval: Duration::from_millis(50),
        ..Default::default()
    };
    let cache = IntelligentCache::new(config);
    cache.start_cleanup_task().await;
    let _ = cache
        .put_with_ttl("expire-me".to_string(), 1, Duration::from_millis(10))
        .await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(cache.get(&"expire-me".to_string()).await, None);
}

// ============================================================================
// Additional coverage: Memory pool exhaustion and recovery
// ============================================================================

#[tokio::test]
async fn memory_pool_exhaustion_beyond_max() {
    let config = MemoryPoolConfig {
        initial_size: 2,
        max_size: 4,
        ..Default::default()
    };
    let pool = MemoryPool::new(config, || 0i32);
    let mut objs = Vec::new();
    for _ in 0..6 {
        objs.push(pool.get().await);
    }
    let stats = pool.get_stats().await;
    assert_eq!(stats.total_allocations, 6);
    drop(objs);
    let obj = pool.get().await;
    assert_eq!(obj.get(), Some(&0));
}

#[tokio::test]
async fn memory_pool_recovery_after_full_exhaustion() {
    let config = MemoryPoolConfig {
        initial_size: 1,
        max_size: 2,
        ..Default::default()
    };
    let pool = MemoryPool::new(config, || vec![1, 2, 3]);
    let o1 = pool.get().await;
    let o2 = pool.get().await;
    drop(o1);
    let o3 = pool.get().await;
    assert_eq!(o3.get().map(|v| v.as_slice()), Some(&[1, 2, 3][..]));
    drop(o2);
    drop(o3);
}

// ============================================================================
// Additional coverage: Monitoring with many samples, multiple workloads
// ============================================================================

#[tokio::test]
async fn monitoring_many_samples_single_workload() {
    let config = OptimizedMonitoringConfig {
        batch_size: 5,
        ..Default::default()
    };
    let monitor = OptimizedResourceMonitor::new(config);
    for i in 0..25 {
        let m = create_test_metrics(i as f64 * 2.0, i as f64 * 2.0);
        monitor.add_sample("heavy", m).await;
    }
    let agg = monitor.get_aggregated_metrics("heavy").await;
    assert!(agg.is_some());
}

#[tokio::test]
async fn monitoring_multiple_workloads() {
    let config = OptimizedMonitoringConfig::default();
    let monitor = OptimizedResourceMonitor::new(config);
    monitor
        .add_sample("w1", create_test_metrics(10.0, 20.0))
        .await;
    monitor
        .add_sample("w2", create_test_metrics(80.0, 90.0))
        .await;
    monitor
        .add_sample("w3", create_test_metrics(50.0, 50.0))
        .await;
    let a1 = monitor.get_aggregated_metrics("w1").await.unwrap();
    let a2 = monitor.get_aggregated_metrics("w2").await.unwrap();
    let a3 = monitor.get_aggregated_metrics("w3").await.unwrap();
    assert!((a1.cpu_usage - 10.0).abs() < 1e-10);
    assert!((a2.cpu_usage - 80.0).abs() < 1e-10);
    assert!((a3.cpu_usage - 50.0).abs() < 1e-10);
}

// ============================================================================
// Additional coverage: async_ops pipeline chaining and error propagation
// ============================================================================

#[tokio::test]
async fn async_ops_batcher_chain_processing() {
    let config = AsyncOptimizationConfig {
        batch_size: 2,
        batch_timeout: Duration::from_millis(50),
        ..Default::default()
    };
    let batcher = AsyncBatcher::new(config, |v: Vec<i32>| {
        Box::pin(async move { v.into_iter().map(|x| x * 3).collect() })
    });
    batcher.start_batch_task().await;
    let r1 = batcher.submit(1).await.unwrap();
    let r2 = batcher.submit(2).await.unwrap();
    assert_eq!(r1, 3);
    assert_eq!(r2, 6);
}

#[tokio::test]
async fn async_ops_batcher_semaphore_try_acquire_path() {
    let config = AsyncOptimizationConfig {
        batch_size: 1,
        concurrency_limit: 1,
        ..Default::default()
    };
    let batcher = AsyncBatcher::new(config, |v: Vec<i32>| {
        Box::pin(async move { v.into_iter().map(|x| x * 2).collect() })
    });
    let r = batcher.submit(7).await.unwrap();
    assert_eq!(r, 14);
}

// ============================================================================
// Additional coverage: Types with non-default values and boundary conditions
// ============================================================================

#[test]
fn types_performance_hardening_config_all_disabled() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: false,
        enable_memory_pools: false,
        enable_caching: false,
        enable_async_optimization: false,
        enable_connection_pooling: false,
        ..Default::default()
    };
    assert!(!config.enable_caching);
}

#[test]
fn types_optimized_monitoring_config_boundary_values() {
    let config = OptimizedMonitoringConfig {
        base_sampling_interval: Duration::from_secs(1),
        high_load_multiplier: 0.1,
        low_load_multiplier: 5.0,
        batch_size: 1,
        ..Default::default()
    };
    assert_eq!(config.batch_size, 1);
}

#[test]
fn types_memory_pool_config_boundary() {
    let config = MemoryPoolConfig {
        initial_size: 1,
        max_size: 1,
        growth_factor: 1.0,
        shrink_threshold: 0.0,
        ..Default::default()
    };
    assert_eq!(config.initial_size, config.max_size);
}

#[test]
fn types_aggregated_metrics_boundary() {
    let m = AggregatedMetrics {
        cpu_usage: 0.0,
        memory_usage: 0,
        active_connections: 0,
        request_rate: 0.0,
        avg_response_time: 0.0,
    };
    assert_eq!(m.memory_usage, 0);
}

// ============================================================================
// Additional coverage: mod.rs initialization with various configs
// ============================================================================

#[tokio::test]
async fn manager_initialize_with_monitoring_disabled() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: false,
        ..Default::default()
    };
    let manager = PerformanceHardeningManager::new(config);
    let result = manager.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn manager_initialize_minimal_config() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: true,
        enable_memory_pools: false,
        enable_caching: false,
        ..Default::default()
    };
    let manager = PerformanceHardeningManager::new(config);
    let result = manager.initialize().await;
    assert!(result.is_ok());
}
