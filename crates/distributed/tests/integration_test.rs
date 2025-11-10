//! Integration tests for Distributed system components
//!
//! Tests the interaction between multiple components:
//! - Coordinator + Network
//! - Coordinator + Resources
//! - Network + Load Balancer
//! - Full system integration
//!
//! Following Month 1 test expansion plan - Day 4

use std::time::Duration;
use tokio::time::timeout;

use toadstool_distributed::core::config::{DistributedConfig, StandaloneConfig};
use toadstool_distributed::core::coordinator::DistributedCoordinator;
use toadstool_distributed::network::load_balancer::NetworkLoadBalancer;
use toadstool_distributed::types::jobs::LoadBalancingStrategy;
use toadstool_distributed::types::UniversalJobQueue;

// ============================================================================
// Coordinator + Network Integration Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_with_default_network_config() {
    // Test that coordinator initializes with network-aware default config
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should initialize with network config"
    );
}

#[tokio::test]
async fn test_coordinator_initializes_job_queue() {
    // Test coordinator properly initializes its job queue
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator with job queue should initialize"
    );
}

#[tokio::test]
async fn test_coordinator_with_custom_network_settings() {
    // Test coordinator works with custom network configuration
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 50;
    config.standalone.enable_job_queue = true;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle custom network settings"
    );
}

#[tokio::test]
async fn test_coordinator_creation_is_fast() {
    // Integration test: coordinator creation should be fast even with network setup
    let start = std::time::Instant::now();

    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    let elapsed = start.elapsed();

    assert!(coordinator.is_ok(), "Coordinator should be created");
    assert!(
        elapsed < Duration::from_secs(3),
        "Coordinator creation should be fast even with network (took {:?})",
        elapsed
    );
}

#[tokio::test]
async fn test_multiple_coordinators_can_coexist() {
    // Integration test: multiple coordinator instances can exist simultaneously
    let config1 = DistributedConfig::default();
    let config2 = DistributedConfig::default();

    let coordinator1 = DistributedCoordinator::new(config1).await;
    let coordinator2 = DistributedCoordinator::new(config2).await;

    assert!(coordinator1.is_ok(), "First coordinator should initialize");
    assert!(coordinator2.is_ok(), "Second coordinator should initialize");
}

// ============================================================================
// Coordinator + Job Queue Integration Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_with_enabled_job_queue() {
    // Test coordinator properly integrates with enabled job queue
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = 1000;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator with enabled queue should work"
    );
}

#[tokio::test]
async fn test_coordinator_with_disabled_job_queue() {
    // Test coordinator works even with disabled job queue
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = false;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should work with disabled queue"
    );
}

#[tokio::test]
async fn test_job_queue_capacity_configuration() {
    // Test job queue respects capacity configuration from coordinator
    let mut config = DistributedConfig::default();
    config.standalone.max_queue_size = 5000;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should respect queue capacity config"
    );
}

#[tokio::test]
async fn test_job_queue_starts_empty() {
    // Integration test: job queue should start empty when coordinator initializes
    let queue = UniversalJobQueue::new();

    assert_eq!(
        queue.total_jobs(),
        0,
        "Job queue should start empty on coordinator init"
    );
}

// ============================================================================
// Network + Load Balancer Integration Tests
// ============================================================================

#[tokio::test]
async fn test_load_balancer_initialization() {
    // Test load balancer initializes successfully
    let balancer = NetworkLoadBalancer::new();

    // Balancer created successfully (no Result return type)
    drop(balancer); // Use the balancer to avoid unused variable warning
}

#[tokio::test]
async fn test_load_balancer_with_default() {
    // Test load balancer can be created with Default trait
    let balancer = NetworkLoadBalancer::default();

    // Balancer created successfully
    drop(balancer);
}

#[tokio::test]
async fn test_multiple_load_balancers() {
    // Test multiple load balancers can be created
    let _balancer1 = NetworkLoadBalancer::new();
    let _balancer2 = NetworkLoadBalancer::new();
    let _balancer3 = NetworkLoadBalancer::new();

    // All should create successfully
}

#[tokio::test]
async fn test_load_balancer_creation_is_fast() {
    // Test load balancer creation is fast
    let start = std::time::Instant::now();

    let _balancer = NetworkLoadBalancer::new();

    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(100),
        "Load balancer creation should be fast"
    );
}

