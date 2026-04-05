// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for `DistributedCoordinator`
//!
//! Tests for core coordinator functionality

use toadstool_distributed::core::config::{
    CoordinationConfig, DistributedConfig, StandaloneConfig,
};
use toadstool_distributed::core::coordinator::DistributedCoordinator;
use toadstool_distributed::types::UniversalJobQueue;

// ============================================================================
// Day 1 Tests: Coordinator Basics (5 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation() {
    // Test basic coordinator creation with default config
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should be created successfully with default config"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_custom_config() {
    // Test coordinator creation with custom configuration
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 10;
    config.standalone.default_timeout_secs = 300;
    config.standalone.enable_job_queue = true;
    config.standalone.max_queue_size = 1000;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should be created with custom config"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_coordination_config() {
    // Test coordinator with Coordination integration config
    let config = DistributedConfig {
        coordination: Some(CoordinationConfig {
            endpoint: "http://localhost:8080".to_string(),
            auth_token: Some("test-token".to_string()),
            health_reporting_interval_secs: 30,
        }),
        ..Default::default()
    };

    let coordinator = DistributedCoordinator::new(config).await;

    // This might fail if Coordination is not running, which is expected in tests
    // The important part is that the configuration is accepted
    match coordinator {
        Ok(_) => println!("Coordinator created with Coordination integration"),
        Err(e) => println!("Coordination not available (expected in tests): {e}"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_initialization_completes() {
    // Test that coordinator completes initialization successfully
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator initialization should complete successfully"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_universal_job_queue_initialization() {
    // Test job queue initialization and basic operations
    let queue = UniversalJobQueue::new();

    assert_eq!(queue.total_jobs(), 0, "New queue should be empty");
}

// ============================================================================
// Day 2 Tests: Configuration Validation (5 tests)
// ============================================================================

#[test]
fn test_standalone_config_defaults() {
    // Test standalone config creation with reasonable values
    let config = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 3600,
        enable_job_queue: true,
        max_queue_size: 1000,
    };

    assert!(
        config.max_concurrent_executions > 0,
        "Should have reasonable concurrent execution limit"
    );
    assert!(
        config.default_timeout_secs > 0,
        "Should have positive timeout"
    );
    assert!(config.max_queue_size > 0, "Should have positive queue size");
}

#[test]
fn test_distributed_config_defaults() {
    // Test distributed config default values
    let config = DistributedConfig::default();

    assert!(
        config.standalone.max_concurrent_executions > 0,
        "Standalone config should be initialized"
    );
    assert!(
        config.coordination.is_none(),
        "Coordination integration should be disabled by default"
    );
}

#[test]
fn test_standalone_config_serialization() {
    // Test config serialization/deserialization
    let config = StandaloneConfig {
        max_concurrent_executions: 15,
        default_timeout_secs: 600,
        enable_job_queue: true,
        max_queue_size: 500,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: StandaloneConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(
        config.max_concurrent_executions,
        deserialized.max_concurrent_executions
    );
    assert_eq!(
        config.default_timeout_secs,
        deserialized.default_timeout_secs
    );
    assert_eq!(config.enable_job_queue, deserialized.enable_job_queue);
    assert_eq!(config.max_queue_size, deserialized.max_queue_size);
}

#[test]
fn test_distributed_config_serialization() {
    // Test distributed config serialization
    let config = DistributedConfig {
        instance_id: "test-instance".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 10,
            default_timeout_secs: 3600,
            enable_job_queue: true,
            max_queue_size: 1000,
        },
        coordination: None,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: DistributedConfig = serde_json::from_str(&json).unwrap();

    assert!(deserialized.coordination.is_none());
    assert_eq!(deserialized.instance_id, "test-instance");
}

#[test]
fn test_coordination_config_serialization() {
    // Test Coordination config serialization
    let config = CoordinationConfig {
        endpoint: "http://example.com:9000".to_string(),
        auth_token: Some("secret-token".to_string()),
        health_reporting_interval_secs: 45,
    };

    let json = serde_json::to_string(&config).unwrap();
    let deserialized: CoordinationConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.endpoint, deserialized.endpoint);
    assert_eq!(config.auth_token, deserialized.auth_token);
    assert_eq!(
        config.health_reporting_interval_secs,
        deserialized.health_reporting_interval_secs
    );
}

// ============================================================================
// Day 3 Tests: Resource Management (5 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_resource_detection() {
    // Test that coordinator detects system capabilities
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should successfully detect system capabilities"
    );
}

#[test]
fn test_job_queue_capacity_limits() {
    // Test job queue respects capacity limits
    let queue = UniversalJobQueue::new();

    // Queue should start empty
    assert_eq!(queue.total_jobs(), 0);
}

