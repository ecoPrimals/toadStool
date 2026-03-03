// SPDX-License-Identifier: AGPL-3.0-or-later
//! Additional tests to expand monitoring coverage
//!
//! Focus areas:
//! - Error handling and error type completeness
//! - Edge cases and boundary conditions
//! - Serialization/deserialization
//! - Clone and Debug trait implementations
//! - Complex configuration scenarios

use std::time::Duration;
use toadstool_management_monitoring::*;

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_process_not_registered_error() {
    let error = ResourceMonitorError::ProcessNotRegistered("test_process".to_string());
    let error_str = error.to_string();

    assert!(error_str.contains("Process not registered"));
    assert!(error_str.contains("test_process"));
}

#[test]
fn test_process_not_found_error() {
    let error = ResourceMonitorError::ProcessNotFound("missing_process".to_string());
    let error_str = error.to_string();

    assert!(error_str.contains("Process not found"));
    assert!(error_str.contains("missing_process"));
}

#[test]
fn test_command_execution_failed_error() {
    let error = ResourceMonitorError::CommandExecutionFailed("Command failed".to_string());
    let error_str = error.to_string();

    assert!(error_str.contains("Command execution failed"));
}

#[test]
fn test_parse_error() {
    let error = ResourceMonitorError::ParseError("Invalid format".to_string());
    let error_str = error.to_string();

    assert!(error_str.contains("Parse error"));
    assert!(error_str.contains("Invalid format"));
}

#[test]
fn test_platform_not_supported_error() {
    let error = ResourceMonitorError::PlatformNotSupported("exotic_os".to_string());
    let error_str = error.to_string();

    assert!(error_str.contains("Platform not supported"));
    assert!(error_str.contains("exotic_os"));
}

#[test]
fn test_resource_limit_exceeded_error() {
    let error = ResourceMonitorError::ResourceLimitExceeded {
        process_id: "proc_123".to_string(),
        resource_type: "memory".to_string(),
        current_value: 1024.0,
        limit: 512.0,
    };
    let error_str = error.to_string();

    assert!(error_str.contains("Resource limit exceeded"));
    assert!(error_str.contains("proc_123"));
    assert!(error_str.contains("memory"));
    assert!(error_str.contains("1024"));
    assert!(error_str.contains("512"));
}

#[test]
fn test_network_monitoring_not_available_error() {
    let error = ResourceMonitorError::NetworkMonitoringNotAvailable;
    let error_str = error.to_string();

    assert!(error_str.contains("Network monitoring is not available"));
}

#[test]
fn test_threshold_violation_error() {
    let error = ResourceMonitorError::ThresholdViolation {
        workload_id: "workload_456".to_string(),
        resource_type: "cpu".to_string(),
        current_value: 95.0,
        threshold: 80.0,
    };
    let error_str = error.to_string();

    assert!(error_str.contains("Threshold violation"));
    assert!(error_str.contains("workload_456"));
    assert!(error_str.contains("cpu"));
    assert!(error_str.contains("95"));
    assert!(error_str.contains("80"));
}

#[test]
fn test_other_error() {
    let error = ResourceMonitorError::Other("Unexpected issue".to_string());
    let error_str = error.to_string();

    assert!(error_str.contains("Other error"));
    assert!(error_str.contains("Unexpected issue"));
}

#[test]
fn test_error_debug_trait() {
    let error = ResourceMonitorError::ProcessNotFound("test".to_string());
    let debug_str = format!("{:?}", error);

    assert!(debug_str.contains("ProcessNotFound"));
}

#[test]
fn test_error_clone_trait() {
    let original = ResourceMonitorError::NetworkMonitoringNotAvailable;
    let cloned = original.clone();

    assert_eq!(original.to_string(), cloned.to_string());
}

// ============================================================================
// MonitoringGranularity Edge Cases
// ============================================================================

#[test]
fn test_granularity_custom_zero() {
    let granularity = MonitoringGranularity::Custom(Duration::from_secs(0));
    let duration = granularity.to_duration();

    assert_eq!(duration, Duration::from_secs(0));
}

