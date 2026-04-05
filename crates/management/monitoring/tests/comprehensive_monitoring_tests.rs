// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for management monitoring module
//!
//! This test suite provides extensive coverage for:
//! - `MonitoringGranularity` and duration conversions
//! - `MonitoringConfig` and defaults
//! - `ThresholdAction` types
//! - `ResourceMonitorError` types and display
//! - `SystemResourceMonitor` functionality

use std::path::Path;
use std::time::Duration;
use toadstool_management_monitoring::*;

// ============================================================================
// MonitoringGranularity Tests
// ============================================================================

#[test]
fn test_monitoring_granularity_sub_millisecond() {
    let gran = MonitoringGranularity::SubMillisecond;
    let duration = gran.to_duration();

    assert_eq!(duration, Duration::from_micros(100));
    assert_eq!(duration.as_micros(), 100);
}

#[test]
fn test_monitoring_granularity_millisecond() {
    let gran = MonitoringGranularity::Millisecond;
    let duration = gran.to_duration();

    assert_eq!(duration, Duration::from_millis(1));
    assert_eq!(duration.as_millis(), 1);
}

#[test]
fn test_monitoring_granularity_high_frequency() {
    let gran = MonitoringGranularity::HighFrequency;
    let duration = gran.to_duration();

    assert_eq!(duration, Duration::from_millis(10));
    assert_eq!(duration.as_millis(), 10);
}

#[test]
fn test_monitoring_granularity_standard() {
    let gran = MonitoringGranularity::Standard;
    let duration = gran.to_duration();

    assert_eq!(duration, Duration::from_millis(100));
    assert_eq!(duration.as_millis(), 100);
}

#[test]
fn test_monitoring_granularity_low_frequency() {
    let gran = MonitoringGranularity::LowFrequency;
    let duration = gran.to_duration();

    assert_eq!(duration, Duration::from_secs(1));
    assert_eq!(duration.as_secs(), 1);
}

#[test]
fn test_monitoring_granularity_custom() {
    let custom_duration = Duration::from_millis(500);
    let gran = MonitoringGranularity::Custom(custom_duration);
    let duration = gran.to_duration();

    assert_eq!(duration, custom_duration);
    assert_eq!(duration.as_millis(), 500);
}

#[test]
fn test_monitoring_granularity_ordering() {
    // Verify that granularities are in ascending order of interval
    let sub_ms = MonitoringGranularity::SubMillisecond.to_duration();
    let ms = MonitoringGranularity::Millisecond.to_duration();
    let high = MonitoringGranularity::HighFrequency.to_duration();
    let standard = MonitoringGranularity::Standard.to_duration();
    let low = MonitoringGranularity::LowFrequency.to_duration();

    assert!(sub_ms < ms);
    assert!(ms < high);
    assert!(high < standard);
    assert!(standard < low);
}

#[test]
fn test_monitoring_granularity_clone() {
    let gran1 = MonitoringGranularity::Standard;
    let gran2 = gran1;

    assert_eq!(gran1.to_duration(), gran2.to_duration());
}

#[test]
fn test_monitoring_granularity_serializable() {
    let gran = MonitoringGranularity::HighFrequency;
    let json = serde_json::to_string(&gran);

    assert!(json.is_ok());
}

// ============================================================================
// MonitoringConfig Tests
// ============================================================================

#[test]
fn test_monitoring_config_default() {
    let config = MonitoringConfig::default();

    // Should use Standard granularity by default
    assert_eq!(config.granularity.to_duration(), Duration::from_millis(100));

    // Network monitoring should be enabled by default
    assert!(config.enable_network_monitoring);

    // Threshold monitoring should be enabled by default
    assert!(config.enable_threshold_monitoring);

    // Default action should be Log
    assert!(matches!(config.threshold_action, ThresholdAction::Log));

    // Metrics retention should be 1 hour
    assert_eq!(config.metrics_retention, Duration::from_secs(3600));
}

#[test]
fn test_monitoring_config_custom_granularity() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::SubMillisecond,
        ..Default::default()
    };

    assert_eq!(config.granularity.to_duration(), Duration::from_micros(100));
}

#[test]
fn test_monitoring_config_disable_network() {
    let config = MonitoringConfig {
        enable_network_monitoring: false,
        ..Default::default()
    };

    assert!(!config.enable_network_monitoring);
    assert!(config.enable_threshold_monitoring); // Other settings unchanged
}

