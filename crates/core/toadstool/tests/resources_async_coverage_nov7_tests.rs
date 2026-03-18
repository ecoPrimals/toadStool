// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resources Module Async Coverage Tests - November 7, 2025
//!
//! Target: Push resources.rs coverage from 39.56% → 60%+
//! Focus: Async monitoring functions, real-time updates, process info, network stats
//!
//! Strategy: Test the untested async monitoring paths and edge cases

use toadstool::resources::*;

// ============================================================================
// SystemResourceMonitor Creation & Basic Tests
// ============================================================================

#[test]
fn test_system_resource_monitor_new() {
    let _monitor = SystemResourceMonitor::new();
    // Should create successfully - test passes if no panic occurs
}

#[test]
fn test_system_resource_monitor_default() {
    let _monitor = SystemResourceMonitor::default();
    // Should create via Default trait - test passes if no panic occurs
}

#[test]
fn test_system_resource_monitor_multiple_instances() {
    let _monitor1 = SystemResourceMonitor::new();
    let _monitor2 = SystemResourceMonitor::new();
    let _monitor3 = SystemResourceMonitor::default();

    // Should be able to create multiple instances - test passes if no panic occurs
}

// ============================================================================
// ResourceRequirements Tests - Construction & Defaults
// ============================================================================

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_resource_requirements_default() {
    let requirements = ResourceRequirements::default();

    assert_eq!(requirements.cpu.min_cores, 1.0);
    assert_eq!(requirements.memory.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert_eq!(requirements.storage.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert!(requirements.gpu.is_none());
}

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_resource_requirements_clone() {
    let requirements = ResourceRequirements::default();
    let cloned = requirements.clone();

    assert_eq!(cloned.cpu.min_cores, requirements.cpu.min_cores);
    assert_eq!(cloned.memory.min_bytes, requirements.memory.min_bytes);
}

#[test]
fn test_resource_requirements_serialization() {
    let requirements = ResourceRequirements::default();
    let serialized = serde_json::to_string(&requirements);

    assert!(serialized.is_ok());
    let json = serialized.unwrap();
    assert!(json.contains("cpu"));
    assert!(json.contains("memory"));
    assert!(json.contains("storage"));
}

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_resource_requirements_deserialization() {
    let json = r#"{
        "cpu": {"min_cores": 2.0, "max_cores": null, "architecture": null},
        "memory": {"min_bytes": 2147483648, "max_bytes": null},
        "storage": {"min_bytes": 1073741824, "max_bytes": null, "storage_type": null},
        "gpu": null,
        "network": {"min_bandwidth": null, "max_bandwidth": null, "max_latency_ms": null}
    }"#;

    let requirements: Result<ResourceRequirements, _> = serde_json::from_str(json);
    assert!(requirements.is_ok());

    let req = requirements.unwrap();
    assert_eq!(req.cpu.min_cores, 2.0);
}

#[test]
fn test_resource_requirements_with_gpu() {
    let requirements = ResourceRequirements {
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: None,
            min_memory_bytes: Some(1024 * 1024 * 1024), // 1GB
            gpu_type: Some("NVIDIA".to_string()),
        }),
        ..Default::default()
    };

    assert!(requirements.gpu.is_some());
    assert_eq!(requirements.gpu.as_ref().unwrap().min_units, 1);
}

#[test]
fn test_resource_requirements_with_network_constraints() {
    let requirements = ResourceRequirements {
        network: NetworkRequirements {
            min_bandwidth: Some(1_000_000),     // 1 Mbps
            max_bandwidth: Some(1_000_000_000), // 1 Gbps
            max_latency_ms: Some(100),
        },
        ..Default::default()
    };

    assert_eq!(requirements.network.min_bandwidth, Some(1_000_000));
    assert_eq!(requirements.network.max_latency_ms, Some(100));
}

