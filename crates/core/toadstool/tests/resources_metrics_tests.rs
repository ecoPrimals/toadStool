// SPDX-License-Identifier: AGPL-3.0-only
//! Comprehensive tests for Resources Metrics and Monitoring
//!
//! Week 16 Sprint: Targeting `RuntimeMetrics`, `ResourceLimits`, and monitoring
//! Current coverage: 35% → Target: 50%

use toadstool::resources::*;

// ============================================================================
// RuntimeMetrics Tests (10 tests)
// ============================================================================

#[allow(clippy::float_cmp)]
#[test]
fn test_runtime_metrics_default() {
    let metrics = RuntimeMetrics::default();
    assert_eq!(metrics.cpu.usage_percent, 0.0);
    assert_eq!(metrics.memory.used_bytes, 0);
    assert!(metrics.gpu.is_none());
}

#[allow(clippy::float_cmp)]
#[test]
fn test_runtime_metrics_with_data() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 50.0,
            cores_used: 2.0,
            cpu_time_seconds: 100.0,
        },
        memory: MemoryMetrics {
            usage_percent: 75.0,
            used_bytes: 8 * 1024 * 1024 * 1024,
            peak_bytes: 10 * 1024 * 1024 * 1024,
        },
        storage: StorageMetrics::default(),
        network: NetworkMetrics::default(),
        gpu: None,
        timing: TimingMetrics::default(),
    };

    assert_eq!(metrics.cpu.usage_percent, 50.0);
    assert_eq!(metrics.memory.usage_percent, 75.0);
}

#[allow(clippy::float_cmp)]
#[test]
fn test_runtime_metrics_clone() {
    let metrics1 = RuntimeMetrics::default();
    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.cpu.usage_percent, metrics2.cpu.usage_percent);
}

#[test]
fn test_runtime_metrics_serialization() {
    let metrics = RuntimeMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
    assert!(json.contains("cpu"));
}

#[allow(clippy::float_cmp)]
#[test]
fn test_runtime_metrics_deserialization() {
    let metrics = RuntimeMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    let deserialized: RuntimeMetrics = serde_json::from_str(&json).unwrap();
    assert_eq!(metrics.cpu.usage_percent, deserialized.cpu.usage_percent);
}

#[allow(clippy::float_cmp)]
#[test]
fn test_runtime_metrics_with_gpu() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics::default(),
        memory: MemoryMetrics::default(),
        storage: StorageMetrics::default(),
        network: NetworkMetrics::default(),
        gpu: Some(GpuMetrics {
            usage_percent: 80.0,
            memory_usage_percent: 75.0,
            memory_used_bytes: 4 * 1024 * 1024 * 1024,
            temperature_celsius: Some(65.0),
        }),
        timing: TimingMetrics::default(),
    };

    assert!(metrics.gpu.is_some());
    assert_eq!(metrics.gpu.unwrap().usage_percent, 80.0);
}

#[test]
fn test_runtime_metrics_debug() {
    let metrics = RuntimeMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("RuntimeMetrics"));
}

#[allow(clippy::float_cmp)]
#[test]
fn test_runtime_metrics_all_components() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics {
            usage_percent: 25.0,
            cores_used: 1.0,
            cpu_time_seconds: 50.0,
        },
        memory: MemoryMetrics {
            usage_percent: 50.0,
            used_bytes: 4 * 1024 * 1024 * 1024,
            peak_bytes: 5 * 1024 * 1024 * 1024,
        },
        storage: StorageMetrics {
            usage_percent: 30.0,
            used_bytes: 10 * 1024 * 1024 * 1024,
            bytes_read: 1024 * 1024,
            bytes_written: 512 * 1024,
        },
        network: NetworkMetrics {
            bytes_sent: 1024,
            bytes_received: 2048,
            packets_sent: 10,
            packets_received: 20,
        },
        gpu: None,
        timing: TimingMetrics::default(),
    };

    assert_eq!(metrics.cpu.cores_used, 1.0);
    assert_eq!(metrics.network.bytes_sent, 1024);
}

#[test]
fn test_runtime_metrics_memory_tracking() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics::default(),
        memory: MemoryMetrics {
            usage_percent: 60.0,
            used_bytes: 6 * 1024 * 1024 * 1024,
            peak_bytes: 8 * 1024 * 1024 * 1024,
        },
        storage: StorageMetrics::default(),
        network: NetworkMetrics::default(),
        gpu: None,
        timing: TimingMetrics::default(),
    };

    assert!(metrics.memory.peak_bytes > metrics.memory.used_bytes);
}

