//! Integration tests for production and performance hardening modules working together
//!
//! These tests verify that production hardening (circuit breakers, leak detection, memory pressure)
//! and performance hardening (monitoring, caching, pooling) work correctly when integrated.

use std::sync::Arc;
use std::time::{Duration, Instant};
use toadstool::performance_hardening::*;
use toadstool::production_hardening::*;
use toadstool::resources::{
    CpuMetrics, MemoryMetrics, NetworkMetrics, ResourceRequirements, RuntimeMetrics,
    StorageMetrics, TimingMetrics,
};
use uuid::Uuid;

// ============================================================================
// Combined Manager Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_production_and_performance_managers_together() {
    let prod_config = ProductionHardeningConfig::default();
    let perf_config = PerformanceHardeningConfig::default();

    let prod_manager = ProductionHardeningManager::new(prod_config);
    let perf_manager = PerformanceHardeningManager::new(perf_config);

    // Initialize both
    prod_manager
        .initialize()
        .await
        .expect("Production hardening should initialize");
    perf_manager
        .initialize()
        .await
        .expect("Performance hardening should initialize");

    // Both should be operational
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_with_resource_monitoring() {
    let prod_config = ProductionHardeningConfig::default();
    let perf_config = PerformanceHardeningConfig::default();

    let prod_manager = ProductionHardeningManager::new(prod_config);
    let perf_manager = PerformanceHardeningManager::new(perf_config);

    // Get circuit breaker
    let breaker = prod_manager.get_circuit_breaker("test-service").await;

    // Get resource monitor
    let monitor = perf_manager.get_resource_monitor();

    // Verify both are operational
    let state = breaker.get_state().await;
    assert_eq!(state, CircuitState::Closed);

    let interval = monitor.get_sampling_interval().await;
    assert!(interval.as_millis() > 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_with_leak_detection() {
    let prod_config = ProductionHardeningConfig::default();
    let perf_config = PerformanceHardeningConfig::default();

    let prod_manager = ProductionHardeningManager::new(prod_config);
    let perf_manager = PerformanceHardeningManager::new(perf_config);

    prod_manager.initialize().await.expect("Should initialize");
    perf_manager.initialize().await.expect("Should initialize");

    // Create memory pool
    let pool = perf_manager
        .create_memory_pool::<Vec<u8>, _>("integration-pool", || Vec::with_capacity(1024))
        .await
        .expect("Should create pool");

    // Track resource allocation for leak detection
    let resource_id = Uuid::new_v4();
    let allocation = ResourceAllocation {
        id: resource_id,
        resource_type: "MemoryPool".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "integration-test".to_string(),
        last_accessed: Instant::now(),
    };

    prod_manager.track_resource(allocation).await;

    // Use the pool
    let obj = pool.get().await;
    assert!(obj.get().is_some());

    // Update resource access
    prod_manager.update_resource_access(resource_id).await;

    // Cleanup
    prod_manager.remove_resource(resource_id).await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cache_with_memory_pressure() {
    let prod_config = ProductionHardeningConfig::default();
    let perf_config = PerformanceHardeningConfig::default();

    let prod_manager = ProductionHardeningManager::new(prod_config);
    let perf_manager = PerformanceHardeningManager::new(perf_config);

    // Create cache
    let cache = perf_manager
        .create_cache::<String, i32>("pressure-test-cache")
        .await
        .expect("Should create cache");

    // Populate cache
    for i in 0..100 {
        cache
            .put(format!("key-{}", i), i)
            .await
            .expect("Should insert");
    }

    // Simulate memory pressure
    prod_manager.update_memory_usage(1000, 900).await; // 90% usage

    // Cache should still work
    let value = cache.get(&"key-50".to_string()).await;
    assert_eq!(value, Some(50));
}

// ============================================================================
// Circuit Breaker Async Execution Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_successful_async_operation() {
    let config = CircuitBreakerConfig::default();
    let breaker = CircuitBreaker::new("async-test-service".to_string(), config);

    // Execute successful async operation
    let result = breaker
        .execute(async { Ok::<i32, std::io::Error>(42) })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 42);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_failed_async_operation() {
    let config = CircuitBreakerConfig::default();
    let breaker = CircuitBreaker::new("failing-service".to_string(), config);

    // Execute failing async operation
    let result = breaker
        .execute(async { Err::<i32, std::io::Error>(std::io::Error::other("test error")) })
        .await;

    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_opens_after_threshold() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        success_threshold: 2,
        timeout: Duration::from_secs(60),
        rolling_window: Duration::from_secs(60),
        half_open_max_requests: 2,
    };
    let breaker = Arc::new(CircuitBreaker::new("threshold-test".to_string(), config));

    // Fail multiple times to open circuit
    for _ in 0..3 {
        let _ = breaker
            .execute(async { Err::<i32, std::io::Error>(std::io::Error::other("fail")) })
            .await;
    }

    // Circuit should be open now
    let state = breaker.get_state().await;
    assert_eq!(state, CircuitState::Open);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_concurrent_executions() {
    let config = CircuitBreakerConfig::default();
    let breaker = Arc::new(CircuitBreaker::new("concurrent-test".to_string(), config));

    // Execute multiple operations concurrently
    let mut handles = vec![];
    for i in 0..10 {
        let breaker_clone = Arc::clone(&breaker);
        let handle = tokio::spawn(async move {
            breaker_clone
                .execute(async move {
                    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
                    Ok::<i32, std::io::Error>(i)
                })
                .await
        });
        handles.push(handle);
    }

    // Wait for all to complete
    let mut success_count = 0;
    for handle in handles {
        if let Ok(result) = handle.await {
            if result.is_ok() {
                success_count += 1;
            }
        }
    }

    assert_eq!(success_count, 10);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_recovery_scenario() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        success_threshold: 2,
        timeout: Duration::from_millis(100),
        rolling_window: Duration::from_secs(60),
        half_open_max_requests: 2,
    };
    let breaker = Arc::new(CircuitBreaker::new("recovery-test".to_string(), config));

    // Fail to open circuit
    for _ in 0..2 {
        let _ = breaker
            .execute(async { Err::<i32, std::io::Error>(std::io::Error::other("fail")) })
            .await;
    }

    assert_eq!(breaker.get_state().await, CircuitState::Open);

    // ✅ INTENTIONAL DELAY: Wait for circuit breaker timeout (necessary for state transition)
    tokio::time::sleep(Duration::from_millis(150)).await;

    // Should transition to half-open and allow requests
    let result = breaker
        .execute(async { Ok::<i32, std::io::Error>(1) })
        .await;

    // First success in half-open
    assert!(result.is_ok());

    // Another success to close circuit
    let result = breaker
        .execute(async { Ok::<i32, std::io::Error>(2) })
        .await;

    assert!(result.is_ok());
}

// ============================================================================
// Resource Monitoring with Circuit Breakers
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_monitoring_during_circuit_breaker_operation() {
    let perf_config = PerformanceHardeningConfig::default();
    let perf_manager = PerformanceHardeningManager::new(perf_config);
    perf_manager.initialize().await.expect("Should initialize");

    let monitor = perf_manager.get_resource_monitor();

    let circuit_config = CircuitBreakerConfig::default();
    let breaker = CircuitBreaker::new("monitored-service".to_string(), circuit_config);

    // Add some metrics
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 50.0,
            cores_used: 2.0,
            cpu_time_seconds: 10.0,
        },
        memory: MemoryMetrics {
            usage_percent: 50.0,
            used_bytes: 4 * 1024 * 1024 * 1024, // 4GB
            peak_bytes: 5 * 1024 * 1024 * 1024, // 5GB
        },
        storage: StorageMetrics {
            usage_percent: 30.0,
            used_bytes: 100 * 1024 * 1024,
            bytes_read: 1000,
            bytes_written: 500,
        },
        network: NetworkMetrics {
            bytes_sent: 1000,
            bytes_received: 2000,
            packets_sent: 10,
            packets_received: 20,
        },
        gpu: None,
        timing: TimingMetrics::default(),
    };

    monitor.add_sample("test-workload", metrics).await;

    // Execute operation through circuit breaker
    let result = breaker
        .execute(async { Ok::<i32, std::io::Error>(100) })
        .await;

    assert!(result.is_ok());

    // Check aggregated metrics
    let agg = monitor.get_aggregated_metrics("test-workload").await;
    assert!(agg.is_some());
}

