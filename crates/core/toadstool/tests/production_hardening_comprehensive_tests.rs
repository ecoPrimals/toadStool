// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for `production_hardening` module
//!
//! Sprint 18: `production_hardening.rs` coverage → 60%+
//! Target: Critical production hardening features
//! Estimated: ~40-50 tests

use std::time::Duration;
use toadstool::production_hardening::*;

// ============================================================================
// CircuitState Tests
// ============================================================================

#[test]
fn test_circuit_state_variants() {
    let closed = CircuitState::Closed;
    let open = CircuitState::Open;
    let half_open = CircuitState::HalfOpen;

    assert_eq!(closed, CircuitState::Closed);
    assert_eq!(open, CircuitState::Open);
    assert_eq!(half_open, CircuitState::HalfOpen);
}

#[test]
fn test_circuit_state_equality() {
    assert_eq!(CircuitState::Closed, CircuitState::Closed);
    assert_ne!(CircuitState::Closed, CircuitState::Open);
    assert_ne!(CircuitState::Open, CircuitState::HalfOpen);
}

#[test]
fn test_circuit_state_clone() {
    let state = CircuitState::Closed;
    let cloned = state.clone();

    assert_eq!(cloned, state);
}

#[test]
fn test_circuit_state_debug() {
    let state = CircuitState::Open;
    let debug = format!("{state:?}");

    assert!(!debug.is_empty());
    assert!(debug.contains("Open"));
}

#[test]
fn test_circuit_state_serialization() {
    let state = CircuitState::HalfOpen;
    let json = serde_json::to_string(&state);

    assert!(json.is_ok());
}

#[test]
fn test_circuit_state_deserialization() {
    let json = r#""Closed""#;
    let state: Result<CircuitState, _> = serde_json::from_str(json);

    assert!(state.is_ok());
    assert_eq!(state.unwrap(), CircuitState::Closed);
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
    assert_eq!(config.rolling_window, Duration::from_secs(60));
    assert_eq!(config.half_open_max_requests, 3);
}

#[test]
fn test_circuit_breaker_config_custom() {
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        success_threshold: 5,
        timeout: Duration::from_secs(120),
        rolling_window: Duration::from_secs(300),
        half_open_max_requests: 5,
    };

    assert_eq!(config.failure_threshold, 10);
    assert_eq!(config.success_threshold, 5);
    assert_eq!(config.timeout, Duration::from_secs(120));
}

#[test]
fn test_circuit_breaker_config_clone() {
    let config = CircuitBreakerConfig::default();
    let cloned = config.clone();

    assert_eq!(cloned.failure_threshold, config.failure_threshold);
    assert_eq!(cloned.success_threshold, config.success_threshold);
}

#[test]
fn test_circuit_breaker_config_debug() {
    let config = CircuitBreakerConfig::default();
    let debug = format!("{config:?}");

    assert!(!debug.is_empty());
    assert!(debug.contains("CircuitBreakerConfig"));
}

#[test]
fn test_circuit_breaker_config_serialization() {
    let config = CircuitBreakerConfig::default();
    let json = serde_json::to_string(&config);

    assert!(json.is_ok());
}

#[test]
fn test_circuit_breaker_config_high_thresholds() {
    let config = CircuitBreakerConfig {
        failure_threshold: 100,
        success_threshold: 50,
        timeout: Duration::from_secs(600),
        rolling_window: Duration::from_secs(3600),
        half_open_max_requests: 10,
    };

    assert_eq!(config.failure_threshold, 100);
    assert_eq!(config.half_open_max_requests, 10);
}

#[test]
fn test_circuit_breaker_config_minimal_thresholds() {
    let config = CircuitBreakerConfig {
        failure_threshold: 1,
        success_threshold: 1,
        timeout: Duration::from_secs(1),
        rolling_window: Duration::from_secs(1),
        half_open_max_requests: 1,
    };

    assert_eq!(config.failure_threshold, 1);
    assert_eq!(config.success_threshold, 1);
}

// ============================================================================
// CircuitBreaker Tests
// ============================================================================

#[test]
fn test_circuit_breaker_new() {
    let config = CircuitBreakerConfig::default();
    let _cb = CircuitBreaker::new("test-service".to_string(), config);

    // Circuit breaker created successfully
}

#[test]
fn test_circuit_breaker_new_with_custom_config() {
    let config = CircuitBreakerConfig {
        failure_threshold: 10,
        success_threshold: 5,
        timeout: Duration::from_secs(30),
        rolling_window: Duration::from_secs(60),
        half_open_max_requests: 2,
    };

    let _cb = CircuitBreaker::new("custom-service".to_string(), config);
}

#[test]
fn test_circuit_breaker_new_with_empty_service_name() {
    let config = CircuitBreakerConfig::default();
    let _cb = CircuitBreaker::new(String::new(), config);
}

#[test]
fn test_circuit_breaker_new_with_long_service_name() {
    let config = CircuitBreakerConfig::default();
    let long_name = "a".repeat(1000);
    let _cb = CircuitBreaker::new(long_name, config);
}

// ============================================================================
// MemoryPressureLevel Tests
// ============================================================================

#[test]
fn test_memory_pressure_level_variants() {
    let warning = MemoryPressureLevel::Warning;
    let critical = MemoryPressureLevel::Critical;
    let emergency = MemoryPressureLevel::Emergency;

    assert!(matches!(warning, MemoryPressureLevel::Warning));
    assert!(matches!(critical, MemoryPressureLevel::Critical));
    assert!(matches!(emergency, MemoryPressureLevel::Emergency));
}

#[test]
fn test_memory_pressure_level_clone() {
    let level = MemoryPressureLevel::Critical;
    let cloned = level;

    assert!(matches!(cloned, MemoryPressureLevel::Critical));
}

