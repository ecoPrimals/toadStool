//! # Performance Hardening Module
//!
//! This module provides performance optimization features for `ToadStool`:
//! - Optimized resource monitoring with configurable sampling
//! - Memory pool management and allocation optimization
//! - Intelligent caching and memoization
//! - Async operation optimization and batching
//! - Connection pooling and resource reuse
//! - Performance metrics and profiling

use std::collections::{HashMap, VecDeque};
use std::hash::Hash;
use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, Semaphore};
use tracing::info;

use crate::resources::RuntimeMetrics;
use crate::{ToadStoolError, ToadStoolResult};

/// Performance hardening configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceHardeningConfig {
    /// Enable optimized resource monitoring
    pub enable_optimized_monitoring: bool,
    /// Enable memory pool management
    pub enable_memory_pools: bool,
    /// Enable intelligent caching
    pub enable_caching: bool,
    /// Enable async optimization
    pub enable_async_optimization: bool,
    /// Enable connection pooling
    pub enable_connection_pooling: bool,
    /// Resource monitoring configuration
    pub monitoring_config: OptimizedMonitoringConfig,
    /// Memory pool configuration
    pub memory_pool_config: MemoryPoolConfig,
    /// Caching configuration
    pub caching_config: CachingConfig,
    /// Async optimization configuration
    pub async_config: AsyncOptimizationConfig,
    /// Connection pooling configuration
    pub connection_pool_config: PerformanceConnectionPoolConfig,
}

impl Default for PerformanceHardeningConfig {
    fn default() -> Self {
        Self {
            enable_optimized_monitoring: true,
            enable_memory_pools: true,
            enable_caching: true,
            enable_async_optimization: true,
            enable_connection_pooling: true,
            monitoring_config: OptimizedMonitoringConfig::default(),
            memory_pool_config: MemoryPoolConfig::default(),
            caching_config: CachingConfig::default(),
            async_config: AsyncOptimizationConfig::default(),
            connection_pool_config: PerformanceConnectionPoolConfig::default(),
        }
    }
}

/// Optimized resource monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizedMonitoringConfig {
    /// Base sampling interval
    pub base_sampling_interval: Duration,
    /// Adaptive sampling enabled
    pub adaptive_sampling: bool,
    /// High-load sampling multiplier
    pub high_load_multiplier: f64,
    /// Low-load sampling multiplier
    pub low_load_multiplier: f64,
    /// Batch size for metrics collection
    pub batch_size: usize,
    /// Metrics aggregation window
    pub aggregation_window: Duration,
}

impl Default for OptimizedMonitoringConfig {
    fn default() -> Self {
        Self {
            base_sampling_interval: Duration::from_millis(100),
            adaptive_sampling: true,
            high_load_multiplier: 0.5,
            low_load_multiplier: 2.0,
            batch_size: 10,
            aggregation_window: Duration::from_secs(60),
        }
    }
}

/// Memory pool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPoolConfig {
    /// Initial pool size
    pub initial_size: usize,
    /// Maximum pool size
    pub max_size: usize,
    /// Growth factor
    pub growth_factor: f64,
    /// Shrink threshold
    pub shrink_threshold: f64,
    /// Cleanup interval
    pub cleanup_interval: Duration,
}

impl Default for MemoryPoolConfig {
    fn default() -> Self {
        Self {
            initial_size: 100,
            max_size: 1000,
            growth_factor: 1.5,
            shrink_threshold: 0.3,
            cleanup_interval: Duration::from_secs(60),
        }
    }
}

/// Caching configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachingConfig {
    /// Maximum cache size
    pub max_size: usize,
    /// Default TTL
    pub default_ttl: Duration,
    /// Cleanup interval
    pub cleanup_interval: Duration,
    /// Cache hit rate threshold for optimization
    pub hit_rate_threshold: f64,
}

impl Default for CachingConfig {
    fn default() -> Self {
        Self {
            max_size: 1000,
            default_ttl: Duration::from_secs(300),
            cleanup_interval: Duration::from_secs(60),
            hit_rate_threshold: 0.8,
        }
    }
}

/// Async optimization configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AsyncOptimizationConfig {
    /// Batch size for async operations
    pub batch_size: usize,
    /// Batch timeout
    pub batch_timeout: Duration,
    /// Concurrency limit
    pub concurrency_limit: usize,
    /// Queue size limit
    pub queue_size_limit: usize,
}

impl Default for AsyncOptimizationConfig {
    fn default() -> Self {
        Self {
            batch_size: 50,
            batch_timeout: Duration::from_millis(100),
            concurrency_limit: 100,
            queue_size_limit: 1000,
        }
    }
}