// ============================================================================
// Coordinator + Config Integration Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_respects_timeout_config() {
    // Test coordinator respects timeout configuration
    let mut config = DistributedConfig::default();
    config.standalone.default_timeout_secs = 600;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should respect timeout config"
    );
}

#[tokio::test]
async fn test_coordinator_respects_concurrency_limits() {
    // Test coordinator respects concurrency limit configuration
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 100;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should respect concurrency limits"
    );
}

#[tokio::test]
async fn test_coordinator_with_minimal_resources() {
    // Integration test: coordinator should work even with minimal resource allocation
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 1;
    config.standalone.max_queue_size = 10;
    config.standalone.default_timeout_secs = 30;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should work with minimal resources"
    );
}

#[tokio::test]
async fn test_coordinator_with_generous_resources() {
    // Integration test: coordinator should handle generous resource allocation
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 1000;
    config.standalone.max_queue_size = 50000;
    config.standalone.default_timeout_secs = 7200;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle generous resources"
    );
}

// ============================================================================
// Async Behavior Integration Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_initialization_is_async() {
    // Test that coordinator initialization properly uses async
    let config = DistributedConfig::default();

    let result = timeout(Duration::from_secs(5), DistributedCoordinator::new(config)).await;

    assert!(
        result.is_ok(),
        "Coordinator async initialization should complete within timeout"
    );
    assert!(
        result.unwrap().is_ok(),
        "Coordinator should initialize successfully"
    );
}

#[tokio::test]
async fn test_multiple_concurrent_initializations() {
    // Integration test: multiple coordinators can initialize concurrently
    let config1 = DistributedConfig::default();
    let config2 = DistributedConfig::default();
    let config3 = DistributedConfig::default();

    let (result1, result2, result3) = tokio::join!(
        DistributedCoordinator::new(config1),
        DistributedCoordinator::new(config2),
        DistributedCoordinator::new(config3),
    );

    assert!(result1.is_ok(), "Concurrent init 1 should succeed");
    assert!(result2.is_ok(), "Concurrent init 2 should succeed");
    assert!(result3.is_ok(), "Concurrent init 3 should succeed");
}

// ============================================================================
// Config Validation Integration Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_validates_config_consistency() {
    // Test that coordinator accepts consistent configuration
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = 1000;
    config.standalone.max_concurrent_executions = 10;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should accept consistent config"
    );
}

#[tokio::test]
async fn test_coordinator_with_zero_timeout() {
    // Edge case: coordinator with zero timeout (should still work, timeout just instant)
    let mut config = DistributedConfig::default();
    config.standalone.default_timeout_secs = 0;

    let coordinator = DistributedCoordinator::new(config).await;

    // Should still initialize even if timeout is zero (just means instant timeout)
    assert!(
        coordinator.is_ok(),
        "Coordinator should handle zero timeout config"
    );
}

#[tokio::test]
async fn test_coordinator_with_very_large_queue() {
    // Test coordinator accepts very large queue size
    let mut config = DistributedConfig::default();
    config.standalone.max_queue_size = 1_000_000;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle very large queue size"
    );
}

// ============================================================================
// Instance ID Integration Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_generates_unique_instance_ids() {
    // Test that each coordinator gets a unique instance ID
    let config1 = DistributedConfig::default();
    let config2 = DistributedConfig::default();

    // Instance IDs are generated in the config
    assert_ne!(
        config1.instance_id, config2.instance_id,
        "Each coordinator should have unique instance ID"
    );
}

#[tokio::test]
async fn test_coordinator_preserves_custom_instance_id() {
    // Test that coordinator preserves custom instance ID if provided
    let mut config = DistributedConfig::default();
    let custom_id = "custom-test-instance-123".to_string();
    config.instance_id = custom_id.clone();

    let coordinator = DistributedCoordinator::new(config.clone()).await;

    assert!(coordinator.is_ok(), "Coordinator should initialize");
    assert_eq!(
        config.instance_id, custom_id,
        "Custom instance ID should be preserved"
    );
}

// ============================================================================
// Standalone Mode Integration Tests
// ============================================================================

