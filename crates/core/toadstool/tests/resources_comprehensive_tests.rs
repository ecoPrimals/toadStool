// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for resources module
//!
//! Sprint 16: resources.rs coverage 44.12% → 65%+
//! Target: 485 lines, ~40-50 comprehensive tests

use toadstool::resources::*;

// ============================================================================
// ResourceRequirements Tests
// ============================================================================

#[test]
fn test_resource_requirements_default() {
    let req = ResourceRequirements::default();

    // Should have default values
    assert_eq!(req.cpu.min_cores, 1.0);
    assert_eq!(req.memory.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert_eq!(req.storage.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert!(req.gpu.is_none());
}

#[test]
fn test_resource_requirements_clone() {
    let req = ResourceRequirements::default();
    let cloned = req.clone();

    assert_eq!(cloned.cpu.min_cores, req.cpu.min_cores);
    assert_eq!(cloned.memory.min_bytes, req.memory.min_bytes);
}

#[test]
fn test_resource_requirements_debug() {
    let req = ResourceRequirements::default();
    let debug = format!("{:?}", req);

    assert!(!debug.is_empty());
    assert!(debug.contains("ResourceRequirements"));
}

#[test]
fn test_resource_requirements_with_gpu() {
    let req = ResourceRequirements {
        cpu: CpuRequirements::default(),
        memory: MemoryRequirements::default(),
        storage: StorageRequirements::default(),
        network: NetworkRequirements::default(),
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: Some(2),
            gpu_type: Some("NVIDIA".to_string()),
            min_memory_bytes: Some(4 * 1024 * 1024 * 1024), // 4GB
        }),
    };

    assert!(req.gpu.is_some());
    assert_eq!(req.gpu.unwrap().min_units, 1);
}

#[test]
fn test_resource_requirements_serialization() {
    let req = ResourceRequirements::default();

    // Should be serializable to JSON
    let json = serde_json::to_string(&req);
    assert!(json.is_ok());
}

#[test]
fn test_resource_requirements_deserialization() {
    let json = r#"{
        "cpu": {"min_cores": 2.0, "max_cores": null, "architecture": null},
        "memory": {"min_bytes": 2147483648, "max_bytes": null},
        "storage": {"min_bytes": 1073741824, "max_bytes": null, "storage_type": null},
        "network": {"min_bandwidth": null, "max_bandwidth": null, "max_latency_ms": null},
        "gpu": null
    }"#;

    let req: Result<ResourceRequirements, _> = serde_json::from_str(json);
    assert!(req.is_ok());
}

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
fn test_cpu_requirements_with_max_cores() {
    let cpu = CpuRequirements {
        min_cores: 2.0,
        max_cores: Some(8.0),
        architecture: None,
    };

    assert_eq!(cpu.min_cores, 2.0);
    assert_eq!(cpu.max_cores, Some(8.0));
}

#[test]
fn test_cpu_requirements_with_architecture() {
    let cpu = CpuRequirements {
        min_cores: 1.0,
        max_cores: None,
        architecture: Some("x86_64".to_string()),
    };

    assert_eq!(cpu.architecture, Some("x86_64".to_string()));
}

#[test]
fn test_cpu_requirements_fractional_cores() {
    let cpu = CpuRequirements {
        min_cores: 0.5,
        max_cores: Some(1.5),
        architecture: None,
    };

    assert_eq!(cpu.min_cores, 0.5);
    assert_eq!(cpu.max_cores, Some(1.5));
}

#[test]
fn test_cpu_requirements_clone() {
    let cpu = CpuRequirements::default();
    let cloned = cpu.clone();

    assert_eq!(cloned.min_cores, cpu.min_cores);
}

#[test]
fn test_cpu_requirements_debug() {
    let cpu = CpuRequirements::default();
    let debug = format!("{:?}", cpu);

    assert!(!debug.is_empty());
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
fn test_memory_requirements_with_max() {
    let mem = MemoryRequirements {
        min_bytes: 1024 * 1024 * 1024,           // 1GB
        max_bytes: Some(4 * 1024 * 1024 * 1024), // 4GB
    };

    assert_eq!(mem.max_bytes, Some(4 * 1024 * 1024 * 1024));
}

#[test]
fn test_memory_requirements_small() {
    let mem = MemoryRequirements {
        min_bytes: 512 * 1024 * 1024, // 512MB
        max_bytes: None,
    };

    assert_eq!(mem.min_bytes, 512 * 1024 * 1024);
}

#[test]
fn test_memory_requirements_large() {
    let mem = MemoryRequirements {
        min_bytes: 16 * 1024 * 1024 * 1024,       // 16GB
        max_bytes: Some(64 * 1024 * 1024 * 1024), // 64GB
    };

    assert_eq!(mem.min_bytes, 16 * 1024 * 1024 * 1024);
}

#[test]
fn test_memory_requirements_clone() {
    let mem = MemoryRequirements::default();
    let cloned = mem.clone();

    assert_eq!(cloned.min_bytes, mem.min_bytes);
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
        min_bytes: 10 * 1024 * 1024 * 1024, // 10GB
        max_bytes: None,
        storage_type: Some("SSD".to_string()),
    };

    assert_eq!(storage.storage_type, Some("SSD".to_string()));
}

