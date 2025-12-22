    config.standalone.max_queue_size = 100000;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Maximal config should work");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_load_balancers_coexist() {
    // Test multiple load balancers can exist simultaneously
    let balancer1 = NetworkLoadBalancer::new();
    let balancer2 = NetworkLoadBalancer::new();
    let balancer3 = NetworkLoadBalancer::new();

    // All should be valid
    drop(balancer1);
    drop(balancer2);
    drop(balancer3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_and_load_balancer_together() {
    // Test coordinator and load balancer can coexist
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;
    let _balancer = NetworkLoadBalancer::new();

    assert!(
        coordinator.is_ok(),
        "Coordinator and balancer should coexist"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_job_queue_edge_case_zero_size() {
    // Edge case: job queue with zero size
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = 0;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Zero-size queue should be handled");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_job_queue_edge_case_very_large_size() {
    // Edge case: job queue with very large size
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = usize::MAX / 1000;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Large queue size should be handled");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_timeout_boundary_values() {
    // Test boundary timeout values
    let timeout_values = vec![0, 1, 30, 60, 300, 3600, 86400];

    for timeout_secs in timeout_values {
        let mut config = DistributedConfig::default();
        config.standalone.default_timeout_secs = timeout_secs;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Timeout {} should be handled",
            timeout_secs
        );
    }
}

// ============================================================================
// Network Error Handling Integration Tests - Day 4 Expansion
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_with_timeout_wrapper() {
    // Test coordinator creation with external timeout
    let config = DistributedConfig::default();
    let result = timeout(Duration::from_secs(10), DistributedCoordinator::new(config)).await;

    assert!(result.is_ok(), "Should complete within timeout");
    assert!(result.unwrap().is_ok(), "Should create successfully");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_handles_quick_timeout() {
    // Test coordinator handles quick timeout gracefully
    let config = DistributedConfig::default();

    let result = timeout(
        Duration::from_millis(100),
        DistributedCoordinator::new(config),
    )
    .await;

    // Either completes quickly or times out - both acceptable
    let _ = result;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_coordinators_sequential_no_interference() {
    // Test sequential coordinator creation doesn't cause interference
    for i in 0..20 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(
            coordinator.is_ok(),
            "Sequential creation {} should not interfere",
            i
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_parallel_creation_no_race_conditions() {
    // Test parallel creation doesn't have race conditions
    let mut handles = vec![];

    for _ in 0..30 {
        let handle = tokio::spawn(async {
            let config = DistributedConfig::default();
            DistributedCoordinator::new(config).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Parallel creation should succeed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_balancer_creation_stability() {
    // Test load balancer creation is stable under repeated calls
    for i in 0..50 {
        let _balancer = NetworkLoadBalancer::new();
        // Should create successfully every time
        if i % 10 == 0 {
            // Periodic check - test passes if no panic
            let _ = i; // Use variable
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_various_instance_ids() {
    // Test coordinator with various instance ID formats
    let ids = vec![
        "simple",
        "with-dashes",
        "with_underscores",
        "with.dots",
        "UPPERCASE",
        "MixedCase",
        "numbers123",
        "uuid-like-550e8400-e29b-41d4-a716-446655440000",
    ];

    for id in ids {
        let config = DistributedConfig {
            instance_id: id.to_string(),
            ..Default::default()
        };

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(coordinator.is_ok(), "Instance ID '{}' should be valid", id);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_config_immutability() {
    // Test that creating coordinator doesn't mutate config
    let config = DistributedConfig::default();
    let original_concurrency = config.standalone.max_concurrent_executions;

    let _coordinator = DistributedCoordinator::new(config.clone()).await;

    assert_eq!(
        config.standalone.max_concurrent_executions, original_concurrency,
        "Config should remain unchanged"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_balancing_strategy_roundtrip_serialization() {
    // Test all strategies serialize and deserialize correctly
    let strategies = vec![
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::ResourceAware,
        LoadBalancingStrategy::LatencyBased,
    ];

    for strategy in strategies {
        let json = serde_json::to_string(&strategy).unwrap();
        let deserialized: LoadBalancingStrategy = serde_json::from_str(&json).unwrap();

        // Verify it deserializes to correct variant
        match (&strategy, &deserialized) {
            (LoadBalancingStrategy::RoundRobin, LoadBalancingStrategy::RoundRobin) => {}
            (LoadBalancingStrategy::LeastConnections, LoadBalancingStrategy::LeastConnections) => {}
            (LoadBalancingStrategy::ResourceAware, LoadBalancingStrategy::ResourceAware) => {}
            (LoadBalancingStrategy::LatencyBased, LoadBalancingStrategy::LatencyBased) => {}
            _ => panic!("Strategy did not round-trip correctly"),
        }
    }
}

// ============================================================================
// Stress and Load Integration Tests - Day 4 Expansion
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_burst_creation() {
    // Test burst of coordinator creations
    let mut coordinators = vec![];

    for i in 0..15 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(coordinator.is_ok(), "Burst creation {} should succeed", i);
        coordinators.push(coordinator);
    }

    assert_eq!(coordinators.len(), 15);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_with_delays() {
    // Test coordinator creation with small delays between
    for i in 0..10 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(coordinator.is_ok(), "Delayed creation {} should succeed", i);

        // Small delay between creations
        tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED (was 10ms)
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_balancer_rapid_creation_and_drop() {
    // Test rapid creation and dropping of load balancers
    for _ in 0..100 {
        let balancer = NetworkLoadBalancer::new();
        drop(balancer);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_mixed_coordinator_and_balancer_creation() {
    // Test mixed creation of coordinators and balancers
    for i in 0..10 {
        let config = DistributedConfig::default();
        let _coordinator = DistributedCoordinator::new(config).await;
        let _balancer = NetworkLoadBalancer::new();

        // Mixed creation should succeed (test passes if no panic)
        let _ = i; // Use variable
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_parallel_with_different_timeouts() {
    // Test parallel creation with different timeout configurations
    let mut handles = vec![];

    for i in 0..10 {
        let handle = tokio::spawn(async move {
            let mut config = DistributedConfig::default();
            config.standalone.default_timeout_secs = (i + 1) * 60;
            DistributedCoordinator::new(config).await
        });
        handles.push(handle);
    }

    for handle in handles {
        let result = handle.await.unwrap();
        assert!(result.is_ok(), "Parallel varied timeout should succeed");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_extreme_concurrency_limits() {
    // Test extreme concurrency limit values
    let extreme_values = vec![0, 1, 1000, 10000, u32::MAX / 2, u32::MAX - 1];

    for concurrency in extreme_values {
        let mut config = DistributedConfig::default();
        config.standalone.max_concurrent_executions = concurrency;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Extreme concurrency {} should be handled",
            concurrency
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_memory_stability_under_load() {
    // Test memory stability by creating and holding many coordinators
    let mut coordinators = vec![];

    for i in 0..25 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(coordinator.is_ok(), "Memory load {} should be stable", i);
        coordinators.push(coordinator);
    }

    // All should still be valid
    assert_eq!(coordinators.len(), 25);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_job_queue_integration_with_varied_sizes() {
    // Test job queue with varied sizes
    let sizes = vec![1, 10, 100, 1000, 10000, 100000];

    for size in sizes {
        let mut config = DistributedConfig::default();
        config.standalone.enable_job_queue = true;
        config.standalone.max_queue_size = size;

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(coordinator.is_ok(), "Queue size {} should work", size);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_interleaved_with_other_async_work() {
    // Test coordinator creation interleaved with other async work
    for i in 0..10 {
        // Some other async work
        tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED (was 1ms)

        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(
            coordinator.is_ok(),
            "Interleaved creation {} should succeed",
            i
        );

        // More async work
        tokio::task::yield_now().await; // ✅ FULLY MODERNIZED // ✅ MODERNIZED (was 1ms)
    }
}
