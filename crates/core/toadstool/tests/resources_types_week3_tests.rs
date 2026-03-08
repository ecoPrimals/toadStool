// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2025 ecoPrimals

//! Comprehensive tests for resources.rs types.
#![allow(clippy::float_cmp)]

use std::time::{Duration, SystemTime};
use toadstool::resources::*;

// ============================================================================
// CpuRequirements Tests
// ============================================================================

#[test]
fn test_cpu_requirements_default() {
    let cpu = CpuRequirements::default();
    assert_eq!(cpu.min_cores, 1.0);
    assert!(cpu.max_cores.is_none());
    assert!(cpu.architecture.is_none());
}

#[test]
fn test_cpu_requirements_with_range() {
    let cpu = CpuRequirements {
        min_cores: 2.0,
        max_cores: Some(8.0),
        architecture: Some("x86_64".to_string()),
    };

    assert_eq!(cpu.min_cores, 2.0);
    assert_eq!(cpu.max_cores, Some(8.0));
    assert_eq!(cpu.architecture, Some("x86_64".to_string()));
}

#[test]
fn test_cpu_requirements_architectures() {
    let x86 = CpuRequirements {
        min_cores: 1.0,
        max_cores: None,
        architecture: Some("x86_64".to_string()),
    };

    let arm = CpuRequirements {
        min_cores: 1.0,
        max_cores: None,
        architecture: Some("aarch64".to_string()),
    };

    assert_eq!(x86.architecture, Some("x86_64".to_string()));
    assert_eq!(arm.architecture, Some("aarch64".to_string()));
}

#[test]
fn test_cpu_requirements_clone() {
    let cpu = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(16.0),
        architecture: Some("x86_64".to_string()),
    };

    let cloned = cpu.clone();
    assert_eq!(cpu.min_cores, cloned.min_cores);
    assert_eq!(cpu.max_cores, cloned.max_cores);
}

// ============================================================================
// MemoryRequirements Tests
// ============================================================================

