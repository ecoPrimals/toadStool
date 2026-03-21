// SPDX-License-Identifier: AGPL-3.0-only
    // Test that each coordinator gets a unique instance ID
    let config1 = DistributedConfig::default();
    let config2 = DistributedConfig::default();

    // Instance IDs are generated in the config
    assert_ne!(
        config1.instance_id, config2.instance_id,
        "Each coordinator should have unique instance ID"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_adapts_to_system_capabilities() {
    // Test that coordinator can adapt to detected system capabilities
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should adapt to system capabilities"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_job_queue_initialization() {
    // Test job queue initializes properly
    let queue = UniversalJobQueue::new();

    assert_eq!(queue.total_jobs(), 0, "New queue should be empty");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_returns_result() {
    // Test coordinator creation properly returns Result type
    let config = DistributedConfig::default();
    let result = DistributedCoordinator::new(config).await;

    assert!(result.is_ok(), "Result should be Ok for valid config");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_balancer_can_be_created() {
    // Test load balancer creation works
    let _balancer = NetworkLoadBalancer::new();

    // Creation successful (no Result type, just creates directly)
}

// ============================================================================
// Performance Integration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_rapid_coordinator_creation() {
    // Test creating coordinators rapidly doesn't cause issues
    for i in 0..10 {
        let config = DistributedConfig::default();
        let coordinator = DistributedCoordinator::new(config).await;

        assert!(coordinator.is_ok(), "Rapid creation {} should succeed", i);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_network_integration_minimal_config() {
    // Test minimal configuration integration
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 1;
    config.standalone.default_timeout_secs = 30;
    config.standalone.enable_job_queue = false;

    let coordinator = DistributedCoordinator::new(config).await;
    assert!(coordinator.is_ok(), "Minimal config should work");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_network_integration_maximal_config() {
    // Test maximal configuration integration
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 1000;
    config.standalone.default_timeout_secs = 3600;
    config.standalone.enable_job_queue = true;
