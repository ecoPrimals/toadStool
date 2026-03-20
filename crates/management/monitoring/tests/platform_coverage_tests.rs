// SPDX-License-Identifier: AGPL-3.0-or-later
//! Integration coverage for [`toadstool_management_monitoring::platform`] and related public types
//! used by platform resource measurement.

use std::time::Duration;
use toadstool::resources::RuntimeMetrics;
use toadstool_management_monitoring::platform::get_platform_metrics;
use toadstool_management_monitoring::{
    MonitoringConfig, MonitoringGranularity, ResourceMonitorError, ThresholdAction,
};

fn sample_monitoring_config() -> MonitoringConfig {
    MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_secs(7200),
    }
}

#[test]
fn monitoring_config_default_clone_debug_serde_roundtrip() {
    let a = MonitoringConfig::default();
    let b = a.clone();
    assert_eq!(format!("{a:?}"), format!("{b:?}"));
    let json = serde_json::to_string(&a).expect("serialize");
    let c: MonitoringConfig = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(format!("{a:?}"), format!("{c:?}"));
}

#[test]
fn monitoring_granularity_all_variants_serde_roundtrip() {
    let variants = [
        MonitoringGranularity::SubMillisecond,
        MonitoringGranularity::Millisecond,
        MonitoringGranularity::HighFrequency,
        MonitoringGranularity::Standard,
        MonitoringGranularity::LowFrequency,
        MonitoringGranularity::Custom(Duration::from_millis(42)),
    ];
    for g in variants {
        let json = serde_json::to_string(&g).expect("serialize granularity");
        let back: MonitoringGranularity = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(format!("{g:?}"), format!("{back:?}"));
    }
}

#[test]
fn threshold_action_clone_debug_serde_roundtrip() {
    for action in [
        ThresholdAction::Log,
        ThresholdAction::Alert,
        ThresholdAction::Terminate,
    ] {
        let cloned = action.clone();
        assert_eq!(format!("{action:?}"), format!("{cloned:?}"));
        let json = serde_json::to_string(&action).expect("serialize");
        let back: ThresholdAction = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(format!("{action:?}"), format!("{back:?}"));
    }
}

#[test]
fn resource_monitor_error_clone_debug_display() {
    let cases = [
        ResourceMonitorError::ProcessNotRegistered("w".to_string()),
        ResourceMonitorError::ProcessNotFound("p".to_string()),
        ResourceMonitorError::CommandExecutionFailed("cmd".to_string()),
        ResourceMonitorError::ParseError("parse".to_string()),
        ResourceMonitorError::PlatformNotSupported("x".to_string()),
        ResourceMonitorError::ResourceLimitExceeded {
            process_id: "a".to_string(),
            resource_type: "cpu".to_string(),
            current_value: 1.0,
            limit: 0.5,
        },
        ResourceMonitorError::NetworkMonitoringNotAvailable,
        ResourceMonitorError::ThresholdViolation {
            workload_id: "w".to_string(),
            resource_type: "mem".to_string(),
            current_value: 9.0,
            threshold: 8.0,
        },
        ResourceMonitorError::Other("o".to_string()),
    ];
    for err in cases {
        let c = err.clone();
        let _ = format!("{err:?}");
        assert!(!err.to_string().is_empty());
        assert_eq!(format!("{err:?}"), format!("{c:?}"));
    }
}

#[tokio::test]
async fn get_platform_metrics_own_process_default_config() {
    let pid = std::process::id();
    let config = MonitoringConfig::default();
    let metrics = get_platform_metrics(pid, &config)
        .await
        .expect("current process should be readable");
    let val = serde_json::to_value(&metrics).expect("serialize metrics");
    let back: RuntimeMetrics = serde_json::from_value(val.clone()).expect("deserialize metrics");
    let val2 = serde_json::to_value(&back).expect("re-serialize");
    assert_eq!(val, val2);
}

#[tokio::test]
async fn get_platform_metrics_own_process_custom_config() {
    let pid = std::process::id();
    let config = sample_monitoring_config();
    let metrics = get_platform_metrics(pid, &config).await.expect("metrics");
    assert!(metrics.cpu.usage_percent >= 0.0);
    assert!(metrics.memory.used_bytes > 0 || metrics.memory.usage_percent >= 0.0);
}

#[tokio::test]
async fn get_platform_metrics_network_disabled() {
    let pid = std::process::id();
    let config = MonitoringConfig {
        enable_network_monitoring: false,
        ..Default::default()
    };
    let metrics = get_platform_metrics(pid, &config).await.expect("metrics");
    let z = toadstool::resources::NetworkMetrics::default();
    assert_eq!(metrics.network.bytes_sent, z.bytes_sent);
    assert_eq!(metrics.network.bytes_received, z.bytes_received);
    assert_eq!(metrics.network.packets_sent, z.packets_sent);
    assert_eq!(metrics.network.packets_received, z.packets_received);
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn get_platform_metrics_network_enabled_linux() {
    let pid = std::process::id();
    let config = MonitoringConfig {
        enable_network_monitoring: true,
        ..Default::default()
    };
    let result = get_platform_metrics(pid, &config).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn get_platform_metrics_invalid_pid_errors() {
    let config = MonitoringConfig::default();
    let err = get_platform_metrics(4_294_967_295, &config)
        .await
        .unwrap_err();
    assert!(
        matches!(
            err,
            ResourceMonitorError::CommandExecutionFailed(_) | ResourceMonitorError::ParseError(_)
        ),
        "unexpected error: {err:?}"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn get_platform_metrics_pid_zero_linux() {
    let config = MonitoringConfig::default();
    let err = get_platform_metrics(0, &config).await.unwrap_err();
    assert!(
        matches!(
            err,
            ResourceMonitorError::CommandExecutionFailed(_) | ResourceMonitorError::ParseError(_)
        ),
        "unexpected error: {err:?}"
    );
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[tokio::test]
async fn get_platform_metrics_pid_zero_errors() {
    let config = MonitoringConfig::default();
    let err = get_platform_metrics(0, &config).await.unwrap_err();
    assert!(
        matches!(
            err,
            ResourceMonitorError::CommandExecutionFailed(_) | ResourceMonitorError::ParseError(_)
        ),
        "unexpected error: {err:?}"
    );
}

#[cfg(target_os = "linux")]
#[tokio::test]
async fn get_platform_metrics_pid_one_when_readable() {
    let config = MonitoringConfig::default();
    match get_platform_metrics(1, &config).await {
        Ok(m) => {
            let _ = format!("{:?}", m.cpu);
        }
        Err(ResourceMonitorError::CommandExecutionFailed(_))
        | Err(ResourceMonitorError::ParseError(_)) => {}
        Err(e) => panic!("unexpected error: {e:?}"),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
#[tokio::test]
async fn get_platform_metrics_unsupported_os() {
    let config = MonitoringConfig::default();
    let err = get_platform_metrics(1, &config).await.unwrap_err();
    assert!(matches!(err, ResourceMonitorError::PlatformNotSupported(_)));
}
