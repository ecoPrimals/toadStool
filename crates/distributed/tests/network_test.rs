// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::float_cmp, clippy::unreadable_literal)]
//! Comprehensive tests for Network module
//!
//! Tests for network types and fault tolerance

use std::time::Duration;
use toadstool_distributed::network::load_balancer::{
    CircuitBreaker, CircuitBreakerState, FaultToleranceManager, NetworkLoadBalancer, NodeHealth,
};
use toadstool_distributed::types::BackoffStrategy;

// ============================================================================
// NetworkLoadBalancer Tests (2 tests)
// ============================================================================

#[test]
fn test_network_load_balancer_creation() {
    // Test creating a network load balancer
    let _lb = NetworkLoadBalancer::new();
    // Constructor succeeds - that's what we're testing
}

#[test]
fn test_network_load_balancer_default() {
    // Test default construction
    let _lb = NetworkLoadBalancer::default();
    // Default constructor succeeds - that's what we're testing
}

// ============================================================================
// NodeHealth Tests (5 tests)
// ============================================================================

#[test]
fn test_node_health_creation() {
    // Test creating node health information
    let health = NodeHealth {
        healthy: true,
        cpu_usage: 45.5,
        memory_usage: 60.2,
        response_time_ms: 150,
    };

    assert!(health.healthy, "Node should be marked as healthy");
    assert_eq!(health.cpu_usage, 45.5);
    assert_eq!(health.memory_usage, 60.2);
    assert_eq!(health.response_time_ms, 150);
}

#[test]
fn test_node_health_unhealthy() {
    // Test creating unhealthy node
    let health = NodeHealth {
        healthy: false,
        cpu_usage: 95.0,
        memory_usage: 98.0,
        response_time_ms: 5000,
    };

    assert!(!health.healthy, "Node should be marked as unhealthy");
    assert!(health.cpu_usage > 90.0, "Should have high CPU usage");
    assert!(health.memory_usage > 90.0, "Should have high memory usage");
    assert!(
        health.response_time_ms > 1000,
        "Should have high response time"
    );
}

#[test]
fn test_node_health_clone() {
    // Test cloning node health
    let health = NodeHealth {
        healthy: true,
        cpu_usage: 30.0,
        memory_usage: 50.0,
        response_time_ms: 100,
    };

    let cloned = health.clone();
    assert_eq!(cloned.healthy, health.healthy);
    assert_eq!(cloned.cpu_usage, health.cpu_usage);
    assert_eq!(cloned.memory_usage, health.memory_usage);
    assert_eq!(cloned.response_time_ms, health.response_time_ms);
}

#[test]
fn test_node_health_debug() {
    // Test debug formatting
    let health = NodeHealth {
        healthy: false,
        cpu_usage: 90.0,
        memory_usage: 95.0,
        response_time_ms: 500,
    };

    let debug_str = format!("{health:?}");
    assert!(
        debug_str.contains("NodeHealth"),
        "Debug output should contain type name"
    );
    assert!(
        debug_str.contains("healthy"),
        "Debug output should contain field names"
    );
}

#[test]
fn test_node_health_various_states() {
    // Test various health states
    let states = vec![
        (true, 10.0, 20.0, 50),       // Low usage
        (true, 50.0, 50.0, 200),      // Medium usage
        (true, 75.0, 80.0, 400),      // High usage
        (false, 100.0, 100.0, 10000), // Overloaded
    ];

    for (healthy, cpu, mem, response) in states {
        let health = NodeHealth {
            healthy,
            cpu_usage: cpu,
            memory_usage: mem,
            response_time_ms: response,
        };

        assert_eq!(health.healthy, healthy);
        assert_eq!(health.cpu_usage, cpu);
        assert_eq!(health.memory_usage, mem);
        assert_eq!(health.response_time_ms, response);
    }
}

// ============================================================================
// FaultToleranceManager Tests (2 tests)
// ============================================================================

#[test]
fn test_fault_tolerance_manager_creation() {
    // Test creating fault tolerance manager
    let _manager = FaultToleranceManager::new();
    // Constructor succeeds - that's what we're testing
}

#[test]
fn test_fault_tolerance_manager_default() {
    // Test default construction
    let _manager = FaultToleranceManager::default();
    // Default constructor succeeds - that's what we're testing
}

// ============================================================================
// CircuitBreaker Tests (5 tests)
// ============================================================================

#[test]
fn test_circuit_breaker_closed_state() {
    // Test circuit breaker in closed state
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Closed,
        failure_count: 0,
        last_failure_time: None,
    };

    assert_eq!(breaker.failure_count, 0);
    assert!(breaker.last_failure_time.is_none());
}

#[test]
fn test_circuit_breaker_open_state() {
    // Test circuit breaker in open state
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Open,
        failure_count: 5,
        last_failure_time: Some(std::time::Instant::now()),
    };

    assert_eq!(breaker.failure_count, 5);
    assert!(breaker.last_failure_time.is_some());
}

