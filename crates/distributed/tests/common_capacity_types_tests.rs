// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Comprehensive tests for common capacity management types

use std::time::Duration;
use std::time::SystemTime;
use toadstool_distributed::common::capacity::types::{
    AvailableCapacity, CapacityAlert, CapacityConfig, CapacityInfo, CapacityRequirement,
    NetworkCapacity, ResourceUsageSnapshot,
};

// ============================================================================
// CapacityInfo Tests
// ============================================================================

#[test]
fn test_capacity_info_creation() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 8_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: Some(2),
        available_gpu_units: Some(1),
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    assert_eq!(info.total_cpu_cores, 8.0);
    assert_eq!(info.available_cpu_cores, 4.0);
    assert_eq!(info.total_gpu_units, Some(2));
}

#[test]
fn test_capacity_info_cpu_utilization() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 2.0,
        total_memory_bytes: 0,
        available_memory_bytes: 0,
        total_storage_bytes: 0,
        available_storage_bytes: 0,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 0,
        timestamp: SystemTime::now(),
    };

    let utilization = info.cpu_utilization();
    assert!((utilization - 0.75).abs() < 0.001); // 75% used
}

#[test]
fn test_capacity_info_cpu_utilization_zero_total() {
    let info = CapacityInfo {
        total_cpu_cores: 0.0,
        available_cpu_cores: 0.0,
        total_memory_bytes: 0,
        available_memory_bytes: 0,
        total_storage_bytes: 0,
        available_storage_bytes: 0,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 0,
        timestamp: SystemTime::now(),
    };

    assert_eq!(info.cpu_utilization(), 0.0);
}

#[test]
fn test_capacity_info_memory_utilization() {
    let info = CapacityInfo {
        total_cpu_cores: 0.0,
        available_cpu_cores: 0.0,
        total_memory_bytes: 1000,
        available_memory_bytes: 250,
        total_storage_bytes: 0,
        available_storage_bytes: 0,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 0,
        timestamp: SystemTime::now(),
    };

    let utilization = info.memory_utilization();
    assert!((utilization - 0.75).abs() < 0.001); // 75% used
}

#[test]
fn test_capacity_info_memory_utilization_zero_total() {
    let info = CapacityInfo {
        total_cpu_cores: 0.0,
        available_cpu_cores: 0.0,
        total_memory_bytes: 0,
        available_memory_bytes: 0,
        total_storage_bytes: 0,
        available_storage_bytes: 0,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 0,
        timestamp: SystemTime::now(),
    };

    assert_eq!(info.memory_utilization(), 0.0);
}

#[test]
fn test_capacity_info_storage_utilization() {
    let info = CapacityInfo {
        total_cpu_cores: 0.0,
        available_cpu_cores: 0.0,
        total_memory_bytes: 0,
        available_memory_bytes: 0,
        total_storage_bytes: 1000,
        available_storage_bytes: 400,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 0,
        timestamp: SystemTime::now(),
    };

    let utilization = info.storage_utilization();
    assert!((utilization - 0.6).abs() < 0.001); // 60% used
}

#[test]
fn test_capacity_info_storage_utilization_zero_total() {
    let info = CapacityInfo {
        total_cpu_cores: 0.0,
        available_cpu_cores: 0.0,
        total_memory_bytes: 0,
        available_memory_bytes: 0,
        total_storage_bytes: 0,
        available_storage_bytes: 0,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 0,
        timestamp: SystemTime::now(),
    };

    assert_eq!(info.storage_utilization(), 0.0);
}

#[test]
fn test_capacity_info_has_capacity_sufficient() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 8_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: Some(2),
        available_gpu_units: Some(1),
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: Some(1),
    };

    assert!(info.has_capacity(&requirement));
}

#[test]
fn test_capacity_info_has_capacity_insufficient_cpu() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 1.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 8_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: None,
    };

    assert!(!info.has_capacity(&requirement));
}

#[test]
fn test_capacity_info_has_capacity_insufficient_memory() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 1_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: None,
    };

    assert!(!info.has_capacity(&requirement));
}

#[test]
fn test_capacity_info_has_capacity_insufficient_gpu() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 8_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: Some(2),
        available_gpu_units: Some(0),
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: Some(1),
    };

    assert!(!info.has_capacity(&requirement));
}

#[test]
fn test_capacity_info_has_capacity_no_gpu_required() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 8_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: None,
    };

    assert!(info.has_capacity(&requirement));
}

#[test]
fn test_capacity_info_has_capacity_no_gpu_available_but_required() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 8_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: Some(1),
    };

    assert!(!info.has_capacity(&requirement));
}

// ============================================================================
// CapacityRequirement Tests
// ============================================================================

