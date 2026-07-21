// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for monitoring module
//!
//! Tests for `MonitoringGranularity`, `MonitoringConfig`, `ThresholdAction`,
//! and `ResourceMonitorError` types.

use std::time::Duration;
use toadstool_management_monitoring::*;

// ============================================================================
// MonitoringGranularity Tests
// ============================================================================

#[test]
fn test_monitoring_granularity_sub_millisecond() {
    let granularity = MonitoringGranularity::SubMillisecond;
    let duration = granularity.to_duration();

    assert_eq!(duration, Duration::from_micros(100));
}

#[test]
fn test_monitoring_granularity_millisecond() {
    let granularity = MonitoringGranularity::Millisecond;
    let duration = granularity.to_duration();

    assert_eq!(duration, Duration::from_millis(1));
}

#[test]
fn test_monitoring_granularity_high_frequency() {
    let granularity = MonitoringGranularity::HighFrequency;
    let duration = granularity.to_duration();

    assert_eq!(duration, Duration::from_millis(10));
}

#[test]
fn test_monitoring_granularity_standard() {
    let granularity = MonitoringGranularity::Standard;
    let duration = granularity.to_duration();

    assert_eq!(duration, Duration::from_millis(100));
}

#[test]
fn test_monitoring_granularity_low_frequency() {
    let granularity = MonitoringGranularity::LowFrequency;
    let duration = granularity.to_duration();

    assert_eq!(duration, Duration::from_secs(1));
}

#[test]
fn test_monitoring_granularity_custom() {
    let custom_duration = Duration::from_millis(500);
    let granularity = MonitoringGranularity::Custom(custom_duration);
    let duration = granularity.to_duration();

    assert_eq!(duration, custom_duration);
}

#[test]
fn test_monitoring_granularity_custom_microseconds() {
    let custom_duration = Duration::from_micros(250);
    let granularity = MonitoringGranularity::Custom(custom_duration);
    let duration = granularity.to_duration();

    assert_eq!(duration, Duration::from_micros(250));
}

#[test]
fn test_monitoring_granularity_custom_seconds() {
    let custom_duration = Duration::from_secs(5);
    let granularity = MonitoringGranularity::Custom(custom_duration);
    let duration = granularity.to_duration();

    assert_eq!(duration, Duration::from_secs(5));
}

#[test]
fn test_monitoring_granularity_serialization() {
    let granularity = MonitoringGranularity::Standard;
    let serialized = serde_json::to_string(&granularity).expect("Failed to serialize");
    let deserialized: MonitoringGranularity =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(granularity.to_duration(), deserialized.to_duration());
}

#[test]
fn test_monitoring_granularity_clone() {
    let granularity = MonitoringGranularity::HighFrequency;
    let cloned = granularity;

    assert_eq!(granularity.to_duration(), cloned.to_duration());
}

#[test]
fn test_monitoring_granularity_debug() {
    let granularity = MonitoringGranularity::Standard;
    let debug_string = format!("{granularity:?}");

    assert!(debug_string.contains("Standard"));
}

#[test]
fn test_monitoring_granularity_ordering() {
    let sub_ms = MonitoringGranularity::SubMillisecond.to_duration();
    let ms = MonitoringGranularity::Millisecond.to_duration();
    let high_freq = MonitoringGranularity::HighFrequency.to_duration();
    let standard = MonitoringGranularity::Standard.to_duration();
    let low_freq = MonitoringGranularity::LowFrequency.to_duration();

    assert!(sub_ms < ms);
    assert!(ms < high_freq);
    assert!(high_freq < standard);
    assert!(standard < low_freq);
}

// ============================================================================
// MonitoringConfig Tests
// ============================================================================

#[test]
fn test_monitoring_config_default() {
    let config = MonitoringConfig::default();

    assert_eq!(config.granularity.to_duration(), Duration::from_millis(100));
    assert!(config.enable_network_monitoring);
    assert!(config.enable_threshold_monitoring);
    assert_eq!(config.metrics_retention, Duration::from_hours(1));
}

