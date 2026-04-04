// SPDX-License-Identifier: AGPL-3.0-only
#![expect(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Distributed Types Tests - Week 5
//! Comprehensive tests for distributed system types

use toadstool_distributed::types::{
    CpuRequirements, DistributedExecutionStatus, MemoryRequirements, NetworkRequirements,
    ResourceAllocation, ResourceAllocationStrategy, ResourceRequirements, StorageRequirements,
};

// ============================================================================
// DistributedExecutionStatus Tests
// ============================================================================

#[test]
fn test_execution_status_variants() {
    let statuses = vec![
        DistributedExecutionStatus::Pending,
        DistributedExecutionStatus::Running,
        DistributedExecutionStatus::Completed,
        DistributedExecutionStatus::Failed("error".to_string()),
        DistributedExecutionStatus::Cancelled,
    ];

    assert_eq!(statuses.len(), 5);
}

#[test]
fn test_execution_status_clone() {
    let status = DistributedExecutionStatus::Running;
    let cloned = status;

    assert!(matches!(cloned, DistributedExecutionStatus::Running));
}

#[test]
fn test_execution_status_debug() {
    let status = DistributedExecutionStatus::Pending;
    let debug_str = format!("{status:?}");

    assert!(debug_str.contains("Pending"));
}

#[test]
fn test_execution_status_failed() {
    let status = DistributedExecutionStatus::Failed("test error".to_string());

    if let DistributedExecutionStatus::Failed(msg) = status {
        assert_eq!(msg, "test error");
    } else {
        panic!("Expected Failed variant");
    }
}

// ============================================================================
// ResourceAllocationStrategy Tests
// ============================================================================

#[test]
fn test_allocation_strategy_variants() {
    let strategies = vec![
        ResourceAllocationStrategy::Fair,
        ResourceAllocationStrategy::Proportional,
        ResourceAllocationStrategy::Priority,
        ResourceAllocationStrategy::Custom("custom".to_string()),
    ];

    assert_eq!(strategies.len(), 4);
}

#[test]
fn test_allocation_strategy_clone() {
    let strategy = ResourceAllocationStrategy::Fair;
    let cloned = strategy;

    assert!(matches!(cloned, ResourceAllocationStrategy::Fair));
}

#[test]
fn test_allocation_strategy_debug() {
    let strategy = ResourceAllocationStrategy::Proportional;
    let debug_str = format!("{strategy:?}");

    assert!(debug_str.contains("Proportional"));
}

#[test]
fn test_allocation_strategy_custom() {
    let strategy = ResourceAllocationStrategy::Custom("weighted".to_string());

    if let ResourceAllocationStrategy::Custom(name) = strategy {
        assert_eq!(name, "weighted");
    } else {
        panic!("Expected Custom variant");
    }
}

// ============================================================================
// CpuRequirements Tests
// ============================================================================

#[test]
fn test_cpu_requirements_minimal() {
    let cpu = CpuRequirements {
        min_cores: 1.0,
        max_cores: None,
    };

    assert_eq!(cpu.min_cores, 1.0);
    assert!(cpu.max_cores.is_none());
}

#[test]
fn test_cpu_requirements_with_max() {
    let cpu = CpuRequirements {
        min_cores: 2.0,
        max_cores: Some(8.0),
    };

    assert_eq!(cpu.min_cores, 2.0);
    assert_eq!(cpu.max_cores, Some(8.0));
}

#[test]
fn test_cpu_requirements_clone() {
    let cpu = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(16.0),
    };

    let cloned = cpu.clone();
    assert_eq!(cloned.min_cores, cpu.min_cores);
}

#[test]
fn test_cpu_requirements_fractional() {
    let cpu = CpuRequirements {
        min_cores: 0.5,
        max_cores: Some(2.5),
    };

    assert_eq!(cpu.min_cores, 0.5);
    assert_eq!(cpu.max_cores, Some(2.5));
}

// ============================================================================
// ResourceRequirements Tests
// ============================================================================

#[test]
fn test_resource_requirements_default() {
    let reqs = ResourceRequirements::default();

    assert_eq!(reqs.cpu.min_cores, 1.0);
    assert_eq!(reqs.memory.min_bytes, 1024 * 1024 * 1024);
    assert!(reqs.gpu.is_none());
}

#[test]
fn test_resource_requirements_custom() {
    let reqs = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(16.0),
        },
        memory: MemoryRequirements {
            min_bytes: 8 * 1024 * 1024 * 1024,        // 8GB
            max_bytes: Some(32 * 1024 * 1024 * 1024), // 32GB
        },
        storage: StorageRequirements {
            min_bytes: 100 * 1024 * 1024 * 1024, // 100GB
            max_bytes: None,
        },
        network: NetworkRequirements {
            bandwidth_mbps: Some(1000),
            latency_ms: Some(10),
        },
        gpu: None,
    };

    assert_eq!(reqs.cpu.min_cores, 4.0);
    assert_eq!(reqs.memory.min_bytes, 8 * 1024 * 1024 * 1024);
}

#[test]
fn test_resource_requirements_clone() {
    let reqs = ResourceRequirements::default();
    let cloned = reqs.clone();

    assert_eq!(cloned.cpu.min_cores, reqs.cpu.min_cores);
}

#[test]
fn test_resource_requirements_with_gpu() {
    let reqs = ResourceRequirements {
        gpu: Some(toadstool_distributed::types::GpuRequirements {
            min_memory_gb: 4.0,
            compute_capability: None,
        }),
        ..Default::default()
    };

    assert!(reqs.gpu.is_some());
    assert_eq!(reqs.gpu.unwrap().min_memory_gb, 4.0);
}