#[test]
fn test_storage_requirements_with_max() {
    let storage = StorageRequirements {
        min_bytes: 1024 * 1024 * 1024,
        max_bytes: Some(100 * 1024 * 1024 * 1024), // 100GB
        storage_type: None,
    };

    assert_eq!(storage.max_bytes, Some(100 * 1024 * 1024 * 1024));
}

#[test]
fn test_storage_requirements_clone() {
    let storage = StorageRequirements::default();
    let cloned = storage.clone();

    assert_eq!(cloned.min_bytes, storage.min_bytes);
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
        min_bandwidth: Some(1024 * 1024),      // 1 MB/s
        max_bandwidth: Some(10 * 1024 * 1024), // 10 MB/s
        max_latency_ms: None,
    };

    assert_eq!(net.min_bandwidth, Some(1024 * 1024));
    assert_eq!(net.max_bandwidth, Some(10 * 1024 * 1024));
}

#[test]
fn test_network_requirements_with_latency() {
    let net = NetworkRequirements {
        min_bandwidth: None,
        max_bandwidth: None,
        max_latency_ms: Some(100), // 100ms
    };

    assert_eq!(net.max_latency_ms, Some(100));
}

#[test]
fn test_network_requirements_clone() {
    let net = NetworkRequirements::default();
    let cloned = net.clone();

    assert_eq!(cloned.min_bandwidth, net.min_bandwidth);
}

// ============================================================================
// GpuRequirements Tests
// ============================================================================

#[test]
fn test_gpu_requirements_basic() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: None,
        gpu_type: None,
        min_memory_bytes: None,
    };

    assert_eq!(gpu.min_units, 1);
    assert!(gpu.max_units.is_none());
}

#[test]
fn test_gpu_requirements_with_type() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: Some(2),
        gpu_type: Some("NVIDIA RTX 4090".to_string()),
        min_memory_bytes: None,
    };

    assert_eq!(gpu.gpu_type, Some("NVIDIA RTX 4090".to_string()));
}

#[test]
fn test_gpu_requirements_with_memory() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: None,
        gpu_type: None,
        min_memory_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
    };

    assert_eq!(gpu.min_memory_bytes, Some(8 * 1024 * 1024 * 1024));
}

#[test]
fn test_gpu_requirements_multiple_units() {
    let gpu = GpuRequirements {
        min_units: 2,
        max_units: Some(4),
        gpu_type: Some("Tesla V100".to_string()),
        min_memory_bytes: Some(16 * 1024 * 1024 * 1024), // 16GB
    };

    assert_eq!(gpu.min_units, 2);
    assert_eq!(gpu.max_units, Some(4));
}

#[test]
fn test_gpu_requirements_clone() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: None,
        gpu_type: None,
        min_memory_bytes: None,
    };
    let cloned = gpu.clone();

    assert_eq!(cloned.min_units, gpu.min_units);
}

// ============================================================================
// RuntimeMetrics Tests
// ============================================================================

#[test]
fn test_runtime_metrics_default() {
    let metrics = RuntimeMetrics::default();

    assert_eq!(metrics.cpu.usage_percent, 0.0);
    assert_eq!(metrics.memory.used_bytes, 0);
    assert!(metrics.gpu.is_none());
}

#[test]
fn test_runtime_metrics_clone() {
    let metrics = RuntimeMetrics::default();
    let cloned = metrics.clone();

    assert_eq!(cloned.cpu.usage_percent, metrics.cpu.usage_percent);
}

#[test]
fn test_runtime_metrics_debug() {
    let metrics = RuntimeMetrics::default();
    let debug = format!("{:?}", metrics);

    assert!(!debug.is_empty());
}

