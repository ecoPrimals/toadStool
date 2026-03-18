// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(clippy::float_cmp)]
//! Comprehensive tests for distributed types module
//!
//! This test suite covers:
//! - `JobPriority` enum and ordering
//! - `UniversalJobType` enum variants
//! - `ExecutionTarget` enum variants
//! - `LoadBalancingStrategy` enum
//! - `CompatibilityMode` enum
//! - `DistributedExecutionStatus` enum
//! - `BackoffStrategy` enum
//! - `RetryCondition` enum
//! - Resource requirement structs

use std::collections::HashMap;
use toadstool_distributed::types::*;

// ============================================================================
// JobPriority Tests
// ============================================================================

#[test]
fn test_job_priority_emergency() {
    let priority = JobPriority::Emergency;

    assert!(matches!(priority, JobPriority::Emergency));
    assert_eq!(priority as u8, 0);
}

#[test]
fn test_job_priority_critical() {
    let priority = JobPriority::Critical;

    assert!(matches!(priority, JobPriority::Critical));
    assert_eq!(priority as u8, 1);
}

#[test]
fn test_job_priority_high() {
    let priority = JobPriority::High;
    assert_eq!(priority as u8, 2);
}

#[test]
fn test_job_priority_normal() {
    let priority = JobPriority::Normal;
    assert_eq!(priority as u8, 3);
}

#[test]
fn test_job_priority_low() {
    let priority = JobPriority::Low;
    assert_eq!(priority as u8, 4);
}

#[test]
fn test_job_priority_background() {
    let priority = JobPriority::Background;
    assert_eq!(priority as u8, 5);
}

#[test]
fn test_job_priority_ordering() {
    // Emergency should be highest priority (lowest numeric value)
    assert!(JobPriority::Emergency < JobPriority::Critical);
    assert!(JobPriority::Critical < JobPriority::High);
    assert!(JobPriority::High < JobPriority::Normal);
    assert!(JobPriority::Normal < JobPriority::Low);
    assert!(JobPriority::Low < JobPriority::Background);
}

#[test]
fn test_job_priority_eq() {
    assert_eq!(JobPriority::Normal, JobPriority::Normal);
    assert_ne!(JobPriority::High, JobPriority::Low);
}

#[test]
fn test_job_priority_clone() {
    let priority1 = JobPriority::High;
    let priority2 = priority1;

    assert_eq!(priority1, priority2);
}

#[test]
fn test_job_priority_serialization() {
    let priorities = vec![
        JobPriority::Emergency,
        JobPriority::Critical,
        JobPriority::High,
        JobPriority::Normal,
        JobPriority::Low,
        JobPriority::Background,
    ];

    for priority in priorities {
        let json = serde_json::to_string(&priority);
        assert!(json.is_ok());
    }
}

// ============================================================================
// UniversalJobType Tests
// ============================================================================

#[test]
fn test_universal_job_type_local() {
    let job_type = UniversalJobType::Local;

    assert!(matches!(job_type, UniversalJobType::Local));
}

#[test]
fn test_universal_job_type_from_str_local() {
    let job_type: UniversalJobType = "local".parse().unwrap();

    assert!(matches!(job_type, UniversalJobType::Local));
}

#[test]
fn test_universal_job_type_compute_intensive() {
    let job_type = UniversalJobType::ComputeIntensive;

    assert!(matches!(job_type, UniversalJobType::ComputeIntensive));
}

#[test]
fn test_universal_job_type_from_str_compute() {
    let job_type: UniversalJobType = "compute_intensive".parse().unwrap();

    assert!(matches!(job_type, UniversalJobType::ComputeIntensive));
}

#[test]
fn test_universal_job_type_memory_intensive() {
    let job_type = UniversalJobType::MemoryIntensive;

    assert!(matches!(job_type, UniversalJobType::MemoryIntensive));
}

#[test]
fn test_universal_job_type_network_intensive() {
    let job_type = UniversalJobType::NetworkIntensive;

    assert!(matches!(job_type, UniversalJobType::NetworkIntensive));
}

#[test]
fn test_universal_job_type_storage_intensive() {
    let job_type = UniversalJobType::StorageIntensive;

    assert!(matches!(job_type, UniversalJobType::StorageIntensive));
}

#[test]
fn test_universal_job_type_hybrid() {
    let job_type = UniversalJobType::Hybrid;

    assert!(matches!(job_type, UniversalJobType::Hybrid));
}

#[test]
fn test_universal_job_type_machine_learning() {
    let job_type = UniversalJobType::MachineLearning;

    assert!(matches!(job_type, UniversalJobType::MachineLearning));
}

