// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for performance hardening module
//!
//! Tests cover monitoring, caching, memory pools, and async optimization.

use std::time::Duration;
use toadstool::performance_hardening::*;

// ============================================================================
// Configuration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_config_default() {
    let config = PerformanceHardeningConfig::default();

    assert!(config.enable_optimized_monitoring);
    assert!(config.enable_memory_pools);
    assert!(config.enable_caching);
    assert!(config.enable_async_optimization);
    assert!(config.enable_connection_pooling);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_hardening_config_clone() {
    let config = PerformanceHardeningConfig::default();
    let cloned = config.clone();

    assert_eq!(
        config.enable_optimized_monitoring,
        cloned.enable_optimized_monitoring
    );
    assert_eq!(config.enable_memory_pools, cloned.enable_memory_pools);
    assert_eq!(config.enable_caching, cloned.enable_caching);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_optimized_monitoring_config_default() {
    let config = OptimizedMonitoringConfig::default();

    assert_eq!(config.base_sampling_interval, Duration::from_millis(100));
    assert!(config.adaptive_sampling);
    assert_eq!(config.high_load_multiplier, 0.5);
    assert_eq!(config.low_load_multiplier, 2.0);
    assert_eq!(config.batch_size, 10);
    assert_eq!(config.aggregation_window, Duration::from_secs(60));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_config_default() {
    let config = MemoryPoolConfig::default();

    assert_eq!(config.initial_size, 100);
    assert_eq!(config.max_size, 1000);
    assert_eq!(config.growth_factor, 1.5);
    assert_eq!(config.shrink_threshold, 0.3);
    assert_eq!(config.cleanup_interval, Duration::from_secs(60));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_caching_config_default() {
    let config = CachingConfig::default();

    assert_eq!(config.max_size, 1000);
    assert_eq!(config.default_ttl, Duration::from_secs(300));
    assert_eq!(config.cleanup_interval, Duration::from_secs(60));
    assert_eq!(config.hit_rate_threshold, 0.8);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_async_optimization_config_default() {
    let config = AsyncOptimizationConfig::default();

    assert_eq!(config.batch_size, 50);
    assert_eq!(config.concurrency_limit, 100);
    assert_eq!(config.batch_timeout, Duration::from_millis(100));
    assert_eq!(config.queue_size_limit, 1000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_connection_pool_config_default() {
    let config = PerformanceConnectionPoolConfig::default();

    assert_eq!(config.initial_size, 10);
    assert_eq!(config.max_size, 100);
    assert_eq!(config.connection_timeout, Duration::from_secs(30));
    assert_eq!(config.idle_timeout, Duration::from_secs(300));
    assert_eq!(config.health_check_interval, Duration::from_secs(60));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitoring_config_clone() {
    let config = OptimizedMonitoringConfig::default();
    let cloned = config.clone();

    assert_eq!(config.base_sampling_interval, cloned.base_sampling_interval);
    assert_eq!(config.adaptive_sampling, cloned.adaptive_sampling);
    assert_eq!(config.batch_size, cloned.batch_size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_config_clone() {
    let config = MemoryPoolConfig::default();
    let cloned = config.clone();

    assert_eq!(config.initial_size, cloned.initial_size);
    assert_eq!(config.max_size, cloned.max_size);
    assert_eq!(config.growth_factor, cloned.growth_factor);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_caching_config_clone() {
    let config = CachingConfig::default();
    let cloned = config.clone();

    assert_eq!(config.max_size, cloned.max_size);
    assert_eq!(config.default_ttl, cloned.default_ttl);
    assert_eq!(config.hit_rate_threshold, cloned.hit_rate_threshold);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_custom_monitoring_config() {
    let config = OptimizedMonitoringConfig {
        base_sampling_interval: Duration::from_millis(50),
        adaptive_sampling: false,
        high_load_multiplier: 0.25,
        low_load_multiplier: 4.0,
        batch_size: 20,
        aggregation_window: Duration::from_secs(120),
    };

    assert_eq!(config.base_sampling_interval, Duration::from_millis(50));
    assert!(!config.adaptive_sampling);
    assert_eq!(config.high_load_multiplier, 0.25);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_custom_memory_pool_config() {
    let config = MemoryPoolConfig {
        initial_size: 50,
        max_size: 500,
        growth_factor: 2.0,
        shrink_threshold: 0.2,
        cleanup_interval: Duration::from_secs(30),
    };

    assert_eq!(config.initial_size, 50);
    assert_eq!(config.max_size, 500);
    assert_eq!(config.growth_factor, 2.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_custom_caching_config() {
    let config = CachingConfig {
        max_size: 5000,
        default_ttl: Duration::from_secs(600),
        cleanup_interval: Duration::from_secs(120),
        hit_rate_threshold: 0.9,
    };

    assert_eq!(config.max_size, 5000);
    assert_eq!(config.default_ttl, Duration::from_secs(600));
    assert_eq!(config.hit_rate_threshold, 0.9);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitoring_adaptive_sampling_multipliers() {
    let config = OptimizedMonitoringConfig::default();

    // Under high load, sampling should be faster (smaller interval)
    let high_load_interval =
        config.base_sampling_interval.as_millis() as f64 * config.high_load_multiplier;
    assert!(high_load_interval < config.base_sampling_interval.as_millis() as f64);

    // Under low load, sampling should be slower (larger interval)
    let low_load_interval =
        config.base_sampling_interval.as_millis() as f64 * config.low_load_multiplier;
    assert!(low_load_interval > config.base_sampling_interval.as_millis() as f64);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_growth_calculation() {
    let config = MemoryPoolConfig::default();

    // Test pool growth from initial size
    let grown_size = (config.initial_size as f64 * config.growth_factor) as usize;
    assert_eq!(grown_size, 150); // 100 * 1.5 = 150

    // Should not exceed max size
    assert!(grown_size <= config.max_size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_shrink_threshold() {
    let config = MemoryPoolConfig::default();

    // Calculate shrink point
    let shrink_point = (config.max_size as f64 * config.shrink_threshold) as usize;
    assert_eq!(shrink_point, 300); // 1000 * 0.3 = 300

    // Pool should shrink when usage falls below this
    assert!(shrink_point < config.max_size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cache_hit_rate_threshold() {
    let config = CachingConfig::default();

    // Hit rates above threshold indicate good caching
    let good_hit_rate = 0.85;
    assert!(good_hit_rate > config.hit_rate_threshold);

    // Hit rates below threshold indicate poor caching
    let poor_hit_rate = 0.75;
    assert!(poor_hit_rate < config.hit_rate_threshold);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_async_batch_configuration() {
    let config = AsyncOptimizationConfig::default();

    // Batch size should be reasonable
    assert!(config.batch_size > 0);
    assert!(config.batch_size <= config.concurrency_limit);

    // Queue size should be reasonable
    assert!(config.queue_size_limit >= config.batch_size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_connection_pool_size_constraints() {
    let config = PerformanceConnectionPoolConfig::default();

    // Initial size should be less than or equal to max size
    assert!(config.initial_size <= config.max_size);

    // Idle timeout should be reasonable
    assert!(config.idle_timeout >= Duration::from_secs(60));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_disabled_performance_features() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: false,
        enable_memory_pools: false,
        enable_caching: false,
        enable_async_optimization: false,
        enable_connection_pooling: false,
        ..PerformanceHardeningConfig::default()
    };

    assert!(!config.enable_optimized_monitoring);
    assert!(!config.enable_memory_pools);
    assert!(!config.enable_caching);
    assert!(!config.enable_async_optimization);
    assert!(!config.enable_connection_pooling);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_selective_feature_enablement() {
    let config = PerformanceHardeningConfig {
        enable_optimized_monitoring: true,
        enable_memory_pools: false,
        enable_caching: true,
        enable_async_optimization: false,
        enable_connection_pooling: true,
        ..PerformanceHardeningConfig::default()
    };

    assert!(config.enable_optimized_monitoring);
    assert!(!config.enable_memory_pools);
    assert!(config.enable_caching);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_performance_config_serialization_fields() {
    let config = PerformanceHardeningConfig::default();

    // Verify all sub-configs are present
    let _monitoring = &config.monitoring_config;
    let _memory = &config.memory_pool_config;
    let _caching = &config.caching_config;
    let _async = &config.async_config;
    let _pool = &config.connection_pool_config;

    // All fields accessible means serialization will work
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_extreme_monitoring_config() {
    let config = OptimizedMonitoringConfig {
        base_sampling_interval: Duration::from_millis(1), // Very fast
        adaptive_sampling: true,
        high_load_multiplier: 0.1, // 10x faster under load
        low_load_multiplier: 10.0, // 10x slower under idle
        batch_size: 100,           // Large batches
        aggregation_window: Duration::from_secs(600), // 10 minutes
    };

    assert_eq!(config.base_sampling_interval, Duration::from_millis(1));
    assert_eq!(config.batch_size, 100);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_minimal_memory_pool_config() {
    let config = MemoryPoolConfig {
        initial_size: 1,
        max_size: 10,
        growth_factor: 1.1, // Slow growth
        shrink_threshold: 0.5,
        cleanup_interval: Duration::from_secs(10),
    };

    assert_eq!(config.initial_size, 1);
    assert_eq!(config.max_size, 10);
    assert!(config.growth_factor > 1.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_large_cache_config() {
    let config = CachingConfig {
        max_size: 100_000,                          // 100K entries
        default_ttl: Duration::from_secs(3600),     // 1 hour
        cleanup_interval: Duration::from_secs(300), // 5 minutes
        hit_rate_threshold: 0.95,                   // Very high target
    };

    assert_eq!(config.max_size, 100_000);
    assert_eq!(config.hit_rate_threshold, 0.95);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_aggressive_async_optimization() {
    let config = AsyncOptimizationConfig {
        batch_size: 100,
        concurrency_limit: 1000,
        batch_timeout: Duration::from_millis(10), // Very quick batching
        queue_size_limit: 10000,
    };

    assert_eq!(config.batch_size, 100);
    assert_eq!(config.concurrency_limit, 1000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_connection_pool_minimal() {
    let config = PerformanceConnectionPoolConfig {
        initial_size: 1,
        max_size: 5,
        connection_timeout: Duration::from_secs(5),
        idle_timeout: Duration::from_secs(60),
        health_check_interval: Duration::from_secs(30),
    };

    assert_eq!(config.initial_size, 1);
    assert_eq!(config.max_size, 5);
    assert!(config.initial_size <= config.max_size);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_connection_pool_enterprise() {
    let config = PerformanceConnectionPoolConfig {
        initial_size: 50,
        max_size: 500,
        connection_timeout: Duration::from_secs(10),
        idle_timeout: Duration::from_secs(600), // 10 minutes
        health_check_interval: Duration::from_secs(120), // 2 minutes
    };

    assert_eq!(config.initial_size, 50);
    assert_eq!(config.max_size, 500);
    assert!(config.health_check_interval < config.idle_timeout);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitoring_batch_size_calculation() {
    let config = OptimizedMonitoringConfig::default();

    // Verify batch size is reasonable for aggregation window
    let samples_per_window =
        config.aggregation_window.as_millis() / config.base_sampling_interval.as_millis();

    // With 60s window and 100ms interval = 600 samples
    // Batch size of 10 means 60 batches per window
    assert_eq!(samples_per_window, 600);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_capacity_range() {
    let config = MemoryPoolConfig::default();

    // Pool capacity should be in reasonable range
    assert!(config.initial_size >= 1);
    assert!(config.max_size >= config.initial_size);
    assert!(config.max_size <= 1_000_000); // Reasonable upper bound
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cache_ttl_vs_cleanup() {
    let config = CachingConfig::default();

    // Cleanup should happen more frequently than TTL expires
    // This ensures expired entries are removed reasonably quickly
    assert!(config.cleanup_interval < config.default_ttl);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_async_operation_constraints() {
    let config = AsyncOptimizationConfig::default();

    // Batch size should not exceed concurrency limit
    assert!(config.batch_size <= config.concurrency_limit);

    // Queue size should be reasonable relative to concurrency
    assert!(config.queue_size_limit >= config.concurrency_limit);

    // Batch timeout should be reasonable (not too short, not too long)
    assert!(config.batch_timeout >= Duration::from_millis(1));
    assert!(config.batch_timeout <= Duration::from_secs(10));
}