#[test]
fn test_monitoring_config_custom() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_hours(2),
    };

    assert_eq!(config.granularity.to_duration(), Duration::from_millis(10));
    assert!(!config.enable_network_monitoring);
    assert!(config.enable_threshold_monitoring);
    assert_eq!(config.metrics_retention, Duration::from_hours(2));
}

#[test]
fn test_monitoring_config_serialization() {
    let config = MonitoringConfig::default();
    let serialized = serde_json::to_string(&config).expect("Failed to serialize");
    let deserialized: MonitoringConfig =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    assert_eq!(
        config.granularity.to_duration(),
        deserialized.granularity.to_duration()
    );
    assert_eq!(config.metrics_retention, deserialized.metrics_retention);
}

#[test]
fn test_monitoring_config_clone() {
    let config = MonitoringConfig::default();
    let cloned = config.clone();

    assert_eq!(
        config.granularity.to_duration(),
        cloned.granularity.to_duration()
    );
    assert_eq!(config.metrics_retention, cloned.metrics_retention);
}

#[test]
fn test_monitoring_config_debug() {
    let config = MonitoringConfig::default();
    let debug_string = format!("{config:?}");

    assert!(debug_string.contains("MonitoringConfig"));
}

#[test]
fn test_monitoring_config_minimal_retention() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::Standard,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_mins(1), // 1 minute
    };

    assert_eq!(config.metrics_retention, Duration::from_mins(1));
}

#[test]
fn test_monitoring_config_extended_retention() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::LowFrequency,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_hours(24), // 24 hours
    };

    assert_eq!(config.metrics_retention, Duration::from_hours(24));
}

#[test]
fn test_monitoring_config_network_only() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::Standard,
        enable_network_monitoring: true,
        enable_threshold_monitoring: false,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_hours(1),
    };

    assert!(config.enable_network_monitoring);
    assert!(!config.enable_threshold_monitoring);
}

#[test]
fn test_monitoring_config_threshold_only() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::Standard,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Terminate,
        metrics_retention: Duration::from_hours(1),
    };

    assert!(!config.enable_network_monitoring);
    assert!(config.enable_threshold_monitoring);
}

#[test]
fn test_monitoring_config_all_disabled() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::LowFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: false,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_hours(1),
    };

    assert!(!config.enable_network_monitoring);
    assert!(!config.enable_threshold_monitoring);
}

// ============================================================================
// ThresholdAction Tests
// ============================================================================

#[test]
fn test_threshold_action_log() {
    let action = ThresholdAction::Log;
    let debug_string = format!("{action:?}");

    assert!(debug_string.contains("Log"));
}

#[test]
fn test_threshold_action_alert() {
    let action = ThresholdAction::Alert;
    let debug_string = format!("{action:?}");

    assert!(debug_string.contains("Alert"));
}

#[test]
fn test_threshold_action_terminate() {
    let action = ThresholdAction::Terminate;
    let debug_string = format!("{action:?}");

    assert!(debug_string.contains("Terminate"));
}

#[test]
fn test_threshold_action_serialization() {
    let action = ThresholdAction::Alert;
    let serialized = serde_json::to_string(&action).expect("Failed to serialize");
    let deserialized: ThresholdAction =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

    let debug1 = format!("{action:?}");
    let debug2 = format!("{deserialized:?}");
    assert_eq!(debug1, debug2);
}

#[test]
fn test_threshold_action_clone() {
    let action = ThresholdAction::Terminate;
    let cloned = action.clone();

    let debug1 = format!("{action:?}");
    let debug2 = format!("{cloned:?}");
    assert_eq!(debug1, debug2);
}

#[test]
fn test_threshold_action_all_variants() {
    let actions = [
        ThresholdAction::Log,
        ThresholdAction::Alert,
        ThresholdAction::Terminate,
    ];

    assert_eq!(actions.len(), 3);
}

// ============================================================================
// ResourceMonitorError Tests
// ============================================================================