#[test]
fn test_memory_requirements_default() {
    let mem = MemoryRequirements::default();
    assert_eq!(mem.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert!(mem.max_bytes.is_none());
}

#[test]
fn test_memory_requirements_with_range() {
    let mem = MemoryRequirements {
        min_bytes: 2 * 1024 * 1024 * 1024,       // 2GB
        max_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
    };

    assert_eq!(mem.min_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(mem.max_bytes, Some(8 * 1024 * 1024 * 1024));
}

#[test]
fn test_memory_requirements_large() {
    let mem = MemoryRequirements {
        min_bytes: 128 * 1024 * 1024 * 1024,       // 128GB
        max_bytes: Some(256 * 1024 * 1024 * 1024), // 256GB
    };

    assert!(mem.min_bytes >= 100 * 1024 * 1024 * 1024);
}

#[test]
fn test_memory_requirements_clone() {
    let mem = MemoryRequirements {
        min_bytes: 4 * 1024 * 1024 * 1024,
        max_bytes: Some(16 * 1024 * 1024 * 1024),
    };

    let cloned = mem.clone();
    assert_eq!(mem.min_bytes, cloned.min_bytes);
    assert_eq!(mem.max_bytes, cloned.max_bytes);
}

// ============================================================================
// StorageRequirements Tests
// ============================================================================

#[test]
fn test_storage_requirements_default() {
    let storage = StorageRequirements::default();
    assert_eq!(storage.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert!(storage.max_bytes.is_none());
    assert!(storage.storage_type.is_none());
}

#[test]
fn test_storage_requirements_with_type() {
    let storage = StorageRequirements {
        min_bytes: 10 * 1024 * 1024 * 1024,        // 10GB
        max_bytes: Some(100 * 1024 * 1024 * 1024), // 100GB
        storage_type: Some("ssd".to_string()),
    };

    assert_eq!(storage.storage_type, Some("ssd".to_string()));
}

#[test]
fn test_storage_requirements_types() {
    let ssd = StorageRequirements {
        min_bytes: 1024,
        max_bytes: None,
        storage_type: Some("ssd".to_string()),
    };

    let hdd = StorageRequirements {
        min_bytes: 1024,
        max_bytes: None,
        storage_type: Some("hdd".to_string()),
    };

    let nvme = StorageRequirements {
        min_bytes: 1024,
        max_bytes: None,
        storage_type: Some("nvme".to_string()),
    };

    assert_eq!(ssd.storage_type, Some("ssd".to_string()));
    assert_eq!(hdd.storage_type, Some("hdd".to_string()));
    assert_eq!(nvme.storage_type, Some("nvme".to_string()));
}

#[test]
fn test_storage_requirements_clone() {
    let storage = StorageRequirements {
        min_bytes: 5 * 1024 * 1024 * 1024,
        max_bytes: Some(50 * 1024 * 1024 * 1024),
        storage_type: Some("ssd".to_string()),
    };

    let cloned = storage.clone();
    assert_eq!(storage.min_bytes, cloned.min_bytes);
    assert_eq!(storage.storage_type, cloned.storage_type);
}

// ============================================================================
// NetworkRequirements Tests
// ============================================================================

#[test]
fn test_network_requirements_default() {
    let net = NetworkRequirements::default();
    assert!(net.min_bandwidth.is_none());
    assert!(net.max_bandwidth.is_none());
    assert!(net.max_latency_ms.is_none());
}

#[test]
fn test_network_requirements_with_bandwidth() {
    let net = NetworkRequirements {
        min_bandwidth: Some(100 * 1024 * 1024),  // 100 Mbps
        max_bandwidth: Some(1000 * 1024 * 1024), // 1 Gbps
        max_latency_ms: Some(10),
    };

    assert_eq!(net.min_bandwidth, Some(100 * 1024 * 1024));
    assert_eq!(net.max_bandwidth, Some(1000 * 1024 * 1024));
    assert_eq!(net.max_latency_ms, Some(10));
}

#[test]
fn test_network_requirements_latency_sensitive() {
    let net = NetworkRequirements {
        min_bandwidth: Some(1024 * 1024 * 1024), // 1 Gbps
        max_bandwidth: None,
        max_latency_ms: Some(1), // Low latency
    };

    assert!(net.max_latency_ms.unwrap() <= 10);
}

#[test]
fn test_network_requirements_clone() {
    let net = NetworkRequirements {
        min_bandwidth: Some(10 * 1024 * 1024),
        max_bandwidth: Some(100 * 1024 * 1024),
        max_latency_ms: Some(50),
    };

    let cloned = net.clone();
    assert_eq!(net.min_bandwidth, cloned.min_bandwidth);
    assert_eq!(net.max_latency_ms, cloned.max_latency_ms);
}

// ============================================================================
// GpuRequirements Tests
// ============================================================================

#[test]
fn test_gpu_requirements_basic() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: Some(4),
        gpu_type: Some("cuda".to_string()),
        min_memory_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
    };

    assert_eq!(gpu.min_units, 1);
    assert_eq!(gpu.max_units, Some(4));
    assert_eq!(gpu.gpu_type, Some("cuda".to_string()));
}

#[test]
fn test_gpu_requirements_types() {
    let cuda = GpuRequirements {
        min_units: 1,
        max_units: None,
        gpu_type: Some("cuda".to_string()),
        min_memory_bytes: None,
    };

    let opencl = GpuRequirements {
        min_units: 1,
        max_units: None,
        gpu_type: Some("opencl".to_string()),
        min_memory_bytes: None,
    };

    assert_eq!(cuda.gpu_type, Some("cuda".to_string()));
    assert_eq!(opencl.gpu_type, Some("opencl".to_string()));
}

#[test]
fn test_gpu_requirements_high_memory() {
    let gpu = GpuRequirements {
        min_units: 2,
        max_units: Some(8),
        gpu_type: Some("cuda".to_string()),
        min_memory_bytes: Some(32 * 1024 * 1024 * 1024), // 32GB
    };

    assert!(gpu.min_memory_bytes.unwrap() >= 16 * 1024 * 1024 * 1024);
}

#[test]
fn test_gpu_requirements_clone() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: Some(2),
        gpu_type: Some("cuda".to_string()),
        min_memory_bytes: Some(8 * 1024 * 1024 * 1024),
    };

    let cloned = gpu.clone();
    assert_eq!(gpu.min_units, cloned.min_units);
    assert_eq!(gpu.gpu_type, cloned.gpu_type);
}

// ============================================================================
// ResourceRequirements Tests
// ============================================================================

#[test]
fn test_resource_requirements_default() {
    let req = ResourceRequirements::default();

    assert_eq!(req.cpu.min_cores, 1.0);
    assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(req.storage.min_bytes, 1024 * 1024 * 1024);
    assert!(req.gpu.is_none());
}

#[test]
fn test_resource_requirements_with_gpu() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(16.0),
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 16 * 1024 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: Some(2),
            gpu_type: Some("cuda".to_string()),
            min_memory_bytes: Some(8 * 1024 * 1024 * 1024),
        }),
        network: NetworkRequirements::default(),
    };

    assert!(req.gpu.is_some());
    assert_eq!(req.gpu.as_ref().unwrap().min_units, 1);
}