#[test]
fn test_circuit_breaker_half_open_state() {
    // Test circuit breaker in half-open state
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::HalfOpen,
        failure_count: 2,
        last_failure_time: Some(std::time::Instant::now()),
    };

    assert_eq!(breaker.failure_count, 2);
}

#[test]
fn test_circuit_breaker_clone() {
    // Test cloning circuit breaker
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Closed,
        failure_count: 1,
        last_failure_time: None,
    };

    let cloned = breaker.clone();
    assert_eq!(cloned.failure_count, breaker.failure_count);
}

#[test]
fn test_circuit_breaker_debug() {
    // Test debug formatting
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Open,
        failure_count: 3,
        last_failure_time: None,
    };

    let debug_str = format!("{breaker:?}");
    assert!(debug_str.contains("CircuitBreaker"));
}

// ============================================================================
// BackoffStrategy Tests (5 tests)
// ============================================================================

#[test]
fn test_backoff_strategy_exponential() {
    // Test exponential backoff strategy
    let strategy = BackoffStrategy::Exponential {
        base_ms: 1000,
        max_ms: 60000,
    };

    match strategy {
        BackoffStrategy::Exponential { base_ms, max_ms } => {
            assert_eq!(base_ms, 1000);
            assert_eq!(max_ms, 60000);
        }
        _ => panic!("Expected exponential backoff"),
    }
}

#[test]
fn test_backoff_strategy_linear() {
    // Test linear backoff strategy
    let strategy = BackoffStrategy::Linear {
        initial_ms: 100,
        increment_ms: 500,
    };

    match strategy {
        BackoffStrategy::Linear {
            initial_ms,
            increment_ms,
        } => {
            assert_eq!(initial_ms, 100);
            assert_eq!(increment_ms, 500);
        }
        _ => panic!("Expected linear backoff"),
    }
}

#[test]
fn test_backoff_strategy_fixed() {
    // Test fixed backoff strategy
    let strategy = BackoffStrategy::Fixed { delay_ms: 2000 };

    match strategy {
        BackoffStrategy::Fixed { delay_ms } => {
            assert_eq!(delay_ms, 2000);
        }
        _ => panic!("Expected fixed backoff"),
    }
}

#[test]
fn test_backoff_strategy_exponential_jittered() {
    // Test exponential jittered backoff strategy
    let strategy = BackoffStrategy::ExponentialJittered {
        base_ms: 1000,
        max_ms: 30000,
    };

    match strategy {
        BackoffStrategy::ExponentialJittered { base_ms, max_ms } => {
            assert_eq!(base_ms, 1000);
            assert_eq!(max_ms, 30000);
        }
        _ => panic!("Expected exponential jittered backoff"),
    }
}

#[test]
fn test_backoff_strategy_clone() {
    // Test cloning backoff strategy
    let strategy = BackoffStrategy::Exponential {
        base_ms: 500,
        max_ms: 15000,
    };

    let cloned = strategy.clone();

    match (strategy, cloned) {
        (
            BackoffStrategy::Exponential {
                base_ms: b1,
                max_ms: m1,
            },
            BackoffStrategy::Exponential {
                base_ms: b2,
                max_ms: m2,
            },
        ) => {
            assert_eq!(b1, b2);
            assert_eq!(m1, m2);
        }
        _ => panic!("Clone should preserve variant"),
    }
}

// ============================================================================
// Extended NetworkDistributorConfig Tests
// ============================================================================

use toadstool_distributed::network::distributor::{NetworkDistributor, NetworkDistributorConfig};

#[test]
fn test_network_distributor_config_disabled() {
    let config = NetworkDistributorConfig {
        enabled: false,
        max_concurrent_distributions: 5,
        distribution_timeout: Duration::from_secs(60),
    };

    assert!(!config.enabled);
    assert_eq!(config.max_concurrent_distributions, 5);
    assert_eq!(config.distribution_timeout, Duration::from_secs(60));
}

#[test]
fn test_network_distributor_config_high_concurrency() {
    let config = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 100,
        distribution_timeout: Duration::from_secs(600),
    };

    assert_eq!(config.max_concurrent_distributions, 100);
}

#[test]
fn test_network_distributor_config_low_concurrency() {
    let config = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 1,
        distribution_timeout: Duration::from_secs(30),
    };

    assert_eq!(config.max_concurrent_distributions, 1);
}

#[test]
fn test_network_distributor_config_short_timeout() {
    let config = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 10,
        distribution_timeout: Duration::from_secs(10),
    };

    assert_eq!(config.distribution_timeout, Duration::from_secs(10));
}

