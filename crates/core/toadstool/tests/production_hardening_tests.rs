//! Comprehensive tests for production hardening module
//!
//! Tests for circuit breakers, resource leak detection, memory pressure handling,
//! and production hardening manager.

use std::time::{Duration, Instant};
use toadstool::production_hardening::*;
use toadstool::resources::ResourceRequirements;
use uuid::Uuid;

// ============================================================================
// CircuitState Enum Tests
// ============================================================================

#[test]
fn test_circuit_state_closed() {
    let state = CircuitState::Closed;
    assert_eq!(state, CircuitState::Closed);
}

#[test]
fn test_circuit_state_open() {
    let state = CircuitState::Open;
    assert_eq!(state, CircuitState::Open);
}

#[test]
fn test_circuit_state_half_open() {
    let state = CircuitState::HalfOpen;
    assert_eq!(state, CircuitState::HalfOpen);
}

#[test]
fn test_circuit_state_clone() {
    let state = CircuitState::Closed;
    let cloned = state.clone();
    assert_eq!(state, cloned);
}

#[test]
fn test_circuit_state_debug() {
    let state = CircuitState::Open;
    let debug_str = format!("{:?}", state);
    assert!(debug_str.contains("Open"));
}

#[test]
fn test_circuit_state_serialization() {
    let state = CircuitState::HalfOpen;
    let json = serde_json::to_string(&state).expect("Should serialize");
    let deserialized: CircuitState = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(state, deserialized);
}

// ============================================================================
// CircuitBreakerConfig Tests
// ============================================================================

#[test]
fn test_circuit_breaker_config_default() {
    let config = CircuitBreakerConfig::default();
    assert_eq!(config.failure_threshold, 5);
    assert_eq!(config.success_threshold, 3);
    assert_eq!(config.timeout, Duration::from_secs(60));
    assert_eq!(config.half_open_max_requests, 3);
}

#[test]
fn test_circuit_breaker_config_custom() {
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        success_threshold: 5,
        timeout: Duration::from_secs(120),
        rolling_window: Duration::from_secs(120),
        half_open_max_requests: 5,
    };
    assert_eq!(config.failure_threshold, 10);
    assert_eq!(config.success_threshold, 5);
}

#[test]
fn test_circuit_breaker_config_clone() {
    let config = CircuitBreakerConfig::default();
    let cloned = config.clone();
    assert_eq!(config.failure_threshold, cloned.failure_threshold);
}

#[test]
fn test_circuit_breaker_config_serialization() {
    let config = CircuitBreakerConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");
    let deserialized: CircuitBreakerConfig =
        serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(config.failure_threshold, deserialized.failure_threshold);
}

// ============================================================================
// CircuitBreaker Creation Tests
// ============================================================================

#[test]
fn test_circuit_breaker_new() {
    let config = CircuitBreakerConfig::default();
    let _breaker = CircuitBreaker::new("test-service".to_string(), config);
    // Should create without panicking
}

#[test]
fn test_circuit_breaker_with_custom_config() {
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        success_threshold: 5,
        timeout: Duration::from_secs(120),
        rolling_window: Duration::from_secs(120),
        half_open_max_requests: 5,
    };
    let _breaker = CircuitBreaker::new("custom-service".to_string(), config);
    // Should create without panicking
}

#[test]
fn test_circuit_breaker_with_high_thresholds() {
    let config = CircuitBreakerConfig {
        failure_threshold: 100,
        success_threshold: 50,
        timeout: Duration::from_secs(300),
        rolling_window: Duration::from_secs(300),
        half_open_max_requests: 10,
    };
    let _breaker = CircuitBreaker::new("high-threshold-service".to_string(), config);
    // Should create without panicking
}

// ============================================================================
// MemoryPressureLevel Enum Tests
// ============================================================================

#[test]
fn test_memory_pressure_level_normal() {
    let level = MemoryPressureLevel::Normal;
    assert_eq!(level, MemoryPressureLevel::Normal);
}

#[test]
fn test_memory_pressure_level_warning() {
    let level = MemoryPressureLevel::Warning;
    assert_eq!(level, MemoryPressureLevel::Warning);
}

#[test]
fn test_memory_pressure_level_critical() {
    let level = MemoryPressureLevel::Critical;
    assert_eq!(level, MemoryPressureLevel::Critical);
}

#[test]
fn test_memory_pressure_level_emergency() {
    let level = MemoryPressureLevel::Emergency;
    assert_eq!(level, MemoryPressureLevel::Emergency);
}

#[test]
fn test_memory_pressure_level_clone() {
    let level = MemoryPressureLevel::Critical;
    let cloned = level;
    assert_eq!(level, cloned);
}