// ============================================================================
// Memory Pool with Resource Tracking
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pool_allocation_tracking() {
    let perf_config = PerformanceHardeningConfig::default();
    let perf_manager = PerformanceHardeningManager::new(perf_config);

    let prod_config = ProductionHardeningConfig::default();
    let prod_manager = ProductionHardeningManager::new(prod_config);
    prod_manager.initialize().await.expect("Should initialize");

    // Create pool
    let pool = perf_manager
        .create_memory_pool::<Vec<u8>, _>("tracked-pool", || Vec::with_capacity(2048))
        .await
        .expect("Should create pool");

    // Track multiple allocations
    let mut resource_ids = vec![];
    for i in 0..5 {
        let id = Uuid::new_v4();
        let allocation = ResourceAllocation {
            id,
            resource_type: format!("PooledBuffer-{}", i),
            allocated_at: Instant::now(),
            requirements: ResourceRequirements::default(),
            owner: "tracking-test".to_string(),
            last_accessed: Instant::now(),
        };
        prod_manager.track_resource(allocation).await;
        resource_ids.push(id);
    }

    // Use pool objects
    let obj1 = pool.get().await;
    let obj2 = pool.get().await;
    assert!(obj1.get().is_some());
    assert!(obj2.get().is_some());

    // Update access times
    for id in &resource_ids {
        prod_manager.update_resource_access(*id).await;
    }

    // Cleanup
    for id in resource_ids {
        prod_manager.remove_resource(id).await;
    }
}

