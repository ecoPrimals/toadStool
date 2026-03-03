// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for resource types and requirements
//!
//! Day 3 of Month 1 test expansion - focusing on resource management

use toadstool_distributed::types::{
    BackoffStrategy, CpuRequirements, DistributedRetryConfig, GpuRequirements, MemoryRequirements,
    NetworkRequirements, ResourceAllocationStrategy, ResourceRequirements, RetryCondition,
    StorageRequirements,
};

// ============================================================================
// ResourceRequirements Tests (5 tests)
// ============================================================================

#[test]
fn test_resource_requirements_default() {
    // Test default resource requirements
    let reqs = ResourceRequirements::default();

    assert_eq!(reqs.cpu.min_cores, 1.0);
    assert_eq!(reqs.memory.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert_eq!(reqs.storage.min_bytes, 1024 * 1024 * 1024); // 1GB
    assert!(reqs.gpu.is_none());
}

#[test]
fn test_resource_requirements_custom() {
    // Test custom resource requirements
    let reqs = ResourceRequirements {
        cpu: CpuRequirements {
            min_cores: 4.0,
            max_cores: Some(8.0),
        },
        memory: MemoryRequirements {
            min_bytes: 4 * 1024 * 1024 * 1024,        // 4GB
            max_bytes: Some(16 * 1024 * 1024 * 1024), // 16GB
        },
        storage: StorageRequirements {
            min_bytes: 10 * 1024 * 1024 * 1024,        // 10GB
            max_bytes: Some(100 * 1024 * 1024 * 1024), // 100GB
        },
        network: NetworkRequirements {
            bandwidth_mbps: Some(1000),
            latency_ms: Some(10),
        },
        gpu: Some(GpuRequirements {
            min_memory_gb: 8.0,
            compute_capability: Some("sm_80".to_string()),
        }),
    };

    assert_eq!(reqs.cpu.min_cores, 4.0);
    assert_eq!(reqs.cpu.max_cores, Some(8.0));
    assert!(reqs.gpu.is_some());
}

#[test]
fn test_resource_requirements_clone() {
    // Test cloning resource requirements
    let reqs = ResourceRequirements::default();
    let cloned = reqs.clone();

    assert_eq!(reqs.cpu.min_cores, cloned.cpu.min_cores);
    assert_eq!(reqs.memory.min_bytes, cloned.memory.min_bytes);
}

#[test]
fn test_resource_requirements_debug() {
    // Test debug formatting
    let reqs = ResourceRequirements::default();
    let debug_str = format!("{:?}", reqs);

    assert!(debug_str.contains("ResourceRequirements"));
}

#[test]
fn test_resource_requirements_with_gpu() {
    // Test resource requirements with GPU
    let reqs = ResourceRequirements {
        gpu: Some(GpuRequirements {
            min_memory_gb: 16.0,
            compute_capability: Some("sm_86".to_string()),
        }),
        ..Default::default()
    };

    assert!(reqs.gpu.is_some());
    assert_eq!(reqs.gpu.as_ref().unwrap().min_memory_gb, 16.0);
}

// ============================================================================
// CpuRequirements Tests (3 tests)
// ============================================================================

#[test]
fn test_cpu_requirements_min_only() {
    // Test CPU requirements with min only
    let cpu = CpuRequirements {
        min_cores: 2.0,
        max_cores: None,
    };

    assert_eq!(cpu.min_cores, 2.0);
    assert!(cpu.max_cores.is_none());
}

#[test]
fn test_cpu_requirements_min_max() {
    // Test CPU requirements with min and max
    let cpu = CpuRequirements {
        min_cores: 4.0,
        max_cores: Some(16.0),
    };

    assert_eq!(cpu.min_cores, 4.0);
    assert_eq!(cpu.max_cores, Some(16.0));
}

#[test]
fn test_cpu_requirements_fractional() {
    // Test fractional CPU cores
    let cpu = CpuRequirements {
        min_cores: 0.5,
        max_cores: Some(2.5),
    };

    assert_eq!(cpu.min_cores, 0.5);
    assert_eq!(cpu.max_cores, Some(2.5));
}

// ============================================================================
// MemoryRequirements Tests (3 tests)
// ============================================================================

#[test]
fn test_memory_requirements_min_only() {
    // Test memory requirements with min only
    let mem = MemoryRequirements {
        min_bytes: 512 * 1024 * 1024, // 512MB
        max_bytes: None,
    };

    assert_eq!(mem.min_bytes, 512 * 1024 * 1024);
    assert!(mem.max_bytes.is_none());
}

#[test]
fn test_memory_requirements_min_max() {
    // Test memory requirements with min and max
    let mem = MemoryRequirements {
        min_bytes: 1024 * 1024 * 1024,           // 1GB
        max_bytes: Some(8 * 1024 * 1024 * 1024), // 8GB
    };

    assert_eq!(mem.min_bytes, 1024 * 1024 * 1024);
    assert_eq!(mem.max_bytes, Some(8 * 1024 * 1024 * 1024));
}

#[test]
fn test_memory_requirements_large() {
    // Test large memory requirements
    let mem = MemoryRequirements {
        min_bytes: 64 * 1024 * 1024 * 1024,        // 64GB
        max_bytes: Some(256 * 1024 * 1024 * 1024), // 256GB
    };

    assert_eq!(mem.min_bytes, 64 * 1024 * 1024 * 1024);
}

// ============================================================================
// StorageRequirements Tests (3 tests)
// ============================================================================

#[test]
fn test_storage_requirements_min_only() {
    // Test storage requirements with min only
    let storage = StorageRequirements {
        min_bytes: 5 * 1024 * 1024 * 1024, // 5GB
        max_bytes: None,
    };

    assert_eq!(storage.min_bytes, 5 * 1024 * 1024 * 1024);
    assert!(storage.max_bytes.is_none());
}

#[test]
fn test_storage_requirements_min_max() {
    // Test storage requirements with min and max
    let storage = StorageRequirements {
        min_bytes: 10 * 1024 * 1024 * 1024,         // 10GB
        max_bytes: Some(1024 * 1024 * 1024 * 1024), // 1TB
    };

    assert_eq!(storage.min_bytes, 10 * 1024 * 1024 * 1024);
    assert_eq!(storage.max_bytes, Some(1024 * 1024 * 1024 * 1024));
}

#[test]
fn test_storage_requirements_small() {
    // Test small storage requirements
    let storage = StorageRequirements {
        min_bytes: 100 * 1024 * 1024,       // 100MB
        max_bytes: Some(500 * 1024 * 1024), // 500MB
    };

    assert_eq!(storage.min_bytes, 100 * 1024 * 1024);
}

// ============================================================================
// NetworkRequirements Tests (3 tests)
// ============================================================================

#[test]
fn test_network_requirements_none() {
    // Test network requirements with no constraints
    let network = NetworkRequirements {
        bandwidth_mbps: None,
        latency_ms: None,
    };

    assert!(network.bandwidth_mbps.is_none());
    assert!(network.latency_ms.is_none());
}

#[test]
fn test_network_requirements_bandwidth_only() {
    // Test network requirements with bandwidth constraint
    let network = NetworkRequirements {
        bandwidth_mbps: Some(100),
        latency_ms: None,
    };

    assert_eq!(network.bandwidth_mbps, Some(100));
    assert!(network.latency_ms.is_none());
}

#[test]
fn test_network_requirements_full() {
    // Test network requirements with all constraints
    let network = NetworkRequirements {
        bandwidth_mbps: Some(10000), // 10Gbps
        latency_ms: Some(5),
    };

    assert_eq!(network.bandwidth_mbps, Some(10000));
    assert_eq!(network.latency_ms, Some(5));
}

// ============================================================================
// GpuRequirements Tests (3 tests)
// ============================================================================

#[test]
fn test_gpu_requirements_memory_only() {
    // Test GPU requirements with memory only
    let gpu = GpuRequirements {
        min_memory_gb: 8.0,
        compute_capability: None,
    };

    assert_eq!(gpu.min_memory_gb, 8.0);
    assert!(gpu.compute_capability.is_none());
}

#[test]
fn test_gpu_requirements_full() {
    // Test GPU requirements with all fields
    let gpu = GpuRequirements {
        min_memory_gb: 24.0,
        compute_capability: Some("sm_90".to_string()),
    };

    assert_eq!(gpu.min_memory_gb, 24.0);
    assert_eq!(gpu.compute_capability, Some("sm_90".to_string()));
}

#[test]
fn test_gpu_requirements_clone() {
    // Test cloning GPU requirements
    let gpu = GpuRequirements {
        min_memory_gb: 16.0,
        compute_capability: Some("sm_86".to_string()),
    };
    let cloned = gpu.clone();

    assert_eq!(gpu.min_memory_gb, cloned.min_memory_gb);
    assert_eq!(gpu.compute_capability, cloned.compute_capability);
}

// ============================================================================
// DistributedRetryConfig Tests (5 tests)
// ============================================================================

#[test]
fn test_retry_config_default() {
    // Test default retry configuration
    let config = DistributedRetryConfig::default();

    assert_eq!(config.max_attempts, 3);
    assert!(!config.retry_conditions.is_empty());
}

#[test]
fn test_retry_config_custom() {
    // Test custom retry configuration
    let config = DistributedRetryConfig {
        max_attempts: 5,
        backoff_strategy: BackoffStrategy::Linear {
            initial_ms: 500,
            increment_ms: 500,
        },
        retry_conditions: vec![RetryCondition::NetworkError],
    };

    assert_eq!(config.max_attempts, 5);
    assert_eq!(config.retry_conditions.len(), 1);
}

#[test]
fn test_retry_config_no_retries() {
    // Test retry config with no retries
    let config = DistributedRetryConfig {
        max_attempts: 0,
        backoff_strategy: BackoffStrategy::Fixed { delay_ms: 0 },
        retry_conditions: vec![],
    };

    assert_eq!(config.max_attempts, 0);
    assert!(config.retry_conditions.is_empty());
}

#[test]
fn test_retry_config_clone() {
    // Test cloning retry config
    let config = DistributedRetryConfig::default();
    let cloned = config.clone();

    assert_eq!(config.max_attempts, cloned.max_attempts);
}

#[test]
fn test_retry_config_many_attempts() {
    // Test retry config with many attempts
    let config = DistributedRetryConfig {
        max_attempts: 100,
        backoff_strategy: BackoffStrategy::Exponential {
            base_ms: 100,
            max_ms: 60000,
        },
        retry_conditions: vec![
            RetryCondition::NetworkError,
            RetryCondition::ResourceUnavailable,
            RetryCondition::TemporaryFailure,
        ],
    };

    assert_eq!(config.max_attempts, 100);
    assert_eq!(config.retry_conditions.len(), 3);
}

// ============================================================================
// ResourceAllocationStrategy Tests (4 tests)
// ============================================================================

#[test]
fn test_allocation_strategy_fair() {
    // Test fair allocation strategy
    let strategy = ResourceAllocationStrategy::Fair;

    match strategy {
        ResourceAllocationStrategy::Fair => {
            // Success
        }
        _ => panic!("Expected Fair strategy"),
    }
}

#[test]
fn test_allocation_strategy_proportional() {
    // Test proportional allocation strategy
    let strategy = ResourceAllocationStrategy::Proportional;

    match strategy {
        ResourceAllocationStrategy::Proportional => {
            // Success
        }
        _ => panic!("Expected Proportional strategy"),
    }
}

#[test]
fn test_allocation_strategy_priority() {
    // Test priority allocation strategy
    let strategy = ResourceAllocationStrategy::Priority;

    match strategy {
        ResourceAllocationStrategy::Priority => {
            // Success
        }
        _ => panic!("Expected Priority strategy"),
    }
}

#[test]
fn test_allocation_strategy_clone() {
    // Test cloning allocation strategy
    let strategy = ResourceAllocationStrategy::Fair;
    let cloned = strategy.clone();

    match (strategy, cloned) {
        (ResourceAllocationStrategy::Fair, ResourceAllocationStrategy::Fair) => {
            // Success
        }
        _ => panic!("Cloned strategy should match"),
    }
}
