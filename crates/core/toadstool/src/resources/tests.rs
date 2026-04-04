// SPDX-License-Identifier: AGPL-3.0-only
#![expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Resource module tests
//!
//! Tests use both submodule paths and re-exported API (mod.rs pub use) for coverage.

use super::monitoring::{ResourceMonitor, SystemResourceMonitor};
use super::types::*;

// --- Tests exercising resources through re-export path (mod.rs coverage) ---
// These use the public API as re-exported by mod.rs

#[test]
fn test_resource_requirements_via_reexport() {
    use super::ResourceRequirements;
    let req = ResourceRequirements::default();
    assert_eq!(req.cpu.min_cores, 1.0);
    assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
}

#[test]
fn test_resource_usage_via_reexport() {
    use super::ResourceUsage;
    let usage = ResourceUsage::default();
    assert!(usage.is_empty());
}

#[test]
fn test_runtime_metrics_via_reexport() {
    use super::RuntimeMetrics;
    let m = RuntimeMetrics::default();
    assert_eq!(m.cpu.usage_percent, 0.0);
}

#[test]
fn test_system_resources_via_reexport() {
    use super::SystemResources;
    let sr = SystemResources::default();
    assert_eq!(sr.total_cpu_cores, 1);
}

#[test]
fn test_resource_limits_via_reexport() {
    use super::ResourceLimits;
    let limits = ResourceLimits::default();
    assert!(limits.execution_timeout.is_some());
}

#[test]
fn test_resource_requirements_serde_roundtrip_via_reexport() {
    use super::ResourceRequirements;
    let req = ResourceRequirements::default();
    let json = serde_json::to_string(&req).expect("serialize");
    let parsed: ResourceRequirements = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(parsed.cpu.min_cores, req.cpu.min_cores);
}

#[test]
fn test_cpu_requirements_via_reexport() {
    use super::CpuRequirements;
    let cpu = CpuRequirements::default();
    assert_eq!(cpu.min_cores, 1.0);
}

#[test]
fn test_memory_requirements_via_reexport() {
    use super::MemoryRequirements;
    let mem = MemoryRequirements::default();
    assert_eq!(mem.min_bytes, 1024 * 1024 * 1024);
}

#[test]
fn test_storage_requirements_via_reexport() {
    use super::StorageRequirements;
    let storage = StorageRequirements::default();
    assert_eq!(storage.min_bytes, 1024 * 1024 * 1024);
}

#[test]
fn test_network_requirements_via_reexport() {
    use super::NetworkRequirements;
    let net = NetworkRequirements::default();
    assert!(net.min_bandwidth.is_none());
}

#[test]
fn test_gpu_requirements_via_reexport() {
    use super::GpuRequirements;
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: Some(2),
        gpu_type: Some("CUDA".to_string()),
        min_memory_bytes: Some(1024 * 1024 * 1024),
    };
    assert_eq!(gpu.min_units, 1);
}

#[test]
fn test_resource_monitor_trait_via_reexport() {
    use super::ResourceMonitor;
    let _monitor = SystemResourceMonitor::new();
    // Trait object through re-export
    let _: &dyn ResourceMonitor = &SystemResourceMonitor::default();
}

#[test]
fn test_load_averages_via_reexport() {
    use super::LoadAverages;
    let load = LoadAverages {
        one_minute: 1.0,
        five_minutes: 0.8,
        fifteen_minutes: 0.5,
    };
    assert!((load.one_minute - 1.0).abs() < 0.01);
}

#[test]
fn test_network_stats_via_reexport() {
    use super::NetworkStats;
    let stats = NetworkStats {
        bytes_received: 100,
        bytes_transmitted: 200,
        packets_received: 10,
        packets_transmitted: 20,
        interfaces: 1,
    };
    assert_eq!(stats.bytes_received, 100);
}

#[test]
fn test_process_info_via_reexport() {
    use super::{ProcessInfo, ProcessStatus};
    let info = ProcessInfo {
        workload_id: "w1".to_string(),
        process_count: 1,
        total_cpu_time: 0.5,
        memory_usage: 1024,
        status: ProcessStatus::Running,
    };
    assert_eq!(info.workload_id, "w1");
}

#[test]
fn test_timing_metrics_via_reexport() {
    use super::TimingMetrics;
    let t = TimingMetrics::default();
    assert_eq!(t.duration.as_secs(), 0);
}

#[test]
fn test_cpu_metrics_via_reexport() {
    use super::CpuMetrics;
    let m = CpuMetrics::default();
    assert_eq!(m.usage_percent, 0.0);
}

#[test]
fn test_memory_metrics_via_reexport() {
    use super::MemoryMetrics;
    let m = MemoryMetrics::default();
    assert_eq!(m.used_bytes, 0);
}

