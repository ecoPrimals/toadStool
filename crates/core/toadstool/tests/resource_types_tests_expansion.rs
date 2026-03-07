// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive expansion tests for resource types
#![allow(
    clippy::float_cmp,
    clippy::no_effect_underscore_binding,
    clippy::items_after_statements
)]

use toadstool::resources::*;

// ============================================================================
// CpuRequirements Advanced Tests
// ============================================================================

#[test]
fn test_cpu_requirements_zero_cores() {
    let cpu = CpuRequirements {
        min_cores: 0.0,
        max_cores: Some(1.0),
        architecture: None,
    };

    assert_eq!(cpu.min_cores, 0.0);
}

#[test]
fn test_cpu_requirements_fractional_precision() {
    let cpu = CpuRequirements {
        min_cores: 0.125,
        max_cores: Some(0.375),
        architecture: None,
    };

    assert_eq!(cpu.min_cores, 0.125);
    assert_eq!(cpu.max_cores, Some(0.375));
}

#[test]
fn test_cpu_requirements_large_core_count() {
    let cpu = CpuRequirements {
        min_cores: 128.0,
        max_cores: Some(256.0),
        architecture: Some("x86_64".to_string()),
    };

    assert_eq!(cpu.min_cores, 128.0);
}

#[test]
fn test_cpu_requirements_various_architectures() {
    let architectures = vec!["x86_64", "aarch64", "arm64", "riscv64", "ppc64le"];

    for arch in architectures {
        let cpu = CpuRequirements {
            min_cores: 1.0,
            max_cores: None,
            architecture: Some(arch.to_string()),
        };

        assert_eq!(cpu.architecture, Some(arch.to_string()));
    }
}

#[test]
fn test_cpu_requirements_equal_min_max() {
    let cpu = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(4.0),
        architecture: None,
    };

    assert_eq!(cpu.min_cores, cpu.max_cores.unwrap());
}

#[test]
fn test_cpu_requirements_serialization_roundtrip() {
    let cpu = CpuRequirements {
        min_cores: 2.5,
        max_cores: Some(4.5),
        architecture: Some("x86_64".to_string()),
    };

    let serialized = serde_json::to_string(&cpu).unwrap();
    let deserialized: CpuRequirements = serde_json::from_str(&serialized).unwrap();

    assert_eq!(cpu.min_cores, deserialized.min_cores);
    assert_eq!(cpu.max_cores, deserialized.max_cores);
}

#[test]
fn test_cpu_requirements_debug_format() {
    let cpu = CpuRequirements::default();
    let debug_str = format!("{cpu:?}");
    assert!(debug_str.contains("CpuRequirements"));
}

// ============================================================================
// MemoryRequirements Advanced Tests
// ============================================================================

#[test]
fn test_memory_requirements_zero_bytes() {
    let memory = MemoryRequirements {
        min_bytes: 0,
        max_bytes: Some(1024),
    };

    assert_eq!(memory.min_bytes, 0);
}

#[test]
fn test_memory_requirements_exact_powers_of_two() {
    let sizes = vec![
        1024,                     // 1KB
        1024 * 1024,              // 1MB
        1024 * 1024 * 1024,       // 1GB
        1024 * 1024 * 1024 * 8,   // 8GB
        1024 * 1024 * 1024 * 16,  // 16GB
        1024 * 1024 * 1024 * 32,  // 32GB
        1024 * 1024 * 1024 * 64,  // 64GB
        1024 * 1024 * 1024 * 128, // 128GB
    ];

    for size in sizes {
        let memory = MemoryRequirements {
            min_bytes: size,
            max_bytes: Some(size * 2),
        };

        assert_eq!(memory.min_bytes, size);
    }
}

#[test]
fn test_memory_requirements_odd_sizes() {
    let memory = MemoryRequirements {
        min_bytes: 1_234_567_890,
        max_bytes: Some(9_876_543_210),
    };

    assert_eq!(memory.min_bytes, 1_234_567_890);
}

#[test]
fn test_memory_requirements_equal_min_max() {
    let memory = MemoryRequirements {
        min_bytes: 2 * 1024 * 1024 * 1024,
        max_bytes: Some(2 * 1024 * 1024 * 1024),
    };

    assert_eq!(memory.min_bytes, memory.max_bytes.unwrap());
}