/// Performance-optimized connection pooling configuration
///
/// This is distinct from `toadstool::config_bases::ConnectionPoolConfig` which is
/// for HTTP client connection pooling. This config is for generic connection pool
/// sizing and lifecycle management in performance-critical contexts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConnectionPoolConfig {
    /// Initial pool size
    pub initial_size: usize,
    /// Maximum pool size
    pub max_size: usize,
    /// Connection timeout
    pub connection_timeout: Duration,
    /// Idle timeout
    pub idle_timeout: Duration,
    /// Health check interval
    pub health_check_interval: Duration,
}

impl Default for PerformanceConnectionPoolConfig {
    fn default() -> Self {
        Self {
            initial_size: 10,
            max_size: 100,
            connection_timeout: Duration::from_secs(30),
            idle_timeout: Duration::from_secs(300),
            health_check_interval: Duration::from_secs(60),
        }
    }
}

/// Optimized resource monitor
pub struct OptimizedResourceMonitor {
    /// Configuration
    config: OptimizedMonitoringConfig,
    /// Metrics buffer
    metrics_buffer: Arc<RwLock<VecDeque<RuntimeMetrics>>>,
    /// Aggregated metrics
    aggregated_metrics: Arc<RwLock<HashMap<String, AggregatedMetrics>>>,
    /// Current load level
    current_load: Arc<RwLock<f64>>,
    /// Sampling interval
    current_sampling_interval: Arc<RwLock<Duration>>,
}

/// Aggregated metrics
#[derive(Debug, Clone)]
pub struct AggregatedMetrics {
    /// Average CPU usage
    pub avg_cpu_usage: f64,
    /// Average memory usage
    pub avg_memory_usage: f64,
    /// Peak memory usage
    pub peak_memory_usage: f64,
    /// Sample count
    pub sample_count: u64,
    /// Last updated
    pub last_updated: Instant,
}

impl OptimizedResourceMonitor {
    /// Create new optimized resource monitor
    #[must_use]
    pub fn new(config: OptimizedMonitoringConfig) -> Self {
        Self {
            config: config.clone(),
            metrics_buffer: Arc::new(RwLock::new(VecDeque::new())),
            aggregated_metrics: Arc::new(RwLock::new(HashMap::new())),
            current_load: Arc::new(RwLock::new(0.0)),
            current_sampling_interval: Arc::new(RwLock::new(config.base_sampling_interval)),
        }
    }

    /// Add metrics sample
    pub async fn add_sample(&self, workload_id: &str, metrics: RuntimeMetrics) {
        let mut buffer = self.metrics_buffer.write().await;
        buffer.push_back(metrics.clone());

        // Keep buffer size manageable
        if buffer.len() > self.config.batch_size * 2 {
            buffer.pop_front();
        }

        // Update aggregated metrics
        self.update_aggregated_metrics(workload_id, &metrics).await;

        // Adjust sampling if adaptive sampling is enabled
        if self.config.adaptive_sampling {
            self.adjust_sampling_interval(&metrics).await;
        }
    }

    /// Update aggregated metrics
    async fn update_aggregated_metrics(&self, workload_id: &str, metrics: &RuntimeMetrics) {
        let mut aggregated = self.aggregated_metrics.write().await;
        let now = Instant::now();

        let agg_metrics = aggregated
            .entry(workload_id.to_string())
            .or_insert_with(|| AggregatedMetrics {
                avg_cpu_usage: 0.0,
                avg_memory_usage: 0.0,
                peak_memory_usage: 0.0,
                sample_count: 0,
                last_updated: now,
            });

        // Update running averages
        let new_count = agg_metrics.sample_count + 1;
        agg_metrics.avg_cpu_usage = (agg_metrics.avg_cpu_usage * agg_metrics.sample_count as f64
            + metrics.cpu.usage_percent)
            / new_count as f64;
        agg_metrics.avg_memory_usage = (agg_metrics.avg_memory_usage
            * agg_metrics.sample_count as f64
            + metrics.memory.usage_percent)
            / new_count as f64;
        agg_metrics.peak_memory_usage = agg_metrics
            .peak_memory_usage
            .max(metrics.memory.usage_percent);
        agg_metrics.sample_count = new_count;
        agg_metrics.last_updated = now;
    }