#[test]
fn test_storage_metrics_via_reexport() {
    use super::StorageMetrics;
    let m = StorageMetrics::default();
    assert_eq!(m.bytes_read, 0);
}

#[test]
fn test_network_metrics_via_reexport() {
    use super::NetworkMetrics;
    let m = NetworkMetrics::default();
    assert_eq!(m.bytes_sent, 0);
}

#[test]
fn test_gpu_metrics_via_reexport() {
    use super::GpuMetrics;
    let m = GpuMetrics::default();
    assert_eq!(m.memory_used_bytes, 0);
}

#[test]
fn test_cpu_limits_via_reexport() {
    use super::CpuLimits;
    let c = CpuLimits::default();
    assert!(c.max_cores.is_none());
}

#[test]
fn test_memory_limits_via_reexport() {
    use super::MemoryLimits;
    let m = MemoryLimits::default();
    assert!(m.max_bytes.is_none());
}

#[test]
fn test_storage_limits_via_reexport() {
    use super::StorageLimits;
    let s = StorageLimits::default();
    assert!(s.max_bytes.is_none());
}

#[test]
fn test_network_limits_via_reexport() {
    use super::NetworkLimits;
    let n = NetworkLimits::default();
    assert!(n.max_bandwidth.is_none());
}

#[test]
fn test_resource_requirements_default() {
    let req = ResourceRequirements::default();
    assert_eq!(req.cpu.min_cores, 1.0);
    assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.storage.min_bytes, 1024 * 1024 * 1024);
    assert!(req.gpu.is_none());
}

#[test]
fn test_cpu_requirements_default() {
    let cpu = CpuRequirements::default();
    assert_eq!(cpu.min_cores, 1.0);
    assert!(cpu.max_cores.is_none());
    assert!(cpu.architecture.is_none());
}

#[test]
fn test_memory_requirements_default() {
    let mem = MemoryRequirements::default();
    assert_eq!(mem.min_bytes, 1024 * 1024 * 1024);
    assert!(mem.max_bytes.is_none());
}

#[test]
fn test_storage_requirements_default() {
    let storage = StorageRequirements::default();
    assert_eq!(storage.min_bytes, 1024 * 1024 * 1024);
    assert!(storage.max_bytes.is_none());
    assert!(storage.storage_type.is_none());
}

#[test]
fn test_network_requirements_default() {
    let network = NetworkRequirements::default();
    assert!(network.min_bandwidth.is_none());
    assert!(network.max_bandwidth.is_none());
    assert!(network.max_latency_ms.is_none());
}

#[test]
fn test_gpu_requirements() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: Some(4),
        gpu_type: Some("NVIDIA".to_string()),
        min_memory_bytes: Some(2 * 1024 * 1024 * 1024),
    };

    assert_eq!(gpu.min_memory_bytes, Some(2 * 1024 * 1024 * 1024));
    assert_eq!(gpu.gpu_type, Some("NVIDIA".to_string()));
}

#[test]
fn test_resource_requirements_serialization() {
    let req = ResourceRequirements::default();
    let json = serde_json::to_string(&req).expect("Failed to serialize");
    let deserialized: ResourceRequirements =
        serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.cpu.min_cores, req.cpu.min_cores);
    assert_eq!(deserialized.memory.min_bytes, req.memory.min_bytes);
}

#[test]
fn test_cpu_requirements_with_architecture() {
    let cpu = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(8.0),
        architecture: Some("x86_64".to_string()),
    };

    assert_eq!(cpu.min_cores, 4.0);
    assert_eq!(cpu.max_cores, Some(8.0));
    assert_eq!(cpu.architecture, Some("x86_64".to_string()));
}

#[test]
fn test_memory_requirements_with_max() {
    let mem = MemoryRequirements {
        min_bytes: 2 * 1024 * 1024 * 1024,
        max_bytes: Some(4 * 1024 * 1024 * 1024),
    };

    assert_eq!(mem.min_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(mem.max_bytes, Some(4 * 1024 * 1024 * 1024));
}

#[test]
fn test_storage_requirements_with_type() {
    let storage = StorageRequirements {
        min_bytes: 10 * 1024 * 1024 * 1024,
        max_bytes: Some(50 * 1024 * 1024 * 1024),
        storage_type: Some("SSD".to_string()),
    };

    assert_eq!(storage.storage_type, Some("SSD".to_string()));
}

#[test]
fn test_network_requirements_with_constraints() {
    let network = NetworkRequirements {
        min_bandwidth: Some(1024 * 1024),
        max_bandwidth: Some(10 * 1024 * 1024),
        max_latency_ms: Some(100),
    };

    assert_eq!(network.min_bandwidth, Some(1024 * 1024));
    assert_eq!(network.max_latency_ms, Some(100));
}

#[test]
fn test_resource_requirements_validate_ok() {
    let req = ResourceRequirements::default();
    assert!(req.validate().is_ok());
}

#[test]
fn test_resource_requirements_validate_zero_cpu() {
    let mut req = ResourceRequirements::default();
    req.cpu.min_cores = 0.0;
    let result = req.validate();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("cpu")
    );
}