#[test]
fn test_granularity_custom_very_large() {
    let large_duration = Duration::from_secs(3600 * 24); // 1 day
    let granularity = MonitoringGranularity::Custom(large_duration);
    let duration = granularity.to_duration();

    assert_eq!(duration, large_duration);
}

#[test]
fn test_granularity_custom_sub_microsecond() {
    let tiny_duration = Duration::from_nanos(100);
    let granularity = MonitoringGranularity::Custom(tiny_duration);
    let duration = granularity.to_duration();

    assert_eq!(duration, tiny_duration);
}

#[test]
fn test_granularity_serialization_sub_millisecond() {
    let granularity = MonitoringGranularity::SubMillisecond;
    let json = serde_json::to_string(&granularity).expect("Should serialize");

    assert!(!json.is_empty());

    let deserialized: MonitoringGranularity =
        serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized.to_duration(), Duration::from_micros(100));
}

#[test]
fn test_granularity_serialization_custom() {
    let custom_duration = Duration::from_millis(500);
    let granularity = MonitoringGranularity::Custom(custom_duration);
    let json = serde_json::to_string(&granularity).expect("Should serialize");

    assert!(!json.is_empty());

    let deserialized: MonitoringGranularity =
        serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(deserialized.to_duration(), custom_duration);
}

#[test]
fn test_granularity_clone() {
    let original = MonitoringGranularity::HighFrequency;
    let cloned = original;

    assert_eq!(original.to_duration(), cloned.to_duration());
}

#[test]
fn test_granularity_debug() {
    let granularity = MonitoringGranularity::Standard;
    let debug_str = format!("{:?}", granularity);

    assert!(debug_str.contains("Standard"));
}

// ============================================================================
// ThresholdAction Tests
// ============================================================================

#[test]
fn test_threshold_action_log() {
    let action = ThresholdAction::Log;
    let json = serde_json::to_string(&action).expect("Should serialize");

    assert!(!json.is_empty());

    let deserialized: ThresholdAction = serde_json::from_str(&json).expect("Should deserialize");
    let debug_str = format!("{:?}", deserialized);
    assert!(debug_str.contains("Log"));
}

#[test]
fn test_threshold_action_alert() {
    let action = ThresholdAction::Alert;
    let json = serde_json::to_string(&action).expect("Should serialize");

    let deserialized: ThresholdAction = serde_json::from_str(&json).expect("Should deserialize");
    let debug_str = format!("{:?}", deserialized);
    assert!(debug_str.contains("Alert"));
}

#[test]
fn test_threshold_action_terminate() {
    let action = ThresholdAction::Terminate;
    let json = serde_json::to_string(&action).expect("Should serialize");

    let deserialized: ThresholdAction = serde_json::from_str(&json).expect("Should deserialize");
    let debug_str = format!("{:?}", deserialized);
    assert!(debug_str.contains("Terminate"));
}

#[test]
fn test_threshold_action_clone() {
    let original = ThresholdAction::Alert;
    let cloned = original.clone();

    assert_eq!(format!("{:?}", original), format!("{:?}", cloned));
}

// ============================================================================
// MonitoringConfig Tests
// ============================================================================

#[test]
fn test_monitoring_config_default_values() {
    let config = MonitoringConfig::default();

    assert_eq!(config.granularity.to_duration(), Duration::from_millis(100));
    assert!(config.enable_network_monitoring);
    assert!(config.enable_threshold_monitoring);
    assert_eq!(config.metrics_retention, Duration::from_secs(3600));
}

#[test]
fn test_monitoring_config_custom() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: false,
        threshold_action: ThresholdAction::Terminate,
        metrics_retention: Duration::from_secs(7200),
    };

    assert_eq!(config.granularity.to_duration(), Duration::from_millis(10));
    assert!(!config.enable_network_monitoring);
    assert!(!config.enable_threshold_monitoring);
    assert_eq!(config.metrics_retention, Duration::from_secs(7200));
}

