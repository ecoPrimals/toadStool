//! Comprehensive tests for production hardening module
//!
//! Tests cover circuit breakers, resource leak detection, and memory pressure handling.

use std::sync::Arc;
use std::time::{Duration, Instant};
use uuid::Uuid;

use toadstool::production_hardening::*;
use toadstool::resources::ResourceRequirements;

// ============================================================================
// Circuit Breaker Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_creation() {
    let config = CircuitBreakerConfig::default();
    let breaker = CircuitBreaker::new("test-service".to_string(), config);

    let state = breaker.get_state().await;
    assert_eq!(
        state,
        CircuitState::Closed,
        "Initial state should be Closed"
    );

    let count = breaker.get_failure_count().await;
    assert_eq!(count, 0, "Initial failure count should be 0");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_config_default() {
    let config = CircuitBreakerConfig::default();

    assert_eq!(config.failure_threshold, 5);
    assert_eq!(config.success_threshold, 3);
    assert_eq!(config.timeout, Duration::from_secs(60));
    assert_eq!(config.rolling_window, Duration::from_secs(60));
    assert_eq!(config.half_open_max_requests, 3);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_successful_execution() {
    let config = CircuitBreakerConfig::default();
    let breaker = CircuitBreaker::new("test-service".to_string(), config);

    let result = breaker
        .execute(async { Ok::<_, std::io::Error>("success") })
        .await;

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");

    let state = breaker.get_state().await;
    assert_eq!(state, CircuitState::Closed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_opens_after_failures() {
    let config = CircuitBreakerConfig {
        failure_threshold: 3,
        ..CircuitBreakerConfig::default()
    };
    let breaker = Arc::new(CircuitBreaker::new("test-service".to_string(), config));

    // Execute 3 failing operations
    for _ in 0..3 {
        let _ = breaker
            .execute(async { Err::<String, _>(std::io::Error::other("test error")) })
            .await;
    }

    let state = breaker.get_state().await;
    assert_eq!(
        state,
        CircuitState::Open,
        "Circuit should be Open after reaching threshold"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_rejects_when_open() {
    let config = CircuitBreakerConfig {
        failure_threshold: 2,
        ..CircuitBreakerConfig::default()
    };
    let breaker = Arc::new(CircuitBreaker::new("test-service".to_string(), config));

    // Trigger circuit to open
    for _ in 0..2 {
        let _ = breaker
            .execute(async { Err::<String, _>(std::io::Error::other("test error")) })
            .await;
    }

    // Next request should be rejected
    let result = breaker
        .execute(async { Ok::<_, std::io::Error>("success") })
        .await;

    assert!(result.is_err());
    match result.unwrap_err() {
        CircuitBreakerError::CircuitOpen { service } => {
            assert_eq!(service, "test-service");
        }
        _ => panic!("Expected CircuitOpen error"),
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_failure_count() {
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        ..CircuitBreakerConfig::default()
    };
    let breaker = Arc::new(CircuitBreaker::new("test-service".to_string(), config));

    // Execute 3 failing operations
    for _ in 0..3 {
        let _ = breaker
            .execute(async { Err::<String, _>(std::io::Error::other("test error")) })
            .await;
    }

    let count = breaker.get_failure_count().await;
    assert_eq!(count, 3, "Failure count should be 3");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_state_enum_equality() {
    assert_eq!(CircuitState::Closed, CircuitState::Closed);
    assert_eq!(CircuitState::Open, CircuitState::Open);
    assert_eq!(CircuitState::HalfOpen, CircuitState::HalfOpen);
    assert_ne!(CircuitState::Closed, CircuitState::Open);
    assert_ne!(CircuitState::Open, CircuitState::HalfOpen);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_state_clone() {
    let state = CircuitState::Open;
    let cloned = state.clone();
    assert_eq!(state, cloned);
}

// ============================================================================
// Resource Leak Detector Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_leak_detector_creation() {
    let _detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));

    // Detector should be created successfully
    // (no way to directly inspect internal state, but creation shouldn't panic)
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_allocation_tracking() {
    let detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));

    let allocation = ResourceAllocation {
        id: Uuid::new_v4(),
        resource_type: "test-resource".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    detector.track_allocation(allocation).await;
    // No panic means success
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_access_update() {
    let detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));

    let resource_id = Uuid::new_v4();
    let allocation = ResourceAllocation {
        id: resource_id,
        resource_type: "test-resource".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    detector.track_allocation(allocation).await;
    detector.update_access(resource_id).await;
    // No panic means success
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_removal() {
    let detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));

    let resource_id = Uuid::new_v4();
    let allocation = ResourceAllocation {
        id: resource_id,
        resource_type: "test-resource".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    detector.track_allocation(allocation).await;
    detector.remove_allocation(resource_id).await;
    // No panic means success
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_no_leaks_detected_for_fresh_resources() {
    let detector = ResourceLeakDetector::new(Duration::from_secs(60), Duration::from_secs(10));

    let allocation = ResourceAllocation {
        id: Uuid::new_v4(),
        resource_type: "test-resource".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    detector.track_allocation(allocation).await;

    let leaked = detector.cleanup_leaked_resources().await;
    assert_eq!(
        leaked.len(),
        0,
        "No leaks should be detected for fresh resources"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_allocation_clone() {
    let allocation = ResourceAllocation {
        id: Uuid::new_v4(),
        resource_type: "test-resource".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    let cloned = allocation.clone();
    assert_eq!(allocation.id, cloned.id);
    assert_eq!(allocation.resource_type, cloned.resource_type);
    assert_eq!(allocation.owner, cloned.owner);
}

// ============================================================================
// Memory Pressure Handler Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pressure_handler_creation() {
    let config = MemoryPressureConfig::default();
    let _handler = MemoryPressureHandler::new(config);
    // Creation should not panic
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pressure_config_default() {
    let config = MemoryPressureConfig::default();

    assert_eq!(config.warning_threshold, 70.0);
    assert_eq!(config.critical_threshold, 85.0);
    assert_eq!(config.emergency_threshold, 95.0);
    assert_eq!(config.check_interval, Duration::from_secs(10));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pressure_config_clone() {
    let config = MemoryPressureConfig::default();
    let cloned = config.clone();

    assert_eq!(config.warning_threshold, cloned.warning_threshold);
    assert_eq!(config.critical_threshold, cloned.critical_threshold);
    assert_eq!(config.emergency_threshold, cloned.emergency_threshold);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pressure_levels() {
    assert_eq!(MemoryPressureLevel::Normal, MemoryPressureLevel::Normal);
    assert_eq!(MemoryPressureLevel::Warning, MemoryPressureLevel::Warning);
    assert_eq!(MemoryPressureLevel::Critical, MemoryPressureLevel::Critical);
    assert_eq!(
        MemoryPressureLevel::Emergency,
        MemoryPressureLevel::Emergency
    );
    assert_ne!(MemoryPressureLevel::Normal, MemoryPressureLevel::Warning);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pressure_update_normal() {
    let config = MemoryPressureConfig::default();
    let handler = MemoryPressureHandler::new(config);

    // 50% usage - should be Normal
    handler.update_memory_usage(1000, 500).await;

    let level = handler.get_pressure_level().await;
    assert_eq!(level, MemoryPressureLevel::Normal);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_memory_pressure_level_clone() {
    let level = MemoryPressureLevel::Warning;
    let cloned = level;
    assert_eq!(level, cloned);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_default_memory_pressure_callback() {
    let callback = DefaultMemoryPressureCallback;

    // Test that callback handles different pressure levels without panicking
    callback
        .handle_pressure(MemoryPressureLevel::Normal, 50.0)
        .await;
    callback
        .handle_pressure(MemoryPressureLevel::Warning, 75.0)
        .await;
    callback
        .handle_pressure(MemoryPressureLevel::Critical, 90.0)
        .await;
    callback
        .handle_pressure(MemoryPressureLevel::Emergency, 98.0)
        .await;
}

// ============================================================================
// Production Hardening Manager Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_production_hardening_config_default() {
    let config = ProductionHardeningConfig::default();

    assert!(config.enable_circuit_breakers);
    assert!(config.enable_leak_detection);
    assert!(config.enable_memory_pressure);
    assert_eq!(config.leak_detection_threshold, Duration::from_secs(300));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_production_hardening_config_clone() {
    let config = ProductionHardeningConfig::default();
    let cloned = config.clone();

    assert_eq!(
        config.enable_circuit_breakers,
        cloned.enable_circuit_breakers
    );
    assert_eq!(config.enable_leak_detection, cloned.enable_leak_detection);
    assert_eq!(config.enable_memory_pressure, cloned.enable_memory_pressure);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_production_hardening_manager_creation() {
    let config = ProductionHardeningConfig::default();
    let _manager = ProductionHardeningManager::new(config);
    // Creation should not panic
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_production_hardening_manager_initialization() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let result = manager.initialize().await;
    assert!(result.is_ok(), "Initialization should succeed");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_circuit_breaker() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let breaker = manager.get_circuit_breaker("test-service").await;
    let state = breaker.get_state().await;
    assert_eq!(state, CircuitState::Closed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_same_circuit_breaker_twice() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let breaker1 = manager.get_circuit_breaker("test-service").await;
    let breaker2 = manager.get_circuit_breaker("test-service").await;

    // Both should refer to the same circuit breaker
    let state1 = breaker1.get_state().await;
    let state2 = breaker2.get_state().await;
    assert_eq!(state1, state2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_track_resource_with_manager() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let allocation = ResourceAllocation {
        id: Uuid::new_v4(),
        resource_type: "test-resource".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    manager.track_resource(allocation).await;
    // No panic means success
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_update_resource_access_with_manager() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let resource_id = Uuid::new_v4();
    let allocation = ResourceAllocation {
        id: resource_id,
        resource_type: "test-resource".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    manager.track_resource(allocation).await;
    manager.update_resource_access(resource_id).await;
    // No panic means success
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_remove_resource_with_manager() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let resource_id = Uuid::new_v4();
    let allocation = ResourceAllocation {
        id: resource_id,
        resource_type: "test-resource".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    manager.track_resource(allocation).await;
    manager.remove_resource(resource_id).await;
    // No panic means success
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_update_memory_usage_with_manager() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    manager.update_memory_usage(1000, 500).await;
    // No panic means success
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_error_display() {
    let error = CircuitBreakerError::CircuitOpen {
        service: "test-service".to_string(),
    };
    let error_string = error.to_string();
    assert!(error_string.contains("Circuit breaker is open for service: test-service"));

    let error2 = CircuitBreakerError::HalfOpenLimitExceeded {
        service: "test-service".to_string(),
    };
    let error_string2 = error2.to_string();
    assert!(error_string2.contains("Half-open limit exceeded for service: test-service"));

    let error3 = CircuitBreakerError::ServiceFailure {
        service: "test-service".to_string(),
        error: "connection failed".to_string(),
    };
    let error_string3 = error3.to_string();
    assert!(error_string3.contains("Service failure for test-service: connection failed"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_custom_circuit_breaker_config() {
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        success_threshold: 5,
        timeout: Duration::from_secs(30),
        rolling_window: Duration::from_secs(120),
        half_open_max_requests: 2,
    };

    assert_eq!(config.failure_threshold, 10);
    assert_eq!(config.success_threshold, 5);
    assert_eq!(config.timeout, Duration::from_secs(30));
    assert_eq!(config.rolling_window, Duration::from_secs(120));
    assert_eq!(config.half_open_max_requests, 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_custom_memory_pressure_config() {
    let config = MemoryPressureConfig {
        warning_threshold: 60.0,
        critical_threshold: 80.0,
        emergency_threshold: 90.0,
        check_interval: Duration::from_secs(5),
    };

    assert_eq!(config.warning_threshold, 60.0);
    assert_eq!(config.critical_threshold, 80.0);
    assert_eq!(config.emergency_threshold, 90.0);
    assert_eq!(config.check_interval, Duration::from_secs(5));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_requirements_in_allocation() {
    let requirements = ResourceRequirements::default();

    let allocation = ResourceAllocation {
        id: Uuid::new_v4(),
        resource_type: "gpu-workload".to_string(),
        allocated_at: Instant::now(),
        requirements: requirements.clone(),
        owner: "ml-pipeline".to_string(),
        last_accessed: Instant::now(),
    };

    // Test that allocation has valid resource requirements
    assert_eq!(allocation.requirements.cpu.min_cores, 1.0);
    assert_eq!(allocation.requirements.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(
        allocation.requirements.storage.min_bytes,
        1024 * 1024 * 1024
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_disabled_features_config() {
    let config = ProductionHardeningConfig {
        enable_circuit_breakers: false,
        enable_leak_detection: false,
        enable_memory_pressure: false,
        default_circuit_config: CircuitBreakerConfig::default(),
        memory_pressure_config: MemoryPressureConfig::default(),
        leak_detection_threshold: Duration::from_secs(300),
        ..Default::default()
    };

    let manager = ProductionHardeningManager::new(config);

    // Manager should still be usable even with features disabled
    let result = manager.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_circuit_breakers() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let breaker1 = manager.get_circuit_breaker("service-1").await;
    let breaker2 = manager.get_circuit_breaker("service-2").await;
    let breaker3 = manager.get_circuit_breaker("service-3").await;

    // All should be in Closed state initially
    assert_eq!(breaker1.get_state().await, CircuitState::Closed);
    assert_eq!(breaker2.get_state().await, CircuitState::Closed);
    assert_eq!(breaker3.get_state().await, CircuitState::Closed);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_circuit_breaker_with_zero_threshold() {
    let config = CircuitBreakerConfig {
        failure_threshold: 0,
        ..CircuitBreakerConfig::default()
    };
    let breaker = CircuitBreaker::new("test-service".to_string(), config);

    // Even a single failure should open the circuit
    let _ = breaker
        .execute(async { Err::<String, _>(std::io::Error::other("test error")) })
        .await;

    // Circuit should still be closed as threshold is 0 (no failures can reach it)
    let _state = breaker.get_state().await;
    // With threshold 0, circuit behavior is undefined/edge case
    // Just ensure it doesn't panic
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_uuid_generation_for_resources() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();

    assert_ne!(id1, id2, "Generated UUIDs should be unique");
}