#[test]
fn test_network_distributor_config_long_timeout() {
    let config = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 10,
        distribution_timeout: Duration::from_secs(3600),
    };

    assert_eq!(config.distribution_timeout, Duration::from_secs(3600));
}

#[test]
fn test_network_distributor_config_clone() {
    let config1 = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 20,
        distribution_timeout: Duration::from_secs(120),
    };

    let config2 = config1.clone();

    assert_eq!(config1.enabled, config2.enabled);
    assert_eq!(
        config1.max_concurrent_distributions,
        config2.max_concurrent_distributions
    );
    assert_eq!(config1.distribution_timeout, config2.distribution_timeout);
}

// ============================================================================
// Extended CircuitBreakerState Tests
// ============================================================================

#[test]
fn test_circuit_breaker_state_closed() {
    let state = CircuitBreakerState::Closed;
    match state {
        CircuitBreakerState::Closed => (), // Success
        _ => panic!("Expected Closed state"),
    }
}

#[test]
fn test_circuit_breaker_state_open() {
    let state = CircuitBreakerState::Open;
    match state {
        CircuitBreakerState::Open => (), // Success
        _ => panic!("Expected Open state"),
    }
}

#[test]
fn test_circuit_breaker_state_half_open() {
    let state = CircuitBreakerState::HalfOpen;
    match state {
        CircuitBreakerState::HalfOpen => (), // Success
        _ => panic!("Expected HalfOpen state"),
    }
}

#[test]
fn test_circuit_breaker_state_clone() {
    let state1 = CircuitBreakerState::Open;
    let state2 = state1.clone();

    match (state1, state2) {
        (CircuitBreakerState::Open, CircuitBreakerState::Open) => (),
        _ => panic!("Clone should preserve state"),
    }
}

// ============================================================================
// Extended NodeHealth Tests - Edge Cases
// ============================================================================

#[test]
fn test_node_health_zero_usage() {
    let health = NodeHealth {
        healthy: true,
        cpu_usage: 0.0,
        memory_usage: 0.0,
        response_time_ms: 10,
    };

    assert_eq!(health.cpu_usage, 0.0);
    assert_eq!(health.memory_usage, 0.0);
}

#[test]
fn test_node_health_max_usage() {
    let health = NodeHealth {
        healthy: false,
        cpu_usage: 100.0,
        memory_usage: 100.0,
        response_time_ms: 60000,
    };

    assert_eq!(health.cpu_usage, 100.0);
    assert_eq!(health.memory_usage, 100.0);
}

#[test]
fn test_node_health_low_latency() {
    let health = NodeHealth {
        healthy: true,
        cpu_usage: 25.0,
        memory_usage: 30.0,
        response_time_ms: 1,
    };

    assert_eq!(health.response_time_ms, 1);
    assert!(health.response_time_ms < 10);
}

#[test]
fn test_node_health_high_latency() {
    let health = NodeHealth {
        healthy: false,
        cpu_usage: 85.0,
        memory_usage: 90.0,
        response_time_ms: 10000,
    };

    assert!(health.response_time_ms > 1000);
}

#[test]
fn test_node_health_medium_usage() {
    let health = NodeHealth {
        healthy: true,
        cpu_usage: 50.0,
        memory_usage: 55.0,
        response_time_ms: 250,
    };

    assert!(health.cpu_usage >= 40.0 && health.cpu_usage <= 60.0);
    assert!(health.memory_usage >= 40.0 && health.memory_usage <= 60.0);
}

// ============================================================================
// Extended CircuitBreaker Tests - Comprehensive Scenarios
// ============================================================================

#[test]
fn test_circuit_breaker_zero_failures() {
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Closed,
        failure_count: 0,
        last_failure_time: None,
    };

    assert_eq!(breaker.failure_count, 0);
    assert!(breaker.last_failure_time.is_none());
}

#[test]
fn test_circuit_breaker_single_failure() {
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Closed,
        failure_count: 1,
        last_failure_time: Some(std::time::Instant::now()),
    };

    assert_eq!(breaker.failure_count, 1);
    assert!(breaker.last_failure_time.is_some());
}

#[test]
fn test_circuit_breaker_threshold_failures() {
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::Open,
        failure_count: 5,
        last_failure_time: Some(std::time::Instant::now()),
    };

    assert!(breaker.failure_count >= 5);
    match breaker.state {
        CircuitBreakerState::Open => (),
        _ => panic!("Should be open after threshold failures"),
    }
}

#[test]
fn test_circuit_breaker_recovery_attempt() {
    let breaker = CircuitBreaker {
        state: CircuitBreakerState::HalfOpen,
        failure_count: 3,
        last_failure_time: Some(std::time::Instant::now()),
    };

    match breaker.state {
        CircuitBreakerState::HalfOpen => (),
        _ => panic!("Expected HalfOpen state for recovery"),
    }
}

// ============================================================================
// Extended BackoffStrategy Tests - Edge Cases
// ============================================================================