#[test]
fn test_monitoring_config_serialization() {
    let config = MonitoringConfig::default();
    let json = serde_json::to_string(&config).expect("Should serialize");

    assert!(!json.is_empty());

    let deserialized: MonitoringConfig = serde_json::from_str(&json).expect("Should deserialize");
    assert_eq!(
        config.granularity.to_duration(),
        deserialized.granularity.to_duration()
    );
    assert_eq!(
        config.enable_network_monitoring,
        deserialized.enable_network_monitoring
    );
}

#[test]
fn test_monitoring_config_clone() {
    let original = MonitoringConfig::default();
    let cloned = original.clone();

    assert_eq!(
        original.granularity.to_duration(),
        cloned.granularity.to_duration()
    );
    assert_eq!(
        original.enable_network_monitoring,
        cloned.enable_network_monitoring
    );
}

#[test]
fn test_monitoring_config_debug() {
    let config = MonitoringConfig::default();
    let debug_str = format!("{:?}", config);

    assert!(debug_str.contains("MonitoringConfig"));
    assert!(debug_str.contains("granularity"));
}

#[test]
fn test_monitoring_config_all_granularities() {
    let granularities = vec![
        MonitoringGranularity::SubMillisecond,
        MonitoringGranularity::Millisecond,
        MonitoringGranularity::HighFrequency,
        MonitoringGranularity::Standard,
        MonitoringGranularity::LowFrequency,
    ];

    for granularity in granularities {
        let config = MonitoringConfig {
            granularity,
            ..Default::default()
        };

        // All configs should be valid
        let json = serde_json::to_string(&config).expect("Should serialize");
        let _deserialized: MonitoringConfig =
            serde_json::from_str(&json).expect("Should deserialize");
    }
}

#[test]
fn test_monitoring_config_all_threshold_actions() {
    let actions = vec![
        ThresholdAction::Log,
        ThresholdAction::Alert,
        ThresholdAction::Terminate,
    ];

    for action in actions {
        let config = MonitoringConfig {
            threshold_action: action,
            ..Default::default()
        };

        // All configs should be valid
        let json = serde_json::to_string(&config).expect("Should serialize");
        let _deserialized: MonitoringConfig =
            serde_json::from_str(&json).expect("Should deserialize");
    }
}

// ============================================================================
// Boundary Conditions
// ============================================================================

#[test]
fn test_zero_metrics_retention() {
    let config = MonitoringConfig {
        metrics_retention: Duration::from_secs(0),
        ..Default::default()
    };

    assert_eq!(config.metrics_retention, Duration::from_secs(0));
}

#[test]
fn test_very_long_metrics_retention() {
    let long_duration = Duration::from_secs(365 * 24 * 3600); // 1 year
    let config = MonitoringConfig {
        metrics_retention: long_duration,
        ..Default::default()
    };

    assert_eq!(config.metrics_retention, long_duration);
}

#[test]
fn test_all_monitoring_disabled() {
    let config = MonitoringConfig {
        enable_network_monitoring: false,
        enable_threshold_monitoring: false,
        ..Default::default()
    };

    assert!(!config.enable_network_monitoring);
    assert!(!config.enable_threshold_monitoring);
}

#[test]
fn test_all_monitoring_enabled() {
    let config = MonitoringConfig {
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        ..Default::default()
    };

    assert!(config.enable_network_monitoring);
    assert!(config.enable_threshold_monitoring);
}