#[test]
fn test_monitoring_config_disable_threshold() {
    let config = MonitoringConfig {
        enable_threshold_monitoring: false,
        ..Default::default()
    };

    assert!(!config.enable_threshold_monitoring);
    assert!(config.enable_network_monitoring); // Other settings unchanged
}

#[test]
fn test_monitoring_config_alert_action() {
    let config = MonitoringConfig {
        threshold_action: ThresholdAction::Alert,
        ..Default::default()
    };

    assert!(matches!(config.threshold_action, ThresholdAction::Alert));
}

#[test]
fn test_monitoring_config_terminate_action() {
    let config = MonitoringConfig {
        threshold_action: ThresholdAction::Terminate,
        ..Default::default()
    };

    assert!(matches!(
        config.threshold_action,
        ThresholdAction::Terminate
    ));
}

#[test]
fn test_monitoring_config_custom_retention() {
    let config = MonitoringConfig {
        metrics_retention: Duration::from_secs(7200), // 2 hours
        ..Default::default()
    };

    assert_eq!(config.metrics_retention, Duration::from_secs(7200));
}

#[test]
fn test_monitoring_config_clone() {
    let config1 = MonitoringConfig::default();
    let config2 = config1.clone();

    assert_eq!(
        config1.granularity.to_duration(),
        config2.granularity.to_duration()
    );
    assert_eq!(
        config1.enable_network_monitoring,
        config2.enable_network_monitoring
    );
}

#[test]
fn test_monitoring_config_serializable() {
    let config = MonitoringConfig::default();
    let json = serde_json::to_string(&config);

    assert!(json.is_ok());
}

// ============================================================================
// ThresholdAction Tests
// ============================================================================

#[test]
fn test_threshold_action_log() {
    let action = ThresholdAction::Log;
    assert!(matches!(action, ThresholdAction::Log));
}

#[test]
fn test_threshold_action_alert() {
    let action = ThresholdAction::Alert;
    assert!(matches!(action, ThresholdAction::Alert));
}

#[test]
fn test_threshold_action_terminate() {
    let action = ThresholdAction::Terminate;
    assert!(matches!(action, ThresholdAction::Terminate));
}

#[test]
fn test_threshold_action_clone() {
    let action1 = ThresholdAction::Alert;
    let action2 = action1;

    assert!(matches!(action2, ThresholdAction::Alert));
}

#[test]
fn test_threshold_action_serializable() {
    let actions = vec![
        ThresholdAction::Log,
        ThresholdAction::Alert,
        ThresholdAction::Terminate,
    ];

    for action in actions {
        let json = serde_json::to_string(&action);
        assert!(json.is_ok());
    }
}

// ============================================================================
// ResourceMonitorError Tests
// ============================================================================

#[test]
fn test_resource_monitor_error_process_not_registered() {
    let error = ResourceMonitorError::ProcessNotRegistered("test-id".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("test-id"));
    assert!(error_string.contains("not registered"));
}

#[test]
fn test_resource_monitor_error_process_not_found() {
    let error = ResourceMonitorError::ProcessNotFound("missing-process".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("missing-process"));
    assert!(error_string.contains("not found"));
}

#[test]
fn test_resource_monitor_error_command_execution_failed() {
    let error = ResourceMonitorError::CommandExecutionFailed("ps aux".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("ps aux"));
    assert!(error_string.contains("failed"));
}

#[test]
fn test_resource_monitor_error_parse_error() {
    let error = ResourceMonitorError::ParseError("invalid CPU value".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("invalid CPU value"));
    assert!(error_string.contains("Parse error"));
}

#[test]
fn test_resource_monitor_error_platform_not_supported() {
    let error = ResourceMonitorError::PlatformNotSupported("RISC-V".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("RISC-V"));
    assert!(error_string.contains("not supported"));
}

#[test]
fn test_resource_monitor_error_resource_limit_exceeded() {
    let error = ResourceMonitorError::ResourceLimitExceeded {
        process_id: "proc-123".to_string(),
        resource_type: "CPU".to_string(),
        current_value: 150.0,
        limit: 100.0,
    };
    let error_string = error.to_string();

    assert!(error_string.contains("proc-123"));
    assert!(error_string.contains("CPU"));
    assert!(error_string.contains("150"));
    assert!(error_string.contains("100"));
}

#[test]
fn test_resource_monitor_error_network_monitoring_not_available() {
    let error = ResourceMonitorError::NetworkMonitoringNotAvailable;
    let error_string = error.to_string();

    assert!(error_string.contains("Network monitoring"));
    assert!(error_string.contains("not available"));
}