#[test]
fn test_capacity_requirement_creation() {
    let requirement = CapacityRequirement {
        cpu_cores: 4.0,
        memory_bytes: 8_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 1_000_000_000,
        gpu_units: Some(2),
    };

    assert_eq!(requirement.cpu_cores, 4.0);
    assert_eq!(requirement.memory_bytes, 8_000_000_000);
    assert_eq!(requirement.gpu_units, Some(2));
}

#[test]
fn test_capacity_requirement_no_gpu() {
    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 50_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: None,
    };

    assert!(requirement.gpu_units.is_none());
}

#[test]
fn test_capacity_requirement_serialization() {
    let requirement = CapacityRequirement {
        cpu_cores: 4.0,
        memory_bytes: 8_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 1_000_000_000,
        gpu_units: Some(2),
    };

    let json = serde_json::to_string(&requirement).unwrap();
    assert!(!json.is_empty());

    let deserialized: CapacityRequirement = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.cpu_cores, 4.0);
}

// ============================================================================
// CapacityConfig Tests
// ============================================================================

#[test]
fn test_capacity_config_default() {
    let config = CapacityConfig::default();

    assert_eq!(config.monitoring_interval, Duration::from_secs(10));
    assert_eq!(config.reserve_percent, 10.0);
    assert!(config.auto_scale);
    assert_eq!(config.scale_up_threshold, 0.8);
    assert_eq!(config.scale_down_threshold, 0.3);
}

#[test]
fn test_capacity_config_custom() {
    let config = CapacityConfig {
        monitoring_interval: Duration::from_secs(30),
        reserve_percent: 20.0,
        auto_scale: false,
        scale_up_threshold: 0.9,
        scale_down_threshold: 0.2,
    };

    assert_eq!(config.monitoring_interval, Duration::from_secs(30));
    assert_eq!(config.reserve_percent, 20.0);
    assert!(!config.auto_scale);
}

#[test]
fn test_capacity_config_serialization() {
    let config = CapacityConfig::default();

    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.is_empty());

    let deserialized: CapacityConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.reserve_percent, 10.0);
}

// ============================================================================
// AvailableCapacity Tests
// ============================================================================

#[test]
fn test_available_capacity_creation() {
    let capacity = AvailableCapacity {
        cpu_cores: 4.0,
        memory_bytes: 8_000_000_000,
        storage_bytes: 500_000_000_000,
        network_bandwidth_bps: 1_000_000_000,
        gpu_units: Some(2),
    };

    assert_eq!(capacity.cpu_cores, 4.0);
    assert_eq!(capacity.gpu_units, Some(2));
}

#[test]
fn test_available_capacity_no_gpu() {
    let capacity = AvailableCapacity {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 100_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: None,
    };

    assert!(capacity.gpu_units.is_none());
}

#[test]
fn test_available_capacity_serialization() {
    let capacity = AvailableCapacity {
        cpu_cores: 4.0,
        memory_bytes: 8_000_000_000,
        storage_bytes: 500_000_000_000,
        network_bandwidth_bps: 1_000_000_000,
        gpu_units: Some(2),
    };

    let json = serde_json::to_string(&capacity).unwrap();
    assert!(!json.is_empty());

    let deserialized: AvailableCapacity = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.cpu_cores, 4.0);
}

// ============================================================================
// NetworkCapacity Tests
// ============================================================================

#[test]
fn test_network_capacity_creation() {
    let capacity = NetworkCapacity {
        total_bandwidth_bps: 10_000_000_000,
        available_bandwidth_bps: 5_000_000_000,
        active_connections: 100,
        max_connections: 1000,
        latency_ms: 15.5,
    };

    assert_eq!(capacity.total_bandwidth_bps, 10_000_000_000);
    assert_eq!(capacity.active_connections, 100);
    assert_eq!(capacity.latency_ms, 15.5);
}

#[test]
fn test_network_capacity_at_capacity() {
    let capacity = NetworkCapacity {
        total_bandwidth_bps: 10_000_000_000,
        available_bandwidth_bps: 0,
        active_connections: 1000,
        max_connections: 1000,
        latency_ms: 50.0,
    };

    assert_eq!(capacity.available_bandwidth_bps, 0);
    assert_eq!(capacity.active_connections, capacity.max_connections);
}

#[test]
fn test_network_capacity_serialization() {
    let capacity = NetworkCapacity {
        total_bandwidth_bps: 10_000_000_000,
        available_bandwidth_bps: 5_000_000_000,
        active_connections: 100,
        max_connections: 1000,
        latency_ms: 15.5,
    };

    let json = serde_json::to_string(&capacity).unwrap();
    assert!(!json.is_empty());

    let deserialized: NetworkCapacity = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.latency_ms, 15.5);
}