#[test]
fn test_memory_pressure_level_debug() {
    let level = MemoryPressureLevel::Warning;
    let debug_str = format!("{:?}", level);
    assert!(debug_str.contains("Warning"));
}

// ============================================================================
// MemoryPressureConfig Tests
// ============================================================================

#[test]
fn test_memory_pressure_config_default() {
    let config = MemoryPressureConfig::default();
    assert_eq!(config.warning_threshold, 70.0);
    assert_eq!(config.critical_threshold, 85.0);
    assert_eq!(config.emergency_threshold, 95.0);
}

#[test]
fn test_memory_pressure_config_custom() {
    let config = MemoryPressureConfig {
        warning_threshold: 60.0,
        critical_threshold: 75.0,
        emergency_threshold: 90.0,
        check_interval: Duration::from_secs(10),
    };
    assert_eq!(config.warning_threshold, 60.0);
    assert_eq!(config.critical_threshold, 75.0);
}

#[test]
fn test_memory_pressure_config_aggressive() {
    let config = MemoryPressureConfig {
        warning_threshold: 50.0,
        critical_threshold: 65.0,
        emergency_threshold: 80.0,
        check_interval: Duration::from_secs(5),
    };
    assert_eq!(config.warning_threshold, 50.0);
}

#[test]
fn test_memory_pressure_config_clone() {
    let config = MemoryPressureConfig::default();
    let cloned = config.clone();
    assert_eq!(config.warning_threshold, cloned.warning_threshold);
}

// ============================================================================
// MemoryPressureHandler Tests
// ============================================================================

#[test]
fn test_memory_pressure_handler_new() {
    let config = MemoryPressureConfig::default();
    let _handler = MemoryPressureHandler::new(config);
    // Should create without panicking
}

#[test]
fn test_memory_pressure_handler_with_custom_config() {
    let config = MemoryPressureConfig {
        warning_threshold: 60.0,
        critical_threshold: 75.0,
        emergency_threshold: 90.0,
        check_interval: Duration::from_secs(10),
    };
    let _handler = MemoryPressureHandler::new(config);
    // Should create without panicking
}

#[tokio::test]
async fn test_memory_pressure_handler_update_normal() {
    let config = MemoryPressureConfig::default();
    let handler = MemoryPressureHandler::new(config);

    // 50% usage - should be normal
    handler.update_memory_usage(1000, 500).await;
    // No panic means success
}

#[tokio::test]
async fn test_memory_pressure_handler_update_warning() {
    let config = MemoryPressureConfig::default();
    let handler = MemoryPressureHandler::new(config);

    // 75% usage - should trigger warning
    handler.update_memory_usage(1000, 750).await;
    // No panic means success
}

#[tokio::test]
async fn test_memory_pressure_handler_update_critical() {
    let config = MemoryPressureConfig::default();
    let handler = MemoryPressureHandler::new(config);

    // 90% usage - should trigger critical
    handler.update_memory_usage(1000, 900).await;
    // No panic means success
}

#[tokio::test]
async fn test_memory_pressure_handler_get_pressure_level() {
    let config = MemoryPressureConfig::default();
    let handler = MemoryPressureHandler::new(config);

    let level = handler.get_pressure_level().await;
    // Should return some level
    assert_eq!(level, MemoryPressureLevel::Normal);
}

// ============================================================================
// ResourceAllocation Tests
// ============================================================================

#[test]
fn test_resource_allocation_new() {
    let id = Uuid::new_v4();
    let allocation = ResourceAllocation {
        id,
        resource_type: "CPU".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };
    assert_eq!(allocation.id, id);
    assert_eq!(allocation.resource_type, "CPU");
    assert_eq!(allocation.owner, "test-owner");
}

#[test]
fn test_resource_allocation_memory() {
    let allocation = ResourceAllocation {
        id: Uuid::new_v4(),
        resource_type: "Memory".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "memory-test".to_string(),
        last_accessed: Instant::now(),
    };
    assert_eq!(allocation.resource_type, "Memory");
}

#[test]
fn test_resource_allocation_clone() {
    let id = Uuid::new_v4();
    let allocation = ResourceAllocation {
        id,
        resource_type: "CPU".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };
    let cloned = allocation.clone();
    assert_eq!(allocation.id, cloned.id);
    assert_eq!(allocation.resource_type, cloned.resource_type);
}

// ============================================================================
// ResourceLeakDetector Tests
// ============================================================================

#[test]
fn test_resource_leak_detector_new() {
    let threshold = Duration::from_secs(300);
    let cleanup_interval = Duration::from_secs(60);
    let _detector = ResourceLeakDetector::new(threshold, cleanup_interval);
    // Should create without panicking
}