#[test]
fn test_backoff_strategy_exponential_min_base() {
    let strategy = BackoffStrategy::Exponential {
        base_ms: 100,
        max_ms: 60000,
    };

    match strategy {
        BackoffStrategy::Exponential { base_ms, .. } => {
            assert_eq!(base_ms, 100);
        }
        _ => panic!("Expected Exponential strategy"),
    }
}

#[test]
fn test_backoff_strategy_exponential_large_max() {
    let strategy = BackoffStrategy::Exponential {
        base_ms: 1000,
        max_ms: 300000,
    };

    match strategy {
        BackoffStrategy::Exponential { max_ms, .. } => {
            assert_eq!(max_ms, 300000);
        }
        _ => panic!("Expected Exponential strategy"),
    }
}

#[test]
fn test_backoff_strategy_linear_zero_initial() {
    let strategy = BackoffStrategy::Linear {
        initial_ms: 0,
        increment_ms: 1000,
    };

    match strategy {
        BackoffStrategy::Linear {
            initial_ms,
            increment_ms,
        } => {
            assert_eq!(initial_ms, 0);
            assert_eq!(increment_ms, 1000);
        }
        _ => panic!("Expected Linear strategy"),
    }
}

#[test]
fn test_backoff_strategy_linear_large_increment() {
    let strategy = BackoffStrategy::Linear {
        initial_ms: 1000,
        increment_ms: 10000,
    };

    match strategy {
        BackoffStrategy::Linear { increment_ms, .. } => {
            assert_eq!(increment_ms, 10000);
        }
        _ => panic!("Expected Linear strategy"),
    }
}

#[test]
fn test_backoff_strategy_fixed_minimum() {
    let strategy = BackoffStrategy::Fixed { delay_ms: 100 };

    match strategy {
        BackoffStrategy::Fixed { delay_ms } => {
            assert_eq!(delay_ms, 100);
        }
        _ => panic!("Expected Fixed strategy"),
    }
}

#[test]
fn test_backoff_strategy_fixed_maximum() {
    let strategy = BackoffStrategy::Fixed { delay_ms: 60000 };

    match strategy {
        BackoffStrategy::Fixed { delay_ms } => {
            assert_eq!(delay_ms, 60000);
        }
        _ => panic!("Expected Fixed strategy"),
    }
}

#[test]
fn test_backoff_strategy_exponential_jittered_small_base() {
    let strategy = BackoffStrategy::ExponentialJittered {
        base_ms: 500,
        max_ms: 10000,
    };

    match strategy {
        BackoffStrategy::ExponentialJittered { base_ms, max_ms } => {
            assert_eq!(base_ms, 500);
            assert_eq!(max_ms, 10000);
        }
        _ => panic!("Expected ExponentialJittered strategy"),
    }
}

// ============================================================================
// NetworkDistributor Integration Tests
// ============================================================================

#[test]
fn test_network_distributor_creation_with_default_config() {
    let config = NetworkDistributorConfig::default();
    let _distributor = NetworkDistributor::new(config);
    // Should create successfully
}

#[test]
fn test_network_distributor_creation_with_custom_config() {
    let config = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 50,
        distribution_timeout: Duration::from_secs(180),
    };
    let _distributor = NetworkDistributor::new(config);
    // Should create successfully
}

#[test]
fn test_network_distributor_with_disabled_config() {
    let config = NetworkDistributorConfig {
        enabled: false,
        max_concurrent_distributions: 10,
        distribution_timeout: Duration::from_secs(300),
    };
    let _distributor = NetworkDistributor::new(config);
    // Should create even when disabled
}

// ============================================================================
// Comprehensive Configuration Combinations
// ============================================================================

#[test]
fn test_config_minimal_resources() {
    let config = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 1,
        distribution_timeout: Duration::from_secs(30),
    };

    assert!(config.enabled);
    assert_eq!(config.max_concurrent_distributions, 1);
    assert_eq!(config.distribution_timeout, Duration::from_secs(30));
}

#[test]
fn test_config_maximum_resources() {
    let config = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 1000,
        distribution_timeout: Duration::from_secs(7200),
    };

    assert_eq!(config.max_concurrent_distributions, 1000);
    assert_eq!(config.distribution_timeout, Duration::from_secs(7200));
}

#[test]
fn test_config_serialization_roundtrip() {
    let config = NetworkDistributorConfig {
        enabled: true,
        max_concurrent_distributions: 25,
        distribution_timeout: Duration::from_secs(450),
    };

    let json = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: NetworkDistributorConfig =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(config.enabled, deserialized.enabled);
    assert_eq!(
        config.max_concurrent_distributions,
        deserialized.max_concurrent_distributions
    );
    assert_eq!(
        config.distribution_timeout,
        deserialized.distribution_timeout
    );
}