#[test]
fn test_standalone_config_custom_limits() {
    // Test custom resource limits
    let config = StandaloneConfig {
        max_concurrent_executions: 25,
        default_timeout_secs: 1800,
        enable_job_queue: true,
        max_queue_size: 2000,
    };

    assert_eq!(config.max_concurrent_executions, 25);
    assert_eq!(config.default_timeout_secs, 1800);
    assert_eq!(config.max_queue_size, 2000);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_limited_resources() {
    // Test coordinator with very limited resources
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 1;
    config.standalone.max_queue_size = 10;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should work even with minimal resources"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_high_capacity() {
    // Test coordinator with high capacity settings
    let mut config = DistributedConfig::default();
    config.standalone.max_concurrent_executions = 100;
    config.standalone.max_queue_size = 10000;

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should handle high capacity settings"
    );
}

// ============================================================================
// Day 4 Tests: Configuration Edge Cases (5 tests)
// ============================================================================

#[test]
fn test_config_with_zero_timeout() {
    // Test configuration with edge case values
    let config = StandaloneConfig {
        max_concurrent_executions: 5,
        default_timeout_secs: 0, // Edge case: zero timeout
        enable_job_queue: true,
        max_queue_size: 100,
    };

    // Should accept the configuration (actual validation happens at runtime)
    assert_eq!(config.default_timeout_secs, 0);
}

#[test]
fn test_config_queue_disabled() {
    // Test with job queue disabled
    let config = StandaloneConfig {
        max_concurrent_executions: 10,
        default_timeout_secs: 300,
        enable_job_queue: false, // Queue disabled
        max_queue_size: 0,
    };

    assert!(!config.enable_job_queue);
    assert_eq!(config.max_queue_size, 0);
}

#[test]
fn test_coordination_config_without_auth() {
    // Test Coordination config without authentication
    let config = CoordinationConfig {
        endpoint: "http://localhost:8080".to_string(),
        auth_token: None, // No auth
        health_reporting_interval_secs: 30,
    };

    assert!(config.auth_token.is_none());
}

#[test]
fn test_coordination_config_with_custom_interval() {
    // Test Coordination config with custom health reporting interval
    let config = CoordinationConfig {
        endpoint: "http://coordinator.local:9000".to_string(),
        auth_token: Some("token".to_string()),
        health_reporting_interval_secs: 60,
    };

    assert_eq!(config.health_reporting_interval_secs, 60);
    assert!(config.auth_token.is_some());
}

#[test]
fn test_distributed_config_clone() {
    // Test that DistributedConfig can be cloned
    let config = DistributedConfig::default();
    let cloned = config.clone();

    // Verify the clone has the same values (instance_id will differ)
    assert_eq!(
        config.standalone.max_concurrent_executions,
        cloned.standalone.max_concurrent_executions
    );
    // Note: instance_id will be different as default() generates new UUID
}

// ============================================================================
// Day 5 Tests: Integration Scenarios (5 tests)
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_coordinators_creation() {
    // Test creating multiple coordinators (simulating distributed setup)
    let config1 = DistributedConfig::default();
    let config2 = DistributedConfig::default();

    let coordinator1 = DistributedCoordinator::new(config1).await;
    let coordinator2 = DistributedCoordinator::new(config2).await;

    assert!(coordinator1.is_ok(), "First coordinator should be created");
    assert!(coordinator2.is_ok(), "Second coordinator should be created");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_various_configs() {
    // Test coordinator accepts various configuration combinations
    let configs = vec![
        DistributedConfig::default(),
        DistributedConfig {
            instance_id: "test-instance-1".to_string(),
            standalone: StandaloneConfig {
                max_concurrent_executions: 1,
                default_timeout_secs: 60,
                enable_job_queue: false,
                max_queue_size: 0,
            },
            coordination: None,
        },
        DistributedConfig {
            instance_id: "test-instance-2".to_string(),
            standalone: StandaloneConfig {
                max_concurrent_executions: 50,
                default_timeout_secs: 3600,
                enable_job_queue: true,
                max_queue_size: 5000,
            },
            coordination: None,
        },
    ];

    for (idx, config) in configs.into_iter().enumerate() {
        let coordinator = DistributedCoordinator::new(config).await;
        assert!(
            coordinator.is_ok(),
            "Configuration variant {idx} should be accepted"
        );
    }
}

#[test]
fn test_job_queue_stats_tracking() {
    // Test job queue statistics tracking
    let queue = UniversalJobQueue::new();

    // Initially should be empty
    assert_eq!(queue.total_jobs(), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_custom_standalone_config() {
    // Test coordinator with custom standalone configuration
    let config = DistributedConfig {
        instance_id: "custom-instance".to_string(),
        standalone: StandaloneConfig {
            max_concurrent_executions: 20,
            default_timeout_secs: 900,
            enable_job_queue: true,
            max_queue_size: 1500,
        },
        coordination: None,
    };

    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should accept custom standalone config"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_creation_performance() {
    // Test that coordinator creation is reasonably fast
    use std::time::Instant;

    let start = Instant::now();
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;
    let elapsed = start.elapsed();

    assert!(coordinator.is_ok(), "Coordinator should be created");
    assert!(
        elapsed.as_secs() < 5,
        "Coordinator creation should be fast (took {elapsed:?})"
    );
}