// ============================================================================
// Cache with Circuit Breaker Protection
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_cache_operations_with_circuit_breaker() {
    let perf_config = PerformanceHardeningConfig::default();
    let perf_manager = PerformanceHardeningManager::new(perf_config);

    let cache = perf_manager
        .create_cache::<String, String>("protected-cache")
        .await
        .expect("Should create cache");

    let breaker_config = CircuitBreakerConfig::default();
    let breaker = CircuitBreaker::new("cache-service".to_string(), breaker_config);

    // Populate cache through circuit breaker
    for i in 0..10 {
        let key = format!("key-{}", i);
        let value = format!("value-{}", i);
        let cache_clone = cache.clone();
        let k = key.clone();
        let v = value.clone();

        let result = breaker
            .execute(async move {
                cache_clone
                    .put(k, v)
                    .await
                    .map_err(|e| std::io::Error::other(e.to_string()))
            })
            .await;

        assert!(result.is_ok());
    }

    // Retrieve through circuit breaker
    let cache_clone = cache.clone();
    let result = breaker
        .execute(
            async move { Ok::<_, std::io::Error>(cache_clone.get(&"key-5".to_string()).await) },
        )
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Some("value-5".to_string()));
}

// ============================================================================
// Full Integration Scenario
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_full_hardening_integration_scenario() {
    // Setup both managers
    let prod_config = ProductionHardeningConfig::default();
    let perf_config = PerformanceHardeningConfig::default();

    let prod_manager = Arc::new(ProductionHardeningManager::new(prod_config));
    let perf_manager = Arc::new(PerformanceHardeningManager::new(perf_config));

    prod_manager
        .initialize()
        .await
        .expect("Should initialize prod");
    perf_manager
        .initialize()
        .await
        .expect("Should initialize perf");

    // Create infrastructure
    let cache = perf_manager
        .create_cache::<String, i32>("integration-cache")
        .await
        .expect("Should create cache");

    let pool = perf_manager
        .create_memory_pool::<Vec<u8>, _>("integration-pool", || Vec::with_capacity(4096))
        .await
        .expect("Should create pool");

    let breaker = prod_manager
        .get_circuit_breaker("integration-service")
        .await;
    let monitor = perf_manager.get_resource_monitor();

    // Scenario: Process multiple requests with monitoring, caching, and circuit protection
    for i in 0..20 {
        // Check cache first
        let cache_key = format!("request-{}", i);
        let cached = cache.get(&cache_key).await;

        if cached.is_none() {
            // Cache miss - process through circuit breaker
            let breaker_clone = breaker.clone();
            let cache_clone = cache.clone();
            let key = cache_key.clone();

            let result = breaker_clone
                .execute(async move {
                    // Simulate processing
                    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED

                    // Store in cache
                    cache_clone
                        .put(key, i)
                        .await
                        .map_err(|e| std::io::Error::other(e.to_string()))?;

                    Ok::<i32, std::io::Error>(i)
                })
                .await;

            assert!(result.is_ok());
        }

        // Track resource usage
        if i % 5 == 0 {
            let metrics = RuntimeMetrics {
                cpu: CpuMetrics {
                    usage_percent: 60.0,
                    cores_used: 4.0,
                    cpu_time_seconds: 20.0,
                },
                memory: MemoryMetrics {
                    usage_percent: 50.0,
                    used_bytes: 8 * 1024 * 1024 * 1024,
                    peak_bytes: 10 * 1024 * 1024 * 1024,
                },
                storage: StorageMetrics {
                    usage_percent: 40.0,
                    used_bytes: 200 * 1024 * 1024,
                    bytes_read: 2000,
                    bytes_written: 1000,
                },
                network: NetworkMetrics {
                    bytes_sent: 5000,
                    bytes_received: 10000,
                    packets_sent: 50,
                    packets_received: 100,
                },
                gpu: None,
                timing: TimingMetrics::default(),
            };
            monitor
                .add_sample(&format!("workload-{}", i), metrics)
                .await;
        }

        // Use pool for buffer allocation
        let _buffer = pool.get().await;
    }

    // Verify everything worked
    let stats = cache.get_stats().await;
    assert!(stats.hits > 0 || stats.misses > 0);

    let pool_stats = pool.get_stats().await;
    assert!(pool_stats.total_allocations > 0);

    let breaker_state = breaker.get_state().await;
    assert_eq!(breaker_state, CircuitState::Closed);
}