#[test]
fn test_memory_requirements_serialization_roundtrip() {
    let memory = MemoryRequirements {
        min_bytes: 4 * 1024 * 1024 * 1024,
        max_bytes: Some(8 * 1024 * 1024 * 1024),
    };

    let serialized = serde_json::to_string(&memory).unwrap();
    let deserialized: MemoryRequirements = serde_json::from_str(&serialized).unwrap();

    assert_eq!(memory.min_bytes, deserialized.min_bytes);
}

#[test]
fn test_memory_requirements_debug_format() {
    let memory = MemoryRequirements::default();
    let debug_str = format!("{memory:?}");
    assert!(debug_str.contains("MemoryRequirements"));
}

#[test]
fn test_memory_requirements_very_large() {
    let memory = MemoryRequirements {
        min_bytes: 1024 * 1024 * 1024 * 1024,            // 1TB
        max_bytes: Some(1024 * 1024 * 1024 * 1024 * 10), // 10TB
    };

    assert_eq!(memory.min_bytes, 1024 * 1024 * 1024 * 1024);
}

// ============================================================================
// StorageRequirements Advanced Tests
// ============================================================================

#[test]
fn test_storage_requirements_zero_bytes() {
    let storage = StorageRequirements {
        min_bytes: 0,
        max_bytes: Some(1024),
        storage_type: None,
    };

    assert_eq!(storage.min_bytes, 0);
}

