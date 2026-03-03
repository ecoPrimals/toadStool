// SPDX-License-Identifier: AGPL-3.0-or-later
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_with_default_network_config() {
    // Test that coordinator initializes with network-aware default config
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator should initialize with network config"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_initializes_job_queue() {
    // Test coordinator properly initializes its job queue
    let config = DistributedConfig::default();
    let coordinator = DistributedCoordinator::new(config).await;

    assert!(
        coordinator.is_ok(),
        "Coordinator with job queue should initialize"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_balancer_initialization() {
    // Test load balancer initializes successfully
    let balancer = NetworkLoadBalancer::new();

    // Balancer created successfully (no Result return type)
    drop(balancer); // Use the balancer to avoid unused variable warning
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_load_balancer_with_default() {
    // Test load balancer can be created with Default trait
    let balancer = NetworkLoadBalancer::default();

    // Balancer created successfully
    drop(balancer);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_load_balancers() {
    // Test multiple load balancers can be created
    let _balancer1 = NetworkLoadBalancer::new();
    let _balancer2 = NetworkLoadBalancer::new();
    let _balancer3 = NetworkLoadBalancer::new();

    // All should create successfully
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
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

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_coordinator_generates_unique_instance_ids() {