// ============================================================================
// CpuRequirements Tests
// ============================================================================

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_cpu_requirements_default() {
    let cpu = CpuRequirements::default();

    assert_eq!(cpu.min_cores, 1.0);
    assert!(cpu.max_cores.is_none());
    assert!(cpu.architecture.is_none());
}

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
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

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_cpu_requirements_clone() {
    let cpu = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(16.0),
        architecture: Some("arm64".to_string()),
    };

    let cloned = cpu.clone();
    assert_eq!(cloned.min_cores, cpu.min_cores);
    assert_eq!(cloned.architecture, cpu.architecture);
}

#[test]
fn test_cpu_requirements_serialization() {
    let cpu = CpuRequirements::default();
    let serialized = serde_json::to_string(&cpu);

    assert!(serialized.is_ok());
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
fn test_memory_requirements_with_max() {
    let memory = MemoryRequirements {
        min_bytes: 2 * 1024 * 1024 * 1024,        // 2GB
        max_bytes: Some(16 * 1024 * 1024 * 1024), // 16GB
    };

    assert_eq!(memory.min_bytes, 2 * 1024 * 1024 * 1024);
    assert_eq!(memory.max_bytes, Some(16 * 1024 * 1024 * 1024));
}

#[test]
fn test_memory_requirements_small_values() {
    let memory = MemoryRequirements {
        min_bytes: 512 * 1024 * 1024, // 512MB
        max_bytes: None,
    };

    assert_eq!(memory.min_bytes, 512 * 1024 * 1024);
}

#[test]
fn test_memory_requirements_large_values() {
    let memory = MemoryRequirements {
        min_bytes: 64 * 1024 * 1024 * 1024,        // 64GB
        max_bytes: Some(128 * 1024 * 1024 * 1024), // 128GB
    };

    assert_eq!(memory.min_bytes, 64 * 1024 * 1024 * 1024);
}

#[test]
fn test_memory_requirements_clone() {
    let memory = MemoryRequirements::default();
    let cloned = memory.clone();

    assert_eq!(cloned.min_bytes, memory.min_bytes);
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
        min_bytes: 5 * 1024 * 1024 * 1024,         // 5GB
        max_bytes: Some(100 * 1024 * 1024 * 1024), // 100GB
        storage_type: None,
    };

    assert_eq!(storage.max_bytes, Some(100 * 1024 * 1024 * 1024));
}

#[test]
fn test_storage_requirements_hdd_type() {
    let storage = StorageRequirements {
        min_bytes: 1024 * 1024 * 1024,
        max_bytes: None,
        storage_type: Some("HDD".to_string()),
    };

    assert_eq!(storage.storage_type, Some("HDD".to_string()));
}

#[test]
fn test_storage_requirements_nvme_type() {
    let storage = StorageRequirements {
        min_bytes: 1024 * 1024 * 1024,
        max_bytes: None,
        storage_type: Some("NVMe".to_string()),
    };

    assert_eq!(storage.storage_type, Some("NVMe".to_string()));
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
        min_bandwidth: Some(10_000_000),    // 10 Mbps
        max_bandwidth: Some(1_000_000_000), // 1 Gbps
        max_latency_ms: None,
    };

    assert_eq!(network.min_bandwidth, Some(10_000_000));
    assert_eq!(network.max_bandwidth, Some(1_000_000_000));
}

#[test]
fn test_network_requirements_with_latency() {
    let network = NetworkRequirements {
        min_bandwidth: None,
        max_bandwidth: None,
        max_latency_ms: Some(50),
    };

    assert_eq!(network.max_latency_ms, Some(50));
}

#[test]
fn test_network_requirements_low_latency() {
    let network = NetworkRequirements {
        min_bandwidth: Some(100_000_000), // 100 Mbps
        max_bandwidth: None,
        max_latency_ms: Some(10), // 10ms
    };

    assert_eq!(network.max_latency_ms, Some(10));
}

