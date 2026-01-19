//! Performance hardening types and configurations
//!
//! This module contains all configuration structs and statistics types
//! used by the performance hardening system.

use serde::{Deserialize, Serialize};
use std::time::Duration;

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

/// Aggregated metrics for resource monitoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregatedMetrics {
    /// CPU usage percentage
    pub cpu_usage: f64,
    /// Memory usage in bytes
    pub memory_usage: u64,
    /// Active connections
    pub active_connections: usize,
    /// Request rate (requests per second)
    pub request_rate: f64,
    /// Average response time in milliseconds
    pub avg_response_time: f64,
}

/// Memory pool statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolStats {
    /// Current pool size
    pub current_size: usize,
    /// Objects in use
    pub in_use: usize,
    /// Objects available
    pub available: usize,
    /// Total allocations
    pub total_allocations: u64,
    /// Total deallocations
    pub total_deallocations: u64,
    /// Pool hit rate
    pub hit_rate: f64,
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    /// Current cache size
    pub current_size: usize,
    /// Total hits
    pub hits: u64,
    /// Total misses
    pub misses: u64,
    /// Hit rate
    pub hit_rate: f64,
    /// Evictions
    pub evictions: u64,
}