#[test]
fn test_runtime_metrics_with_values() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 50.0,
            cores_used: 2.0,
            cpu_time_seconds: 10.5,
        },
        memory: MemoryMetrics {
            usage_percent: 50.0,
            used_bytes: 2 * 1024 * 1024 * 1024, // 2GB
            peak_bytes: 3 * 1024 * 1024 * 1024, // 3GB
        },
        storage: StorageMetrics::default(),
        network: NetworkMetrics::default(),
        gpu: None,
        timing: TimingMetrics::default(),
    };

    assert_eq!(metrics.cpu.usage_percent, 50.0);
    assert_eq!(metrics.memory.used_bytes, 2 * 1024 * 1024 * 1024);
}

// ============================================================================
// CpuMetrics Tests
// ============================================================================

#[test]
fn test_cpu_metrics_default() {
    let cpu = CpuMetrics::default();

    assert_eq!(cpu.usage_percent, 0.0);
    assert_eq!(cpu.cores_used, 0.0);
    assert_eq!(cpu.cpu_time_seconds, 0.0);
}

#[test]
fn test_cpu_metrics_with_values() {
    let cpu = CpuMetrics {
        usage_percent: 75.5,
        cores_used: 3.2,
        cpu_time_seconds: 120.5,
    };

    assert_eq!(cpu.usage_percent, 75.5);
    assert_eq!(cpu.cores_used, 3.2);
    assert_eq!(cpu.cpu_time_seconds, 120.5);
}

#[test]
fn test_cpu_metrics_clone() {
    let cpu = CpuMetrics::default();
    let cloned = cpu.clone();

    assert_eq!(cloned.usage_percent, cpu.usage_percent);
}

// ============================================================================
// MemoryMetrics Tests
// ============================================================================

#[test]
fn test_memory_metrics_default() {
    let mem = MemoryMetrics::default();

    assert_eq!(mem.usage_percent, 0.0);
    assert_eq!(mem.used_bytes, 0);
    assert_eq!(mem.peak_bytes, 0);
}

#[test]
fn test_memory_metrics_with_values() {
    let mem = MemoryMetrics {
        usage_percent: 75.0,
        used_bytes: 1024 * 1024 * 1024,     // 1GB
        peak_bytes: 2 * 1024 * 1024 * 1024, // 2GB
    };

    assert_eq!(mem.usage_percent, 75.0);
    assert_eq!(mem.used_bytes, 1024 * 1024 * 1024);
    assert_eq!(mem.peak_bytes, 2 * 1024 * 1024 * 1024);
}

#[test]
fn test_memory_metrics_clone() {
    let mem = MemoryMetrics::default();
    let cloned = mem.clone();

    assert_eq!(cloned.used_bytes, mem.used_bytes);
}

// ============================================================================
// Serialization Round-trip Tests
// ============================================================================

#[test]
fn test_resource_requirements_round_trip() {
    let original = ResourceRequirements::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: ResourceRequirements = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.cpu.min_cores, original.cpu.min_cores);
    assert_eq!(deserialized.memory.min_bytes, original.memory.min_bytes);
}

#[test]
fn test_runtime_metrics_round_trip() {
    let original = RuntimeMetrics::default();

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: RuntimeMetrics = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.cpu.usage_percent, original.cpu.usage_percent);
}

#[test]
fn test_cpu_requirements_round_trip() {
    let original = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(8.0),
        architecture: Some("arm64".to_string()),
    };

    let json = serde_json::to_string(&original).unwrap();
    let deserialized: CpuRequirements = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.min_cores, original.min_cores);
    assert_eq!(deserialized.architecture, original.architecture);
}

// ============================================================================
// Edge Cases and Boundaries
// ============================================================================

#[test]
fn test_zero_memory_requirement() {
    let mem = MemoryRequirements {
        min_bytes: 0,
        max_bytes: None,
    };

    assert_eq!(mem.min_bytes, 0);
}

#[test]
fn test_zero_cores_requirement() {
    let cpu = CpuRequirements {
        min_cores: 0.0,
        max_cores: None,
        architecture: None,
    };

    assert_eq!(cpu.min_cores, 0.0);
}

#[test]
fn test_very_large_memory() {
    let mem = MemoryRequirements {
        min_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
        max_bytes: None,
    };

    assert_eq!(mem.min_bytes, 1024 * 1024 * 1024 * 1024);
}

#[test]
fn test_many_gpu_units() {
    let gpu = GpuRequirements {
        min_units: 8,
        max_units: Some(16),
        gpu_type: None,
        min_memory_bytes: None,
    };

    assert_eq!(gpu.min_units, 8);
}
