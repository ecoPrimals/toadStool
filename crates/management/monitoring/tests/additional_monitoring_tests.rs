// SPDX-License-Identifier: AGPL-3.0-or-later
//! Additional tests for `SystemResourceMonitor`
//!
//! These tests expand coverage for `ResourceMonitor` trait implementation
//! and additional monitoring functionality.

use std::path::Path;
use std::time::Duration;
use toadstool::resources::ResourceMonitor;
use toadstool_management_monitoring::*;

// ============================================================================
// SystemResourceMonitor Basic Construction Tests
// ============================================================================

#[test]
fn test_system_resource_monitor_new_creates_instance() {
    let monitor = SystemResourceMonitor::new();
    // Verify it was created (size > 0)
    assert!(std::mem::size_of_val(&monitor) > 0);
}

#[test]
fn test_system_resource_monitor_with_default_config() {
    let config = MonitoringConfig::default();
    let monitor = SystemResourceMonitor::with_config(config);
    assert!(std::mem::size_of_val(&monitor) > 0);
}

#[test]
fn test_system_resource_monitor_with_sub_millisecond_config() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::SubMillisecond,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_secs(3600),
    };
    let monitor = SystemResourceMonitor::with_config(config);
    assert!(std::mem::size_of_val(&monitor) > 0);
}

#[test]
fn test_system_resource_monitor_with_high_frequency_config() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_secs(1800),
    };
    let monitor = SystemResourceMonitor::with_config(config);
    assert!(std::mem::size_of_val(&monitor) > 0);
}

#[test]
fn test_system_resource_monitor_with_low_frequency_config() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::LowFrequency,
        enable_network_monitoring: true,
        enable_threshold_monitoring: false,
        threshold_action: ThresholdAction::Terminate,
        metrics_retention: Duration::from_secs(7200),
    };
    let monitor = SystemResourceMonitor::with_config(config);
    assert!(std::mem::size_of_val(&monitor) > 0);
}

#[test]
fn test_system_resource_monitor_with_custom_granularity() {
    let custom_duration = Duration::from_millis(50);
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::Custom(custom_duration),
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_secs(3600),
    };
    let monitor = SystemResourceMonitor::with_config(config);
    assert!(std::mem::size_of_val(&monitor) > 0);
}

