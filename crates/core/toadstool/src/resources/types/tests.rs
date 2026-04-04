// SPDX-License-Identifier: AGPL-3.0-only

use std::time::Duration;

use super::*;

#[test]
fn resource_requirements_default_is_valid() {
    let req = ResourceRequirements::default();
    assert!(req.validate().is_ok());
    assert!(req.cpu.min_cores > 0.0);
    assert!(req.memory.min_bytes > 0);
    assert!(req.gpu.is_none());
}

#[test]
fn resource_requirements_validate_zero_cpu() {
    let mut req = ResourceRequirements::default();
    req.cpu.min_cores = 0.0;
    let err = req.validate().unwrap_err();
    assert!(err.to_string().contains("cpu.min_cores"));
}

#[test]
fn resource_requirements_validate_zero_memory() {
    let mut req = ResourceRequirements::default();
    req.memory.min_bytes = 0;
    let err = req.validate().unwrap_err();
    assert!(err.to_string().contains("memory.min_bytes"));
}

#[test]
fn resource_usage_default_is_empty() {
    let usage = ResourceUsage::default();
    assert!(usage.is_empty());
}

#[test]
fn resource_usage_nonzero_not_empty() {
    let usage = ResourceUsage {
        memory_used_mb: 128,
        ..ResourceUsage::default()
    };
    assert!(!usage.is_empty());
}

#[test]
fn defaults_round_trip_serde() {
    let metrics = RuntimeMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    let deser: RuntimeMetrics = serde_json::from_str(&json).unwrap();
    assert!((deser.cpu.cores_used - metrics.cpu.cores_used).abs() < f64::EPSILON);
    assert_eq!(deser.memory.used_bytes, metrics.memory.used_bytes);
}

#[test]
fn resource_limits_default_has_timeout() {
    let limits = ResourceLimits::default();
    assert!(limits.execution_timeout.is_some());
}

#[test]
fn system_resources_default() {
    let res = SystemResources::default();
    assert!(res.available_cpu_cores > 0.0);
    assert!(res.available_memory_bytes > 0);
    assert_eq!(res.available_gpu_units, 0);
}

#[test]
fn process_status_debug() {
    let statuses = [
        ProcessStatus::Running,
        ProcessStatus::Sleeping,
        ProcessStatus::Stopped,
        ProcessStatus::Zombie,
        ProcessStatus::Unknown,
    ];
    for s in &statuses {
        assert!(!format!("{s:?}").is_empty());
    }
}

#[test]
fn cpu_metrics_default() {
    let m = CpuMetrics::default();
    assert!((m.usage_percent - 0.0).abs() < f64::EPSILON);
    assert!((m.cores_used - 0.0).abs() < f64::EPSILON);
}

#[test]
fn gpu_metrics_default() {
    let m = GpuMetrics::default();
    assert!((m.usage_percent - 0.0).abs() < f64::EPSILON);
    assert!(m.temperature_celsius.is_none());
}

#[test]
fn storage_requirements_default() {
    let s = StorageRequirements::default();
    assert!(s.min_bytes > 0);
    assert!(s.max_bytes.is_none());
}

#[test]
fn network_requirements_default() {
    let n = NetworkRequirements::default();
    assert!(n.min_bandwidth.is_none());
    assert!(n.max_latency_ms.is_none());
}

#[test]
fn timing_metrics_default() {
    let t = TimingMetrics::default();
    assert!(t.end_time.is_none());
    assert_eq!(t.duration, Duration::ZERO);
}

#[test]
fn load_averages_serde() {
    let la = LoadAverages {
        one_minute: 1.5,
        five_minutes: 2.0,
        fifteen_minutes: 1.8,
    };
    let json = serde_json::to_string(&la).unwrap();
    let deser: LoadAverages = serde_json::from_str(&json).unwrap();
    assert!((deser.one_minute - 1.5).abs() < f64::EPSILON);
}

#[test]
fn network_stats_serde() {
    let ns = NetworkStats {
        bytes_received: 100,
        bytes_transmitted: 200,
        packets_received: 10,
        packets_transmitted: 20,
        interfaces: 2,
    };
    let json = serde_json::to_string(&ns).unwrap();
    let deser: NetworkStats = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.interfaces, 2);
}

#[test]
fn process_info_serde() {
    let pi = ProcessInfo {
        workload_id: "w1".to_string(),
        process_count: 3,
        total_cpu_time: 10.5,
        memory_usage: 1024,
        status: ProcessStatus::Running,
    };
    let json = serde_json::to_string(&pi).unwrap();
    let deser: ProcessInfo = serde_json::from_str(&json).unwrap();
    assert_eq!(deser.workload_id, "w1");
}