// ============================================================================
// ResourceAllocation Tests
// ============================================================================

#[test]
fn test_resource_allocation_creation() {
    let allocation = ResourceAllocation {
        cpu_cores: 4.0,
        memory_bytes: 2 * 1024 * 1024 * 1024,   // 2GB
        storage_bytes: 10 * 1024 * 1024 * 1024, // 10GB
        network_bandwidth: 100,
        gpu_allocation: None,
        custom_resources: std::collections::HashMap::new(),
    };

    assert_eq!(allocation.cpu_cores, 4.0);
    assert_eq!(allocation.memory_bytes, 2 * 1024 * 1024 * 1024);
}

#[test]
fn test_resource_allocation_with_bandwidth() {
    let allocation = ResourceAllocation {
        cpu_cores: 2.0,
        memory_bytes: 1024 * 1024 * 1024,
        storage_bytes: 5 * 1024 * 1024 * 1024,
        network_bandwidth: 1000,
        gpu_allocation: None,
        custom_resources: std::collections::HashMap::new(),
    };

    assert_eq!(allocation.network_bandwidth, 1000);
}

#[test]
fn test_resource_allocation_clone() {
    let allocation = ResourceAllocation {
        cpu_cores: 8.0,
        memory_bytes: 16 * 1024 * 1024 * 1024,
        storage_bytes: 100 * 1024 * 1024 * 1024,
        network_bandwidth: 1000,
        gpu_allocation: None,
        custom_resources: std::collections::HashMap::new(),
    };

    let cloned = allocation.clone();
    assert_eq!(cloned.cpu_cores, allocation.cpu_cores);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_resource_allocation_workflow() {
    // 1. Create resource requirements
    let requirements = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(8.0),
        },
        memory: MemoryRequirements {
            min_bytes: 8 * 1024 * 1024 * 1024,
            max_bytes: None,
        },
        storage: StorageRequirements {
            min_bytes: 20 * 1024 * 1024 * 1024,
            max_bytes: None,
        },
        network: NetworkRequirements {
            bandwidth_mbps: None,
            latency_ms: None,
        },
        gpu: None,
    };

    // 2. Allocate resources
    let allocation = ResourceAllocation {
        cpu_cores: requirements.cpu.min_cores,
        memory_bytes: requirements.memory.min_bytes,
        storage_bytes: requirements.storage.min_bytes,
        network_bandwidth: 500,
        gpu_allocation: None,
        custom_resources: std::collections::HashMap::new(),
    };

    // 3. Verify allocation
    assert_eq!(allocation.cpu_cores, requirements.cpu.min_cores);
    assert_eq!(allocation.memory_bytes, requirements.memory.min_bytes);
}

#[test]
fn test_allocation_strategy_application() {
    let strategies = vec![
        ResourceAllocationStrategy::Fair,
        ResourceAllocationStrategy::Proportional,
        ResourceAllocationStrategy::Priority,
    ];

    // All strategies should be available
    assert_eq!(strategies.len(), 3);
}

#[test]
fn test_execution_status_transitions() {
    // Typical execution flow
    let states = vec![
        DistributedExecutionStatus::Pending,
        DistributedExecutionStatus::Running,
        DistributedExecutionStatus::Completed,
    ];

    assert!(matches!(states[0], DistributedExecutionStatus::Pending));
    assert!(matches!(states[1], DistributedExecutionStatus::Running));
    assert!(matches!(states[2], DistributedExecutionStatus::Completed));
}

#[test]
fn test_resource_scaling() {
    let small = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: Some(2.0),
        },
        memory: MemoryRequirements {
            min_bytes: 1024 * 1024 * 1024,
            max_bytes: Some(2 * 1024 * 1024 * 1024),
        },
        ..Default::default()
    };

    let large = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 16.0,
            max_cores: Some(32.0),
        },
        memory: MemoryRequirements {
            min_bytes: 64 * 1024 * 1024 * 1024,
            max_bytes: Some(128 * 1024 * 1024 * 1024),
        },
        ..Default::default()
    };

    assert!(small.cpu.min_cores < large.cpu.min_cores);
    assert!(small.memory.min_bytes < large.memory.min_bytes);
}

// ============================================================================
// Edge Case Tests
// ============================================================================

#[test]
fn test_fractional_cores() {
    let cpu = CpuRequirements {
        min_cores: 0.5,
        max_cores: Some(1.5),
    };

    assert!(cpu.min_cores < 1.0);
    assert!(cpu.max_cores.unwrap() < 2.0);
}

#[test]
fn test_unbounded_resources() {
    let reqs = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 1.0,
            max_cores: None, // Unbounded
        },
        memory: MemoryRequirements {
            min_bytes: 1024 * 1024 * 1024,
            max_bytes: None, // Unbounded
        },
        ..Default::default()
    };

    assert!(reqs.cpu.max_cores.is_none());
    assert!(reqs.memory.max_bytes.is_none());
}

#[test]
fn test_custom_allocation_strategy() {
    let strategy = ResourceAllocationStrategy::Custom("ml-optimized".to_string());

    if let ResourceAllocationStrategy::Custom(name) = strategy {
        assert!(name.contains("ml"));
    }
}

#[test]
fn test_failed_execution_status() {
    let errors = vec![
        DistributedExecutionStatus::Failed("out of memory".to_string()),
        DistributedExecutionStatus::Failed("timeout".to_string()),
        DistributedExecutionStatus::Failed("node failure".to_string()),
    ];

    assert_eq!(errors.len(), 3);

    for error in errors {
        assert!(matches!(error, DistributedExecutionStatus::Failed(_)));
    }
}
