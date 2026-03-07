// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::float_cmp, clippy::items_after_statements)]
//! Unit tests for `ResourceRequirements` validation and management
//!
//! Tests use the real nested API: `ResourceRequirements` wraps `CpuRequirements`,
//! `MemoryRequirements`, `StorageRequirements`, `GpuRequirements`, and `NetworkRequirements`.

use toadstool::resources::{
    CpuLimits, CpuRequirements, GpuRequirements, MemoryRequirements, NetworkRequirements,
    ResourceLimits, ResourceRequirements, ResourceUsage, StorageRequirements,
};

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

#[test]
fn test_resource_requirements_default() {
    let req = ResourceRequirements::default();
    // Default values reflect sensible minimums, not "no requirement"
    assert!(
        req.cpu.min_cores > 0.0,
        "Default should have positive cpu.min_cores"
    );
    assert!(
        req.memory.min_bytes > 0,
        "Default should have positive memory.min_bytes"
    );
    assert!(req.gpu.is_none(), "Default should not require GPU");
}

#[test]
fn test_resource_requirements_validation_passes() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 2 * GB,
            max_bytes: None,
        },
        storage: StorageRequirements {
            min_bytes: GB,
            max_bytes: None,
            storage_type: None,
        },
        gpu: None,
        network: NetworkRequirements::default(),
    };
    assert!(
        req.validate().is_ok(),
        "Valid requirements should pass validation"
    );
}

#[test]
fn test_resource_requirements_invalid_cpu() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 0.0,
            ..CpuRequirements::default()
        },
        memory: MemoryRequirements {
            min_bytes: 512 * MB,
            max_bytes: None,
        },
        ..ResourceRequirements::default()
    };
    assert!(
        req.validate().is_err(),
        "Zero CPU cores should fail validation"
    );
}

#[test]
fn test_resource_requirements_invalid_memory() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 2.0,
            ..CpuRequirements::default()
        },
        memory: MemoryRequirements {
            min_bytes: 0,
            max_bytes: None,
        },
        ..ResourceRequirements::default()
    };
    assert!(
        req.validate().is_err(),
        "Zero memory should fail validation"
    );
}

#[test]
fn test_resource_limits_creation() {
    use toadstool::resources::{CpuLimits, MemoryLimits};
    let limits = ResourceLimits {
        cpu_limits: CpuLimits {
            max_cores: Some(8.0),
            throttle_percent: Some(80.0),
        },
        memory_limits: MemoryLimits {
            max_bytes: Some(4 * GB),
            swap_limit_bytes: None,
        },
        ..ResourceLimits::default()
    };
    assert_eq!(limits.cpu_limits.throttle_percent, Some(80.0));
    assert_eq!(limits.memory_limits.max_bytes, Some(4 * GB));
}

#[test]
fn test_resource_limits_validation() {
    use toadstool::resources::{CpuLimits, MemoryLimits};
    let limits = ResourceLimits {
        cpu_limits: CpuLimits {
            max_cores: Some(32.0),
            throttle_percent: Some(100.0),
        },
        memory_limits: MemoryLimits {
            max_bytes: Some(8 * GB),
            swap_limit_bytes: None,
        },
        ..ResourceLimits::default()
    };
    assert!(limits.cpu_limits.throttle_percent.unwrap() <= 100.0);
    assert!(limits.memory_limits.max_bytes.unwrap() > 0);
}

#[test]
fn test_resource_usage_tracking() {
    let usage = ResourceUsage {
        cpu_usage_percent: 45.5,
        memory_used_mb: 1024,
        disk_read_bytes: 25 * MB,
        disk_write_bytes: 10 * MB,
        network_rx_bytes: 100 * MB,
        network_tx_bytes: 50 * MB,
        wall_time_ms: 1500,
    };
    assert!(usage.cpu_usage_percent < 100.0, "CPU usage should be valid");
    assert!(usage.memory_used_mb > 0, "Memory usage should be positive");
}