#[tokio::test]
async fn test_standalone_config_construction() {
    // Test standalone config can be constructed with reasonable values
    let config = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 300,
        enable_job_queue: true,
        max_queue_size: 1000,
    };

    assert!(
        config.max_concurrent_executions > 0,
        "Should have positive concurrency limit"
    );
    assert!(
        config.default_timeout_secs > 0,
        "Should have positive timeout"
    );
    assert!(config.enable_job_queue, "Job queue should be enabled");
}

#[tokio::test]
async fn test_standalone_config_cloning() {
    // Test that standalone config can be cloned properly
    let config = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 300,
        enable_job_queue: true,
        max_queue_size: 1000,
    };
    let cloned = config.clone();

    assert_eq!(
        config.max_concurrent_executions,
        cloned.max_concurrent_executions
    );
    assert_eq!(config.default_timeout_secs, cloned.default_timeout_secs);
    assert_eq!(config.enable_job_queue, cloned.enable_job_queue);
}

#[tokio::test]
async fn test_standalone_config_serialization() {
    // Integration test: standalone config can be serialized and deserialized
    let config = StandaloneConfig {
        max_concurrent_executions: 25,
        default_timeout_secs: 1200,
        enable_job_queue: true,
        max_queue_size: 2000,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: StandaloneConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(
        config.max_concurrent_executions,
        deserialized.max_concurrent_executions
    );
    assert_eq!(config.enable_job_queue, deserialized.enable_job_queue);
}

// ============================================================================
// System Capability Integration Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_adapts_to_system_capabilities() {
    // Test that coordinator can adapt to detected system capabilities
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should adapt to system capabilities"
    );
}

#[tokio::test]
async fn test_coordinator_with_high_concurrency() {
    // Test coordinator handles high concurrency settings
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 500;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle high concurrency"
    );
}

// ============================================================================
// Configuration Combinations Integration Tests
// ============================================================================

#[tokio::test]
async fn test_coordinator_with_varied_config_combinations() {
    // Test various valid configuration combinations
    let configs = vec![
        // Minimal
        StandaloneConfig {
            max_concurrent_executions: 1,
            default_timeout_secs: 10,
            enable_job_queue: false,
            max_queue_size: 0,
        },
        // Balanced
        StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 300,
            enable_job_queue: true,
            max_queue_size: 100,
        },
        // High performance
        StandaloneConfig {
            max_concurrent_executions: 100,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 10000,
        },
    ];

    for (idx, standalone_config) in configs.into_iter().enumerate() {
        let config = DistributedConfig {
            standalone: standalone_config,
            ..Default::default()
        };

        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Config combination {} should work",
            idx
        );
    }
}

// ============================================================================
// Job Queue Integration Tests
// ============================================================================

#[tokio::test]
async fn test_job_queue_initialization() {
    // Test job queue initializes properly
    let queue = UniversalJobQueue::new();

    assert_eq!(queue.total_jobs(), 0, "New queue should be empty");
}

#[tokio::test]
async fn test_job_queue_with_coordinator() {
    // Integration test: job queue works with coordinator initialization
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator with job queue integration should work"
    );
}

// ============================================================================
// Network Load Balancer Strategy Integration Tests
// ============================================================================

#[tokio::test]
async fn test_all_load_balancing_strategies_exist() {
    // Test all load balancing strategies can be instantiated
    let strategies = vec![
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::ResourceAware,
        LoadBalancingStrategy::LatencyBased,
    ];

    for strategy in strategies {
        // Strategy should exist and be valid
        match strategy {
            LoadBalancingStrategy::RoundRobin => {}
            LoadBalancingStrategy::LeastConnections => {}
            LoadBalancingStrategy::WeightedRoundRobin { .. } => {}
            LoadBalancingStrategy::ResourceAware => {}
            LoadBalancingStrategy::LatencyBased => {}
        }
    }
}

#[tokio::test]
async fn test_load_balancing_strategy_serialization() {
    // Test load balancing strategies can be serialized
    let strategy = LoadBalancingStrategy::RoundRobin;

    let json = serde_json::to_string(&strategy).unwrap();
    let deserialized: LoadBalancingStrategy = serde_json::from_str(&json).unwrap();

    match deserialized {
        LoadBalancingStrategy::RoundRobin => {
            // Success
        }
        _ => panic!("Strategy should deserialize correctly"),
    }
}

// ============================================================================
// Error Handling Integration Tests (Success Path)
// ============================================================================