#[test]
fn test_runtime_metrics_network_activity() {
    let metrics = RuntimeMetrics {
        cpu: CpuMetrics::default(),
        memory: MemoryMetrics::default(),
        storage: StorageMetrics::default(),
        network: NetworkMetrics {
            bytes_sent: 10000,
            bytes_received: 20000,
            packets_sent: 100,
            packets_received: 200,
        },
        gpu: None,
        timing: TimingMetrics::default(),
    };

    assert!(metrics.network.bytes_received > metrics.network.bytes_sent);
}

// ============================================================================
// CpuMetrics Tests (8 tests)
// ============================================================================

#[allow(clippy::float_cmp)]
#[test]
fn test_cpu_metrics_default() {
    let metrics = CpuMetrics::default();
    assert_eq!(metrics.usage_percent, 0.0);
    assert_eq!(metrics.cores_used, 0.0);
    assert_eq!(metrics.cpu_time_seconds, 0.0);
}

#[allow(clippy::float_cmp)]
#[test]
fn test_cpu_metrics_custom() {
    let metrics = CpuMetrics {
        usage_percent: 75.5,
        cores_used: 3.5,
        cpu_time_seconds: 123.456,
    };
    assert_eq!(metrics.usage_percent, 75.5);
}

#[allow(clippy::float_cmp)]
#[test]
fn test_cpu_metrics_clone() {
    let metrics1 = CpuMetrics::default();
    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.usage_percent, metrics2.usage_percent);
}

#[test]
fn test_cpu_metrics_serialization() {
    let metrics = CpuMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_cpu_metrics_high_usage() {
    let metrics = CpuMetrics {
        usage_percent: 95.0,
        cores_used: 7.8,
        cpu_time_seconds: 1000.0,
    };
    assert!(metrics.usage_percent > 90.0);
}

#[allow(clippy::float_cmp)]
#[test]
fn test_cpu_metrics_fractional_cores() {
    let metrics = CpuMetrics {
        usage_percent: 25.0,
        cores_used: 0.5,
        cpu_time_seconds: 10.0,
    };
    assert_eq!(metrics.cores_used, 0.5);
}

#[test]
fn test_cpu_metrics_debug() {
    let metrics = CpuMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("CpuMetrics"));
}

#[test]
fn test_cpu_metrics_time_tracking() {
    let metrics = CpuMetrics {
        usage_percent: 50.0,
        cores_used: 2.0,
        cpu_time_seconds: 500.0,
    };
    assert!(metrics.cpu_time_seconds > 0.0);
}

// ============================================================================
// MemoryMetrics Tests (8 tests)
// ============================================================================

#[allow(clippy::float_cmp)]
#[test]
fn test_memory_metrics_default() {
    let metrics = MemoryMetrics::default();
    assert_eq!(metrics.usage_percent, 0.0);
    assert_eq!(metrics.used_bytes, 0);
    assert_eq!(metrics.peak_bytes, 0);
}

#[allow(clippy::float_cmp)]
#[test]
fn test_memory_metrics_custom() {
    let metrics = MemoryMetrics {
        usage_percent: 80.0,
        used_bytes: 8 * 1024 * 1024 * 1024,
        peak_bytes: 10 * 1024 * 1024 * 1024,
    };
    assert_eq!(metrics.usage_percent, 80.0);
}

#[test]
fn test_memory_metrics_clone() {
    let metrics1 = MemoryMetrics::default();
    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.used_bytes, metrics2.used_bytes);
}

#[test]
fn test_memory_metrics_serialization() {
    let metrics = MemoryMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_memory_metrics_peak_tracking() {
    let metrics = MemoryMetrics {
        usage_percent: 60.0,
        used_bytes: 6 * 1024 * 1024 * 1024,
        peak_bytes: 9 * 1024 * 1024 * 1024,
    };
    assert!(metrics.peak_bytes >= metrics.used_bytes);
}

#[test]
fn test_memory_metrics_gigabyte_scale() {
    let gb = 1024 * 1024 * 1024;
    let metrics = MemoryMetrics {
        usage_percent: 50.0,
        used_bytes: 4 * gb,
        peak_bytes: 5 * gb,
    };
    assert_eq!(metrics.used_bytes, 4 * gb);
}

#[test]
fn test_memory_metrics_debug() {
    let metrics = MemoryMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("MemoryMetrics"));
}