// ============================================================================
// SystemResourceMonitor Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_creation() {
    let monitor = SystemResourceMonitor::new();

    // Monitor should be created successfully
    let debug_str = format!("{:?}", monitor);
    assert!(debug_str.contains("SystemResourceMonitor"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitoring_config_variations() {
    // Test various config combinations
    let configs = vec![
        MonitoringConfig {
            granularity: MonitoringGranularity::HighFrequency,
            enable_network_monitoring: false,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Alert,
            metrics_retention: Duration::from_secs(1800),
        },
        MonitoringConfig {
            granularity: MonitoringGranularity::LowFrequency,
            enable_network_monitoring: true,
            enable_threshold_monitoring: false,
            threshold_action: ThresholdAction::Log,
            metrics_retention: Duration::from_secs(7200),
        },
    ];

    for config in configs {
        // All configs should be valid
        let json = serde_json::to_string(&config).expect("Should serialize");
        let _deserialized: MonitoringConfig =
            serde_json::from_str(&json).expect("Should deserialize");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_default() {
    let monitor = SystemResourceMonitor::default();

    // Should create monitor with default config
    let _debug_str = format!("{:?}", monitor);
}

// ============================================================================
// Complex Scenarios
// ============================================================================

#[test]
fn test_error_type_completeness() {
    // Ensure all error types can be created and displayed
    let errors = vec![
        ResourceMonitorError::ProcessNotRegistered("test".to_string()),
        ResourceMonitorError::ProcessNotFound("test".to_string()),
        ResourceMonitorError::CommandExecutionFailed("test".to_string()),
        ResourceMonitorError::ParseError("test".to_string()),
        ResourceMonitorError::PlatformNotSupported("test".to_string()),
        ResourceMonitorError::ResourceLimitExceeded {
            process_id: "test".to_string(),
            resource_type: "test".to_string(),
            current_value: 1.0,
            limit: 0.5,
        },
        ResourceMonitorError::NetworkMonitoringNotAvailable,
        ResourceMonitorError::ThresholdViolation {
            workload_id: "test".to_string(),
            resource_type: "test".to_string(),
            current_value: 1.0,
            threshold: 0.5,
        },
        ResourceMonitorError::Other("test".to_string()),
    ];

    for error in errors {
        // All errors should display
        let _error_str = error.to_string();

        // All errors should support Debug
        let _debug_str = format!("{:?}", error);

        // All errors should clone
        let _cloned = error.clone();
    }
}

#[test]
fn test_config_scenarios() {
    // Low-latency scenario
    let low_latency = MonitoringConfig {
        granularity: MonitoringGranularity::SubMillisecond,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_secs(300),
    };
    assert_eq!(
        low_latency.granularity.to_duration(),
        Duration::from_micros(100)
    );

    // Background process scenario
    let background = MonitoringConfig {
        granularity: MonitoringGranularity::LowFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: false,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_secs(7200),
    };
    assert_eq!(background.granularity.to_duration(), Duration::from_secs(1));

    // Production scenario
    let production = MonitoringConfig {
        granularity: MonitoringGranularity::Standard,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Terminate,
        metrics_retention: Duration::from_secs(3600),
    };
    assert_eq!(
        production.granularity.to_duration(),
        Duration::from_millis(100)
    );
}

#[test]
fn test_duration_ordering() {
    let durations = vec![
        MonitoringGranularity::SubMillisecond.to_duration(),
        MonitoringGranularity::Millisecond.to_duration(),
        MonitoringGranularity::HighFrequency.to_duration(),
        MonitoringGranularity::Standard.to_duration(),
        MonitoringGranularity::LowFrequency.to_duration(),
    ];

    // Verify they are in ascending order
    for i in 0..durations.len() - 1 {
        assert!(durations[i] < durations[i + 1]);
    }
}

#[test]
fn test_serialization_roundtrip_completeness() {
    // Test all enum variants can round-trip through JSON

    // Granularities
    let granularities = vec![
        MonitoringGranularity::SubMillisecond,
        MonitoringGranularity::Millisecond,
        MonitoringGranularity::HighFrequency,
        MonitoringGranularity::Standard,
        MonitoringGranularity::LowFrequency,
        MonitoringGranularity::Custom(Duration::from_millis(250)),
    ];

    for granularity in granularities {
        let json = serde_json::to_string(&granularity).unwrap();
        let _deserialized: MonitoringGranularity = serde_json::from_str(&json).unwrap();
    }

    // Threshold actions
    let actions = vec![
        ThresholdAction::Log,
        ThresholdAction::Alert,
        ThresholdAction::Terminate,
    ];

    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let _deserialized: ThresholdAction = serde_json::from_str(&json).unwrap();
    }
}