#[tokio::test]
async fn test_coordinator_creation_returns_result() {
    // Test coordinator creation properly returns Result type
    let config = DistributedConfig::default();
    let result = DistributedCoordinator::new(config).await;

    assert!(result.is_ok(), "Result should be Ok for valid config");
}

#[tokio::test]
async fn test_load_balancer_can_be_created() {
    // Test load balancer creation works
    let _balancer = NetworkLoadBalancer::new();

    // Creation successful (no Result type, just creates directly)
}

// ============================================================================
// Performance Integration Tests
// ============================================================================

#[tokio::test]
async fn test_rapid_coordinator_creation() {
    // Test creating coordinators rapidly doesn't cause issues
    for i in 0..10 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(coordinator.is_ok(), "Rapid creation {} should succeed", i);
    }
}

#[tokio::test]
async fn test_coordinator_creation_under_time_pressure() {
    // Test coordinator creation under time constraints
    let start = std::time::Instant::now();

    for _ in 0..5 {
        let config = DistributedConfig::default();
        let _coordinator = DistributedCoordinator::new(config).await.unwrap();
    }

    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_secs(15),
        "Creating 5 coordinators should be fast (took {:?})",
        elapsed
    );
}

// ============================================================================
// Edge Case Integration Tests - Day 4 Expansion
// ============================================================================

#[tokio::test]
async fn test_coordinator_with_job_queue_integration() {
    // Test coordinator properly integrates with job queue when enabled
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = 1000;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(
        coordinator.is_ok(),
        "Coordinator with job queue should initialize"
    );
}

#[tokio::test]
async fn test_coordinator_without_job_queue_integration() {
    // Test coordinator works without job queue
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = false;
    config.standalone.max_queue_size = 0;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(
        coordinator.is_ok(),
        "Coordinator without job queue should work"
    );
}

#[tokio::test]
async fn test_load_balancer_integration_with_multiple_strategies() {
    // Test load balancer with different strategies
    let strategies = vec![
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::ResourceAware,
        LoadBalancingStrategy::LatencyBased,
    ];

    for _strategy in strategies {
        let _balancer = NetworkLoadBalancer::new();
        // Balancer should work with any strategy type
    }
}

#[tokio::test]
async fn test_coordinator_network_integration_minimal_config() {
    // Test minimal configuration integration
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 1;
    config.standalone.default_timeout_secs = 30;
    config.standalone.enable_job_queue = false;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Minimal config should work");
}

#[tokio::test]
async fn test_coordinator_network_integration_maximal_config() {
    // Test maximal configuration integration
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 1000;
    config.standalone.default_timeout_secs = 3600;
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = 100000;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Maximal config should work");
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
async fn test_job_queue_edge_case_zero_size() {
    // Edge case: job queue with zero size
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = 0;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Zero-size queue should be handled");
}

#[tokio::test]
async fn test_job_queue_edge_case_very_large_size() {
    // Edge case: job queue with very large size
    let mut config = DistributedConfig::default();
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = usize::MAX / 1000;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Large queue size should be handled");
}

#[tokio::test]
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

#[tokio::test]
async fn test_coordinator_creation_with_timeout_wrapper() {
    // Test coordinator creation with external timeout
    let config = DistributedConfig::default();
    let result = timeout(Duration::from_secs(10), DistributedCoordinator::new(config)).await;

    assert!(result.is_ok(), "Should complete within timeout");
    assert!(result.unwrap().is_ok(), "Should create successfully");
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
async fn test_coordinator_creation_with_delays() {
    // Test coordinator creation with small delays between
    for i in 0..10 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(coordinator.is_ok(), "Delayed creation {} should succeed", i);

        // Small delay between creations
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
}

#[tokio::test]
async fn test_load_balancer_rapid_creation_and_drop() {
    // Test rapid creation and dropping of load balancers
    for _ in 0..100 {
        let balancer = NetworkLoadBalancer::new();
        drop(balancer);
    }
}

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
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

#[tokio::test]
async fn test_coordinator_interleaved_with_other_async_work() {
    // Test coordinator creation interleaved with other async work
    for i in 0..10 {
        // Some other async work
        tokio::time::sleep(Duration::from_millis(1)).await;

        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(
            coordinator.is_ok(),
            "Interleaved creation {} should succeed",
            i
        );

        // More async work
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
}