#[test]
fn test_resource_requirements_validate_zero_memory() {
    let mut req = ResourceRequirements::default();
    req.memory.min_bytes = 0;
    let result = req.validate();
    assert!(result.is_err());
    assert!(
        result
            .unwrap_err()
            .to_string()
            .to_lowercase()
            .contains("memory")
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_creation() {
    let _monitor = SystemResourceMonitor::new();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_default() {
    let _monitor = SystemResourceMonitor::default();
}

#[test]
fn test_resource_usage_is_empty() {
    let usage = ResourceUsage::default();
    assert!(usage.is_empty());
}

#[test]
fn test_resource_usage_not_empty_when_used() {
    let usage = ResourceUsage {
        cpu_usage_percent: 1.0,
        ..Default::default()
    };
    assert!(!usage.is_empty());
}

#[test]
fn test_resource_limits_default() {
    let limits = ResourceLimits::default();
    assert!(limits.execution_timeout.is_some());
}

#[test]
fn test_resource_limits_serialization() {
    let limits = ResourceLimits::default();
    let json = serde_json::to_string(&limits).expect("serialize");
    let _: ResourceLimits = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn test_cpu_limits_default() {
    let cpu = CpuLimits::default();
    assert!(cpu.max_cores.is_none());
}

#[test]
fn test_memory_limits_default() {
    let mem = MemoryLimits::default();
    assert!(mem.max_bytes.is_none());
}

#[test]
fn test_storage_limits_default() {
    let storage = StorageLimits::default();
    assert!(storage.max_bytes.is_none());
}

#[test]
fn test_network_limits_default() {
    let net = NetworkLimits::default();
    assert!(net.max_bandwidth.is_none());
}

#[test]
fn test_runtime_metrics_default() {
    let m = RuntimeMetrics::default();
    assert_eq!(m.cpu.usage_percent, 0.0);
    assert_eq!(m.memory.used_bytes, 0);
}

#[test]
fn test_runtime_metrics_serialization() {
    let m = RuntimeMetrics::default();
    let json = serde_json::to_string(&m).expect("serialize");
    let _: RuntimeMetrics = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn test_system_resources_default() {
    let sr = SystemResources::default();
    assert_eq!(sr.available_cpu_cores, 1.0);
    assert_eq!(sr.total_cpu_cores, 1);
}

#[test]
fn test_system_resources_serialization() {
    let sr = SystemResources::default();
    let json = serde_json::to_string(&sr).expect("serialize");
    let _: SystemResources = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn test_process_status_variants() {
    for s in [
        ProcessStatus::Running,
        ProcessStatus::Sleeping,
        ProcessStatus::Stopped,
        ProcessStatus::Zombie,
        ProcessStatus::Unknown,
    ] {
        let json = serde_json::to_value(&s).expect("serialize");
        let _: ProcessStatus = serde_json::from_value(json).expect("deserialize");
    }
}

#[test]
fn test_process_info_serialization() {
    let info = ProcessInfo {
        workload_id: "w1".to_string(),
        process_count: 5,
        total_cpu_time: 10.5,
        memory_usage: 1024,
        status: ProcessStatus::Running,
    };
    let json = serde_json::to_string(&info).expect("serialize");
    let _: ProcessInfo = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn test_network_stats_constructor() {
    let stats = NetworkStats {
        bytes_received: 100,
        bytes_transmitted: 200,
        packets_received: 10,
        packets_transmitted: 20,
        interfaces: 2,
    };
    assert_eq!(stats.bytes_received, 100);
    assert_eq!(stats.interfaces, 2);
}

#[test]
fn test_load_averages_constructor() {
    let load = LoadAverages {
        one_minute: 0.5,
        five_minutes: 0.4,
        fifteen_minutes: 0.3,
    };
    assert!((load.one_minute - 0.5).abs() < 0.01);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_get_metrics_empty() {
    let monitor = SystemResourceMonitor::new();
    let metrics = monitor.get_metrics("nonexistent").await.unwrap();
    assert_eq!(metrics.cpu.usage_percent, 0.0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_start_real_time_monitoring() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.start_real_time_monitoring("wl-1").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_get_process_info() {
    let monitor = SystemResourceMonitor::new();
    let result = monitor.get_process_info("wl-1").await;
    assert!(result.is_ok());
    let info = result.unwrap().unwrap();
    assert_eq!(info.workload_id, "wl-1");
}

#[test]
fn test_resource_usage_not_empty_disk_io() {
    let usage = ResourceUsage {
        disk_read_bytes: 1,
        ..Default::default()
    };
    assert!(!usage.is_empty());
}

#[test]
fn test_resource_usage_not_empty_network() {
    let usage = ResourceUsage {
        network_rx_bytes: 100,
        ..Default::default()
    };
    assert!(!usage.is_empty());
}

#[test]
fn test_resource_usage_not_empty_memory() {
    let usage = ResourceUsage {
        memory_used_mb: 1,
        ..Default::default()
    };
    assert!(!usage.is_empty());
}

#[test]
fn test_resource_usage_not_empty_wall_time() {
    let usage = ResourceUsage {
        wall_time_ms: 1,
        ..Default::default()
    };
    assert!(!usage.is_empty());
}

#[test]
fn test_resource_usage_serialization() {
    let usage = ResourceUsage {
        cpu_usage_percent: 50.0,
        memory_used_mb: 1024,
        disk_read_bytes: 1000,
        disk_write_bytes: 500,
        network_rx_bytes: 2000,
        network_tx_bytes: 1000,
        wall_time_ms: 5000,
    };
    let json = serde_json::to_string(&usage).expect("serialize");
    let parsed: ResourceUsage = serde_json::from_str(&json).expect("deserialize");
    assert!((parsed.cpu_usage_percent - 50.0).abs() < 0.01);
    assert_eq!(parsed.memory_used_mb, 1024);
}

#[test]
fn test_gpu_requirements_default_construction() {
    let gpu = GpuRequirements {
        min_units: 0,
        max_units: None,
        gpu_type: None,
        min_memory_bytes: None,
    };
    assert_eq!(gpu.min_units, 0);
}

#[test]
fn test_gpu_requirements_serialization() {
    let gpu = GpuRequirements {
        min_units: 2,
        max_units: Some(4),
        gpu_type: Some("NVIDIA A100".to_string()),
        min_memory_bytes: Some(40 * 1024 * 1024 * 1024),
    };
    let json = serde_json::to_string(&gpu).expect("serialize");
    let _: GpuRequirements = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn test_cpu_metrics_constructor() {
    let m = CpuMetrics {
        usage_percent: 75.5,
        cores_used: 6.0,
        cpu_time_seconds: 120.0,
    };
    assert!((m.usage_percent - 75.5).abs() < 0.01);
}

#[test]
fn test_storage_metrics_default() {
    let m = StorageMetrics::default();
    assert_eq!(m.bytes_read, 0);
    assert_eq!(m.bytes_written, 0);
}

#[test]
fn test_timing_metrics_serialization() {
    use std::time::{Duration, SystemTime};
    let t = TimingMetrics {
        start_time: SystemTime::now(),
        end_time: Some(SystemTime::now()),
        duration: Duration::from_secs(60),
    };
    let json = serde_json::to_string(&t).expect("serialize");
    let _: TimingMetrics = serde_json::from_str(&json).expect("deserialize");
}

#[test]
fn test_load_averages_serialization() {
    let load = LoadAverages {
        one_minute: 1.5,
        five_minutes: 1.2,
        fifteen_minutes: 0.9,
    };
    let json = serde_json::to_string(&load).expect("serialize");
    let parsed: LoadAverages = serde_json::from_str(&json).expect("deserialize");
    assert!((parsed.one_minute - 1.5).abs() < 0.01);
}

#[test]
fn test_resource_limits_with_timeout() {
    use std::time::Duration;
    let limits = ResourceLimits {
        cpu_limits: CpuLimits::default(),
        memory_limits: MemoryLimits::default(),
        storage_limits: StorageLimits::default(),
        network_limits: NetworkLimits::default(),
        execution_timeout: Some(Duration::from_secs(600)),
    };
    assert_eq!(limits.execution_timeout.unwrap().as_secs(), 600);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_system_resource_monitor_update_workload_metrics() {
    let monitor = SystemResourceMonitor::new();
    let _ = monitor.start_real_time_monitoring("wl-metrics").await;
    let result = monitor.update_workload_metrics("wl-metrics").await;
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_monitor_stop_monitoring() {
    let monitor = SystemResourceMonitor::new();
    let _ = monitor.start_monitoring("wl-stop");
    let result = monitor.stop_monitoring("wl-stop");
    assert!(result.is_ok());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_resource_monitor_get_metrics_after_start() {
    let monitor = SystemResourceMonitor::new();
    let _ = monitor.start_monitoring("wl-get");
    let metrics = monitor.get_metrics("wl-get").await.unwrap();
    assert!(metrics.timing.start_time <= std::time::SystemTime::now());
}
