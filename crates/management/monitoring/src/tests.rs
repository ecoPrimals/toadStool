// SPDX-License-Identifier: AGPL-3.0-only

use super::*;
use crate::process::ProcessInfo;
use crate::reporting::{memory_usage_percent, read_system_info};
use std::path::Path;
use std::time::Duration;
use toadstool::resources::{
    CpuMetrics, CpuRequirements, MemoryMetrics, MemoryRequirements, ResourceMonitor,
    ResourceRequirements, RuntimeMetrics, StorageMetrics, StorageRequirements,
};

#[test]
fn system_resource_monitor_new() {
    let monitor = SystemResourceMonitor::new();
    let _ = monitor;
}

#[test]
fn system_resource_monitor_with_config() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_secs(120),
    };
    let monitor = SystemResourceMonitor::with_config(config);
    assert_eq!(
        monitor.config.granularity,
        MonitoringGranularity::HighFrequency
    );
    assert!(!monitor.config.enable_network_monitoring);
    assert!(monitor.config.enable_threshold_monitoring);
    assert_eq!(monitor.config.threshold_action, ThresholdAction::Alert);
    assert_eq!(monitor.config.metrics_retention, Duration::from_secs(120));
}

#[test]
fn system_resource_monitor_default() {
    let monitor = SystemResourceMonitor::default();
    let _ = monitor;
}

#[tokio::test]
async fn register_and_unregister_process() {
    let monitor = SystemResourceMonitor::new();
    let path = std::path::Path::new("test_executable");
    monitor
        .register_process("workload-1", 12345, path)
        .await
        .unwrap();
    monitor.unregister_process("workload-1").await.unwrap();
}

#[tokio::test]
async fn unregister_nonexistent_returns_error() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.unregister_process("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn get_metrics_nonexistent_returns_error() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.get_metrics_async("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn start_and_stop_monitoring_loop() {
    let monitor = SystemResourceMonitor::new();
    monitor.start_monitoring_loop().await.unwrap();
    monitor.stop_monitoring_loop().await.unwrap();
}

// -------------------------------------------------------------------------
// update_config tests
// -------------------------------------------------------------------------

#[tokio::test]
async fn update_config_when_not_monitoring() {
    let mut monitor = SystemResourceMonitor::new();
    let new_config = MonitoringConfig {
        granularity: MonitoringGranularity::LowFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: false,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_secs(1800),
    };
    let result = monitor.update_config(new_config.clone()).await;
    assert!(result.is_ok());
    assert_eq!(
        monitor.config.granularity,
        MonitoringGranularity::LowFrequency
    );
    assert!(!monitor.config.enable_network_monitoring);
    assert_eq!(monitor.config.metrics_retention, Duration::from_secs(1800));
}

#[tokio::test]
async fn update_config_while_monitoring() {
    let mut monitor = SystemResourceMonitor::new();
    monitor.start_monitoring_loop().await.unwrap();

    let new_config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: true,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_secs(7200),
    };
    let result = monitor.update_config(new_config.clone()).await;
    assert!(result.is_ok());
    assert_eq!(
        monitor.config.granularity,
        MonitoringGranularity::HighFrequency
    );
    assert!(monitor.config.enable_network_monitoring);
    assert!(monitor.config.enable_threshold_monitoring);
    assert_eq!(monitor.config.threshold_action, ThresholdAction::Alert);
    assert_eq!(monitor.config.metrics_retention, Duration::from_secs(7200));

    monitor.stop_monitoring_loop().await.unwrap();
}

// -------------------------------------------------------------------------
// ResourceMonitor trait method tests
// -------------------------------------------------------------------------

#[tokio::test]
async fn resource_monitor_start_monitoring() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.start_monitoring("workload-1");
    assert!(result.is_ok());
}

#[tokio::test]
async fn resource_monitor_stop_monitoring() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.stop_monitoring("workload-1");
    assert!(result.is_ok());
    // Yield to allow the spawned task to complete
    tokio::task::yield_now().await;
}

#[tokio::test]
async fn resource_monitor_get_metrics_trait() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.get_metrics("nonexistent").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn resource_monitor_get_metrics_registered() {
    let monitor = SystemResourceMonitor::new();
    let path = Path::new("test_exec");
    monitor.register_process("w1", 12345, path).await.unwrap();
    // No metrics yet (monitoring loop not started), so get_metrics still fails
    let result = monitor.get_metrics("w1").await;
    assert!(result.is_err());
}