    /// Adjust sampling interval based on system load
    async fn adjust_sampling_interval(&self, metrics: &RuntimeMetrics) {
        let load = (metrics.cpu.usage_percent + metrics.memory.usage_percent) / 200.0;

        let mut current_load = self.current_load.write().await;
        *current_load = load;

        let mut sampling_interval = self.current_sampling_interval.write().await;

        if load > 0.8 {
            // High load - sample more frequently
            *sampling_interval = Duration::from_millis(
                (self.config.base_sampling_interval.as_millis() as f64
                    * self.config.high_load_multiplier) as u64,
            );
        } else if load < 0.2 {
            // Low load - sample less frequently
            *sampling_interval = Duration::from_millis(
                (self.config.base_sampling_interval.as_millis() as f64
                    * self.config.low_load_multiplier) as u64,
            );
        } else {
            // Normal load - use base interval
            *sampling_interval = self.config.base_sampling_interval;
        }
    }

    /// Get aggregated metrics
    pub async fn get_aggregated_metrics(&self, workload_id: &str) -> Option<AggregatedMetrics> {
        let aggregated = self.aggregated_metrics.read().await;
        aggregated.get(workload_id).cloned()
    }

    /// Get current sampling interval
    pub async fn get_sampling_interval(&self) -> Duration {
        *self.current_sampling_interval.read().await
    }
}

/// Memory pool for object reuse
pub struct MemoryPool<T> {
    /// Configuration
    _config: MemoryPoolConfig,
    /// Available objects
    available: Arc<RwLock<Vec<T>>>,
    /// Factory function
    factory: Arc<dyn Fn() -> T + Send + Sync>,
    /// Usage statistics
    stats: Arc<RwLock<PoolStats>>,
}

/// Pool statistics
#[derive(Debug, Clone)]
pub struct PoolStats {
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Current size
    pub current_size: usize,
    /// Peak size
    pub peak_size: usize,
    /// Hit rate
    pub hit_rate: f64,
}

impl<T> MemoryPool<T> {
    /// Create new memory pool
    pub fn new<F>(config: MemoryPoolConfig, factory: F) -> Self
    where
        F: Fn() -> T + Send + Sync + 'static,
    {
        let factory = Arc::new(factory);
        let mut available = Vec::new();

        // Pre-allocate initial objects
        for _ in 0..config.initial_size {
            available.push(factory());
        }

        Self {
            _config: config.clone(),
            available: Arc::new(RwLock::new(available)),
            factory,
            stats: Arc::new(RwLock::new(PoolStats {
                total_allocations: 0,
                total_deallocations: 0,
                current_size: config.initial_size,
                peak_size: config.initial_size,
                hit_rate: 0.0,
            })),
        }
    }

    /// Get object from pool
    pub async fn get(&self) -> PooledObject<T>
    where
        T: Send + Sync + 'static,
    {
        let mut available = self.available.write().await;
        let mut stats = self.stats.write().await;

        let object = if let Some(obj) = available.pop() {
            stats.hit_rate = (stats.hit_rate * stats.total_allocations as f64 + 1.0)
                / (stats.total_allocations as f64 + 1.0);
            obj
        } else {
            // No available objects, create new one
            let new_obj = (self.factory)();
            stats.hit_rate = (stats.hit_rate * stats.total_allocations as f64)
                / (stats.total_allocations as f64 + 1.0);
            new_obj
        };

        stats.total_allocations += 1;
        stats.current_size = available.len();

        PooledObject {
            object: Some(object),
            pool: Arc::clone(&self.available),
            stats: Arc::clone(&self.stats),
        }
    }

    /// Return object to pool
    async fn _return_object(&self, object: T) {
        let mut available = self.available.write().await;
        let mut stats = self.stats.write().await;

        if available.len() < self._config.max_size {
            available.push(object);
            stats.current_size = available.len();
            stats.peak_size = stats.peak_size.max(stats.current_size);
        }

        stats.total_deallocations += 1;
    }

    /// Get pool statistics
    pub async fn get_stats(&self) -> PoolStats {
        self.stats.read().await.clone()
    }
}

/// Pooled object wrapper
pub struct PooledObject<T: Send + Sync + 'static> {
    /// The actual object
    object: Option<T>,
    /// Pool reference
    pool: Arc<RwLock<Vec<T>>>,
    /// Stats reference
    stats: Arc<RwLock<PoolStats>>,
}

impl<T: Send + Sync + 'static> PooledObject<T> {
    /// Get reference to the object
    pub fn get(&self) -> Option<&T> {
        self.object.as_ref()
    }

    /// Get mutable reference to the object
    pub fn get_mut(&mut self) -> Option<&mut T> {
        self.object.as_mut()
    }
}