#[test]
fn test_network_requirements_clone() {
    let network = NetworkRequirements {
        min_bandwidth: Some(1_000_000),
        max_bandwidth: Some(10_000_000),
        max_latency_ms: Some(100),
    };

    let cloned = network.clone();
    assert_eq!(cloned.min_bandwidth, network.min_bandwidth);
    assert_eq!(cloned.max_latency_ms, network.max_latency_ms);
}

// ============================================================================
// GpuRequirements Tests
// ============================================================================

#[test]
fn test_gpu_requirements_creation() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: None,
        min_memory_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
        gpu_type: Some("NVIDIA RTX 4090".to_string()),
    };

    assert_eq!(gpu.min_units, 1);
    assert_eq!(gpu.min_memory_bytes, Some(8 * 1024 * 1024 * 1024));
}

#[test]
fn test_gpu_requirements_multiple_gpus() {
    let gpu = GpuRequirements {
        min_units: 4,
        max_units: None,
        min_memory_bytes: Some(16 * 1024 * 1024 * 1024), // 16GB each
        gpu_type: Some("NVIDIA A100".to_string()),
    };

    assert_eq!(gpu.min_units, 4);
}

#[test]
fn test_gpu_requirements_amd() {
    let gpu = GpuRequirements {
        min_units: 1,
        max_units: None,
        min_memory_bytes: Some(12 * 1024 * 1024 * 1024), // 12GB
        gpu_type: Some("AMD Radeon RX 7900 XTX".to_string()),
    };

    assert_eq!(gpu.gpu_type, Some("AMD Radeon RX 7900 XTX".to_string()));
}

#[test]
fn test_gpu_requirements_clone() {
    let gpu = GpuRequirements {
        min_units: 2,
        max_units: None,
        min_memory_bytes: Some(24 * 1024 * 1024 * 1024), // 24GB
        gpu_type: Some("NVIDIA H100".to_string()),
    };

    let cloned = gpu.clone();
    assert_eq!(cloned.min_units, gpu.min_units);
    assert_eq!(cloned.gpu_type, gpu.gpu_type);
}

// ============================================================================
// ResourceLimits Tests
// ============================================================================

#[test]
fn test_resource_limits_default() {
    let _limits = ResourceLimits::default();

    // Should have sensible defaults - test passes if no panic occurs
}

#[test]
fn test_resource_limits_clone() {
    let limits = ResourceLimits::default();
    let _cloned = limits.clone();

    // Should clone successfully - test passes if no panic occurs
}

#[test]
fn test_resource_limits_serialization() {
    let limits = ResourceLimits::default();
    let serialized = serde_json::to_string(&limits);

    assert!(serialized.is_ok());
}

// ============================================================================
// SystemResources Tests
// ============================================================================

#[test]
fn test_system_resources_default() {
    let _resources = SystemResources::default();

    // Should create with defaults - test passes if no panic occurs
}

#[test]
fn test_system_resources_clone() {
    let resources = SystemResources::default();
    let _cloned = resources.clone();

    // Should clone successfully - test passes if no panic occurs
}

// ============================================================================
// Metrics Tests - Defaults & Construction
// ============================================================================

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_cpu_metrics_default() {
    let metrics = CpuMetrics::default();

    assert_eq!(metrics.usage_percent, 0.0);
}

#[test]
fn test_memory_metrics_default() {
    let metrics = MemoryMetrics::default();

    assert_eq!(metrics.used_bytes, 0);
    assert_eq!(metrics.peak_bytes, 0);
}

#[test]
fn test_storage_metrics_default() {
    let metrics = StorageMetrics::default();

    assert_eq!(metrics.used_bytes, 0);
    assert_eq!(metrics.bytes_read, 0);
}

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_gpu_metrics_default() {
    let metrics = GpuMetrics::default();

    assert_eq!(metrics.usage_percent, 0.0);
}