#[test]
fn test_storage_requirements_various_types() {
    let types = vec!["ssd", "hdd", "nvme", "ram", "network", "ephemeral"];

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
fn test_storage_requirements_petabyte_scale() {
    let storage = StorageRequirements {
        min_bytes: 1024 * 1024 * 1024 * 1024 * 1024, // 1PB
        max_bytes: None,
        storage_type: Some("distributed".to_string()),
    };

    assert_eq!(storage.min_bytes, 1024 * 1024 * 1024 * 1024 * 1024);
}

#[test]
fn test_storage_requirements_equal_min_max() {
    let storage = StorageRequirements {
        min_bytes: 50 * 1024 * 1024 * 1024,
        max_bytes: Some(50 * 1024 * 1024 * 1024),
        storage_type: Some("ssd".to_string()),
    };

    assert_eq!(storage.min_bytes, storage.max_bytes.unwrap());
}

#[test]
fn test_storage_requirements_serialization_roundtrip() {
    let storage = StorageRequirements {
        min_bytes: 100 * 1024 * 1024 * 1024,
        max_bytes: Some(500 * 1024 * 1024 * 1024),
        storage_type: Some("nvme".to_string()),
    };

    let serialized = serde_json::to_string(&storage).unwrap();
    let deserialized: StorageRequirements = serde_json::from_str(&serialized).unwrap();

    assert_eq!(storage.min_bytes, deserialized.min_bytes);
    assert_eq!(storage.storage_type, deserialized.storage_type);
}

#[test]
fn test_storage_requirements_debug_format() {
    let storage = StorageRequirements::default();
    let debug_str = format!("{storage:?}");
    assert!(debug_str.contains("StorageRequirements"));
}

// ============================================================================
// NetworkRequirements Advanced Tests
// ============================================================================

#[test]
fn test_network_requirements_zero_bandwidth() {
    let network = NetworkRequirements {
        min_bandwidth: Some(0),
        max_bandwidth: Some(0),
        max_latency_ms: None,
    };

    assert_eq!(network.min_bandwidth, Some(0));
}

#[test]
fn test_network_requirements_high_bandwidth() {
    let network = NetworkRequirements {
        min_bandwidth: Some(1_000_000_000),  // 1 GB/s
        max_bandwidth: Some(10_000_000_000), // 10 GB/s
        max_latency_ms: Some(1),
    };

    assert_eq!(network.min_bandwidth, Some(1_000_000_000));
}

#[test]
fn test_network_requirements_various_latencies() {
    let latencies = vec![1, 5, 10, 20, 50, 100, 200, 500, 1000];

    for latency in latencies {
        let network = NetworkRequirements {
            min_bandwidth: None,
            max_bandwidth: None,
            max_latency_ms: Some(latency),
        };

        assert_eq!(network.max_latency_ms, Some(latency));
    }
}

#[test]
fn test_network_requirements_equal_bandwidth() {
    let network = NetworkRequirements {
        min_bandwidth: Some(1_000_000),
        max_bandwidth: Some(1_000_000),
        max_latency_ms: None,
    };

    assert_eq!(network.min_bandwidth, network.max_bandwidth);
}

#[test]
fn test_network_requirements_serialization_roundtrip() {
    let network = NetworkRequirements {
        min_bandwidth: Some(500_000),
        max_bandwidth: Some(5_000_000),
        max_latency_ms: Some(50),
    };

    let serialized = serde_json::to_string(&network).unwrap();
    let deserialized: NetworkRequirements = serde_json::from_str(&serialized).unwrap();

    assert_eq!(network.min_bandwidth, deserialized.min_bandwidth);
    assert_eq!(network.max_latency_ms, deserialized.max_latency_ms);
}

#[test]
fn test_network_requirements_debug_format() {
    let network = NetworkRequirements::default();
    let debug_str = format!("{network:?}");
    assert!(debug_str.contains("NetworkRequirements"));
}

// ============================================================================
// GpuRequirements Advanced Tests
// ============================================================================

#[test]
fn test_gpu_requirements_zero_units() {
    let gpu = GpuRequirements {
        min_units: 0,
        max_units: Some(1),
        gpu_type: None,
        min_memory_bytes: None,
    };

    assert_eq!(gpu.min_units, 0);
}

#[test]
fn test_gpu_requirements_various_types() {
    let types = vec!["NVIDIA", "AMD", "Intel", "Apple", "Custom"];

    for gpu_type in types {
        let gpu = GpuRequirements {
            min_units: 1,
            max_units: None,
            gpu_type: Some(gpu_type.to_string()),
            min_memory_bytes: None,
        };

        assert_eq!(gpu.gpu_type, Some(gpu_type.to_string()));
    }
}

#[test]
fn test_gpu_requirements_various_memory_sizes() {
    let sizes = vec![
        2 * 1024 * 1024 * 1024,  // 2GB
        4 * 1024 * 1024 * 1024,  // 4GB
        8 * 1024 * 1024 * 1024,  // 8GB
        16 * 1024 * 1024 * 1024, // 16GB
        24 * 1024 * 1024 * 1024, // 24GB
        32 * 1024 * 1024 * 1024, // 32GB
        48 * 1024 * 1024 * 1024, // 48GB
        80 * 1024 * 1024 * 1024, // 80GB
    ];

    for size in sizes {
        let gpu = GpuRequirements {
            min_units: 1,
            max_units: None,
            gpu_type: None,
            min_memory_bytes: Some(size),
        };

        assert_eq!(gpu.min_memory_bytes, Some(size));
    }
}

#[test]
fn test_gpu_requirements_many_units() {
    let gpu = GpuRequirements {
        min_units: 8,
        max_units: Some(16),
        gpu_type: Some("NVIDIA".to_string()),
        min_memory_bytes: Some(80 * 1024 * 1024 * 1024),
    };

    assert_eq!(gpu.min_units, 8);
    assert_eq!(gpu.max_units, Some(16));
}

#[test]
fn test_gpu_requirements_equal_min_max_units() {
    let gpu = GpuRequirements {
        min_units: 4,
        max_units: Some(4),
        gpu_type: None,
        min_memory_bytes: None,
    };

    assert_eq!(gpu.min_units, gpu.max_units.unwrap());
}

#[test]
fn test_gpu_requirements_serialization_roundtrip() {
    let gpu = GpuRequirements {
        min_units: 2,
        max_units: Some(4),
        gpu_type: Some("NVIDIA".to_string()),
        min_memory_bytes: Some(16 * 1024 * 1024 * 1024),
    };

    let serialized = serde_json::to_string(&gpu).unwrap();
    let deserialized: GpuRequirements = serde_json::from_str(&serialized).unwrap();

    assert_eq!(gpu.min_units, deserialized.min_units);
    assert_eq!(gpu.gpu_type, deserialized.gpu_type);
}

#[test]
fn test_gpu_requirements_debug_format() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: None,
        gpu_type: None,
        min_memory_bytes: None,
    };

    let debug_str = format!("{gpu:?}");
    assert!(debug_str.contains("GpuRequirements"));
}

// ============================================================================
// ResourceRequirements Advanced Tests
// ============================================================================

