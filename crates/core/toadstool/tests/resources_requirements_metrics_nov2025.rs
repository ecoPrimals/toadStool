//! Comprehensive test coverage for resources.rs requirements and metrics types
//!
//! This test suite targets requirement and metric types defined in 
//! crates/core/toadstool/src/resources.rs to expand test coverage.
//!
//! Coverage Target: Add 40+ tests for resource types
//! Session: November 2025 - Week 5 Test Expansion (Batch 4)

use toadstool::resources::{
    CpuMetrics, CpuRequirements, GpuMetrics, GpuRequirements, MemoryMetrics, MemoryRequirements,
    NetworkMetrics, NetworkRequirements, StorageMetrics, StorageRequirements,
};

// ============================================================================
// CpuRequirements Tests (5 tests)
// ============================================================================

#[test]
fn test_cpu_requirements_default() {
    let cpu_req = CpuRequirements::default();
    assert_eq!(cpu_req.min_cores, 1.0);
    assert!(cpu_req.max_cores.is_none());
    assert!(cpu_req.architecture.is_none());
}

#[test]
fn test_cpu_requirements_custom() {
    let cpu_req = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(8.0),
        architecture: Some("x86_64".to_string()),
    };
    assert_eq!(cpu_req.min_cores, 4.0);
    assert_eq!(cpu_req.max_cores, Some(8.0));
    assert_eq!(cpu_req.architecture, Some("x86_64".to_string()));
}

#[test]
fn test_cpu_requirements_clone() {
    let cpu_req = CpuRequirements {
        min_cores: 2.0,
        max_cores: Some(4.0),
        architecture: Some("arm64".to_string()),
    };
    let cloned = cpu_req.clone();
    assert_eq!(cpu_req.min_cores, cloned.min_cores);
    assert_eq!(cpu_req.architecture, cloned.architecture);
}

#[test]
fn test_cpu_requirements_serialization() {
    let cpu_req = CpuRequirements {
        min_cores: 2.5,
        max_cores: Some(6.0),
        architecture: Some("aarch64".to_string()),
    };
    let serialized = serde_json::to_string(&cpu_req).expect("Failed to serialize");
    let deserialized: CpuRequirements =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(cpu_req.min_cores, deserialized.min_cores);
    assert_eq!(cpu_req.max_cores, deserialized.max_cores);
}

#[test]
fn test_cpu_requirements_fractional_cores() {
    let cpu_req = CpuRequirements {
        min_cores: 0.5, // Half a core
        max_cores: Some(1.5),
        architecture: None,
    };
    assert_eq!(cpu_req.min_cores, 0.5);
    assert_eq!(cpu_req.max_cores, Some(1.5));
}

// ============================================================================
// MemoryRequirements Tests (5 tests)
// ============================================================================