#[tokio::test]
async fn resource_monitor_get_system_resources() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.get_system_resources().await;
    assert!(result.is_ok());
    let resources = result.unwrap();
    assert!(resources.total_cpu_cores >= 1);
    assert!(resources.total_memory_bytes > 0);
    assert!(resources.available_memory_bytes > 0);
}

// -------------------------------------------------------------------------
// register_process edge cases
// -------------------------------------------------------------------------

#[tokio::test]
async fn register_process_path_without_filename() {
    let monitor = SystemResourceMonitor::new();
    // Path::new(".") has no file_name, uses unwrap_or_default -> ""
    let path = Path::new(".");
    let result = monitor.register_process("w1", 12345, path).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn register_process_empty_path() {
    let monitor = SystemResourceMonitor::new();
    let path = Path::new("");
    let result = monitor.register_process("w1", 12345, path).await;
    assert!(result.is_ok());
}

// -------------------------------------------------------------------------
// set_thresholds and get_metrics with monitoring loop
// -------------------------------------------------------------------------

#[tokio::test]
async fn set_thresholds_then_get_metrics_after_monitoring_tick() {
    let monitor = SystemResourceMonitor::new();
    let path = Path::new("test_exec");
    let pid = std::process::id();

    monitor.register_process("w1", pid, path).await.unwrap();
    monitor
        .set_thresholds("w1", ResourceRequirements::default())
        .await
        .unwrap();

    monitor.start_monitoring_loop().await.unwrap();
    // Poll until metrics appear (confirms at least one monitoring tick ran)
    let metrics = tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            match monitor.get_metrics_async("w1").await {
                Ok(m) => break m,
                Err(_) => tokio::task::yield_now().await,
            }
        }
    })
    .await
    .expect("metrics should appear within 500ms");
    assert!(metrics.cpu.usage_percent >= 0.0);
    assert!(
        metrics.memory.used_bytes <= metrics.memory.peak_bytes
            || metrics.memory.peak_bytes == 0
    );

    monitor.stop_monitoring_loop().await.unwrap();
}

#[tokio::test]
async fn set_thresholds_with_custom_requirements() {
    let monitor = SystemResourceMonitor::new();
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.5,
            max_cores: Some(4.0),
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 256 * 1024 * 1024,
            max_bytes: Some(8 * 1024 * 1024 * 1024),
        },
        storage: StorageRequirements {
            min_bytes: 1024 * 1024 * 1024,
            max_bytes: Some(100 * 1024 * 1024 * 1024),
            storage_type: None,
        },
        gpu: None,
        network: toadstool::resources::NetworkRequirements::default(),
    };
    let result = monitor
        .set_thresholds("workload-threshold", requirements)
        .await;
    assert!(result.is_ok());
}

// -------------------------------------------------------------------------
// Monitoring loop with threshold checking (exercises check_thresholds)
// -------------------------------------------------------------------------

#[tokio::test]
async fn monitoring_loop_with_threshold_violation_log_action() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Log,
        metrics_retention: Duration::from_secs(3600),
    };
    let monitor = SystemResourceMonitor::with_config(config);
    let path = Path::new("test_exec");
    let pid = std::process::id();

    monitor.register_process("w1", pid, path).await.unwrap();
    // Set very low memory threshold so we likely trigger violation
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.1,
            max_cores: Some(0.001),
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 1024,
            max_bytes: Some(1),
        },
        storage: StorageRequirements {
            min_bytes: 1024,
            max_bytes: Some(0),
            storage_type: None,
        },
        gpu: None,
        network: toadstool::resources::NetworkRequirements::default(),
    };
    monitor.set_thresholds("w1", requirements).await.unwrap();

    monitor.start_monitoring_loop().await.unwrap();
    // Poll until metrics appear (confirms at least one monitoring tick ran)
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if monitor.get_metrics_async("w1").await.is_ok() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("monitoring tick should complete within 500ms");
    monitor.stop_monitoring_loop().await.unwrap();
}

#[tokio::test]
async fn monitoring_loop_with_threshold_alert_action() {
    let config = MonitoringConfig {
        granularity: MonitoringGranularity::HighFrequency,
        enable_network_monitoring: false,
        enable_threshold_monitoring: true,
        threshold_action: ThresholdAction::Alert,
        metrics_retention: Duration::from_secs(3600),
    };
    let monitor = SystemResourceMonitor::with_config(config);
    let path = Path::new("test_exec");
    let pid = std::process::id();

    monitor.register_process("w1", pid, path).await.unwrap();
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.1,
            max_cores: Some(0.0001),
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 1024,
            max_bytes: Some(1),
        },
        storage: toadstool::resources::StorageRequirements::default(),
        gpu: None,
        network: toadstool::resources::NetworkRequirements::default(),
    };
    monitor.set_thresholds("w1", requirements).await.unwrap();

    monitor.start_monitoring_loop().await.unwrap();
    // Poll until metrics appear (confirms at least one monitoring tick ran)
    tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            if monitor.get_metrics_async("w1").await.is_ok() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("monitoring tick should complete within 500ms");
    monitor.stop_monitoring_loop().await.unwrap();
}