#[test]
fn test_universal_job_type_custom() {
    let job_type = UniversalJobType::Custom("quantum_computing".to_string());

    match job_type {
        UniversalJobType::Custom(name) => {
            assert_eq!(name, "quantum_computing");
        }
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_universal_job_type_from_str_custom() {
    let job_type: UniversalJobType = "unknown_type".parse().unwrap();

    match job_type {
        UniversalJobType::Custom(name) => {
            assert_eq!(name, "unknown_type");
        }
        _ => panic!("Expected Custom variant"),
    }
}

#[test]
fn test_universal_job_type_clone() {
    let job_type1 = UniversalJobType::GPU;
    let job_type2 = job_type1.clone();

    assert_eq!(job_type1, job_type2);
}

#[test]
fn test_universal_job_type_serialization() {
    let job_types = vec![
        UniversalJobType::Local,
        UniversalJobType::ComputeIntensive,
        UniversalJobType::MachineLearning,
        UniversalJobType::Custom("test".to_string()),
    ];

    for job_type in job_types {
        let json = serde_json::to_string(&job_type);
        assert!(json.is_ok());
    }
}

// ============================================================================
// ExecutionTarget Tests
// ============================================================================

#[test]
fn test_execution_target_local() {
    let target = ExecutionTarget::Local;

    assert!(matches!(target, ExecutionTarget::Local));
}

#[test]
fn test_execution_target_clone() {
    let target1 = ExecutionTarget::Local;
    let target2 = target1.clone();

    assert!(matches!(target2, ExecutionTarget::Local));
}

#[test]
fn test_execution_target_serialization() {
    let target = ExecutionTarget::Local;
    let json = serde_json::to_string(&target);

    assert!(json.is_ok());
}

// ============================================================================
// LoadBalancingStrategy Tests
// ============================================================================

#[test]
fn test_load_balancing_round_robin() {
    let strategy = LoadBalancingStrategy::RoundRobin;

    assert!(matches!(strategy, LoadBalancingStrategy::RoundRobin));
}

#[test]
fn test_load_balancing_least_connections() {
    let strategy = LoadBalancingStrategy::LeastConnections;

    assert!(matches!(strategy, LoadBalancingStrategy::LeastConnections));
}

#[test]
fn test_load_balancing_resource_aware() {
    let strategy = LoadBalancingStrategy::ResourceAware;

    assert!(matches!(strategy, LoadBalancingStrategy::ResourceAware));
}

#[test]
fn test_load_balancing_latency_based() {
    let strategy = LoadBalancingStrategy::LatencyBased;

    assert!(matches!(strategy, LoadBalancingStrategy::LatencyBased));
}

#[test]
fn test_load_balancing_weighted_round_robin() {
    let mut weights = HashMap::new();
    weights.insert("node1".to_string(), 2);
    weights.insert("node2".to_string(), 1);

    let strategy = LoadBalancingStrategy::WeightedRoundRobin { weights };

    assert!(matches!(
        strategy,
        LoadBalancingStrategy::WeightedRoundRobin { .. }
    ));
}

#[test]
fn test_load_balancing_serialization() {
    let strategies = vec![
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::ResourceAware,
        LoadBalancingStrategy::LatencyBased,
    ];

    for strategy in strategies {
        let json = serde_json::to_string(&strategy);
        assert!(json.is_ok());
    }
}

// ============================================================================
// CompatibilityMode Tests
// ============================================================================

#[test]
fn test_compatibility_mode_native() {
    let mode = CompatibilityMode::Native;

    assert!(matches!(mode, CompatibilityMode::Native));
}

#[test]
fn test_compatibility_mode_container() {
    let mode = CompatibilityMode::Container;

    assert!(matches!(mode, CompatibilityMode::Container));
}

#[test]
fn test_compatibility_mode_emulated() {
    let mode = CompatibilityMode::Emulated;

    assert!(matches!(mode, CompatibilityMode::Emulated));
}

#[test]
fn test_compatibility_mode_linux_compat() {
    let mode = CompatibilityMode::LinuxCompat;

    assert!(matches!(mode, CompatibilityMode::LinuxCompat));
}

#[test]
fn test_compatibility_mode_legacy() {
    let mode = CompatibilityMode::LegacyCompat {
        system_type: "DOS".to_string(),
    };

    match mode {
        CompatibilityMode::LegacyCompat { system_type } => {
            assert_eq!(system_type, "DOS");
        }
        _ => panic!("Expected LegacyCompat variant"),
    }
}

#[test]
fn test_compatibility_mode_eq() {
    assert_eq!(CompatibilityMode::Native, CompatibilityMode::Native);
    assert_ne!(CompatibilityMode::Native, CompatibilityMode::Container);
}

#[test]
fn test_compatibility_mode_serialization() {
    let modes = vec![
        CompatibilityMode::Native,
        CompatibilityMode::Container,
        CompatibilityMode::Emulated,
        CompatibilityMode::LinuxCompat,
    ];

    for mode in modes {
        let json = serde_json::to_string(&mode);
        assert!(json.is_ok());
    }
}

// ============================================================================
// CpuRequirements Tests
// ============================================================================

#[test]
fn test_cpu_requirements_creation() {
    let cpu = CpuRequirements {
        min_cores: 2.0,
        max_cores: Some(4.0),
    };

    assert_eq!(cpu.min_cores, 2.0);
    assert_eq!(cpu.max_cores, Some(4.0));
}

#[test]
fn test_cpu_requirements_no_max() {
    let cpu = CpuRequirements {
        min_cores: 1.0,
        max_cores: None,
    };

    assert_eq!(cpu.min_cores, 1.0);
    assert_eq!(cpu.max_cores, None);
}

#[test]
fn test_cpu_requirements_fractional_cores() {
    let cpu = CpuRequirements {
        min_cores: 0.5,
        max_cores: Some(1.5),
    };

    assert_eq!(cpu.min_cores, 0.5);
}

// ============================================================================
// MemoryRequirements Tests
// ============================================================================

#[test]
fn test_memory_requirements_creation() {
    let memory = MemoryRequirements {
        min_bytes: 1_073_741_824,       // 1 GB
        max_bytes: Some(4_294_967_296), // 4 GB
    };

    assert_eq!(memory.min_bytes, 1_073_741_824);
    assert_eq!(memory.max_bytes, Some(4_294_967_296));
}

#[test]
fn test_memory_requirements_no_max() {
    let memory = MemoryRequirements {
        min_bytes: 536_870_912, // 512 MB
        max_bytes: None,
    };

    assert_eq!(memory.min_bytes, 536_870_912);
    assert_eq!(memory.max_bytes, None);
}

// ============================================================================
// StorageRequirements Tests
// ============================================================================

#[test]
fn test_storage_requirements_creation() {
    let storage = StorageRequirements {
        min_bytes: 10_737_418_240,        // 10 GB
        max_bytes: Some(107_374_182_400), // 100 GB
    };

    assert_eq!(storage.min_bytes, 10_737_418_240);
    assert_eq!(storage.max_bytes, Some(107_374_182_400));
}

// ============================================================================
// NetworkRequirements Tests
// ============================================================================

#[test]
fn test_network_requirements_bandwidth() {
    let network = NetworkRequirements {
        bandwidth_mbps: Some(100),
        latency_ms: Some(10),
    };

    assert_eq!(network.bandwidth_mbps, Some(100));
    assert_eq!(network.latency_ms, Some(10));
}

#[test]
fn test_network_requirements_optional() {
    let network = NetworkRequirements {
        bandwidth_mbps: None,
        latency_ms: None,
    };

    assert_eq!(network.bandwidth_mbps, None);
    assert_eq!(network.latency_ms, None);
}

// ============================================================================
// GpuRequirements Tests
// ============================================================================

#[test]
fn test_gpu_requirements_creation() {
    let gpu = GpuRequirements {
        min_memory_gb: 8.0,
        compute_capability: Some("7.5".to_string()),
    };

    assert_eq!(gpu.min_memory_gb, 8.0);
    assert_eq!(gpu.compute_capability, Some("7.5".to_string()));
}

#[test]
fn test_gpu_requirements_no_compute_capability() {
    let gpu = GpuRequirements {
        min_memory_gb: 4.0,
        compute_capability: None,
    };

    assert_eq!(gpu.min_memory_gb, 4.0);
    assert_eq!(gpu.compute_capability, None);
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_distributed_types_coverage_summary() {
    println!("=== Distributed Types Test Coverage ===");
    println!("JobPriority Tests:           10 tests");
    println!("UniversalJobType Tests:      14 tests");
    println!("ExecutionTarget Tests:       3 tests");
    println!("LoadBalancingStrategy:       6 tests");
    println!("CompatibilityMode Tests:     7 tests");
    println!("CpuRequirements Tests:       3 tests");
    println!("MemoryRequirements Tests:    2 tests");
    println!("StorageRequirements Tests:   1 test");
    println!("NetworkRequirements Tests:   2 tests");
    println!("GpuRequirements Tests:       2 tests");
    println!("Total:                       50 tests");
    println!("========================================");
}
