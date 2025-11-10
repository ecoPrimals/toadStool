//! Comprehensive tests for resource types

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
fn test_cpu_requirements_with_cores() {
    let cpu = CpuRequirements {
        min_cores: 2.0,
        max_cores: Some(4.0),
        architecture: None,
    };

    assert_eq!(cpu.min_cores, 2.0);
    assert_eq!(cpu.max_cores, Some(4.0));
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
    let cpu1 = CpuRequirements::default();
    let cpu2 = cpu1.clone();

    assert_eq!(cpu1.min_cores, cpu2.min_cores);
    assert_eq!(cpu1.max_cores, cpu2.max_cores);
}

#[test]
fn test_cpu_requirements_serialization() {
    let cpu = CpuRequirements::default();
    let serialized = serde_json::to_string(&cpu).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// MemoryRequirements Tests
// ============================================================================

#[test]
fn test_memory_requirements_default() {
    let memory = MemoryRequirements::default();

    assert_eq!(memory.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert!(memory.max_bytes.is_none());
}

#[test]
fn test_memory_requirements_with_limits() {
    let memory = MemoryRequirements {
        min_bytes: 512 * 1024 * 1024,            // 512MB
        max_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
    };

    assert_eq!(memory.min_bytes, 512 * 1024 * 1024);
    assert_eq!(memory.max_bytes, Some(2 * 1024 * 1024 * 1024));
}

#[test]
fn test_memory_requirements_small() {
    let memory = MemoryRequirements {
        min_bytes: 1024,              // 1KB
        max_bytes: Some(1024 * 1024), // 1MB
    };

    assert_eq!(memory.min_bytes, 1024);
}

#[test]
fn test_memory_requirements_large() {
    let memory = MemoryRequirements {
        min_bytes: 16 * 1024 * 1024 * 1024,       // 16GB
        max_bytes: Some(64 * 1024 * 1024 * 1024), // 64GB
    };

    assert_eq!(memory.min_bytes, 16 * 1024 * 1024 * 1024);
}

#[test]
fn test_memory_requirements_clone() {
    let mem1 = MemoryRequirements::default();
    let mem2 = mem1.clone();

    assert_eq!(mem1.min_bytes, mem2.min_bytes);
    assert_eq!(mem1.max_bytes, mem2.max_bytes);
}

#[test]
fn test_memory_requirements_serialization() {
    let memory = MemoryRequirements::default();
    let serialized = serde_json::to_string(&memory).unwrap();
    assert!(!serialized.is_empty());
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
fn test_storage_requirements_different_types() {
    let types = vec!["ssd", "hdd", "nvme", "ram"];

    for storage_type in types {
        let storage = StorageRequirements {
            min_bytes: 1024 * 1024,
            max_bytes: None,
            storage_type: Some(storage_type.to_string()),
        };

        assert_eq!(storage.storage_type, Some(storage_type.to_string()));
    }
}

#[test]
fn test_storage_requirements_clone() {
    let storage1 = StorageRequirements::default();
    let storage2 = storage1.clone();

    assert_eq!(storage1.min_bytes, storage2.min_bytes);
    assert_eq!(storage1.storage_type, storage2.storage_type);
}

#[test]
fn test_storage_requirements_serialization() {
    let storage = StorageRequirements::default();
    let serialized = serde_json::to_string(&storage).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// NetworkRequirements Tests
// ============================================================================

#[test]
fn test_network_requirements_default() {
    let network = NetworkRequirements::default();

    assert!(network.min_bandwidth.is_none());
    assert!(network.max_bandwidth.is_none());
    assert!(network.max_latency_ms.is_none());
}

#[test]
fn test_network_requirements_with_bandwidth() {
    let network = NetworkRequirements {
        min_bandwidth: Some(1_000_000),  // 1 MB/s
        max_bandwidth: Some(10_000_000), // 10 MB/s
        max_latency_ms: None,
    };

    assert_eq!(network.min_bandwidth, Some(1_000_000));
    assert_eq!(network.max_bandwidth, Some(10_000_000));
}

#[test]
fn test_network_requirements_with_latency() {
    let network = NetworkRequirements {
        min_bandwidth: None,
        max_bandwidth: None,
        max_latency_ms: Some(100), // 100ms
    };

    assert_eq!(network.max_latency_ms, Some(100));
}

#[test]
fn test_network_requirements_all_fields() {
    let network = NetworkRequirements {
        min_bandwidth: Some(500_000),
        max_bandwidth: Some(5_000_000),
        max_latency_ms: Some(50),
    };

    assert!(network.min_bandwidth.is_some());
    assert!(network.max_bandwidth.is_some());
    assert!(network.max_latency_ms.is_some());
}

#[test]
fn test_network_requirements_clone() {
    let network1 = NetworkRequirements::default();
    let network2 = network1.clone();

    assert_eq!(network1.min_bandwidth, network2.min_bandwidth);
    assert_eq!(network1.max_latency_ms, network2.max_latency_ms);
}

#[test]
fn test_network_requirements_serialization() {
    let network = NetworkRequirements::default();
    let serialized = serde_json::to_string(&network).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// GpuRequirements Tests
// ============================================================================

#[test]
fn test_gpu_requirements_basic() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: Some(2),
        gpu_type: None,
        min_memory_bytes: None,
    };

    assert_eq!(gpu.min_units, 1);
    assert_eq!(gpu.max_units, Some(2));
}

#[test]
fn test_gpu_requirements_with_type() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: None,
        gpu_type: Some("NVIDIA".to_string()),
        min_memory_bytes: None,
    };

    assert_eq!(gpu.gpu_type, Some("NVIDIA".to_string()));
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
fn test_gpu_requirements_all_fields() {
    let gpu = GpuRequirements {
        min_units: 2,
        max_units: Some(4),
        gpu_type: Some("AMD".to_string()),
        min_memory_bytes: Some(16 * 1024 * 1024 * 1024), // 16GB
    };

    assert_eq!(gpu.min_units, 2);
    assert!(gpu.max_units.is_some());
    assert!(gpu.gpu_type.is_some());
    assert!(gpu.min_memory_bytes.is_some());
}

#[test]
fn test_gpu_requirements_clone() {
    let gpu1 = GpuRequirements {
        min_units: 1,
        max_units: Some(2),
        gpu_type: Some("Intel".to_string()),
        min_memory_bytes: Some(4 * 1024 * 1024 * 1024),
    };

    let gpu2 = gpu1.clone();

    assert_eq!(gpu1.min_units, gpu2.min_units);
    assert_eq!(gpu1.gpu_type, gpu2.gpu_type);
}

#[test]
fn test_gpu_requirements_serialization() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: None,
        gpu_type: None,
        min_memory_bytes: None,
    };

    let serialized = serde_json::to_string(&gpu).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// ResourceRequirements Tests
// ============================================================================

#[test]
fn test_resource_requirements_default() {
    let resources = ResourceRequirements::default();

    assert_eq!(resources.cpu.min_cores, 1.0);
    assert_eq!(resources.memory.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(resources.storage.min_bytes, 1024 * 1024 * 1024);
    assert!(resources.gpu.is_none());
}

#[test]
fn test_resource_requirements_with_gpu() {
    let resources = ResourceRequirements {
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: None,
            gpu_type: None,
            min_memory_bytes: None,
        }),
        ..Default::default()
    };

    assert!(resources.gpu.is_some());
}