#[tokio::test]
async fn start_monitoring_loop_idempotent() {
    let monitor = SystemResourceMonitor::new();
    let r1 = monitor.start_monitoring_loop().await;
    let r2 = monitor.start_monitoring_loop().await;
    assert!(r1.is_ok());
    assert!(r2.is_ok());
    monitor.stop_monitoring_loop().await.unwrap();
}

// -------------------------------------------------------------------------
// measure_process_resources path via injected ProcessInfo (mock fields)
// -------------------------------------------------------------------------

#[tokio::test]
async fn monitoring_tick_applies_injected_process_info_fields() {
    let monitor = SystemResourceMonitor::new();
    let path = Path::new("mock_exec");
    let pid = std::process::id();
    monitor.register_process("mock-w", pid, path).await.unwrap();

    let last_cpu_time = 4_200u64;
    let memory_usage = 512 * 1024u64;
    {
        let mut map = monitor.process_map.write().await;
        map.insert(
            "mock-w".to_string(),
            ProcessInfo {
                pid,
                name: "mock_exec".to_string(),
                last_cpu_time,
                memory_usage,
                start_time: 60,
            },
        );
    }

    monitor.start_monitoring_loop().await.unwrap();
    let metrics = tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            if let Ok(m) = monitor.get_metrics_async("mock-w").await {
                return m;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("metrics within timeout");

    assert!((metrics.cpu.usage_percent - (last_cpu_time as f64 / 100.0)).abs() < f64::EPSILON);
    assert_eq!(metrics.memory.used_bytes, memory_usage);
    assert_eq!(metrics.memory.peak_bytes, memory_usage);

    monitor.stop_monitoring_loop().await.unwrap();
}

// -------------------------------------------------------------------------
// get_system_resources / read_system_info / memory_usage_percent
// -------------------------------------------------------------------------

#[tokio::test]
async fn get_system_resources_memory_percent_is_valid_range() {
    let monitor = SystemResourceMonitor::new();
    let resources = monitor.get_system_resources().await.unwrap();
    assert!(resources.total_cpu_cores >= 1);
    assert!(resources.total_memory_bytes > 0);
    assert!(resources.memory_usage_percent >= 0.0);
    assert!(resources.memory_usage_percent <= 100.0);
}

#[test]
fn memory_usage_percent_zero_total_yields_zero() {
    assert!(memory_usage_percent(0, 0).abs() < f64::EPSILON);
    assert!(memory_usage_percent(0, 1024).abs() < f64::EPSILON);
}

#[test]
fn memory_usage_percent_when_available_exceeds_total_saturates_used() {
    let p = memory_usage_percent(1024, 2048);
    assert!(p.abs() < f64::EPSILON);
}

#[test]
fn memory_usage_percent_half_used() {
    let p = memory_usage_percent(1000, 500);
    assert!((p - 50.0).abs() < 1e-9);
}

#[test]
fn read_system_info_sane_values() {
    let (cores, total, avail) = read_system_info();
    assert!(cores >= 1);
    assert!(total > 0);
    assert!(avail > 0);
}

// -------------------------------------------------------------------------
// check_thresholds boundary conditions
// -------------------------------------------------------------------------

#[test]
fn check_thresholds_cpu_exactly_at_max_cores_ok_with_terminate() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 100.0,
            cores_used: 1.0,
            cpu_time_seconds: 0.0,
        },
        ..Default::default()
    };
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.1,
            max_cores: Some(1.0),
            architecture: None,
        },
        ..ResourceRequirements::default()
    };
    let r = SystemResourceMonitor::check_thresholds(
        "w",
        &metrics,
        &requirements,
        &ThresholdAction::Terminate,
    );
    assert!(r.is_ok());
}

#[test]
fn check_thresholds_cpu_just_over_max_cores_terminate_err() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 100.01,
            cores_used: 1.0,
            cpu_time_seconds: 0.0,
        },
        ..Default::default()
    };
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.1,
            max_cores: Some(1.0),
            architecture: None,
        },
        ..ResourceRequirements::default()
    };
    let r = SystemResourceMonitor::check_thresholds(
        "w",
        &metrics,
        &requirements,
        &ThresholdAction::Terminate,
    );
    assert!(r.is_err());
}