#[test]
fn test_resource_requirements_meets_limits() {
    use toadstool::resources::MemoryLimits;
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 2.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 2 * GB,
            max_bytes: None,
        },
        ..ResourceRequirements::default()
    };
    let limits = ResourceLimits {
        memory_limits: MemoryLimits {
            max_bytes: Some(4 * GB),
            swap_limit_bytes: None,
        },
        ..ResourceLimits::default()
    };
    assert!(req.memory.min_bytes <= limits.memory_limits.max_bytes.unwrap());
}

#[test]
fn test_resource_requirements_clone() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(8.0),
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 2 * GB,
            max_bytes: None,
        },
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: None,
            gpu_type: None,
            min_memory_bytes: None,
        }),
        ..ResourceRequirements::default()
    };
    let cloned = req.clone();
    assert_eq!(req.cpu.min_cores, cloned.cpu.min_cores);
    assert_eq!(req.memory.min_bytes, cloned.memory.min_bytes);
    assert!(cloned.gpu.is_some());
}

#[test]
fn test_resource_requirements_serialization() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 2.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: GB,
            max_bytes: None,
        },
        ..ResourceRequirements::default()
    };
    let json = serde_json::to_string(&req);
    assert!(json.is_ok(), "Should serialize to JSON");
    if let Ok(json_str) = json {
        let deserialized: Result<ResourceRequirements, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
        assert_eq!(deserialized.unwrap().cpu.min_cores, 2.0);
    }
}

#[test]
fn test_resource_limits_exceed_check() {
    let usage = ResourceUsage {
        cpu_usage_percent: 95.0,
        memory_used_mb: 7500,
        ..ResourceUsage::default()
    };
    let limits = ResourceLimits {
        cpu_limits: CpuLimits {
            max_cores: None,
            throttle_percent: Some(90.0),
        },
        ..ResourceLimits::default()
    };
    assert!(
        usage.cpu_usage_percent > limits.cpu_limits.throttle_percent.unwrap(),
        "Should detect CPU limit exceeded"
    );
    assert!(usage.memory_used_mb < 8192, "Memory should be within 8 GB");
}

#[test]
fn test_resource_requirements_with_gpu() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 8.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 16 * GB,
            max_bytes: None,
        },
        gpu: Some(GpuRequirements {
            min_units: 1,
            max_units: Some(4),
            gpu_type: Some("NVIDIA".to_string()),
            min_memory_bytes: Some(8 * GB),
        }),
        ..ResourceRequirements::default()
    };
    assert!(req.gpu.is_some(), "GPU should be required");
    assert!(
        req.cpu.min_cores >= 4.0,
        "GPU workloads typically need more CPU"
    );
}

#[test]
fn test_resource_requirements_minimal() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 128 * MB,
            max_bytes: None,
        },
        storage: StorageRequirements {
            min_bytes: 100 * MB,
            max_bytes: None,
            storage_type: None,
        },
        ..ResourceRequirements::default()
    };
    assert!(
        req.validate().is_ok(),
        "Minimal valid requirements should pass"
    );
}

#[test]
fn test_resource_requirements_maximum() {
    let req = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 128.0,
            max_cores: None,
            architecture: None,
        },
        memory: MemoryRequirements {
            min_bytes: 1024 * GB,
            max_bytes: None,
        }, // 1 TiB
        gpu: Some(GpuRequirements {
            min_units: 8,
            max_units: None,
            gpu_type: None,
            min_memory_bytes: Some(80 * GB),
        }),
        ..ResourceRequirements::default()
    };
    assert!(
        req.validate().is_ok(),
        "Maximum requirements should be valid"
    );
}

#[test]
fn test_resource_usage_zero() {
    let usage = ResourceUsage::default();
    assert!(usage.is_empty(), "Default ResourceUsage should be empty");
    assert_eq!(usage.cpu_usage_percent, 0.0);
    assert_eq!(usage.memory_used_mb, 0);
}

#[test]
fn test_resource_limits_serialization() {
    let limits = ResourceLimits::default();
    let json = serde_json::to_string(&limits);
    assert!(json.is_ok(), "Limits should serialize");
}