#[test]
fn test_resource_requirements_high_performance() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 32.0,
            max_cores: Some(64.0),
            architecture: Some("x86_64".to_string()),
        },
        memory: MemoryRequirements {
            min_bytes: 128 * 1024 * 1024 * 1024,
            max_bytes: Some(256 * 1024 * 1024 * 1024),
        },
        storage: StorageRequirements {
            min_bytes: 1000 * 1024 * 1024 * 1024,
            max_bytes: Some(10000 * 1024 * 1024 * 1024),
            storage_type: Some("nvme".to_string()),
        },
        gpu: Some(GpuRequirements {
            min_units: 8,
            max_units: Some(16),
            gpu_type: Some("cuda".to_string()),
            min_memory_bytes: Some(80 * 1024 * 1024 * 1024),
        }),
        network: NetworkRequirements {
            min_bandwidth: Some(10 * 1024 * 1024 * 1024),
            max_bandwidth: Some(100 * 1024 * 1024 * 1024),
            max_latency_ms: Some(1),
        },
    };

    assert!(req.cpu.min_cores >= 16.0);
    assert!(req.memory.min_bytes >= 64 * 1024 * 1024 * 1024);
    assert!(req.gpu.is_some());
}

#[test]
fn test_resource_requirements_clone() {
    let req = ResourceRequirements::default();
    let cloned = req.clone();

    assert_eq!(req.cpu.min_cores, cloned.cpu.min_cores);
    assert_eq!(req.memory.min_bytes, cloned.memory.min_bytes);
}

// ============================================================================
// CpuMetrics Tests
// ============================================================================

#[test]
fn test_cpu_metrics_default() {
    let metrics = CpuMetrics::default();
    assert_eq!(metrics.usage_percent, 0.0);
    assert_eq!(metrics.cores_used, 0.0);
    assert_eq!(metrics.cpu_time_seconds, 0.0);
}

#[test]
fn test_cpu_metrics_active() {
    let metrics = CpuMetrics {
        usage_percent: 75.5,
        cores_used: 6.0,
        cpu_time_seconds: 120.5,
    };

    assert_eq!(metrics.usage_percent, 75.5);
    assert_eq!(metrics.cores_used, 6.0);
    assert_eq!(metrics.cpu_time_seconds, 120.5);
}

#[test]
fn test_cpu_metrics_high_usage() {
    let metrics = CpuMetrics {
        usage_percent: 98.5,
        cores_used: 15.8,
        cpu_time_seconds: 3600.0,
    };

    assert!(metrics.usage_percent > 90.0);
    assert!(metrics.cores_used >= 15.0);
}

#[test]
fn test_cpu_metrics_clone() {
    let metrics = CpuMetrics {
        usage_percent: 50.0,
        cores_used: 4.0,
        cpu_time_seconds: 60.0,
    };

    let cloned = metrics.clone();
    assert_eq!(metrics.usage_percent, cloned.usage_percent);
    assert_eq!(metrics.cores_used, cloned.cores_used);
}

// ============================================================================
// MemoryMetrics Tests
// ============================================================================

#[test]
fn test_memory_metrics_default() {
    let metrics = MemoryMetrics::default();
    assert_eq!(metrics.usage_percent, 0.0);
    assert_eq!(metrics.used_bytes, 0);
    assert_eq!(metrics.peak_bytes, 0);
}

#[test]
fn test_memory_metrics_active() {
    let metrics = MemoryMetrics {
        usage_percent: 75.5,
        used_bytes: 4 * 1024 * 1024 * 1024, // 4GB
        peak_bytes: 6 * 1024 * 1024 * 1024, // 6GB
    };

    assert_eq!(metrics.used_bytes, 4 * 1024 * 1024 * 1024);
    assert!(metrics.peak_bytes >= metrics.used_bytes);
    assert_eq!(metrics.usage_percent, 75.5);
}

#[test]
fn test_memory_metrics_peak_tracking() {
    let metrics = MemoryMetrics {
        usage_percent: 50.0,
        used_bytes: 2 * 1024 * 1024 * 1024,
        peak_bytes: 8 * 1024 * 1024 * 1024,
    };

    assert!(metrics.peak_bytes >= metrics.used_bytes);
}

#[test]
fn test_memory_metrics_clone() {
    let metrics = MemoryMetrics {
        usage_percent: 60.0,
        used_bytes: 1024 * 1024 * 1024,
        peak_bytes: 2 * 1024 * 1024 * 1024,
    };

    let cloned = metrics.clone();
    assert_eq!(metrics.used_bytes, cloned.used_bytes);
    assert_eq!(metrics.usage_percent, cloned.usage_percent);
}