#[test]
fn test_resource_monitor_error_process_not_registered() {
    let error = ResourceMonitorError::ProcessNotRegistered("proc-123".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Process not registered"));
    assert!(error_string.contains("proc-123"));
}

#[test]
fn test_resource_monitor_error_process_not_found() {
    let error = ResourceMonitorError::ProcessNotFound("proc-456".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Process not found"));
    assert!(error_string.contains("proc-456"));
}

#[test]
fn test_resource_monitor_error_command_execution_failed() {
    let error = ResourceMonitorError::CommandExecutionFailed("ps command failed".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Command execution failed"));
    assert!(error_string.contains("ps command failed"));
}

#[test]
fn test_resource_monitor_error_parse_error() {
    let error = ResourceMonitorError::ParseError("Invalid number format".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Parse error"));
    assert!(error_string.contains("Invalid number format"));
}

#[test]
fn test_resource_monitor_error_platform_not_supported() {
    let error = ResourceMonitorError::PlatformNotSupported("FreeBSD".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Platform not supported"));
    assert!(error_string.contains("FreeBSD"));
}

#[test]
fn test_resource_monitor_error_resource_limit_exceeded() {
    let error = ResourceMonitorError::ResourceLimitExceeded {
        process_id: "proc-789".to_string(),
        resource_type: "memory".to_string(),
        current_value: 1024.0,
        limit: 512.0,
    };
    let error_string = error.to_string();

    assert!(error_string.contains("Resource limit exceeded"));
    assert!(error_string.contains("proc-789"));
    assert!(error_string.contains("memory"));
    assert!(error_string.contains("1024"));
    assert!(error_string.contains("512"));
}

#[test]
fn test_resource_monitor_error_network_monitoring_not_available() {
    let error = ResourceMonitorError::NetworkMonitoringNotAvailable;
    let error_string = error.to_string();

    assert!(error_string.contains("Network monitoring is not available"));
}

#[test]
fn test_resource_monitor_error_threshold_violation() {
    let error = ResourceMonitorError::ThresholdViolation {
        workload_id: "workload-001".to_string(),
        resource_type: "cpu".to_string(),
        current_value: 95.5,
        threshold: 80.0,
    };
    let error_string = error.to_string();

    assert!(error_string.contains("Threshold violation"));
    assert!(error_string.contains("workload-001"));
    assert!(error_string.contains("cpu"));
    assert!(error_string.contains("95.5"));
    assert!(error_string.contains("80"));
}

#[test]
fn test_resource_monitor_error_other() {
    let error = ResourceMonitorError::Other("Unknown error occurred".to_string());
    let error_string = error.to_string();

    assert!(error_string.contains("Other error"));
    assert!(error_string.contains("Unknown error occurred"));
}

#[test]
fn test_resource_monitor_error_clone() {
    let error = ResourceMonitorError::ProcessNotFound("test".to_string());
    let cloned = error.clone();

    assert_eq!(error.to_string(), cloned.to_string());
}

#[test]
fn test_resource_monitor_error_debug() {
    let error = ResourceMonitorError::ParseError("test".to_string());
    let debug_string = format!("{error:?}");

    assert!(debug_string.contains("ParseError"));
}

#[test]
fn test_resource_monitor_error_display_all_variants() {
    let errors = vec![
        ResourceMonitorError::ProcessNotRegistered("p1".to_string()),
        ResourceMonitorError::ProcessNotFound("p2".to_string()),
        ResourceMonitorError::CommandExecutionFailed("cmd".to_string()),
        ResourceMonitorError::ParseError("parse".to_string()),
        ResourceMonitorError::PlatformNotSupported("platform".to_string()),
        ResourceMonitorError::ResourceLimitExceeded {
            process_id: "p".to_string(),
            resource_type: "mem".to_string(),
            current_value: 100.0,
            limit: 50.0,
        },
        ResourceMonitorError::NetworkMonitoringNotAvailable,
        ResourceMonitorError::ThresholdViolation {
            workload_id: "w".to_string(),
            resource_type: "cpu".to_string(),
            current_value: 90.0,
            threshold: 80.0,
        },
        ResourceMonitorError::Other("other".to_string()),
    ];

    for error in errors {
        let error_string = error.to_string();
        assert!(!error_string.is_empty());
    }
}

// ============================================================================
// SystemResourceMonitor Tests
// ============================================================================

#[test]
fn test_system_resource_monitor_creation() {
    let monitor = SystemResourceMonitor::new();

    // Just verify it can be created
    let debug_string = format!("{monitor:?}");
    assert!(debug_string.contains("SystemResourceMonitor"));
}

#[test]
fn test_system_resource_monitor_with_config() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_mins(30),
    };

    let monitor = SystemResourceMonitor::with_config(config);
    let debug_string = format!("{monitor:?}");

    assert!(debug_string.contains("SystemResourceMonitor"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_monitoring_config_with_all_granularities() {
    let granularities = vec![
        MonitoringGranularity::SubMillisecond,
        MonitoringGranularity::Millisecond,
        MonitoringGranularity::HighFrequency,
        MonitoringGranularity::Standard,
        MonitoringGranularity::LowFrequency,
        MonitoringGranularity::Custom(Duration::from_millis(250)),
    ];

    for granularity in granularities {
        let config = MonitoringConfig {
            granularity,
            enable_network_monitoring: true,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Log,
            metrics_retention: Duration::from_hours(1),
        };

        assert!(config.granularity.to_duration() > Duration::from_nanos(0));
    }
}

#[test]
fn test_monitoring_config_with_all_threshold_actions() {
    let actions = vec![
        ThresholdAction::Log,
        ThresholdAction::Alert,
        ThresholdAction::Terminate,
    ];

    for action in actions {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::Standard,
            enable_network_monitoring: true,
            enable_threshold_monitoring: true,
            threshold_action: action,
            metrics_retention: Duration::from_hours(1),
        };

        let debug_string = format!("{:?}", config.threshold_action);
        assert!(!debug_string.is_empty());
    }
}

#[test]
fn test_monitoring_config_production_scenario() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::Standard,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_hours(24), // 24 hours
    };

    assert_eq!(config.granularity.to_duration(), Duration::from_millis(100));
    assert!(config.enable_network_monitoring);
    assert!(config.enable_threshold_monitoring);
    assert_eq!(config.metrics_retention, Duration::from_hours(24));
}

#[test]
fn test_monitoring_config_high_performance_scenario() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::SubMillisecond,
        enable_network_monitoring: false, // Disable to reduce overhead
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Terminate, // Strict enforcement
        metrics_retention: Duration::from_mins(5),  // 5 minutes (limited)
    };

    assert_eq!(config.granularity.to_duration(), Duration::from_micros(100));
    assert!(!config.enable_network_monitoring);
    assert_eq!(config.metrics_retention, Duration::from_mins(5));
}

#[test]
fn test_monitoring_config_development_scenario() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::LowFrequency, // Reduce overhead
        enable_network_monitoring: true,
        enable_threshold_monitoring: false, // Permissive during dev
        threshold_action: ThresholdAction::Log, // Just log
        metrics_retention: Duration::from_hours(1),
    };

    assert_eq!(config.granularity.to_duration(), Duration::from_secs(1));
    assert!(!config.enable_threshold_monitoring);
}

#[test]
fn test_resource_monitor_error_converts_to_toadstool_error() {
    let err = ResourceMonitorError::ProcessNotRegistered("test-123".to_string());
    let toadstool_err: toadstool::ToadStoolError = err.into();
    let err_str = toadstool_err.to_string();
    assert!(err_str.contains("Process not registered") || err_str.contains("test-123"));
}

#[test]
fn test_monitoring_granularity_custom_zero_duration() {
    let granularity = MonitoringGranularity::Custom(Duration::ZERO);
    let duration = granularity.to_duration();
    assert_eq!(duration, Duration::ZERO);
}

#[test]
fn test_system_resource_monitor_default() {
    let monitor = SystemResourceMonitor::default();
    let debug_string = format!("{monitor:?}");
    assert!(debug_string.contains("SystemResourceMonitor"));
}