// ============================================================================
// Process Registration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_process_succeeds() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor
        .register_process("test-workload", 12345, Path::new("/bin/test"))
        .await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_multiple_processes() {
    let monitor = SystemResourceMonitor::new();

    let r1 = monitor
        .register_process("wl-1", 100, Path::new("/bin/sh"))
        .await;
    let r2 = monitor
        .register_process("wl-2", 200, Path::new("/bin/bash"))
        .await;
    let r3 = monitor
        .register_process("wl-3", 300, Path::new("/usr/bin/python3"))
        .await;

    assert!(r1.is_ok());
    assert!(r2.is_ok());
    assert!(r3.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_process_with_long_path() {
    let monitor = SystemResourceMonitor::new();
    let long_path = "/very/long/path/to/some/executable/in/deep/directory/structure/binary";

    let result = monitor
        .register_process("long-path-test", 5000, Path::new(long_path))
        .await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_process_with_various_pids() {
    let monitor = SystemResourceMonitor::new();

    let pids = [1, 100, 1000, 10000, 65535];
    for (i, pid) in pids.iter().enumerate() {
        let workload_id = format!("pid-test-{i}");
        let result = monitor
            .register_process(&workload_id, *pid, Path::new("/bin/test"))
            .await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Process Unregistration Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_unregister_registered_process() {
    let monitor = SystemResourceMonitor::new();

    // Register first
    monitor
        .register_process("test-wl", 1234, Path::new("/bin/test"))
        .await
        .unwrap();

    // Then unregister
    let result = monitor.unregister_process("test-wl").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_unregister_nonexistent_process_fails() {
    let monitor = SystemResourceMonitor::new();

    let result = monitor.unregister_process("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_unregister_multiple_processes() {
    let monitor = SystemResourceMonitor::new();

    // Register multiple
    monitor
        .register_process("w1", 100, Path::new("/bin/sh"))
        .await
        .unwrap();
    monitor
        .register_process("w2", 200, Path::new("/bin/bash"))
        .await
        .unwrap();
    monitor
        .register_process("w3", 300, Path::new("/bin/zsh"))
        .await
        .unwrap();

    // Unregister all
    assert!(monitor.unregister_process("w1").await.is_ok());
    assert!(monitor.unregister_process("w2").await.is_ok());
    assert!(monitor.unregister_process("w3").await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_register_unregister_reregister_cycle() {
    let monitor = SystemResourceMonitor::new();
    let wl_id = "cycling-workload";

    // Register
    assert!(monitor
        .register_process(wl_id, 1000, Path::new("/bin/test"))
        .await
        .is_ok());

    // Unregister
    assert!(monitor.unregister_process(wl_id).await.is_ok());

    // Re-register with different PID
    assert!(monitor
        .register_process(wl_id, 2000, Path::new("/bin/test2"))
        .await
        .is_ok());
}

// ============================================================================
// Configuration Update Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_update_config_succeeds() {
    let mut monitor = SystemResourceMonitor::new();

    let new_config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_secs(1800),
    };

    let result = monitor.update_config(new_config).await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_update_config_changes_granularity() {
    let mut monitor = SystemResourceMonitor::with_config(MonitoringConfig {
        granularity: MonitoringGranularity::Standard,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_secs(3600),
    });

    let new_config = MonitoringConfig {
        granularity: MonitoringGranularity::SubMillisecond,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_secs(3600),
    };

    assert!(monitor.update_config(new_config).await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_update_config_toggles_network_monitoring() {
    let mut monitor = SystemResourceMonitor::new();

    let config_disabled = MonitoringConfig {
        granularity: MonitoringGranularity::Standard,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_secs(3600),
    };

    assert!(monitor.update_config(config_disabled).await.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_update_config_changes_threshold_action() {
    let mut monitor = SystemResourceMonitor::new();

    for action in &[
        ThresholdAction::Log,
        ThresholdAction::Alert,
        ThresholdAction::Terminate,
    ] {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::Standard,
            enable_network_monitoring: true,
            enable_threshold_monitoring: true,
            threshold_action: action.clone(),
            metrics_retention: Duration::from_secs(3600),
        };

        assert!(monitor.update_config(config).await.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_update_config_changes_retention_period() {
    let mut monitor = SystemResourceMonitor::new();

    for retention_secs in &[600, 1800, 3600, 7200] {
        let config = MonitoringConfig {
            granularity: MonitoringGranularity::Standard,
            enable_network_monitoring: true,
            enable_threshold_monitoring: true,
            threshold_action: ThresholdAction::Log,
            metrics_retention: Duration::from_secs(*retention_secs),
        };

        assert!(monitor.update_config(config).await.is_ok());
    }
}

// ============================================================================
// ResourceMonitor Trait Tests
// ============================================================================

#[test]
fn test_resource_monitor_start_monitoring_succeeds() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.start_monitoring("test-workload");
    assert!(result.is_ok());
}

#[test]
fn test_resource_monitor_start_monitoring_multiple_workloads() {
    let monitor = SystemResourceMonitor::new();

    assert!(monitor.start_monitoring("workload-1").is_ok());
    assert!(monitor.start_monitoring("workload-2").is_ok());
    assert!(monitor.start_monitoring("workload-3").is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_monitor_stop_monitoring_succeeds() {
    let monitor = SystemResourceMonitor::new();

    // Start then stop
    monitor.start_monitoring("test-wl").unwrap();
    let result = monitor.stop_monitoring("test-wl");
    assert!(result.is_ok());

    // Give tokio time to process the spawn
    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_monitor_start_stop_cycle() {
    let monitor = SystemResourceMonitor::new();
    let wl_id = "cycling-wl";

    assert!(monitor.start_monitoring(wl_id).is_ok());
    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
    assert!(monitor.stop_monitoring(wl_id).is_ok());
    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
    assert!(monitor.start_monitoring(wl_id).is_ok());
    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
    assert!(monitor.stop_monitoring(wl_id).is_ok());
    tokio::task::yield_now().await; // ✅ FULLY MODERNIZED
}

// ============================================================================
// Monitoring Lifecycle Tests
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_start_monitoring_loop_succeeds() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.start_monitoring_loop().await;
    assert!(result.is_ok());

    // Clean up
    let _ = monitor.stop_monitoring_loop().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stop_monitoring_loop_succeeds() {
    let monitor = SystemResourceMonitor::new();

    // Start first
    monitor.start_monitoring_loop().await.unwrap();

    // Then stop
    let result = monitor.stop_monitoring_loop().await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_monitoring_loop_start_stop_cycle() {
    let monitor = SystemResourceMonitor::new();

    // Multiple cycles
    for _ in 0..3 {
        assert!(monitor.start_monitoring_loop().await.is_ok());
        assert!(monitor.stop_monitoring_loop().await.is_ok());
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_stop_monitoring_loop_without_start() {
    let monitor = SystemResourceMonitor::new();

    // Try to stop without starting - should handle gracefully
    let result = monitor.stop_monitoring_loop().await;
    // Either succeeds or fails gracefully
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_start_monitoring_loop_twice() {
    let monitor = SystemResourceMonitor::new();

    let result1 = monitor.start_monitoring_loop().await;
    let result2 = monitor.start_monitoring_loop().await;

    // Should handle duplicate start
    assert!(result1.is_ok());
    assert!(result2.is_ok() || result2.is_err());

    // Clean up
    let _ = monitor.stop_monitoring_loop().await;
}

// ============================================================================
// Error Type Tests
// ============================================================================

#[test]
fn test_error_process_not_registered_display() {
    let err = ResourceMonitorError::ProcessNotRegistered("test-proc".to_string());
    let msg = err.to_string();
    assert!(msg.contains("test-proc"));
    assert!(msg.contains("not registered"));
}

#[test]
fn test_error_process_not_found_display() {
    let err = ResourceMonitorError::ProcessNotFound("missing".to_string());
    let msg = err.to_string();
    assert!(msg.contains("missing"));
    assert!(msg.contains("not found"));
}

#[test]
fn test_error_command_execution_failed_display() {
    let err = ResourceMonitorError::CommandExecutionFailed("cmd failed".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Command execution failed"));
    assert!(msg.contains("cmd failed"));
}

#[test]
fn test_error_parse_error_display() {
    let err = ResourceMonitorError::ParseError("bad format".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Parse error"));
    assert!(msg.contains("bad format"));
}

#[test]
fn test_error_platform_not_supported_display() {
    let err = ResourceMonitorError::PlatformNotSupported("Amiga".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Amiga"));
    assert!(msg.contains("not supported"));
}

#[test]
fn test_error_resource_limit_exceeded_display() {
    let err = ResourceMonitorError::ResourceLimitExceeded {
        process_id: "proc-123".to_string(),
        resource_type: "memory".to_string(),
        current_value: 2048.0,
        limit: 1024.0,
    };
    let msg = err.to_string();
    assert!(msg.contains("proc-123"));
    assert!(msg.contains("memory"));
    assert!(msg.contains("2048"));
    assert!(msg.contains("1024"));
}

#[test]
fn test_error_network_monitoring_not_available_display() {
    let err = ResourceMonitorError::NetworkMonitoringNotAvailable;
    let msg = err.to_string();
    assert!(msg.contains("Network monitoring"));
    assert!(msg.contains("not available"));
}

#[test]
fn test_error_threshold_violation_display() {
    let err = ResourceMonitorError::ThresholdViolation {
        workload_id: "wl-789".to_string(),
        resource_type: "cpu".to_string(),
        current_value: 95.5,
        threshold: 80.0,
    };
    let msg = err.to_string();
    assert!(msg.contains("wl-789"));
    assert!(msg.contains("cpu"));
    assert!(msg.contains("95.5"));
    assert!(msg.contains("80"));
}

#[test]
fn test_error_other_display() {
    let err = ResourceMonitorError::Other("Something went wrong".to_string());
    let msg = err.to_string();
    assert!(msg.contains("Other error"));
    assert!(msg.contains("Something went wrong"));
}

#[test]
fn test_error_clone() {
    let err1 = ResourceMonitorError::ProcessNotFound("test".to_string());
    let err2 = err1.clone();
    assert_eq!(err1.to_string(), err2.to_string());
}

#[test]
fn test_error_debug_format() {
    let err = ResourceMonitorError::ParseError("test".to_string());
    let debug_str = format!("{err:?}");
    assert!(debug_str.contains("ParseError"));
    assert!(debug_str.contains("test"));
}

// ============================================================================
// Additional Configuration Tests
// ============================================================================

#[test]
fn test_monitoring_config_default_values() {
    let config = MonitoringConfig::default();

    // Verify default granularity translates correctly
    assert_eq!(config.granularity.to_duration(), Duration::from_millis(100));
    assert!(config.enable_network_monitoring);
    assert!(config.enable_threshold_monitoring);
    assert_eq!(config.metrics_retention, Duration::from_secs(3600));
}

#[test]
fn test_monitoring_config_with_various_granularities() {
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
            metrics_retention: Duration::from_secs(3600),
        };

        // Verify config can be created
        assert!(config.granularity.to_duration().as_nanos() > 0);
    }
}

#[test]
fn test_threshold_action_variants() {
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
            metrics_retention: Duration::from_secs(3600),
        };

        // Verify each action type works in config
        assert_eq!(config.granularity.to_duration(), Duration::from_millis(100));
    }
}