// ============================================================================
// StorageMetrics Tests
// ============================================================================

#[test]
fn test_storage_metrics_default() {
    let metrics = StorageMetrics::default();
    assert_eq!(metrics.usage_percent, 0.0);
    assert_eq!(metrics.used_bytes, 0);
    assert_eq!(metrics.bytes_read, 0);
    assert_eq!(metrics.bytes_written, 0);
}

#[test]
fn test_storage_metrics_active() {
    let metrics = StorageMetrics {
        usage_percent: 45.0,
        used_bytes: 50 * 1024 * 1024 * 1024, // 50GB
        bytes_read: 1024 * 1024 * 100,       // 100MB
        bytes_written: 1024 * 1024 * 50,     // 50MB
    };

    assert_eq!(metrics.bytes_read, 1024 * 1024 * 100);
    assert_eq!(metrics.usage_percent, 45.0);
}

#[test]
fn test_storage_metrics_heavy_io() {
    let metrics = StorageMetrics {
        usage_percent: 85.0,
        used_bytes: 850 * 1024 * 1024 * 1024,  // 850GB
        bytes_read: 10 * 1024 * 1024 * 1024,   // 10GB
        bytes_written: 5 * 1024 * 1024 * 1024, // 5GB
    };

    assert!(metrics.bytes_read > 1024 * 1024 * 1024);
    assert!(metrics.usage_percent > 50.0);
}

#[test]
fn test_storage_metrics_clone() {
    let metrics = StorageMetrics {
        usage_percent: 25.0,
        used_bytes: 100 * 1024 * 1024 * 1024,
        bytes_read: 1024,
        bytes_written: 512,
    };

    let cloned = metrics.clone();
    assert_eq!(metrics.bytes_read, cloned.bytes_read);
    assert_eq!(metrics.bytes_written, cloned.bytes_written);
}

// ============================================================================
// NetworkMetrics Tests
// ============================================================================

#[test]
fn test_network_metrics_default() {
    let metrics = NetworkMetrics::default();
    assert_eq!(metrics.bytes_sent, 0);
    assert_eq!(metrics.bytes_received, 0);
    assert_eq!(metrics.packets_sent, 0);
    assert_eq!(metrics.packets_received, 0);
}

#[test]
fn test_network_metrics_active() {
    let metrics = NetworkMetrics {
        bytes_sent: 1024 * 1024,         // 1MB
        bytes_received: 2 * 1024 * 1024, // 2MB
        packets_sent: 1000,
        packets_received: 2000,
    };

    assert_eq!(metrics.bytes_sent, 1024 * 1024);
    assert_eq!(metrics.packets_received, 2000);
}

#[test]
fn test_network_metrics_high_traffic() {
    let metrics = NetworkMetrics {
        bytes_sent: 100 * 1024 * 1024 * 1024,     // 100GB
        bytes_received: 200 * 1024 * 1024 * 1024, // 200GB
        packets_sent: 1_000_000,
        packets_received: 2_000_000,
    };

    assert!(metrics.bytes_sent > 10 * 1024 * 1024 * 1024);
    assert!(metrics.packets_sent > 100_000);
}

#[test]
fn test_network_metrics_clone() {
    let metrics = NetworkMetrics {
        bytes_sent: 1024,
        bytes_received: 2048,
        packets_sent: 10,
        packets_received: 20,
    };

    let cloned = metrics.clone();
    assert_eq!(metrics.bytes_sent, cloned.bytes_sent);
    assert_eq!(metrics.packets_received, cloned.packets_received);
}

// ============================================================================
// TimingMetrics Tests
// ============================================================================

#[test]
fn test_timing_metrics_default() {
    let metrics = TimingMetrics::default();
    assert_eq!(metrics.duration, Duration::ZERO);
    assert!(metrics.end_time.is_none());
}

#[test]
fn test_timing_metrics_with_duration() {
    let start = SystemTime::now();
    let end = start + Duration::from_secs(120);
    let metrics = TimingMetrics {
        start_time: start,
        end_time: Some(end),
        duration: Duration::from_secs(120),
    };

    assert_eq!(metrics.duration, Duration::from_secs(120));
    assert!(metrics.end_time.is_some());
}

#[test]
fn test_timing_metrics_long_running() {
    let start = SystemTime::now();
    let metrics = TimingMetrics {
        start_time: start,
        end_time: Some(start + Duration::from_secs(3600)),
        duration: Duration::from_secs(3600),
    };

    assert!(metrics.duration >= Duration::from_secs(3600));
}

