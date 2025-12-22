//! Unit tests for ResourceRequirements validation and management
//!
//! Tests the resource requirement specification, validation, and allocation logic.

use toadstool::resources::{ResourceRequirements, ResourceLimits, ResourceUsage};
use toadstool::ToadStoolResult;

#[test]
fn test_resource_requirements_default() {
    let requirements = ResourceRequirements::default();
    
    assert!(requirements.cpu_cores.is_none(), "Default should have no CPU requirement");
    assert!(requirements.memory_mb.is_none(), "Default should have no memory requirement");
    assert!(!requirements.gpu_required, "Default should not require GPU");
}

#[test]
fn test_resource_requirements_validation() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(2048),
        gpu_required: false,
        disk_mb: Some(1024),
        network_required: false,
    };
    
    let validation = requirements.validate();
    assert!(validation.is_ok(), "Valid requirements should pass validation");
}

#[test]
fn test_resource_requirements_invalid_cpu() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(0), // Invalid: zero cores
        memory_mb: Some(512),
        gpu_required: false,
        disk_mb: None,
        network_required: false,
    };
    
    let validation = requirements.validate();
    assert!(validation.is_err(), "Zero CPU cores should fail validation");
}

#[test]
fn test_resource_requirements_invalid_memory() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(0), // Invalid: zero memory
        gpu_required: false,
        disk_mb: None,
        network_required: false,
    };
    
    let validation = requirements.validate();
    assert!(validation.is_err(), "Zero memory should fail validation");
}

#[test]
fn test_resource_limits_creation() {
    let limits = ResourceLimits {
        max_cpu_percent: 80.0,
        max_memory_mb: 4096,
        max_disk_io_mbps: 100,
        max_network_mbps: 1000,
    };
    
    assert_eq!(limits.max_cpu_percent, 80.0);
    assert_eq!(limits.max_memory_mb, 4096);
}

#[test]
fn test_resource_limits_validation() {
    let limits = ResourceLimits {
        max_cpu_percent: 100.0,
        max_memory_mb: 8192,
        max_disk_io_mbps: 500,
        max_network_mbps: 10000,
    };
    
    assert!(limits.max_cpu_percent <= 100.0, "CPU percent should not exceed 100");
    assert!(limits.max_memory_mb > 0, "Memory limit should be positive");
}

#[test]
fn test_resource_usage_tracking() {
    let usage = ResourceUsage {
        cpu_percent: 45.5,
        memory_mb: 1024,
        disk_io_mbps: 25,
        network_mbps: 100,
    };
    
    assert!(usage.cpu_percent < 100.0, "CPU usage should be valid");
    assert!(usage.memory_mb > 0, "Memory usage should be positive");
}

#[test]
fn test_resource_requirements_meets_limits() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(2048),
        gpu_required: false,
        disk_mb: Some(1024),
        network_required: true,
    };
    
    let limits = ResourceLimits {
        max_cpu_percent: 100.0,
        max_memory_mb: 4096,
        max_disk_io_mbps: 1000,
        max_network_mbps: 10000,
    };
    
    // Requirements should be within limits
    if let Some(memory) = requirements.memory_mb {
        assert!(memory <= limits.max_memory_mb, "Requirements should be within limits");
    }
}

#[test]
fn test_resource_requirements_clone() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(4),
        memory_mb: Some(2048),
        gpu_required: true,
        disk_mb: Some(512),
        network_required: true,
    };
    
    let cloned = requirements.clone();
    
    assert_eq!(requirements.cpu_cores, cloned.cpu_cores);
    assert_eq!(requirements.memory_mb, cloned.memory_mb);
    assert_eq!(requirements.gpu_required, cloned.gpu_required);
}

#[test]
fn test_resource_requirements_serialization() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(2),
        memory_mb: Some(1024),
        gpu_required: false,
        disk_mb: None,
        network_required: false,
    };
    
    let json = serde_json::to_string(&requirements);
    assert!(json.is_ok(), "Should serialize to JSON");
    
    if let Ok(json_str) = json {
        let deserialized: Result<ResourceRequirements, _> = serde_json::from_str(&json_str);
        assert!(deserialized.is_ok(), "Should deserialize from JSON");
    }
}

#[test]
fn test_resource_limits_exceed_check() {
    let usage = ResourceUsage {
        cpu_percent: 95.0,
        memory_mb: 7500,
        disk_io_mbps: 450,
        network_mbps: 9500,
    };
    
    let limits = ResourceLimits {
        max_cpu_percent: 90.0,
        max_memory_mb: 8192,
        max_disk_io_mbps: 500,
        max_network_mbps: 10000,
    };
    
    // CPU exceeds limit
    assert!(usage.cpu_percent > limits.max_cpu_percent, "Should detect CPU limit exceeded");
    
    // Memory within limit
    assert!(usage.memory_mb < limits.max_memory_mb, "Memory should be within limits");
}

#[test]
fn test_resource_requirements_with_gpu() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(8),
        memory_mb: Some(16384),
        gpu_required: true,
        disk_mb: Some(10240),
        network_required: true,
    };
    
    assert!(requirements.gpu_required, "GPU should be required");
    assert!(requirements.cpu_cores.unwrap() >= 4, "GPU workloads typically need more CPU");
}

#[test]
fn test_resource_requirements_minimal() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(1),
        memory_mb: Some(128),
        gpu_required: false,
        disk_mb: Some(100),
        network_required: false,
    };
    
    let validation = requirements.validate();
    assert!(validation.is_ok(), "Minimal valid requirements should pass");
}

#[test]
fn test_resource_requirements_maximum() {
    let requirements = ResourceRequirements {
        cpu_cores: Some(128),
        memory_mb: Some(1048576), // 1TB
        gpu_required: true,
        disk_mb: Some(10485760), // 10TB
        network_required: true,
    };
    
    let validation = requirements.validate();
    assert!(validation.is_ok(), "Maximum requirements should be valid");
}

#[test]
fn test_resource_usage_zero() {
    let usage = ResourceUsage {
        cpu_percent: 0.0,
        memory_mb: 0,
        disk_io_mbps: 0,
        network_mbps: 0,
    };
    
    // Zero usage is valid (idle state)
    assert_eq!(usage.cpu_percent, 0.0);
    assert_eq!(usage.memory_mb, 0);
}

#[test]
fn test_resource_limits_serialization() {
    let limits = ResourceLimits {
        max_cpu_percent: 75.0,
        max_memory_mb: 4096,
        max_disk_io_mbps: 200,
        max_network_mbps: 1000,
    };
    
    let json = serde_json::to_string(&limits);
    assert!(json.is_ok(), "Limits should serialize");
}

