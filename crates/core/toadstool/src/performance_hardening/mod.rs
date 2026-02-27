//! # Performance Hardening Module
//!
//! This module provides performance optimization features for `ToadStool`:
//! - Optimized resource monitoring with configurable sampling
//! - Memory pool management and allocation optimization
//! - Intelligent caching and memoization
//! - Async operation optimization and batching
//! - Connection pooling and resource reuse
//! - Performance metrics and profiling
//!
//! ## Organization
//!
//! The module is organized by logical resource domains:
//! - `types`: All configuration and statistics types
//! - `monitoring`: Resource monitoring and metrics collection
//! - `memory`: Memory pool management
//! - `caching`: Intelligent caching with LRU and TTL
//! - `async_ops`: Async operation batching and optimization
//!
//! This organization follows Deep Debt principles of smart refactoring by domain
//! rather than arbitrary line count limits.

// Module declarations
pub mod async_ops;
pub mod caching;
pub mod memory;
pub mod monitoring;
pub mod types;

// Re-exports for public API
pub use async_ops::AsyncBatcher;
pub use caching::IntelligentCache;
pub use memory::{MemoryPool, PooledObject};
pub use monitoring::OptimizedResourceMonitor;
pub use types::*;

use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{ToadStoolError, ToadStoolResult};

/// Performance hardening manager
///
/// Central manager for all performance hardening features, providing
/// unified configuration and lifecycle management.
pub struct PerformanceHardeningManager {
    /// Configuration
    config: PerformanceHardeningConfig,
    /// Optimized resource monitor
    resource_monitor: Arc<OptimizedResourceMonitor>,
    /// Memory pools (type-erased for storage)
    memory_pools: Arc<RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
    /// Intelligent caches (type-erased for storage)
    caches: Arc<RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
}

impl PerformanceHardeningManager {
    /// Create new performance hardening manager
    #[must_use]
    pub fn new(config: PerformanceHardeningConfig) -> Self {
        let resource_monitor = Arc::new(OptimizedResourceMonitor::new(
            config.monitoring_config.clone(),
        ));

        Self {
            config,
            resource_monitor,
            memory_pools: Arc::new(RwLock::new(HashMap::new())),
            caches: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Initialize performance hardening
    pub async fn initialize(&self) -> ToadStoolResult<()> {
        info!("Initializing performance hardening");

        // Start monitoring
        if self.config.enable_optimized_monitoring {
            self.start_monitoring_task().await;
        }

        info!("Performance hardening initialized");
        Ok(())
    }

    /// Start monitoring task
    async fn start_monitoring_task(&self) {
        let resource_monitor = Arc::clone(&self.resource_monitor);
        let base_interval = self.config.monitoring_config.base_sampling_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(base_interval);

            loop {
                interval.tick().await;

                // Get current sampling interval
                let current_interval = resource_monitor.get_sampling_interval().await;

                // Adjust interval if needed
                if current_interval != base_interval {
                    interval = tokio::time::interval(current_interval);
                }

                // Future enhancement: Collect actual metrics and add samples
                // This would integrate with the real resource monitoring system
                // Current implementation provides basic performance monitoring
            }
        });
    }

    /// Get resource monitor
    #[must_use]
    pub fn get_resource_monitor(&self) -> Arc<OptimizedResourceMonitor> {
        Arc::clone(&self.resource_monitor)
    }

    /// Create memory pool
    pub async fn create_memory_pool<T, F>(
        &self,
        name: &str,
        factory: F,
    ) -> ToadStoolResult<Arc<MemoryPool<T>>>
    where
        T: Send + Sync + 'static,
        F: Fn() -> T + Send + Sync + 'static,
    {
        if !self.config.enable_memory_pools {
            return Err(ToadStoolError::runtime(
                "Memory pools are disabled".to_string(),
            ));
        }

        let pool = Arc::new(MemoryPool::new(
            self.config.memory_pool_config.clone(),
            factory,
        ));

        let mut pools = self.memory_pools.write().await;
        pools.insert(name.to_string(), pool.clone());

        Ok(pool)
    }

    /// Create intelligent cache
    pub async fn create_cache<K, V>(
        &self,
        name: &str,
    ) -> ToadStoolResult<Arc<IntelligentCache<K, V>>>
    where
        K: Hash + Eq + Clone + Send + Sync + 'static,
        V: Clone + Send + Sync + 'static,
    {
        if !self.config.enable_caching {
            return Err(ToadStoolError::runtime("Caching is disabled".to_string()));
        }

        let cache = Arc::new(IntelligentCache::new(self.config.caching_config.clone()));
        cache.start_cleanup_task().await;

        let mut caches = self.caches.write().await;
        caches.insert(name.to_string(), cache.clone());

        Ok(cache)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::{
        CpuMetrics, MemoryMetrics, NetworkMetrics, RuntimeMetrics, StorageMetrics,
    };
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
                used_bytes: (memory_percent / 100.0 * 8_000_000_000.0) as u64,
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
}
