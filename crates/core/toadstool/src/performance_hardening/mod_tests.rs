// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::resources::{CpuMetrics, MemoryMetrics, NetworkMetrics, RuntimeMetrics, StorageMetrics};
use std::time::Duration;

// Helper function to create test metrics
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
                clippy::cast_sign_loss
            )] // synthetic test metrics; non-negative
            used_bytes: (memory_percent / 100.0 * 8_000_000_000.0) as u64,
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss
            )]
            peak_bytes: (memory_percent / 100.0 * 8_000_000_000.0) as u64,
        },
        storage: StorageMetrics::default(),
        network: NetworkMetrics::default(),
        gpu: None,
        timing: crate::resources::TimingMetrics {
            start_time: std::time::SystemTime::now(),
            end_time: None,
            duration: std::time::Duration::ZERO,
        },
    }
}

// ===== OptimizedResourceMonitor Tests =====

#[tokio::test]
async fn test_optimized_monitor_creation() {
    let config = OptimizedMonitoringConfig::default();
    let monitor = OptimizedResourceMonitor::new(config.clone());

    let interval = monitor.get_sampling_interval().await;
    assert_eq!(interval, config.base_sampling_interval);
}

#[tokio::test]
async fn test_optimized_monitor_add_sample() {
    let config = OptimizedMonitoringConfig::default();
    let monitor = OptimizedResourceMonitor::new(config);

    let metrics = create_test_metrics(50.0, 60.0);
    monitor.add_sample("workload-1", metrics.clone()).await;

    let aggregated = monitor.get_aggregated_metrics("workload-1").await;
    assert!(aggregated.is_some());
}

#[tokio::test]
async fn test_adaptive_sampling_high_load() {
    let config = OptimizedMonitoringConfig {
        adaptive_sampling: true,
        base_sampling_interval: Duration::from_millis(100),
        high_load_multiplier: 0.5,
        ..Default::default()
    };

    let monitor = OptimizedResourceMonitor::new(config);

    // Simulate high load (>80%)
    let high_load_metrics = create_test_metrics(90.0, 90.0);
    monitor.add_sample("workload-1", high_load_metrics).await;

    let interval = monitor.get_sampling_interval().await;
    // Should be reduced (faster sampling)
    assert_eq!(interval, Duration::from_millis(50)); // 100 * 0.5
}

// ===== MemoryPool Tests =====

#[tokio::test]
async fn test_memory_pool_creation() {
    let config = MemoryPoolConfig::default();
    let pool = MemoryPool::new(config, || String::from("test"));

    let stats = pool.get_stats().await;
    assert_eq!(stats.total_allocations, 0);
}

#[tokio::test]
async fn test_memory_pool_get_release() {
    let config = MemoryPoolConfig::default();
    let pool = MemoryPool::new(config, || String::from("test"));

    // Get an object
    let obj = pool.get().await;
    assert_eq!(obj.get().map(|s| s.as_str()), Some("test"));

    let stats = pool.get_stats().await;
    assert_eq!(stats.total_allocations, 1);

    // Release back to pool (synchronous via Drop's try_lock)
    drop(obj);

    // Verify return is immediate (no sleeps needed!)
    let stats = pool.get_stats().await;
    assert_eq!(stats.total_deallocations, 1);
    assert!(stats.available > 0); // Should be back in pool immediately
}

// ===== IntelligentCache Tests =====

#[tokio::test]
async fn test_cache_creation() {
    let config = CachingConfig::default();
    let cache: IntelligentCache<String, i32> = IntelligentCache::new(config);

    let stats = cache.get_stats().await;
    assert_eq!(stats.hits, 0);
    assert_eq!(stats.misses, 0);
}

#[tokio::test]
async fn test_cache_put_get() {
    let config = CachingConfig::default();
    let cache = IntelligentCache::new(config);

    let _ = cache.put("key1".to_string(), 42).await;

    let value = cache.get(&"key1".to_string()).await;
    assert_eq!(value, Some(42));

    let stats = cache.get_stats().await;
    assert_eq!(stats.hits, 1);
    assert_eq!(stats.misses, 0);
}

#[tokio::test]
async fn test_cache_expiration() {
    let config = CachingConfig {
        default_ttl: Duration::from_secs(300), // Normal TTL
        ..Default::default()
    };
    let cache = IntelligentCache::new(config);

    // Put with very short custom TTL that will expire immediately
    let _ = cache
        .put_with_ttl("key1".to_string(), 42, Duration::from_nanos(1))
        .await;

    // Get will check expiration - nanosecond has definitely passed
    // No sleep needed - expiration is checked on every get()
    assert_eq!(cache.get(&"key1".to_string()).await, None);

    // Verify it was counted as a miss
    let stats = cache.get_stats().await;
    assert_eq!(stats.misses, 1);
}

// ===== AsyncBatcher Tests =====

#[tokio::test]
async fn test_batcher_creation() {
    let config = AsyncOptimizationConfig::default();
    let _batcher: AsyncBatcher<i32, i32> = AsyncBatcher::new(config, |items: Vec<i32>| {
        Box::pin(async move { items.into_iter().map(|x| x * 2).collect() })
    });
}

#[tokio::test]
async fn test_batcher_submit() {
    let config = AsyncOptimizationConfig {
        batch_size: 1, // Process immediately
        batch_timeout: Duration::from_millis(100),
        ..Default::default()
    };

    let batcher = AsyncBatcher::new(config, |items: Vec<i32>| {
        Box::pin(async move { items.into_iter().map(|x| x * 2).collect() })
    });

    let result = batcher.submit(5).await.unwrap();
    assert_eq!(result, 10); // 5 * 2
}

// ===== PerformanceHardeningManager Tests =====

#[tokio::test]
async fn test_manager_creation() {
    let config = PerformanceHardeningConfig::default();
    let _manager = PerformanceHardeningManager::new(config);
}

#[tokio::test]
async fn test_manager_resource_monitor() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);

    let monitor = manager.get_resource_monitor();

    // Add a sample
    let metrics = create_test_metrics(50.0, 60.0);
    monitor.add_sample("test-workload", metrics).await;
}

#[tokio::test]
async fn test_manager_memory_pool() {
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
async fn test_manager_cache() {
    let config = PerformanceHardeningConfig::default();
    let manager = PerformanceHardeningManager::new(config);

    let cache: Arc<IntelligentCache<String, i32>> =
        manager.create_cache("test-cache").await.unwrap();

    let _ = cache.put("key1".to_string(), 42).await;
    assert_eq!(cache.get(&"key1".to_string()).await, Some(42));
}

#[tokio::test]
async fn test_manager_disabled_features() {
    let config = PerformanceHardeningConfig {
        enable_memory_pools: false,
        enable_caching: false,
        ..Default::default()
    };

    let manager = PerformanceHardeningManager::new(config);

    // Should fail when features are disabled
    let pool_result = manager.create_memory_pool("test", || 0).await;
    assert!(pool_result.is_err());

    let cache_result: ToadStoolResult<Arc<IntelligentCache<String, i32>>> =
        manager.create_cache("test").await;
    assert!(cache_result.is_err());
}

// ===== Configuration Tests =====

#[test]
fn test_default_configs() {
    let _ph_config = PerformanceHardeningConfig::default();
    let _monitor_config = OptimizedMonitoringConfig::default();
    let _pool_config = MemoryPoolConfig::default();
    let _cache_config = CachingConfig::default();
    let _async_config = AsyncOptimizationConfig::default();
    let _conn_config = PerformanceConnectionPoolConfig::default();
}
