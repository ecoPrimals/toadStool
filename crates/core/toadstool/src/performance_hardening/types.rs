// SPDX-License-Identifier: AGPL-3.0-or-later
//! Performance hardening types and configurations
//!
//! This module contains all configuration structs and statistics types
//! used by the performance hardening system.

use serde::{Deserialize, Serialize};
use std::time::Duration;
use toadstool_common::constants::timeouts;

const DEFAULT_CLEANUP_INTERVAL_SECS: u64 = 60;
const DEFAULT_AGGREGATION_WINDOW_SECS: u64 = 60;
const DEFAULT_BASE_SAMPLING_INTERVAL_MS: u64 = 100;
const DEFAULT_BATCH_TIMEOUT_MS: u64 = 100;
const DEFAULT_CONNECTION_TIMEOUT_SECS: u64 = 30;
const DEFAULT_IDLE_TIMEOUT_SECS: u64 = 300;
const DEFAULT_POOL_HEALTH_CHECK_INTERVAL_SECS: u64 = 60;

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
            base_sampling_interval: Duration::from_millis(DEFAULT_BASE_SAMPLING_INTERVAL_MS),
            adaptive_sampling: true,
            high_load_multiplier: 0.5,
            low_load_multiplier: 2.0,
            batch_size: 10,
            aggregation_window: Duration::from_secs(DEFAULT_AGGREGATION_WINDOW_SECS),
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
            default_ttl: timeouts::DEFAULT_CACHE_TTL,
            cleanup_interval: Duration::from_secs(DEFAULT_CLEANUP_INTERVAL_SECS),
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
            batch_timeout: Duration::from_millis(DEFAULT_BATCH_TIMEOUT_MS),
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
            connection_timeout: Duration::from_secs(DEFAULT_CONNECTION_TIMEOUT_SECS),
            idle_timeout: Duration::from_secs(DEFAULT_IDLE_TIMEOUT_SECS),
            health_check_interval: Duration::from_secs(DEFAULT_POOL_HEALTH_CHECK_INTERVAL_SECS),
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

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_performance_hardening_config_construction() {
        let config = PerformanceHardeningConfig {
            enable_optimized_monitoring: false,
            enable_memory_pools: false,
            enable_caching: true,
            enable_async_optimization: false,
            enable_connection_pooling: false,
            monitoring_config: OptimizedMonitoringConfig::default(),
            memory_pool_config: MemoryPoolConfig::default(),
            caching_config: CachingConfig::default(),
            async_config: AsyncOptimizationConfig::default(),
            connection_pool_config: PerformanceConnectionPoolConfig::default(),
        };
        assert!(!config.enable_optimized_monitoring);
        assert!(config.enable_caching);
    }

    #[test]
    fn test_performance_hardening_config_serialization() {
        let config = PerformanceHardeningConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: PerformanceHardeningConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.enable_caching, deserialized.enable_caching);
    }

    #[test]
    fn test_optimized_monitoring_config_default() {
        let config = OptimizedMonitoringConfig::default();
        assert_eq!(config.base_sampling_interval, Duration::from_millis(100));
        assert!(config.adaptive_sampling);
        assert!((config.high_load_multiplier - 0.5).abs() < 1e-10);
        assert!((config.low_load_multiplier - 2.0).abs() < 1e-10);
        assert_eq!(config.batch_size, 10);
    }

    #[test]
    fn test_memory_pool_config_default() {
        let config = MemoryPoolConfig::default();
        assert_eq!(config.initial_size, 100);
        assert_eq!(config.max_size, 1000);
        assert!((config.growth_factor - 1.5).abs() < 1e-10);
        assert!((config.shrink_threshold - 0.3).abs() < 1e-10);
        assert_eq!(config.cleanup_interval, Duration::from_secs(60));
    }

    #[test]
    fn test_memory_pool_config_construction() {
        let config = MemoryPoolConfig {
            initial_size: 10,
            max_size: 50,
            growth_factor: 2.0,
            shrink_threshold: 0.2,
            cleanup_interval: Duration::from_secs(30),
        };
        assert_eq!(config.initial_size, 10);
        assert_eq!(config.max_size, 50);
        assert!((config.growth_factor - 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_caching_config_default() {
        let config = CachingConfig::default();
        assert_eq!(config.max_size, 1000);
        assert_eq!(config.default_ttl, Duration::from_secs(300));
        assert_eq!(config.cleanup_interval, Duration::from_secs(60));
        assert!((config.hit_rate_threshold - 0.8).abs() < 1e-10);
    }

    #[test]
    fn test_caching_config_serialization() {
        let config = CachingConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let _deserialized: CachingConfig = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_cache_stats_construction() {
        let stats = CacheStats {
            current_size: 5,
            hits: 10,
            misses: 2,
            hit_rate: 0.833,
            evictions: 1,
        };
        assert_eq!(stats.current_size, 5);
        assert_eq!(stats.hits, 10);
        assert_eq!(stats.evictions, 1);
    }

    #[test]
    fn test_cache_stats_serialization() {
        let stats = CacheStats {
            current_size: 0,
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            evictions: 0,
        };
        let json = serde_json::to_string(&stats).unwrap();
        let deserialized: CacheStats = serde_json::from_str(&json).unwrap();
        assert_eq!(stats.current_size, deserialized.current_size);
    }

    #[test]
    fn test_pool_stats_default_values() {
        let stats = PoolStats {
            current_size: 10,
            in_use: 3,
            available: 7,
            total_allocations: 100,
            total_deallocations: 97,
            hit_rate: 0.9,
        };
        assert_eq!(stats.in_use + stats.available, stats.current_size);
    }

    #[test]
    fn test_async_optimization_config_default() {
        let config = AsyncOptimizationConfig::default();
        assert_eq!(config.batch_size, 50);
        assert_eq!(config.batch_timeout, Duration::from_millis(100));
        assert_eq!(config.concurrency_limit, 100);
        assert_eq!(config.queue_size_limit, 1000);
    }

    #[test]
    fn test_performance_connection_pool_config_default() {
        let config = PerformanceConnectionPoolConfig::default();
        assert_eq!(config.initial_size, 10);
        assert_eq!(config.max_size, 100);
        assert_eq!(config.connection_timeout, Duration::from_secs(30));
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
        assert_eq!(config.health_check_interval, Duration::from_secs(60));
    }

    #[test]
    fn test_aggregated_metrics_construction() {
        let metrics = AggregatedMetrics {
            cpu_usage: 75.5,
            memory_usage: 1_000_000,
            active_connections: 5,
            request_rate: 10.0,
            avg_response_time: 50.0,
        };
        assert!((metrics.cpu_usage - 75.5).abs() < 1e-10);
        assert_eq!(metrics.memory_usage, 1_000_000);
        assert_eq!(metrics.active_connections, 5);
    }

    #[test]
    fn test_types_debug_impl() {
        let config = PerformanceHardeningConfig::default();
        let debug_str = format!("{config:?}");
        assert!(debug_str.contains("PerformanceHardeningConfig"));

        let stats = CacheStats {
            current_size: 0,
            hits: 0,
            misses: 0,
            hit_rate: 0.0,
            evictions: 0,
        };
        let debug_str = format!("{stats:?}");
        assert!(debug_str.contains("CacheStats"));
    }
}