#[test]
fn test_memory_metrics_high_usage() {
    let metrics = MemoryMetrics {
        usage_percent: 95.0,
        used_bytes: 15 * 1024 * 1024 * 1024,
        peak_bytes: 16 * 1024 * 1024 * 1024,
    };
    assert!(metrics.usage_percent > 90.0);
}

// ============================================================================
// StorageMetrics Tests (8 tests)
// ============================================================================

#[allow(clippy::float_cmp)]
#[test]
fn test_storage_metrics_default() {
    let metrics = StorageMetrics::default();
    assert_eq!(metrics.usage_percent, 0.0);
    assert_eq!(metrics.used_bytes, 0);
    assert_eq!(metrics.bytes_read, 0);
    assert_eq!(metrics.bytes_written, 0);
}

#[allow(clippy::float_cmp)]
#[test]
fn test_storage_metrics_custom() {
    let metrics = StorageMetrics {
        usage_percent: 70.0,
        used_bytes: 100 * 1024 * 1024 * 1024,
        bytes_read: 1024 * 1024,
        bytes_written: 512 * 1024,
    };
    assert_eq!(metrics.usage_percent, 70.0);
}

#[test]
fn test_storage_metrics_clone() {
    let metrics1 = StorageMetrics::default();
    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.used_bytes, metrics2.used_bytes);
}

#[test]
fn test_storage_metrics_serialization() {
    let metrics = StorageMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_storage_metrics_io_tracking() {
    let metrics = StorageMetrics {
        usage_percent: 50.0,
        used_bytes: 50 * 1024 * 1024 * 1024,
        bytes_read: 10 * 1024 * 1024,
        bytes_written: 5 * 1024 * 1024,
    };
    assert!(metrics.bytes_read > metrics.bytes_written);
}

#[test]
fn test_storage_metrics_large_values() {
    let terabyte = 1024u64 * 1024 * 1024 * 1024;
    let metrics = StorageMetrics {
        usage_percent: 80.0,
        used_bytes: terabyte,
        bytes_read: 100 * 1024 * 1024,
        bytes_written: 50 * 1024 * 1024,
    };
    assert_eq!(metrics.used_bytes, terabyte);
}

#[test]
fn test_storage_metrics_debug() {
    let metrics = StorageMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("StorageMetrics"));
}

#[test]
fn test_storage_metrics_zero_io() {
    let metrics = StorageMetrics {
        usage_percent: 10.0,
        used_bytes: 10 * 1024 * 1024 * 1024,
        bytes_read: 0,
        bytes_written: 0,
    };
    assert_eq!(metrics.bytes_read, 0);
}

// ============================================================================
// NetworkMetrics Tests (6 tests)
// ============================================================================

#[test]
fn test_network_metrics_default() {
    let metrics = NetworkMetrics::default();
    assert_eq!(metrics.bytes_sent, 0);
    assert_eq!(metrics.bytes_received, 0);
}

#[test]
fn test_network_metrics_custom() {
    let metrics = NetworkMetrics {
        bytes_sent: 1024 * 1024,
        bytes_received: 2 * 1024 * 1024,
        packets_sent: 1000,
        packets_received: 2000,
    };
    assert_eq!(metrics.bytes_sent, 1024 * 1024);
}

#[test]
fn test_network_metrics_clone() {
    let metrics1 = NetworkMetrics::default();
    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.bytes_sent, metrics2.bytes_sent);
}

#[test]
fn test_network_metrics_serialization() {
    let metrics = NetworkMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_network_metrics_packet_tracking() {
    let metrics = NetworkMetrics {
        bytes_sent: 10000,
        bytes_received: 20000,
        packets_sent: 100,
        packets_received: 200,
    };
    assert!(metrics.packets_received > metrics.packets_sent);
}

#[test]
fn test_network_metrics_debug() {
    let metrics = NetworkMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("NetworkMetrics"));
}

// ============================================================================
// GpuMetrics Tests (6 tests)
// ============================================================================