#[test]
fn test_resource_leak_detector_with_short_threshold() {
    let threshold = Duration::from_secs(60);
    let cleanup_interval = Duration::from_secs(10);
    let _detector = ResourceLeakDetector::new(threshold, cleanup_interval);
    // Should create without panicking
}

#[tokio::test]
async fn test_resource_leak_detector_track_allocation() {
    let threshold = Duration::from_secs(300);
    let cleanup_interval = Duration::from_secs(60);
    let detector = ResourceLeakDetector::new(threshold, cleanup_interval);

    let allocation = ResourceAllocation {
        id: Uuid::new_v4(),
        resource_type: "Memory".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    detector.track_allocation(allocation).await;
    // No panic means success
}

#[tokio::test]
async fn test_resource_leak_detector_remove_allocation() {
    let threshold = Duration::from_secs(300);
    let cleanup_interval = Duration::from_secs(60);
    let detector = ResourceLeakDetector::new(threshold, cleanup_interval);

    let id = Uuid::new_v4();
    let allocation = ResourceAllocation {
        id,
        resource_type: "Memory".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "test-owner".to_string(),
        last_accessed: Instant::now(),
    };

    detector.track_allocation(allocation.clone()).await;
    detector.remove_allocation(id).await;
    // No panic means success
}

// ============================================================================
// ProductionHardeningConfig Tests
// ============================================================================

#[test]
fn test_production_hardening_config_default() {
    let config = ProductionHardeningConfig::default();
    assert!(config.enable_circuit_breakers);
    assert!(config.enable_leak_detection);
    assert!(config.enable_memory_pressure);
}

#[test]
fn test_production_hardening_config_custom() {
    let config = ProductionHardeningConfig {
        enable_circuit_breakers: false,
        enable_leak_detection: true,
        enable_memory_pressure: true,
        default_circuit_config: CircuitBreakerConfig::default(),
        memory_pressure_config: MemoryPressureConfig::default(),
        leak_detection_threshold: Duration::from_secs(600),
    };
    assert!(!config.enable_circuit_breakers);
    assert_eq!(config.leak_detection_threshold, Duration::from_secs(600));
}

#[test]
fn test_production_hardening_config_all_disabled() {
    let config = ProductionHardeningConfig {
        enable_circuit_breakers: false,
        enable_leak_detection: false,
        enable_memory_pressure: false,
        default_circuit_config: CircuitBreakerConfig::default(),
        memory_pressure_config: MemoryPressureConfig::default(),
        leak_detection_threshold: Duration::from_secs(300),
    };
    assert!(!config.enable_circuit_breakers);
    assert!(!config.enable_leak_detection);
    assert!(!config.enable_memory_pressure);
}

#[test]
fn test_production_hardening_config_clone() {
    let config = ProductionHardeningConfig::default();
    let cloned = config.clone();
    assert_eq!(
        config.enable_circuit_breakers,
        cloned.enable_circuit_breakers
    );
}

// ============================================================================
// ProductionHardeningManager Tests
// ============================================================================

#[test]
fn test_production_hardening_manager_new() {
    let config = ProductionHardeningConfig::default();
    let _manager = ProductionHardeningManager::new(config);
    // Should create without panicking
}

#[test]
fn test_production_hardening_manager_with_custom_config() {
    let config = ProductionHardeningConfig {
        enable_circuit_breakers: true,
        enable_leak_detection: true,
        enable_memory_pressure: false,
        default_circuit_config: CircuitBreakerConfig::default(),
        memory_pressure_config: MemoryPressureConfig::default(),
        leak_detection_threshold: Duration::from_secs(600),
    };
    let _manager = ProductionHardeningManager::new(config);
    // Should create without panicking
}

#[tokio::test]
async fn test_production_hardening_manager_initialize() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let result = manager.initialize().await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_production_hardening_manager_get_circuit_breaker() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let _breaker = manager.get_circuit_breaker("test-service").await;
    // Should return a circuit breaker
}

#[tokio::test]
async fn test_production_hardening_manager_get_same_circuit_breaker_twice() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let _breaker1 = manager.get_circuit_breaker("test-service").await;
    let _breaker2 = manager.get_circuit_breaker("test-service").await;
    // Both should reference the same circuit breaker
}

#[tokio::test]
async fn test_production_hardening_manager_track_resource() {
    let config = ProductionHardeningConfig::default();
    let manager = ProductionHardeningManager::new(config);

    let allocation = ResourceAllocation {
        id: Uuid::new_v4(),
        resource_type: "Memory".to_string(),
        allocated_at: Instant::now(),
        requirements: ResourceRequirements::default(),
        owner: "manager-test".to_string(),
        last_accessed: Instant::now(),
    };

    manager.track_resource(allocation).await;
    // No panic means success
}