#[test]
fn test_resource_requirements_minimal() {
    let resources = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.1,
            max_cores: Some(0.5),
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 128 * 1024 * 1024, // 128MB
            max_bytes: Some(256 * 1024 * 1024),
        },
        storage: StorageRequirements {
            min_bytes: 100 * 1024 * 1024, // 100MB
            max_bytes: None,
            storage_type: None,
        },
        gpu: None,
        network: NetworkRequirements::default(),
    };

    assert_eq!(resources.cpu.min_cores, 0.1);
}

#[test]
fn test_resource_requirements_maximal() {
    let resources = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 128.0,
            max_cores: Some(256.0),
            architecture: Some("x86_64".to_string()),
        },
        memory: MemoryRequirements {
            min_bytes: 1024 * 1024 * 1024 * 1024, // 1TB
            max_bytes: Some(1024 * 1024 * 1024 * 1024 * 2),
        },
        storage: StorageRequirements {
            min_bytes: 1024 * 1024 * 1024 * 1024 * 10, // 10TB
            max_bytes: None,
            storage_type: Some("nvme".to_string()),
        },
        gpu: Some(GpuRequirements {
            min_units: 8,
            max_units: Some(16),
            gpu_type: Some("NVIDIA".to_string()),
            min_memory_bytes: Some(80 * 1024 * 1024 * 1024),
        }),
        network: NetworkRequirements {
            min_bandwidth: Some(1_000_000_000),
            max_bandwidth: Some(10_000_000_000),
            max_latency_ms: Some(1),
        },
    };

    assert!(resources.gpu.is_some());
    assert_eq!(resources.cpu.min_cores, 128.0);
}

#[test]
fn test_resource_requirements_various_combinations() {
    let combinations = vec![
        (1.0, 1024 * 1024 * 1024, true),
        (2.0, 2 * 1024 * 1024 * 1024, false),
        (4.0, 4 * 1024 * 1024 * 1024, true),
        (8.0, 8 * 1024 * 1024 * 1024, false),
    ];

    for (cores, memory, has_gpu) in combinations {
        let resources = ResourceRequirements {
            cpu: CpuRequirements {
                min_cores: cores,
                max_cores: None,
                architecture: None,
            },
            memory: MemoryRequirements {
                min_bytes: memory,
                max_bytes: None,
            },
            storage: StorageRequirements::default(),
            gpu: if has_gpu {
                Some(GpuRequirements {
                    min_units: 1,
                    max_units: None,
                    gpu_type: None,
                    min_memory_bytes: None,
                })
            } else {
                None
            },
            network: NetworkRequirements::default(),
        };

        assert_eq!(resources.cpu.min_cores, cores);
        assert_eq!(resources.gpu.is_some(), has_gpu);
    }
}

#[test]
fn test_resource_requirements_serialization_roundtrip() {
    let resources = ResourceRequirements::default();

    let serialized = serde_json::to_string(&resources).unwrap();
    let deserialized: ResourceRequirements = serde_json::from_str(&serialized).unwrap();

    assert_eq!(resources.cpu.min_cores, deserialized.cpu.min_cores);
    assert_eq!(resources.memory.min_bytes, deserialized.memory.min_bytes);
}

#[test]
fn test_resource_requirements_debug_format() {
    let resources = ResourceRequirements::default();
    let debug_str = format!("{resources:?}");
    assert!(debug_str.contains("ResourceRequirements"));
}

#[test]
fn test_resource_requirements_network_critical() {
    let resources = ResourceRequirements {
        cpu: CpuRequirements::default(),
        memory: MemoryRequirements::default(),
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements {
            min_bandwidth: Some(100_000_000),   // 100 MB/s
            max_bandwidth: Some(1_000_000_000), // 1 GB/s
            max_latency_ms: Some(5),
        },
    };

    assert!(resources.network.min_bandwidth.is_some());
    assert_eq!(resources.network.max_latency_ms, Some(5));
}

// ============================================================================
// CpuMetrics Advanced Tests
// ============================================================================

#[test]
fn test_cpu_metrics_zero_usage() {
    let metrics = CpuMetrics {
        usage_percent: 0.0,
        cores_used: 0.0,
        cpu_time_seconds: 0.0,
    };

    assert_eq!(metrics.usage_percent, 0.0);
}