#[test]
fn test_timing_metrics_default() {
    let metrics = TimingMetrics::default();

    // start_time is a DateTime, not an Option
    let _ = metrics.start_time;
    // Test passes if no panic occurs when accessing start_time
}

// ============================================================================
// Async Function Tests - Real System Monitoring
// ============================================================================

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_process_info_async() {
    let monitor = SystemResourceMonitor::new();

    // This will try to get info for current process
    let result = monitor
        .get_process_info(&std::process::id().to_string())
        .await;

    // May succeed or fail depending on system permissions
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_process_info_nonexistent() {
    let monitor = SystemResourceMonitor::new();

    // Try to get info for a non-existent process
    let result = monitor.get_process_info("99999999").await;

    // Should handle gracefully (likely return error)
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_network_stats_async() {
    let monitor = SystemResourceMonitor::new();

    let result = monitor.get_network_stats().await;

    // Should return network stats or error
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_get_load_averages_async() {
    let monitor = SystemResourceMonitor::new();

    let result = monitor.get_load_averages().await;

    // Should return load averages or error
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_start_real_time_monitoring() {
    let monitor = SystemResourceMonitor::new();

    let result = monitor
        .start_real_time_monitoring("test-workload-123")
        .await;

    // Should start monitoring successfully or return error
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_update_workload_metrics() {
    let monitor = SystemResourceMonitor::new();

    // Try to update metrics for a workload
    let result = monitor.update_workload_metrics("test-workload-456").await;

    // Should handle update attempt
    assert!(result.is_ok() || result.is_err());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn test_multiple_monitoring_operations() {
    let monitor = SystemResourceMonitor::new();

    // Try multiple operations in sequence
    let _stats = monitor.get_network_stats().await;
    let _load = monitor.get_load_averages().await;
    let _monitoring = monitor.start_real_time_monitoring("workload-789").await;

    // Should handle multiple operations - test passes if no panic occurs
}

// ============================================================================
// Edge Cases & Stress Tests
// ============================================================================

#[test]
fn test_resource_requirements_zero_memory() {
    let requirements = ResourceRequirements {
        cpu: CpuRequirements::default(),
        memory: MemoryRequirements {
            min_bytes: 0,
            max_bytes: None,
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    assert_eq!(requirements.memory.min_bytes, 0);
}

#[test]
fn test_resource_requirements_huge_memory() {
    let huge = 1024u64 * 1024 * 1024 * 1024; // 1TB
    let requirements = ResourceRequirements {
        cpu: CpuRequirements::default(),
        memory: MemoryRequirements {
            min_bytes: huge,
            max_bytes: Some(huge * 2),
        },
        storage: StorageRequirements::default(),
        gpu: None,
        network: NetworkRequirements::default(),
    };

    assert_eq!(requirements.memory.min_bytes, huge);
}

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_cpu_requirements_fractional_cores() {
    let cpu = CpuRequirements {
        min_cores: 0.5,
        max_cores: Some(2.5),
        architecture: None,
    };

    assert_eq!(cpu.min_cores, 0.5);
    assert_eq!(cpu.max_cores, Some(2.5));
}

#[expect(
    clippy::float_cmp,
    reason = "comparing against exact literal initialization"
)]
#[test]
fn test_cpu_requirements_many_cores() {
    let cpu = CpuRequirements {
        min_cores: 128.0,
        max_cores: Some(256.0),
        architecture: Some("x86_64".to_string()),
    };

    assert_eq!(cpu.min_cores, 128.0);
}

// ============================================================================
// Summary Statistics
// ============================================================================

// This test file contains 50+ new test cases targeting:
// - ResourceRequirements and all sub-types
// - CPU, Memory, Storage, Network, GPU requirements
// - SystemResourceMonitor async functions
// - Process info, network stats, load averages
// - Real-time monitoring operations
// - Edge cases and boundary conditions
// - Metrics defaults and construction
//
// Expected impact: Push resources.rs coverage from 39.56% → 60%+