impl<T: Send + Sync + 'static> Drop for PooledObject<T> {
    fn drop(&mut self) {
        if let Some(object) = self.object.take() {
            let pool = Arc::clone(&self.pool);
            let stats = Arc::clone(&self.stats);

            tokio::spawn(async move {
                let mut available = pool.write().await;
                let mut stats = stats.write().await;

                available.push(object);
                stats.current_size = available.len();
                stats.total_deallocations += 1;
            });
        }
    }
}

/// Intelligent cache
pub struct IntelligentCache<K, V> {
    /// Configuration
    config: CachingConfig,
    /// Cache entries
    entries: Arc<RwLock<HashMap<K, CacheEntry<V>>>>,
    /// Access order for LRU
    access_order: Arc<RwLock<VecDeque<K>>>,
    /// Statistics
    stats: Arc<RwLock<CacheStats>>,
}

/// Cache entry
#[derive(Debug, Clone)]
struct CacheEntry<V> {
    /// Value
    value: V,
    /// Expiry time
    expires_at: Instant,
    /// Access count
    access_count: u64,
    /// Last accessed
    last_accessed: Instant,
}

/// Cache statistics
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Total hits
    pub hits: u64,
    /// Total misses
    pub misses: u64,
    /// Current size
    pub current_size: usize,
    /// Hit rate
    pub hit_rate: f64,
}

impl<K, V> IntelligentCache<K, V>
where
    K: Hash + Eq + Clone + Send + Sync + 'static,
    V: Clone + Send + Sync + 'static,
{
    /// Create new intelligent cache
    #[must_use]
    pub fn new(config: CachingConfig) -> Self {
        Self {
            config,
            entries: Arc::new(RwLock::new(HashMap::new())),
            access_order: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(CacheStats {
                hits: 0,
                misses: 0,
                current_size: 0,
                hit_rate: 0.0,
            })),
        }
    }

    /// Get value from cache
    pub async fn get(&self, key: &K) -> Option<V> {
        let mut entries = self.entries.write().await;
        let mut stats = self.stats.write().await;
        let now = Instant::now();

        if let Some(entry) = entries.get_mut(key) {
            // Check if expired
            if now > entry.expires_at {
                entries.remove(key);
                stats.misses += 1;
                None
            } else {
                // Update access info
                entry.access_count += 1;
                entry.last_accessed = now;

                // Update access order
                let mut access_order = self.access_order.write().await;
                if let Some(pos) = access_order.iter().position(|k| k == key) {
                    access_order.remove(pos);
                }
                access_order.push_back(key.clone());

                stats.hits += 1;
                stats.hit_rate = stats.hits as f64 / (stats.hits + stats.misses) as f64;

                Some(entry.value.clone())
            }
        } else {
            stats.misses += 1;
            stats.hit_rate = stats.hits as f64 / (stats.hits + stats.misses) as f64;
            None
        }
    }

    /// Put value in cache
    pub async fn put(&self, key: K, value: V) -> ToadStoolResult<()> {
        self.put_with_ttl(key, value, self.config.default_ttl).await
    }

    /// Put value in cache with custom TTL
    pub async fn put_with_ttl(&self, key: K, value: V, ttl: Duration) -> ToadStoolResult<()> {
        let mut entries = self.entries.write().await;
        let mut access_order = self.access_order.write().await;
        let mut stats = self.stats.write().await;
        let now = Instant::now();

        // Check if we need to evict
        if entries.len() >= self.config.max_size {
            self.evict_lru(&mut entries, &mut access_order).await;
        }

        // Add new entry
        entries.insert(
            key.clone(),
            CacheEntry {
                value,
                expires_at: now + ttl,
                access_count: 1,
                last_accessed: now,
            },
        );

        access_order.push_back(key);
        stats.current_size = entries.len();

        Ok(())
    }

    /// Evict least recently used entry
    async fn evict_lru(
        &self,
        entries: &mut HashMap<K, CacheEntry<V>>,
        access_order: &mut VecDeque<K>,
    ) {
        while let Some(key) = access_order.pop_front() {
            if entries.remove(&key).is_some() {
                break;
            }
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> CacheStats {
        self.stats.read().await.clone()
    }

    /// Start cleanup task
    pub async fn start_cleanup_task(&self) {
        let entries = Arc::clone(&self.entries);
        let access_order = Arc::clone(&self.access_order);
        let stats = Arc::clone(&self.stats);
        let cleanup_interval = self.config.cleanup_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(cleanup_interval);

            loop {
                interval.tick().await;

                let mut entries = entries.write().await;
                let mut access_order = access_order.write().await;
                let mut stats = stats.write().await;
                let now = Instant::now();

                // Remove expired entries
                entries.retain(|key, entry| {
                    let expired = now > entry.expires_at;
                    if expired {
                        // Remove from access order
                        if let Some(pos) = access_order.iter().position(|k| k == key) {
                            access_order.remove(pos);
                        }
                    }
                    !expired
                });

                stats.current_size = entries.len();
            }
        });
    }
}