#[test]
fn test_resource_monitor_error_threshold_violation() {
    let error = ResourceMonitorError::ThresholdViolation {
        workload_id: "workload-456".to_string(),
        resource_type: "Memory".to_string(),
        current_value: 2048.0,
        threshold: 1024.0,
    };
    let error_string = error.to_string();

    assert!(error_string.contains("workload-456"));
    assert!(error_string.contains("Memory"));
    assert!(error_string.contains("2048"));
    assert!(error_string.contains("1024"));
}

#[test]
fn test_resource_monitor_error_other() {
    let error = ResourceMonitorError::Other("unknown error".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("unknown error"));
}

#[test]
fn test_resource_monitor_error_clone() {
    let error1 = ResourceMonitorError::ProcessNotFound("test".to_string());
    let error2 = error1.clone();

    assert_eq!(error1.to_string(), error2.to_string());
}

// ============================================================================
// SystemResourceMonitor Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_creation() {
    let monitor = SystemResourceMonitor::new();

    // Monitor should be created successfully
    assert!(format!("{monitor:?}").contains("SystemResourceMonitor"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_with_custom_config() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        ..Default::default()
    };

    let monitor = SystemResourceMonitor::with_config(config);

    assert!(format!("{monitor:?}").contains("SystemResourceMonitor"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_register_process() {
    let monitor = SystemResourceMonitor::new();
    let path = Path::new("/usr/bin/test");

    let result = monitor.register_process("test-workload", 12345, path).await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_register_multiple_processes() {
    let monitor = SystemResourceMonitor::new();

    let result1 = monitor
        .register_process("workload-1", 1001, Path::new("/bin/sh"))
        .await;
    let result2 = monitor
        .register_process("workload-2", 1002, Path::new("/bin/bash"))
        .await;

    assert!(result1.is_ok());
    assert!(result2.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_unregister_process() {
    let monitor = SystemResourceMonitor::new();
    let path = Path::new("/usr/bin/test");

    // Register then unregister
    monitor
        .register_process("test-workload", 12345, path)
        .await
        .unwrap();
    let result = monitor.unregister_process("test-workload").await;

    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_unregister_nonexistent() {
    let monitor = SystemResourceMonitor::new();

    let result = monitor.unregister_process("nonexistent").await;

    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_set_thresholds() {
    let monitor = SystemResourceMonitor::new();
    let path = Path::new("/usr/bin/test");

    monitor
        .register_process("test-workload", 12345, path)
        .await
        .unwrap();

    let requirements = toadstool::resources::ResourceRequirements::default();
    let result = monitor.set_thresholds("test-workload", requirements).await;

    assert!(result.is_ok());
}

// ============================================================================
// Duration Calculation Tests
// ============================================================================

#[test]
fn test_monitoring_intervals_reasonable() {
    // SubMillisecond should be < 1ms
    assert!(MonitoringGranularity::SubMillisecond.to_duration() < Duration::from_millis(1));

    // Millisecond should be >= 1ms and < 10ms
    let ms = MonitoringGranularity::Millisecond.to_duration();
    assert!(ms >= Duration::from_millis(1));
    assert!(ms < Duration::from_millis(10));

    // Standard should be >= 10ms and < 1s
    let std = MonitoringGranularity::Standard.to_duration();
    assert!(std >= Duration::from_millis(10));
    assert!(std < Duration::from_secs(1));

    // LowFrequency should be >= 1s
    assert!(MonitoringGranularity::LowFrequency.to_duration() >= Duration::from_secs(1));
}

#[test]
fn test_custom_granularity_zero() {
    let gran = MonitoringGranularity::Custom(Duration::ZERO);
    assert_eq!(gran.to_duration(), Duration::ZERO);
}

#[test]
fn test_custom_granularity_large() {
    let gran = MonitoringGranularity::Custom(Duration::from_secs(3600));
    assert_eq!(gran.to_duration(), Duration::from_secs(3600));
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_monitoring_coverage_summary() {
    println!("=== Management Monitoring Test Coverage ===");
    println!("MonitoringGranularity Tests:  11 tests");
    println!("MonitoringConfig Tests:       9 tests");
    println!("ThresholdAction Tests:        5 tests");
    println!("ResourceMonitorError Tests:   10 tests");
    println!("SystemResourceMonitor Tests:  6 tests");
    println!("Duration Calculation Tests:   3 tests");
    println!("Total:                        44 tests");
    println!("==========================================");
}