#[test]
fn test_cpu_metrics_fractional_cores() {
    let metrics = CpuMetrics {
        usage_percent: 12.5,
        cores_used: 0.125,
        cpu_time_seconds: 1.5,
    };

    assert_eq!(metrics.cores_used, 0.125);
}

#[test]
fn test_cpu_metrics_over_100_percent() {
    let metrics = CpuMetrics {
        usage_percent: 150.0,
        cores_used: 1.5,
        cpu_time_seconds: 100.0,
    };

    assert_eq!(metrics.usage_percent, 150.0);
}

#[test]
fn test_cpu_metrics_long_running() {
    let metrics = CpuMetrics {
        usage_percent: 75.0,
        cores_used: 3.0,
        cpu_time_seconds: 86400.0, // 24 hours
    };

    assert_eq!(metrics.cpu_time_seconds, 86400.0);
}

#[test]
fn test_cpu_metrics_serialization_roundtrip() {
    let metrics = CpuMetrics {
        usage_percent: 42.5,
        cores_used: 2.5,
        cpu_time_seconds: 30.0,
    };

    let serialized = serde_json::to_string(&metrics).unwrap();
    let deserialized: CpuMetrics = serde_json::from_str(&serialized).unwrap();

    assert_eq!(metrics.usage_percent, deserialized.usage_percent);
}

#[test]
fn test_cpu_metrics_debug_format() {
    let metrics = CpuMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("CpuMetrics"));
}

// ============================================================================
// RuntimeMetrics Advanced Tests
// ============================================================================

#[test]
fn test_runtime_metrics_with_gpu() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 50.0,
            cores_used: 2.0,
            cpu_time_seconds: 10.0,
        },
        gpu: Some(GpuMetrics {
            usage_percent: 80.0,
            memory_usage_percent: 50.0,
            memory_used_bytes: 4 * 1024 * 1024 * 1024,
            temperature_celsius: Some(65.0),
        }),
        ..Default::default()
    };

    assert!(metrics.gpu.is_some());
    assert_eq!(metrics.gpu.unwrap().usage_percent, 80.0);
}

#[test]
fn test_runtime_metrics_gpu_high_usage() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics::default(),
        gpu: Some(GpuMetrics {
            usage_percent: 95.0,
            memory_usage_percent: 90.0,
            memory_used_bytes: 7 * 1024 * 1024 * 1024,
            temperature_celsius: Some(85.0),
        }),
        ..Default::default()
    };

    let gpu = metrics.gpu.unwrap();
    assert_eq!(gpu.usage_percent, 95.0);
    assert_eq!(gpu.memory_usage_percent, 90.0);
}

#[test]
fn test_runtime_metrics_serialization_roundtrip() {
    let metrics = RuntimeMetrics::default();

    let serialized = serde_json::to_string(&metrics).unwrap();
    let deserialized: RuntimeMetrics = serde_json::from_str(&serialized).unwrap();

    assert_eq!(metrics.cpu.usage_percent, deserialized.cpu.usage_percent);
}

#[test]
fn test_runtime_metrics_debug_format() {
    let metrics = RuntimeMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("RuntimeMetrics"));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_requirements_to_metrics_flow() {
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 2.0,
            max_cores: Some(4.0),
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 2 * 1024 * 1024 * 1024,
            max_bytes: Some(4 * 1024 * 1024 * 1024),
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 50.0,
            cores_used: 2.0,
            cpu_time_seconds: 30.0,
        },
        ..Default::default()
    };

    // Verify that actual usage is within requirements
    assert!(metrics.cpu.cores_used >= requirements.cpu.min_cores);
    assert!(metrics.cpu.cores_used <= requirements.cpu.max_cores.unwrap());
}

#[test]
fn test_complete_resource_specification() {
    let resources = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(8.0),
            architecture: Some("x86_64".to_string()),
        },
        memory: MemoryRequirements {
            min_bytes: 8 * 1024 * 1024 * 1024,
            max_bytes: Some(16 * 1024 * 1024 * 1024),
        },
        storage: StorageRequirements {
            min_bytes: 100 * 1024 * 1024 * 1024,
            max_bytes: Some(500 * 1024 * 1024 * 1024),
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

    // Verify all components are properly set
    assert!(resources.cpu.architecture.is_some());
    assert!(resources.storage.storage_type.is_some());
    assert!(resources.gpu.is_some());
    assert!(resources.network.min_bandwidth.is_some());
}