// ============================================================================
// ResourceUsageSnapshot Tests
// ============================================================================

#[test]
fn test_resource_usage_snapshot_creation() {
    let capacity_info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 8_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: Some(2),
        available_gpu_units: Some(1),
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let snapshot = ResourceUsageSnapshot {
        target_id: "node-001".to_string(),
        capacity: capacity_info,
        active_workloads: 10,
        pending_workloads: 5,
        timestamp: SystemTime::now(),
    };

    assert_eq!(snapshot.target_id, "node-001");
    assert_eq!(snapshot.active_workloads, 10);
    assert_eq!(snapshot.pending_workloads, 5);
}

#[test]
fn test_resource_usage_snapshot_serialization() {
    let capacity_info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 16_000_000_000,
        available_memory_bytes: 8_000_000_000,
        total_storage_bytes: 1_000_000_000_000,
        available_storage_bytes: 500_000_000_000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let snapshot = ResourceUsageSnapshot {
        target_id: "node-002".to_string(),
        capacity: capacity_info,
        active_workloads: 20,
        pending_workloads: 10,
        timestamp: SystemTime::now(),
    };

    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(!json.is_empty());

    let deserialized: ResourceUsageSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.target_id, "node-002");
}

// ============================================================================
// CapacityAlert Tests
// ============================================================================

#[test]
fn test_capacity_alert_low_capacity() {
    let alert = CapacityAlert::LowCapacity {
        resource_type: "cpu".to_string(),
        utilization_percent: 85.5,
    };

    match alert {
        CapacityAlert::LowCapacity {
            resource_type,
            utilization_percent,
        } => {
            assert_eq!(resource_type, "cpu");
            assert_eq!(utilization_percent, 85.5);
        }
        _ => panic!("Expected LowCapacity variant"),
    }
}

#[test]
fn test_capacity_alert_exhausted() {
    let alert = CapacityAlert::Exhausted {
        resource_type: "memory".to_string(),
    };

    match alert {
        CapacityAlert::Exhausted { resource_type } => {
            assert_eq!(resource_type, "memory");
        }
        _ => panic!("Expected Exhausted variant"),
    }
}

#[test]
fn test_capacity_alert_restored() {
    let alert = CapacityAlert::Restored {
        resource_type: "storage".to_string(),
    };

    match alert {
        CapacityAlert::Restored { resource_type } => {
            assert_eq!(resource_type, "storage");
        }
        _ => panic!("Expected Restored variant"),
    }
}

#[test]
fn test_capacity_alert_serialization() {
    let alert = CapacityAlert::LowCapacity {
        resource_type: "cpu".to_string(),
        utilization_percent: 90.0,
    };

    let json = serde_json::to_string(&alert).unwrap();
    assert!(!json.is_empty());

    let deserialized: CapacityAlert = serde_json::from_str(&json).unwrap();
    match deserialized {
        CapacityAlert::LowCapacity {
            resource_type,
            utilization_percent,
        } => {
            assert_eq!(resource_type, "cpu");
            assert_eq!(utilization_percent, 90.0);
        }
        _ => panic!("Expected LowCapacity variant"),
    }
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_capacity_workflow_sufficient_resources() {
    let info = CapacityInfo {
        total_cpu_cores: 16.0,
        available_cpu_cores: 12.0,
        total_memory_bytes: 32_000_000_000,
        available_memory_bytes: 24_000_000_000,
        total_storage_bytes: 2_000_000_000_000,
        available_storage_bytes: 1_500_000_000_000,
        total_gpu_units: Some(4),
        available_gpu_units: Some(3),
        network_bandwidth_bps: 10_000_000_000,
        timestamp: SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 4.0,
        memory_bytes: 8_000_000_000,
        storage_bytes: 500_000_000_000,
        network_bandwidth_bps: 5_000_000_000,
        gpu_units: Some(2),
    };

    assert!(info.has_capacity(&requirement));
    assert!(info.cpu_utilization() < 0.5);
}

#[test]
fn test_capacity_workflow_insufficient_resources() {
    let info = CapacityInfo {
        total_cpu_cores: 4.0,
        available_cpu_cores: 0.5,
        total_memory_bytes: 8_000_000_000,
        available_memory_bytes: 1_000_000_000,
        total_storage_bytes: 500_000_000_000,
        available_storage_bytes: 10_000_000_000,
        total_gpu_units: Some(1),
        available_gpu_units: Some(0),
        network_bandwidth_bps: 1_000_000_000,
        timestamp: SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 4_000_000_000,
        storage_bytes: 50_000_000_000,
        network_bandwidth_bps: 500_000_000,
        gpu_units: Some(1),
    };

    assert!(!info.has_capacity(&requirement));
    assert!(info.cpu_utilization() > 0.8);
}