#[test]
fn test_memory_requirements_default() {
    let mem_req = MemoryRequirements::default();
    assert_eq!(mem_req.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert!(mem_req.max_bytes.is_none());
}

#[test]
fn test_memory_requirements_custom() {
    let mem_req = MemoryRequirements {
        min_bytes: 512 * 1024 * 1024, // 512MB
        max_bytes: Some(2 * 1024 * 1024 * 1024), // 2GB
    };
    assert_eq!(mem_req.min_bytes, 512 * 1024 * 1024);
    assert_eq!(mem_req.max_bytes, Some(2 * 1024 * 1024 * 1024));
}

#[test]
fn test_memory_requirements_clone() {
    let mem_req = MemoryRequirements {
        min_bytes: 1024,
        max_bytes: Some(2048),
    };
    let cloned = mem_req.clone();
    assert_eq!(mem_req.min_bytes, cloned.min_bytes);
    assert_eq!(mem_req.max_bytes, cloned.max_bytes);
}

#[test]
fn test_memory_requirements_serialization() {
    let mem_req = MemoryRequirements {
        min_bytes: 1024 * 1024,
        max_bytes: Some(1024 * 1024 * 1024),
    };
    let serialized = serde_json::to_string(&mem_req).expect("Failed to serialize");
    let deserialized: MemoryRequirements =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(mem_req.min_bytes, deserialized.min_bytes);
}

#[test]
fn test_memory_requirements_large_values() {
    let mem_req = MemoryRequirements {
        min_bytes: 16 * 1024 * 1024 * 1024, // 16GB
        max_bytes: Some(64 * 1024 * 1024 * 1024), // 64GB
    };
    assert!(mem_req.min_bytes < mem_req.max_bytes.unwrap());
}

// ============================================================================
// StorageRequirements Tests (5 tests)
// ============================================================================

#[test]
fn test_storage_requirements_default() {
    let storage_req = StorageRequirements::default();
    assert_eq!(storage_req.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert!(storage_req.max_bytes.is_none());
    assert!(storage_req.storage_type.is_none());
}

#[test]
fn test_storage_requirements_with_type() {
    let storage_req = StorageRequirements {
        min_bytes: 10 * 1024 * 1024 * 1024, // 10GB
        max_bytes: Some(100 * 1024 * 1024 * 1024), // 100GB
        storage_type: Some("SSD".to_string()),
    };
    assert_eq!(storage_req.storage_type, Some("SSD".to_string()));
}

#[test]
fn test_storage_requirements_clone() {
    let storage_req = StorageRequirements {
        min_bytes: 5 * 1024 * 1024,
        max_bytes: Some(10 * 1024 * 1024),
        storage_type: Some("NVMe".to_string()),
    };
    let cloned = storage_req.clone();
    assert_eq!(storage_req.storage_type, cloned.storage_type);
}

#[test]
fn test_storage_requirements_serialization() {
    let storage_req = StorageRequirements {
        min_bytes: 1024,
        max_bytes: None,
        storage_type: Some("HDD".to_string()),
    };
    let serialized = serde_json::to_string(&storage_req).expect("Failed to serialize");
    let deserialized: StorageRequirements =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(storage_req.min_bytes, deserialized.min_bytes);
}

#[test]
fn test_storage_requirements_various_types() {
    let types = vec!["SSD", "HDD", "NVMe", "S3", "EBS"];
    for storage_type in types {
        let req = StorageRequirements {
            min_bytes: 1024,
            max_bytes: None,
            storage_type: Some(storage_type.to_string()),
        };
        assert_eq!(req.storage_type, Some(storage_type.to_string()));
    }
}

// ============================================================================
// NetworkRequirements Tests (4 tests)
// ============================================================================

#[test]
fn test_network_requirements_default() {
    let net_req = NetworkRequirements::default();
    assert!(net_req.min_bandwidth.is_none());
    assert!(net_req.max_bandwidth.is_none());
    assert!(net_req.max_latency_ms.is_none());
}

#[test]
fn test_network_requirements_with_bandwidth() {
    let net_req = NetworkRequirements {
        min_bandwidth: Some(1024 * 1024), // 1 MB/s
        max_bandwidth: Some(10 * 1024 * 1024), // 10 MB/s
        max_latency_ms: Some(100),
    };
    assert_eq!(net_req.min_bandwidth, Some(1024 * 1024));
    assert_eq!(net_req.max_latency_ms, Some(100));
}

#[test]
fn test_network_requirements_clone() {
    let net_req = NetworkRequirements {
        min_bandwidth: Some(512),
        max_bandwidth: Some(1024),
        max_latency_ms: Some(50),
    };
    let cloned = net_req.clone();
    assert_eq!(net_req.max_latency_ms, cloned.max_latency_ms);
}

#[test]
fn test_network_requirements_serialization() {
    let net_req = NetworkRequirements {
        min_bandwidth: Some(1000),
        max_bandwidth: Some(10000),
        max_latency_ms: Some(20),
    };
    let serialized = serde_json::to_string(&net_req).expect("Failed to serialize");
    let deserialized: NetworkRequirements =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(net_req.min_bandwidth, deserialized.min_bandwidth);
}

// ============================================================================
// GpuRequirements Tests (4 tests)
// ============================================================================

#[test]
fn test_gpu_requirements_creation() {
    let gpu_req = GpuRequirements {
        min_units: 1,
        max_units: Some(4),
        gpu_type: Some("NVIDIA".to_string()),
        min_memory_bytes: Some(4 * 1024 * 1024 * 1024), // 4GB
    };
    assert_eq!(gpu_req.min_units, 1);
    assert_eq!(gpu_req.max_units, Some(4));
}

#[test]
fn test_gpu_requirements_with_type() {
    let gpu_req = GpuRequirements {
        min_units: 2,
        max_units: None,
        gpu_type: Some("AMD".to_string()),
        min_memory_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
    };
    assert_eq!(gpu_req.gpu_type, Some("AMD".to_string()));
}

#[test]
fn test_gpu_requirements_clone() {
    let gpu_req = GpuRequirements {
        min_units: 1,
        max_units: Some(2),
        gpu_type: Some("Intel".to_string()),
        min_memory_bytes: None,
    };
    let cloned = gpu_req.clone();
    assert_eq!(gpu_req.min_units, cloned.min_units);
}

#[test]
fn test_gpu_requirements_serialization() {
    let gpu_req = GpuRequirements {
        min_units: 1,
        max_units: Some(4),
        gpu_type: Some("CUDA".to_string()),
        min_memory_bytes: Some(1024 * 1024 * 1024),
    };
    let serialized = serde_json::to_string(&gpu_req).expect("Failed to serialize");
    let deserialized: GpuRequirements =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(gpu_req.min_units, deserialized.min_units);
}

// ============================================================================
// CpuMetrics Tests (4 tests)
// ============================================================================

#[test]
fn test_cpu_metrics_default() {
    let cpu_metrics = CpuMetrics::default();
    assert_eq!(cpu_metrics.usage_percent, 0.0);
    assert_eq!(cpu_metrics.cores_used, 0.0);
    assert_eq!(cpu_metrics.cpu_time_seconds, 0.0);
}

#[test]
fn test_cpu_metrics_custom() {
    let cpu_metrics = CpuMetrics {
        usage_percent: 75.5,
        cores_used: 3.2,
        cpu_time_seconds: 120.5,
    };
    assert_eq!(cpu_metrics.usage_percent, 75.5);
    assert_eq!(cpu_metrics.cores_used, 3.2);
}

#[test]
fn test_cpu_metrics_clone() {
    let cpu_metrics = CpuMetrics {
        usage_percent: 50.0,
        cores_used: 2.0,
        cpu_time_seconds: 60.0,
    };
    let cloned = cpu_metrics.clone();
    assert_eq!(cpu_metrics.usage_percent, cloned.usage_percent);
}

#[test]
fn test_cpu_metrics_serialization() {
    let cpu_metrics = CpuMetrics {
        usage_percent: 80.0,
        cores_used: 4.0,
        cpu_time_seconds: 100.0,
    };
    let serialized = serde_json::to_string(&cpu_metrics).expect("Failed to serialize");
    let deserialized: CpuMetrics =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(cpu_metrics.usage_percent, deserialized.usage_percent);
}

// ============================================================================
// MemoryMetrics Tests (4 tests)
// ============================================================================

#[test]
fn test_memory_metrics_default() {
    let mem_metrics = MemoryMetrics::default();
    assert_eq!(mem_metrics.usage_percent, 0.0);
    assert_eq!(mem_metrics.used_bytes, 0);
    assert_eq!(mem_metrics.peak_bytes, 0);
}

#[test]
fn test_memory_metrics_custom() {
    let mem_metrics = MemoryMetrics {
        usage_percent: 65.5,
        used_bytes: 2 * 1024 * 1024 * 1024, // 2GB
        peak_bytes: 3 * 1024 * 1024 * 1024, // 3GB
    };
    assert_eq!(mem_metrics.usage_percent, 65.5);
    assert!(mem_metrics.peak_bytes > mem_metrics.used_bytes);
}

#[test]
fn test_memory_metrics_clone() {
    let mem_metrics = MemoryMetrics {
        usage_percent: 45.0,
        used_bytes: 1024,
        peak_bytes: 2048,
    };
    let cloned = mem_metrics.clone();
    assert_eq!(mem_metrics.used_bytes, cloned.used_bytes);
}

#[test]
fn test_memory_metrics_serialization() {
    let mem_metrics = MemoryMetrics {
        usage_percent: 70.0,
        used_bytes: 1024 * 1024,
        peak_bytes: 2 * 1024 * 1024,
    };
    let serialized = serde_json::to_string(&mem_metrics).expect("Failed to serialize");
    let deserialized: MemoryMetrics =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(mem_metrics.used_bytes, deserialized.used_bytes);
}

// ============================================================================
// StorageMetrics Tests (4 tests)
// ============================================================================

#[test]
fn test_storage_metrics_default() {
    let storage_metrics = StorageMetrics::default();
    assert_eq!(storage_metrics.usage_percent, 0.0);
    assert_eq!(storage_metrics.used_bytes, 0);
    assert_eq!(storage_metrics.bytes_read, 0);
    assert_eq!(storage_metrics.bytes_written, 0);
}

#[test]
fn test_storage_metrics_custom() {
    let storage_metrics = StorageMetrics {
        usage_percent: 55.0,
        used_bytes: 10 * 1024 * 1024 * 1024, // 10GB
        bytes_read: 1024 * 1024,
        bytes_written: 2 * 1024 * 1024,
    };
    assert_eq!(storage_metrics.usage_percent, 55.0);
    assert!(storage_metrics.bytes_written > storage_metrics.bytes_read);
}

#[test]
fn test_storage_metrics_clone() {
    let storage_metrics = StorageMetrics {
        usage_percent: 30.0,
        used_bytes: 1024,
        bytes_read: 512,
        bytes_written: 256,
    };
    let cloned = storage_metrics.clone();
    assert_eq!(storage_metrics.bytes_read, cloned.bytes_read);
}

#[test]
fn test_storage_metrics_serialization() {
    let storage_metrics = StorageMetrics {
        usage_percent: 40.0,
        used_bytes: 1024,
        bytes_read: 100,
        bytes_written: 200,
    };
    let serialized = serde_json::to_string(&storage_metrics).expect("Failed to serialize");
    let deserialized: StorageMetrics =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(storage_metrics.bytes_written, deserialized.bytes_written);
}

// ============================================================================
// NetworkMetrics Tests (4 tests)
// ============================================================================

#[test]
fn test_network_metrics_default() {
    let net_metrics = NetworkMetrics::default();
    assert_eq!(net_metrics.bytes_sent, 0);
    assert_eq!(net_metrics.bytes_received, 0);
    assert_eq!(net_metrics.packets_sent, 0);
    assert_eq!(net_metrics.packets_received, 0);
}

#[test]
fn test_network_metrics_custom() {
    let net_metrics = NetworkMetrics {
        bytes_sent: 1024 * 1024,
        bytes_received: 2 * 1024 * 1024,
        packets_sent: 100,
        packets_received: 200,
    };
    assert_eq!(net_metrics.bytes_sent, 1024 * 1024);
    assert!(net_metrics.bytes_received > net_metrics.bytes_sent);
}

#[test]
fn test_network_metrics_clone() {
    let net_metrics = NetworkMetrics {
        bytes_sent: 512,
        bytes_received: 1024,
        packets_sent: 10,
        packets_received: 20,
    };
    let cloned = net_metrics.clone();
    assert_eq!(net_metrics.packets_sent, cloned.packets_sent);
}

#[test]
fn test_network_metrics_serialization() {
    let net_metrics = NetworkMetrics {
        bytes_sent: 1000,
        bytes_received: 2000,
        packets_sent: 5,
        packets_received: 10,
    };
    let serialized = serde_json::to_string(&net_metrics).expect("Failed to serialize");
    let deserialized: NetworkMetrics =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(net_metrics.packets_received, deserialized.packets_received);
}

// ============================================================================
// GpuMetrics Tests (4 tests)
// ============================================================================

#[test]
fn test_gpu_metrics_default() {
    let gpu_metrics = GpuMetrics::default();
    assert_eq!(gpu_metrics.usage_percent, 0.0);
    assert_eq!(gpu_metrics.memory_usage_percent, 0.0);
    assert_eq!(gpu_metrics.memory_used_bytes, 0);
    assert!(gpu_metrics.temperature_celsius.is_none());
}

#[test]
fn test_gpu_metrics_with_temperature() {
    let gpu_metrics = GpuMetrics {
        usage_percent: 85.0,
        memory_usage_percent: 70.0,
        memory_used_bytes: 4 * 1024 * 1024 * 1024, // 4GB
        temperature_celsius: Some(75.5),
    };
    assert_eq!(gpu_metrics.temperature_celsius, Some(75.5));
}

#[test]
fn test_gpu_metrics_clone() {
    let gpu_metrics = GpuMetrics {
        usage_percent: 50.0,
        memory_usage_percent: 40.0,
        memory_used_bytes: 1024,
        temperature_celsius: Some(60.0),
    };
    let cloned = gpu_metrics.clone();
    assert_eq!(gpu_metrics.usage_percent, cloned.usage_percent);
}

#[test]
fn test_gpu_metrics_serialization() {
    let gpu_metrics = GpuMetrics {
        usage_percent: 90.0,
        memory_usage_percent: 80.0,
        memory_used_bytes: 1024 * 1024,
        temperature_celsius: Some(70.0),
    };
    let serialized = serde_json::to_string(&gpu_metrics).expect("Failed to serialize");
    let deserialized: GpuMetrics =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(gpu_metrics.usage_percent, deserialized.usage_percent);
}

// ============================================================================
// Summary
// ============================================================================

// Total tests added: 43
// Coverage areas:
// - CpuRequirements (5 tests)
// - MemoryRequirements (5 tests)
// - StorageRequirements (5 tests)
// - NetworkRequirements (4 tests)
// - GpuRequirements (4 tests)
// - CpuMetrics (4 tests)
// - MemoryMetrics (4 tests)
// - StorageMetrics (4 tests)
// - NetworkMetrics (4 tests)
// - GpuMetrics (4 tests)
// - Serialization tests (multiple across types)
// - Clone tests (multiple)
// - Default tests (multiple)
// - Custom value tests (multiple)