/// Async operation batcher
pub struct AsyncBatcher<T, R> {
    /// Configuration
    config: AsyncOptimizationConfig,
    /// Pending operations
    pending: Arc<RwLock<Vec<BatchItem<T, R>>>>,
    /// Batch processor
    processor: Arc<dyn Fn(Vec<T>) -> futures::future::BoxFuture<'static, Vec<R>> + Send + Sync>,
    /// Semaphore for concurrency control
    semaphore: Arc<Semaphore>,
}

/// Batch item
struct BatchItem<T, R> {
    /// Input
    input: T,
    /// Response sender
    response_sender: tokio::sync::oneshot::Sender<R>,
}

impl<T, R> AsyncBatcher<T, R>
where
    T: Send + Clone + Sync + 'static,
    R: Send + 'static,
{
    /// Create new async batcher
    pub fn new<F>(config: AsyncOptimizationConfig, processor: F) -> Self
    where
        F: Fn(Vec<T>) -> futures::future::BoxFuture<'static, Vec<R>> + Send + Sync + 'static,
    {
        Self {
            config: config.clone(),
            pending: Arc::new(RwLock::new(Vec::new())),
            processor: Arc::new(processor),
            semaphore: Arc::new(Semaphore::new(config.concurrency_limit)),
        }
    }

    /// Submit operation for batching
    pub async fn submit(&self, input: T) -> ToadStoolResult<R> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        {
            let mut pending = self.pending.write().await;
            if pending.len() >= self.config.queue_size_limit {
                return Err(ToadStoolError::resource("Batch queue full".to_string()));
            }

            pending.push(BatchItem {
                input,
                response_sender: tx,
            });

            // Check if we should process batch
            if pending.len() >= self.config.batch_size {
                self.process_batch().await;
            }
        }

        // Wait for response
        rx.await
            .map_err(|_| ToadStoolError::runtime("Batch operation cancelled".to_string()))
    }

    /// Process current batch
    async fn process_batch(&self) {
        let Ok(_permit) = self.semaphore.acquire().await else {
            tracing::error!("Failed to acquire semaphore permit for batch processing");
            return;
        };

        let batch = {
            let mut pending = self.pending.write().await;
            if pending.is_empty() {
                return;
            }

            let batch_size = pending.len().min(self.config.batch_size);
            pending.drain(..batch_size).collect::<Vec<_>>()
        };

        if batch.is_empty() {
            return;
        }

        let inputs: Vec<T> = batch.iter().map(|item| &item.input).cloned().collect();
        let results = (self.processor)(inputs).await;

        // Send results back
        for (item, result) in batch.into_iter().zip(results) {
            let _ = item.response_sender.send(result);
        }
    }

    /// Start batch processing task
    pub async fn start_batch_task(&self) {
        let pending = Arc::clone(&self.pending);
        let processor = Arc::clone(&self.processor);
        let semaphore = Arc::clone(&self.semaphore);
        let batch_timeout = self.config.batch_timeout;
        let batch_size = self.config.batch_size;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(batch_timeout);

            loop {
                interval.tick().await;

                let Ok(_permit) = semaphore.acquire().await else {
                    tracing::error!("Failed to acquire semaphore permit for batch timer");
                    continue;
                };

                let batch = {
                    let mut pending = pending.write().await;
                    if pending.is_empty() {
                        continue;
                    }

                    let batch_size = pending.len().min(batch_size);
                    pending.drain(..batch_size).collect::<Vec<_>>()
                };

                if batch.is_empty() {
                    continue;
                }

                let inputs: Vec<T> = batch.iter().map(|item| item.input.clone()).collect();
                let results = processor(inputs).await;

                // Send results back
                for (item, result) in batch.into_iter().zip(results) {
                    let _ = item.response_sender.send(result);
                }
            }
        });
    }
}

/// Performance hardening manager
pub struct PerformanceHardeningManager {
    /// Configuration
    config: PerformanceHardeningConfig,
    /// Optimized resource monitor
    resource_monitor: Arc<OptimizedResourceMonitor>,
    /// Memory pools
    memory_pools: Arc<RwLock<HashMap<String, Arc<dyn std::any::Any + Send + Sync>>>>,
    /// Intelligent caches
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