#[test]
fn test_resource_requirements_custom() {
    let resources = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(8.0),
            architecture: Some("arm64".to_string()),
        },
        memory: MemoryRequirements {
            min_bytes: 4 * 1024 * 1024 * 1024,
            max_bytes: Some(16 * 1024 * 1024 * 1024),
        },
        storage: StorageRequirements {
            min_bytes: 50 * 1024 * 1024 * 1024,
            max_bytes: None,
            storage_type: Some("ssd".to_string()),
        },
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: Some(2),
            gpu_type: Some("NVIDIA".to_string()),
            min_memory_bytes: Some(8 * 1024 * 1024 * 1024),
        }),
        network: NetworkRequirements {
            min_bandwidth: Some(1_000_000),
            max_bandwidth: Some(100_000_000),
            max_latency_ms: Some(20),
        },
    };

    assert_eq!(resources.cpu.min_cores, 4.0);
    assert!(resources.gpu.is_some());
    assert_eq!(resources.network.max_latency_ms, Some(20));
}

#[test]
fn test_resource_requirements_clone() {
    let resources1 = ResourceRequirements::default();
    let resources2 = resources1.clone();

    assert_eq!(resources1.cpu.min_cores, resources2.cpu.min_cores);
    assert_eq!(resources1.memory.min_bytes, resources2.memory.min_bytes);
}

#[test]
fn test_resource_requirements_serialization() {
    let resources = ResourceRequirements::default();
    let serialized = serde_json::to_string(&resources).unwrap();
    assert!(!serialized.is_empty());
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
fn test_cpu_metrics_with_values() {
    let metrics = CpuMetrics {
        usage_percent: 75.5,
        cores_used: 3.2,
        cpu_time_seconds: 42.0,
    };

    assert_eq!(metrics.usage_percent, 75.5);
    assert_eq!(metrics.cores_used, 3.2);
    assert_eq!(metrics.cpu_time_seconds, 42.0);
}

#[test]
fn test_cpu_metrics_max_usage() {
    let metrics = CpuMetrics {
        usage_percent: 100.0,
        cores_used: 16.0,
        cpu_time_seconds: 1000.0,
    };

    assert_eq!(metrics.usage_percent, 100.0);
}

#[test]
fn test_cpu_metrics_clone() {
    let metrics1 = CpuMetrics {
        usage_percent: 50.0,
        cores_used: 2.0,
        cpu_time_seconds: 10.0,
    };

    let metrics2 = metrics1.clone();

    assert_eq!(metrics1.usage_percent, metrics2.usage_percent);
}

#[test]
fn test_cpu_metrics_serialization() {
    let metrics = CpuMetrics::default();
    let serialized = serde_json::to_string(&metrics).unwrap();
    assert!(!serialized.is_empty());
}

// ============================================================================
// RuntimeMetrics Tests
// ============================================================================

#[test]
fn test_runtime_metrics_default() {
    let metrics = RuntimeMetrics::default();

    assert_eq!(metrics.cpu.usage_percent, 0.0);
    assert!(metrics.gpu.is_none());
}

#[test]
fn test_runtime_metrics_clone() {
    let metrics1 = RuntimeMetrics::default();
    let metrics2 = metrics1.clone();

    assert_eq!(metrics1.cpu.usage_percent, metrics2.cpu.usage_percent);
}

#[test]
fn test_runtime_metrics_serialization() {
    let metrics = RuntimeMetrics::default();
    let serialized = serde_json::to_string(&metrics).unwrap();
    assert!(!serialized.is_empty());
}