#[test]
fn check_thresholds_memory_exactly_at_max_bytes_ok_with_terminate() {
    let metrics = RuntimeMetrics {
        memory: MemoryMetrics {
            usage_percent: 0.0,
            used_bytes: 4096,
            peak_bytes: 4096,
        },
        ..Default::default()
    };
    let requirements = ResourceRequirements {
        memory: MemoryRequirements {
            min_bytes: 1024,
            max_bytes: Some(4096),
        },
        ..ResourceRequirements::default()
    };
    let r = SystemResourceMonitor::check_thresholds(
        "w",
        &metrics,
        &requirements,
        &ThresholdAction::Terminate,
    );
    assert!(r.is_ok());
}

#[test]
fn check_thresholds_memory_one_byte_over_max_terminate_err() {
    let metrics = RuntimeMetrics {
        memory: MemoryMetrics {
            usage_percent: 0.0,
            used_bytes: 4097,
            peak_bytes: 4097,
        },
        ..Default::default()
    };
    let requirements = ResourceRequirements {
        memory: MemoryRequirements {
            min_bytes: 1024,
            max_bytes: Some(4096),
        },
        ..ResourceRequirements::default()
    };
    let r = SystemResourceMonitor::check_thresholds(
        "w",
        &metrics,
        &requirements,
        &ThresholdAction::Terminate,
    );
    assert!(r.is_err());
}

#[test]
fn check_thresholds_storage_exactly_at_max_io_ok_with_terminate() {
    let metrics = RuntimeMetrics {
        storage: StorageMetrics {
            usage_percent: 0.0,
            used_bytes: 0,
            bytes_read: 60,
            bytes_written: 40,
        },
        ..Default::default()
    };
    let requirements = ResourceRequirements {
        storage: StorageRequirements {
            min_bytes: 0,
            max_bytes: Some(100),
            storage_type: None,
        },
        ..ResourceRequirements::default()
    };
    let r = SystemResourceMonitor::check_thresholds(
        "w",
        &metrics,
        &requirements,
        &ThresholdAction::Terminate,
    );
    assert!(r.is_ok());
}

#[test]
fn check_thresholds_storage_one_byte_over_max_io_terminate_err() {
    let metrics = RuntimeMetrics {
        storage: StorageMetrics {
            usage_percent: 0.0,
            used_bytes: 0,
            bytes_read: 101,
            bytes_written: 0,
        },
        ..Default::default()
    };
    let requirements = ResourceRequirements {
        storage: StorageRequirements {
            min_bytes: 0,
            max_bytes: Some(100),
            storage_type: None,
        },
        ..ResourceRequirements::default()
    };
    let r = SystemResourceMonitor::check_thresholds(
        "w",
        &metrics,
        &requirements,
        &ThresholdAction::Terminate,
    );
    assert!(r.is_err());
}

#[test]
fn check_thresholds_terminate_returns_err_on_cpu_violation() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 200.0,
            cores_used: 2.0,
            cpu_time_seconds: 0.0,
        },
        ..Default::default()
    };
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.1,
            max_cores: Some(0.5),
            architecture: None,
        },
        ..ResourceRequirements::default()
    };
    let err = SystemResourceMonitor::check_thresholds(
        "w",
        &metrics,
        &requirements,
        &ThresholdAction::Terminate,
    );
    assert!(err.is_err());
}

// -------------------------------------------------------------------------
// Multiple process registration / deregistration
// -------------------------------------------------------------------------

#[tokio::test]
async fn multiple_processes_register_unregister_and_metrics_isolated() {
    let monitor = SystemResourceMonitor::new();
    let path = Path::new("proc_a");
    let pid = std::process::id();
    monitor.register_process("a", pid, path).await.unwrap();
    monitor.register_process("b", pid, path).await.unwrap();

    monitor.start_monitoring_loop().await.unwrap();

    tokio::time::timeout(Duration::from_millis(800), async {
        loop {
            let a_ok = monitor.get_metrics_async("a").await.is_ok();
            let b_ok = monitor.get_metrics_async("b").await.is_ok();
            if a_ok && b_ok {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("both workloads should get metrics");

    monitor.unregister_process("a").await.unwrap();
    assert!(monitor.get_metrics_async("a").await.is_err());
    assert!(monitor.get_metrics_async("b").await.is_ok());

    monitor.unregister_process("b").await.unwrap();
    assert!(monitor.get_metrics_async("b").await.is_err());

    monitor.stop_monitoring_loop().await.unwrap();
}