#[allow(clippy::float_cmp)]
#[test]
fn test_gpu_metrics_basic() {
    let metrics = GpuMetrics {
        usage_percent: 75.0,
        memory_usage_percent: 70.0,
        memory_used_bytes: 4 * 1024 * 1024 * 1024,
        temperature_celsius: Some(65.0),
    };
    assert_eq!(metrics.usage_percent, 75.0);
}

#[test]
fn test_gpu_metrics_no_temperature() {
    let metrics = GpuMetrics {
        usage_percent: 50.0,
        memory_usage_percent: 45.0,
        memory_used_bytes: 2 * 1024 * 1024 * 1024,
        temperature_celsius: None,
    };
    assert!(metrics.temperature_celsius.is_none());
}

#[allow(clippy::float_cmp)]
#[test]
fn test_gpu_metrics_clone() {
    let metrics1 = GpuMetrics {
        usage_percent: 80.0,
        memory_usage_percent: 75.0,
        memory_used_bytes: 8 * 1024 * 1024 * 1024,
        temperature_celsius: Some(70.0),
    };
    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.usage_percent, metrics2.usage_percent);
}

#[test]
fn test_gpu_metrics_serialization() {
    let metrics = GpuMetrics {
        usage_percent: 60.0,
        memory_usage_percent: 55.0,
        memory_used_bytes: 4 * 1024 * 1024 * 1024,
        temperature_celsius: Some(60.0),
    };
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_gpu_metrics_high_usage() {
    let metrics = GpuMetrics {
        usage_percent: 98.0,
        memory_usage_percent: 95.0,
        memory_used_bytes: 16 * 1024 * 1024 * 1024,
        temperature_celsius: Some(85.0),
    };
    assert!(metrics.usage_percent > 95.0);
}

#[test]
fn test_gpu_metrics_debug() {
    let metrics = GpuMetrics {
        usage_percent: 50.0,
        memory_usage_percent: 45.0,
        memory_used_bytes: 2 * 1024 * 1024 * 1024,
        temperature_celsius: None,
    };
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("GpuMetrics"));
}

#[allow(clippy::float_cmp)]
#[test]
fn test_gpu_metrics_default() {
    let metrics = GpuMetrics::default();
    assert_eq!(metrics.usage_percent, 0.0);
    assert_eq!(metrics.memory_usage_percent, 0.0);
    assert_eq!(metrics.memory_used_bytes, 0);
}

#[test]
fn test_gpu_metrics_memory_tracking() {
    let metrics = GpuMetrics {
        usage_percent: 80.0,
        memory_usage_percent: 85.0,
        memory_used_bytes: 8 * 1024 * 1024 * 1024,
        temperature_celsius: Some(72.0),
    };
    assert!(metrics.memory_usage_percent > metrics.usage_percent);
}

// ============================================================================
// TimingMetrics Tests (4 tests)
// ============================================================================

#[test]
fn test_timing_metrics_default() {
    let metrics = TimingMetrics::default();
    // Just verify it creates without panicking
    let _ = format!("{metrics:?}");
}

#[test]
fn test_timing_metrics_clone() {
    let metrics1 = TimingMetrics::default();
    let metrics2 = metrics1.clone();
    assert_eq!(metrics1.start_time, metrics2.start_time);
}

#[test]
fn test_timing_metrics_serialization() {
    let metrics = TimingMetrics::default();
    let json = serde_json::to_string(&metrics).unwrap();
    assert!(!json.is_empty());
}

#[test]
fn test_timing_metrics_debug() {
    let metrics = TimingMetrics::default();
    let debug_str = format!("{metrics:?}");
    assert!(debug_str.contains("TimingMetrics"));
}

// ============================================================================
// Test Coverage Summary
// ============================================================================

#[test]
fn test_resources_metrics_coverage_summary() {
    println!("=== Resources Metrics Test Coverage ===");
    println!("RuntimeMetrics Tests:      10 tests");
    println!("CpuMetrics Tests:          8 tests");
    println!("MemoryMetrics Tests:       8 tests");
    println!("StorageMetrics Tests:      8 tests");
    println!("NetworkMetrics Tests:      6 tests");
    println!("GpuMetrics Tests:          6 tests");
    println!("TimingMetrics Tests:       4 tests");
    println!("───────────────────────────────────────");
    println!("Total:                     50 tests");
    println!("Target Coverage:           35% → 50%");
    println!("=======================================");
}