#[test]
fn test_memory_pressure_level_debug() {
    let level = MemoryPressureLevel::Emergency;
    let debug = format!("{level:?}");

    assert!(!debug.is_empty());
}

#[test]
fn test_memory_pressure_level_serialization() {
    let level = MemoryPressureLevel::Warning;
    let json = serde_json::to_string(&level);

    assert!(json.is_ok());
}

// ============================================================================
// MemoryPressureConfig Tests
// ============================================================================

#[test]
fn test_memory_pressure_config_default() {
    let config = MemoryPressureConfig::default();

    // Check that default values are reasonable
    assert!(config.warning_threshold > 0.0);
    assert!(config.critical_threshold > config.warning_threshold);
    assert!(config.emergency_threshold > config.critical_threshold);
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_memory_pressure_config_custom() {
    let config = MemoryPressureConfig {
        warning_threshold: 70.0,
        critical_threshold: 85.0,
        emergency_threshold: 95.0,
        check_interval: Duration::from_secs(30),
    };

    assert_eq!(config.warning_threshold, 70.0);
    assert_eq!(config.critical_threshold, 85.0);
    assert_eq!(config.emergency_threshold, 95.0);
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_memory_pressure_config_clone() {
    let config = MemoryPressureConfig::default();
    let cloned = config.clone();

    assert_eq!(cloned.warning_threshold, config.warning_threshold);
    assert_eq!(cloned.critical_threshold, config.critical_threshold);
}

#[test]
fn test_memory_pressure_config_debug() {
    let config = MemoryPressureConfig::default();
    let debug = format!("{config:?}");

    assert!(!debug.is_empty());
}

#[test]
fn test_memory_pressure_config_serialization() {
    let config = MemoryPressureConfig::default();
    let json = serde_json::to_string(&config);

    assert!(json.is_ok());
}

// ============================================================================
// ProductionHardeningConfig Tests
// ============================================================================

#[test]
fn test_production_hardening_config_default() {
    let config = ProductionHardeningConfig::default();

    assert!(config.enable_leak_detection);
    assert!(config.enable_memory_pressure);
}

#[test]
fn test_production_hardening_config_custom() {
    let config = ProductionHardeningConfig {
        enable_circuit_breakers: true,
        enable_leak_detection: false,
        enable_memory_pressure: true,
        default_circuit_config: CircuitBreakerConfig::default(),
        memory_pressure_config: MemoryPressureConfig::default(),
        leak_detection_threshold: Duration::from_secs(1800),
        ..Default::default()
    };

    assert!(config.enable_circuit_breakers);
    assert!(!config.enable_leak_detection);
    assert!(config.enable_memory_pressure);
}

#[test]
fn test_production_hardening_config_clone() {
    let config = ProductionHardeningConfig::default();
    let cloned = config.clone();

    assert_eq!(cloned.enable_leak_detection, config.enable_leak_detection);
}

#[test]
fn test_production_hardening_config_debug() {
    let config = ProductionHardeningConfig::default();
    let debug = format!("{config:?}");

    assert!(!debug.is_empty());
}

#[test]
fn test_production_hardening_config_serialization() {
    let config = ProductionHardeningConfig::default();
    let json = serde_json::to_string(&config);

    assert!(json.is_ok());
}

// ============================================================================
// Serialization Round-trip Tests
// ============================================================================

#[test]
fn test_circuit_state_round_trip() {
    let original = CircuitState::Open;

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: CircuitState = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized, original);
}

#[test]
fn test_circuit_breaker_config_round_trip() {
    let original = CircuitBreakerConfig::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: CircuitBreakerConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.failure_threshold, original.failure_threshold);
    assert_eq!(deserialized.success_threshold, original.success_threshold);
}

#[test]
fn test_memory_pressure_level_round_trip() {
    let original = MemoryPressureLevel::Critical;

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: MemoryPressureLevel = serde_json::from_str(&json).unwrap();

    assert!(matches!(deserialized, MemoryPressureLevel::Critical));
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_memory_pressure_config_round_trip() {
    let original = MemoryPressureConfig::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: MemoryPressureConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.warning_threshold, original.warning_threshold);
    assert_eq!(deserialized.critical_threshold, original.critical_threshold);
}

#[test]
fn test_production_hardening_config_round_trip() {
    let original = ProductionHardeningConfig::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ProductionHardeningConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(
        deserialized.enable_leak_detection,
        original.enable_leak_detection
    );
}

// ============================================================================
// Edge Cases
// ============================================================================

#[test]
fn test_circuit_breaker_config_zero_timeout() {
    let config = CircuitBreakerConfig {
        failure_threshold: 5,
        success_threshold: 3,
        timeout: Duration::from_secs(0),
        rolling_window: Duration::from_secs(60),
        half_open_max_requests: 3,
    };

    assert_eq!(config.timeout, Duration::from_secs(0));
}

#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
#[test]
fn test_memory_pressure_config_extreme_thresholds() {
    let config = MemoryPressureConfig {
        warning_threshold: 50.0,
        critical_threshold: 75.0,
        emergency_threshold: 100.0,
        check_interval: Duration::from_secs(1),
    };

    assert_eq!(config.emergency_threshold, 100.0);
}

#[test]
fn test_production_hardening_config_all_disabled() {
    let config = ProductionHardeningConfig {
        enable_circuit_breakers: false,
        enable_leak_detection: false,
        enable_memory_pressure: false,
        default_circuit_config: CircuitBreakerConfig::default(),
        memory_pressure_config: MemoryPressureConfig::default(),
        leak_detection_threshold: Duration::from_secs(3600),
        ..Default::default()
    };

    assert!(!config.enable_circuit_breakers);
    assert!(!config.enable_leak_detection);
    assert!(!config.enable_memory_pressure);
}