#[test]
fn test_timing_metrics_clone() {
    let start = SystemTime::now();
    let metrics = TimingMetrics {
        start_time: start,
        end_time: Some(start + Duration::from_secs(400)),
        duration: Duration::from_secs(400),
    };

    let cloned = metrics.clone();
    assert_eq!(metrics.duration, cloned.duration);
}

// ============================================================================
// RuntimeMetrics Tests
// ============================================================================

#[test]
fn test_runtime_metrics_default() {
    let metrics = RuntimeMetrics::default();

    assert_eq!(metrics.cpu.usage_percent, 0.0);
    assert_eq!(metrics.memory.used_bytes, 0);
    assert_eq!(metrics.storage.bytes_read, 0);
    assert_eq!(metrics.network.bytes_sent, 0);
    assert!(metrics.gpu.is_none());
}

#[test]
fn test_runtime_metrics_complete() {
    let start = SystemTime::now();
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 50.0,
            cores_used: 4.0,
            cpu_time_seconds: 120.0,
        },
        memory: MemoryMetrics {
            usage_percent: 60.0,
            used_bytes: 2 * 1024 * 1024 * 1024,
            peak_bytes: 3 * 1024 * 1024 * 1024,
        },
        storage: StorageMetrics {
            usage_percent: 40.0,
            used_bytes: 100 * 1024 * 1024 * 1024,
            bytes_read: 1024 * 1024 * 100,
            bytes_written: 1024 * 1024 * 50,
        },
        network: NetworkMetrics {
            bytes_sent: 1024 * 1024,
            bytes_received: 2 * 1024 * 1024,
            packets_sent: 100,
            packets_received: 200,
        },
        gpu: None,
        timing: TimingMetrics {
            start_time: start,
            end_time: Some(start + Duration::from_secs(120)),
            duration: Duration::from_secs(120),
        },
    };

    assert_eq!(metrics.cpu.cores_used, 4.0);
    assert_eq!(metrics.memory.used_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(metrics.timing.duration, Duration::from_secs(120));
}

#[test]
fn test_runtime_metrics_clone() {
    let metrics = RuntimeMetrics::default();
    let cloned = metrics.clone();

    assert_eq!(metrics.cpu.usage_percent, cloned.cpu.usage_percent);
    assert_eq!(metrics.memory.used_bytes, cloned.memory.used_bytes);
}

// ============================================================================
// SystemResources Tests
// ============================================================================

#[test]
fn test_system_resources_default() {
    let sys = SystemResources::default();

    assert_eq!(sys.available_cpu_cores, 1.0);
    assert_eq!(sys.available_memory_bytes, 1024 * 1024 * 1024);
    assert_eq!(sys.available_storage_bytes, 1024 * 1024 * 1024);
    assert!(sys.available_network_bandwidth.is_none());
    assert_eq!(sys.available_gpu_units, 0);
}

#[test]
fn test_system_resources_typical_server() {
    let sys = SystemResources {
        available_cpu_cores: 16.0,
        available_memory_bytes: 64 * 1024 * 1024 * 1024,
        available_storage_bytes: 1000 * 1024 * 1024 * 1024,
        available_network_bandwidth: Some(10 * 1024 * 1024 * 1024),
        available_gpu_units: 2,
        ..Default::default()
    };

    assert!(sys.available_cpu_cores >= 8.0);
    assert!(sys.available_memory_bytes >= 32 * 1024 * 1024 * 1024);
    assert_eq!(sys.available_gpu_units, 2);
}

#[test]
fn test_system_resources_high_end() {
    let sys = SystemResources {
        available_cpu_cores: 128.0,
        available_memory_bytes: 512 * 1024 * 1024 * 1024,
        available_storage_bytes: 10000 * 1024 * 1024 * 1024,
        available_network_bandwidth: Some(100 * 1024 * 1024 * 1024),
        available_gpu_units: 8,
        ..Default::default()
    };

    assert!(sys.available_cpu_cores >= 64.0);
    assert!(sys.available_memory_bytes >= 256 * 1024 * 1024 * 1024);
    assert!(sys.available_gpu_units >= 4);
}

#[test]
fn test_system_resources_clone() {
    let sys = SystemResources::default();
    let cloned = sys.clone();

    assert_eq!(sys.available_cpu_cores, cloned.available_cpu_cores);
    assert_eq!(sys.available_memory_bytes, cloned.available_memory_bytes);
}
