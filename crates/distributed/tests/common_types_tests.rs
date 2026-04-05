// SPDX-License-Identifier: AGPL-3.0-or-later
//! Comprehensive tests for common types (capacity, distribution, load balancing)

#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
#![allow(clippy::unreadable_literal)]

use std::collections::HashMap;
use std::time::Duration;
use toadstool_distributed::common::capacity::types::*;
use toadstool_distributed::common::distribution::types::*;
use toadstool_distributed::common::load_balancing::types::*;
use uuid::Uuid;

// ==================== Capacity Types Tests ====================

#[test]
fn test_capacity_info_cpu_utilization_zero() {
    let info = CapacityInfo {
        total_cpu_cores: 0.0,
        available_cpu_cores: 0.0,
        total_memory_bytes: 1000,
        available_memory_bytes: 500,
        total_storage_bytes: 10000,
        available_storage_bytes: 5000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(info.cpu_utilization(), 0.0);
}

#[test]
fn test_capacity_info_cpu_utilization_half() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 1000,
        available_memory_bytes: 500,
        total_storage_bytes: 10000,
        available_storage_bytes: 5000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(info.cpu_utilization(), 0.5);
}

#[test]
fn test_capacity_info_cpu_utilization_full() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 0.0,
        total_memory_bytes: 1000,
        available_memory_bytes: 500,
        total_storage_bytes: 10000,
        available_storage_bytes: 5000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(info.cpu_utilization(), 1.0);
}

#[test]
fn test_capacity_info_memory_utilization() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 1000,
        available_memory_bytes: 250,
        total_storage_bytes: 10000,
        available_storage_bytes: 5000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(info.memory_utilization(), 0.75);
}

#[test]
fn test_capacity_info_storage_utilization() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 1000,
        available_memory_bytes: 500,
        total_storage_bytes: 10000,
        available_storage_bytes: 1000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(info.storage_utilization(), 0.9);
}

#[test]
fn test_capacity_info_has_capacity_sufficient() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 10000,
        available_memory_bytes: 5000,
        total_storage_bytes: 100000,
        available_storage_bytes: 50000,
        total_gpu_units: Some(2),
        available_gpu_units: Some(1),
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 2000,
        storage_bytes: 10000,
        network_bandwidth_bps: 500000,
        gpu_units: None,
    };

    assert!(info.has_capacity(&requirement));
}

#[test]
fn test_capacity_info_has_capacity_insufficient_cpu() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 1.0,
        total_memory_bytes: 10000,
        available_memory_bytes: 5000,
        total_storage_bytes: 100000,
        available_storage_bytes: 50000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 2000,
        storage_bytes: 10000,
        network_bandwidth_bps: 500000,
        gpu_units: None,
    };

    assert!(!info.has_capacity(&requirement));
}

#[test]
fn test_capacity_info_has_capacity_with_gpu() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 10000,
        available_memory_bytes: 5000,
        total_storage_bytes: 100000,
        available_storage_bytes: 50000,
        total_gpu_units: Some(2),
        available_gpu_units: Some(2),
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 2000,
        storage_bytes: 10000,
        network_bandwidth_bps: 500000,
        gpu_units: Some(1),
    };

    assert!(info.has_capacity(&requirement));
}

#[test]
fn test_capacity_info_has_capacity_insufficient_gpu() {
    let info = CapacityInfo {
        total_cpu_cores: 8.0,
        available_cpu_cores: 4.0,
        total_memory_bytes: 10000,
        available_memory_bytes: 5000,
        total_storage_bytes: 100000,
        available_storage_bytes: 50000,
        total_gpu_units: None,
        available_gpu_units: None,
        network_bandwidth_bps: 1000000,
        timestamp: std::time::SystemTime::now(),
    };

    let requirement = CapacityRequirement {
        cpu_cores: 2.0,
        memory_bytes: 2000,
        storage_bytes: 10000,
        network_bandwidth_bps: 500000,
        gpu_units: Some(1),
    };

    assert!(!info.has_capacity(&requirement));
}

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
fn test_capacity_requirement_clone() {
    let req = CapacityRequirement {
        cpu_cores: 4.0,
        memory_bytes: 8000,
        storage_bytes: 50000,
        network_bandwidth_bps: 1000000,
        gpu_units: Some(1),
    };

    let cloned = req.clone();
    assert_eq!(cloned.cpu_cores, req.cpu_cores);
    assert_eq!(cloned.gpu_units, req.gpu_units);
}

#[test]
fn test_capacity_alert_low_capacity() {
    let alert = CapacityAlert::LowCapacity {
        resource_type: "CPU".to_string(),
        utilization_percent: 85.0,
    };

    let debug_str = format!("{alert:?}");
    assert!(debug_str.contains("LowCapacity"));
}

#[test]
fn test_capacity_alert_exhausted() {
    let alert = CapacityAlert::Exhausted {
        resource_type: "Memory".to_string(),
    };

    let debug_str = format!("{alert:?}");
    assert!(debug_str.contains("Exhausted"));
}

#[test]
fn test_capacity_alert_restored() {
    let alert = CapacityAlert::Restored {
        resource_type: "Storage".to_string(),
    };

    let debug_str = format!("{alert:?}");
    assert!(debug_str.contains("Restored"));
}

#[test]
fn test_network_capacity_creation() {
    let capacity = NetworkCapacity {
        total_bandwidth_bps: 10000000,
        available_bandwidth_bps: 5000000,
        active_connections: 50,
        max_connections: 100,
        latency_ms: 25.5,
    };

    assert_eq!(capacity.total_bandwidth_bps, 10000000);
    assert_eq!(capacity.active_connections, 50);
    assert_eq!(capacity.latency_ms, 25.5);
}

#[test]
fn test_available_capacity_creation() {
    let capacity = AvailableCapacity {
        cpu_cores: 4.0,
        memory_bytes: 8000000000,
        storage_bytes: 500000000000,
        network_bandwidth_bps: 1000000000,
        gpu_units: Some(2),
    };

    assert_eq!(capacity.cpu_cores, 4.0);
    assert_eq!(capacity.gpu_units, Some(2));
}

// ==================== Distribution Types Tests ====================

#[test]
fn test_distribution_strategy_default() {
    let strategy = DistributionStrategy::default();
    assert_eq!(strategy, DistributionStrategy::Single);
}

#[test]
fn test_distribution_strategy_equal() {
    let strategy = DistributionStrategy::Equal;
    let debug_str = format!("{strategy:?}");
    assert!(debug_str.contains("Equal"));
}

#[test]
fn test_distribution_strategy_weighted() {
    let mut weights = HashMap::new();
    weights.insert("node1".to_string(), 0.6);
    weights.insert("node2".to_string(), 0.4);

    let strategy = DistributionStrategy::Weighted {
        weights: weights.clone(),
    };

    if let DistributionStrategy::Weighted { weights: w } = strategy {
        assert_eq!(w.len(), 2);
        assert_eq!(w.get("node1"), Some(&0.6));
    } else {
        panic!("Expected Weighted strategy");
    }
}

#[test]
fn test_distribution_strategy_cost_optimized() {
    let strategy = DistributionStrategy::CostOptimized;
    assert_eq!(strategy, DistributionStrategy::CostOptimized);
}

#[test]
fn test_distribution_strategy_performance_optimized() {
    let strategy = DistributionStrategy::PerformanceOptimized;
    assert_eq!(strategy, DistributionStrategy::PerformanceOptimized);
}

#[test]
fn test_distribution_strategy_latency_optimized() {
    let strategy = DistributionStrategy::LatencyOptimized;
    assert_eq!(strategy, DistributionStrategy::LatencyOptimized);
}

#[test]
fn test_distribution_strategy_regional_affinity() {
    let strategy = DistributionStrategy::RegionalAffinity {
        preferred_regions: vec!["us-west-2".to_string(), "eu-west-1".to_string()],
    };

    if let DistributionStrategy::RegionalAffinity { preferred_regions } = strategy {
        assert_eq!(preferred_regions.len(), 2);
    } else {
        panic!("Expected RegionalAffinity strategy");
    }
}

#[test]
fn test_distribution_strategy_replicated() {
    let strategy = DistributionStrategy::Replicated {
        replication_factor: 3,
    };

    if let DistributionStrategy::Replicated { replication_factor } = strategy {
        assert_eq!(replication_factor, 3);
    } else {
        panic!("Expected Replicated strategy");
    }
}

#[test]
fn test_distribution_strategy_burst() {
    let strategy = DistributionStrategy::Burst {
        primary_target: "node1".to_string(),
        burst_targets: vec!["node2".to_string(), "node3".to_string()],
    };

    if let DistributionStrategy::Burst { burst_targets, .. } = strategy {
        assert_eq!(burst_targets.len(), 2);
    } else {
        panic!("Expected Burst strategy");
    }
}

#[test]
fn test_distribution_plan_creation() {
    let plan = DistributionPlan {
        plan_id: Uuid::new_v4(),
        strategy: DistributionStrategy::Equal,
        targets: vec![],
        total_units: 1000,
        estimated_duration_secs: 60,
    };

    assert_eq!(plan.total_units, 1000);
    assert_eq!(plan.estimated_duration_secs, 60);
}

#[test]
fn test_target_type_variants() {
    let local = TargetType::LocalNode;
    let coordination = TargetType::CoordinationNode;
    let cloud = TargetType::CloudProvider;
    let k8s = TargetType::Kubernetes;
    let edge = TargetType::EdgeDevice;
    let self_hosted = TargetType::SelfHosted;

    assert_eq!(local, TargetType::LocalNode);
    assert_eq!(coordination, TargetType::CoordinationNode);
    assert_eq!(cloud, TargetType::CloudProvider);
    assert_eq!(k8s, TargetType::Kubernetes);
    assert_eq!(edge, TargetType::EdgeDevice);
    assert_eq!(self_hosted, TargetType::SelfHosted);
}

#[test]
fn test_distribution_target_creation() {
    let target = DistributionTarget {
        id: "node1".to_string(),
        target_type: TargetType::LocalNode,
        allocated_units: 500,
        weight: 1.0,
        capacity: ResourceCapacity {
            cpu_cores: 8.0,
            memory_bytes: 16000000000,
            storage_bytes: 1000000000000,
            network_bandwidth_bps: 1000000000,
            gpu_units: None,
        },
    };

    assert_eq!(target.allocated_units, 500);
    assert_eq!(target.weight, 1.0);
}

#[test]
fn test_resource_capacity_with_gpu() {
    let capacity = ResourceCapacity {
        cpu_cores: 16.0,
        memory_bytes: 32000000000,
        storage_bytes: 2000000000000,
        network_bandwidth_bps: 10000000000,
        gpu_units: Some(4),
    };

    assert_eq!(capacity.gpu_units, Some(4));
}

#[test]
fn test_distribution_algorithm_variants() {
    let algorithms = vec![
        DistributionAlgorithm::RoundRobin,
        DistributionAlgorithm::LeastLoaded,
        DistributionAlgorithm::LoadBased,
        DistributionAlgorithm::WeightedRoundRobin,
        DistributionAlgorithm::Random,
        DistributionAlgorithm::ConsistentHashing,
        DistributionAlgorithm::PowerOfTwoChoices,
        DistributionAlgorithm::CapabilityMatched,
        DistributionAlgorithm::GeographicOptimized,
        DistributionAlgorithm::Custom("MyAlgorithm".to_string()),
    ];

    assert_eq!(algorithms.len(), 10);
}

#[test]
fn test_distribution_config_default() {
    let config = DistributionConfig::default();

    assert_eq!(config.default_strategy, DistributionStrategy::Single);
    assert_eq!(config.max_targets, 100);
    assert_eq!(config.min_units_per_target, 1);
    assert!(config.auto_rebalance);
    assert_eq!(config.rebalance_threshold_percent, 20.0);
}

#[test]
fn test_distribution_result_success() {
    let result = DistributionResult {
        request_id: Uuid::new_v4(),
        plan: DistributionPlan {
            plan_id: Uuid::new_v4(),
            strategy: DistributionStrategy::Equal,
            targets: vec![],
            total_units: 1000,
            estimated_duration_secs: 60,
        },
        targets_used: vec!["node1".to_string(), "node2".to_string()],
        started_at: std::time::SystemTime::now(),
        success: true,
        error: None,
    };

    assert!(result.success);
    assert!(result.error.is_none());
    assert_eq!(result.targets_used.len(), 2);
}

#[test]
fn test_distribution_result_failure() {
    let result = DistributionResult {
        request_id: Uuid::new_v4(),
        plan: DistributionPlan {
            plan_id: Uuid::new_v4(),
            strategy: DistributionStrategy::Equal,
            targets: vec![],
            total_units: 1000,
            estimated_duration_secs: 60,
        },
        targets_used: vec![],
        started_at: std::time::SystemTime::now(),
        success: false,
        error: Some("No available targets".to_string()),
    };

    assert!(!result.success);
    assert!(result.error.is_some());
}

// ==================== Load Balancing Types Tests ====================

#[test]
fn test_load_balancing_strategy_default() {
    let strategy = LoadBalancingStrategy::default();
    assert_eq!(strategy, LoadBalancingStrategy::RoundRobin);
}

#[test]
fn test_load_balancing_strategy_variants() {
    let strategies = vec![
        LoadBalancingStrategy::PrimaryOnly,
        LoadBalancingStrategy::RoundRobin,
        LoadBalancingStrategy::LeastConnections,
        LoadBalancingStrategy::LeastLoaded,
        LoadBalancingStrategy::LatencyBased,
        LoadBalancingStrategy::CostOptimized,
        LoadBalancingStrategy::Random,
        LoadBalancingStrategy::ConsistentHashing,
        LoadBalancingStrategy::IpHash,
        LoadBalancingStrategy::Custom("MyStrategy".to_string()),
    ];

    assert!(strategies.len() >= 10);
}

#[test]
fn test_load_balancing_strategy_weighted() {
    let mut weights = HashMap::new();
    weights.insert("server1".to_string(), 70);
    weights.insert("server2".to_string(), 30);

    let strategy = LoadBalancingStrategy::Weighted {
        weights: weights.clone(),
    };

    if let LoadBalancingStrategy::Weighted { weights: w } = strategy {
        assert_eq!(w.len(), 2);
        assert_eq!(w.get("server1"), Some(&70));
    } else {
        panic!("Expected Weighted strategy");
    }
}

#[test]
fn test_load_balancing_strategy_regional_affinity() {
    let strategy = LoadBalancingStrategy::RegionalAffinity {
        preferred_regions: vec!["us-east-1".to_string()],
    };

    if let LoadBalancingStrategy::RegionalAffinity { preferred_regions } = strategy {
        assert_eq!(preferred_regions.len(), 1);
    } else {
        panic!("Expected RegionalAffinity strategy");
    }
}

#[test]
fn test_load_balancing_algorithm_variants() {
    let algorithms = [
        LoadBalancingAlgorithm::RoundRobin,
        LoadBalancingAlgorithm::WeightedRoundRobin,
        LoadBalancingAlgorithm::LeastConnections,
        LoadBalancingAlgorithm::LeastResponseTime,
        LoadBalancingAlgorithm::ResourceBased,
        LoadBalancingAlgorithm::ResourceAware,
        LoadBalancingAlgorithm::CostAware,
        LoadBalancingAlgorithm::Random,
        LoadBalancingAlgorithm::PowerOfTwoChoices,
    ];

    assert_eq!(algorithms.len(), 9);
}

#[test]
fn test_load_balancer_config_default() {
    let config = LoadBalancerConfig::default();

    assert_eq!(config.strategy, LoadBalancingStrategy::RoundRobin);
    assert!(!config.session_affinity);
    assert_eq!(config.feedback_interval_secs, 10);
}

#[test]
fn test_health_check_config_default() {
    let config = HealthCheckConfig::default();

    assert!(config.enabled);
    assert_eq!(config.interval, Duration::from_secs(30));
    assert_eq!(config.timeout, Duration::from_secs(10)); // Default is 10 seconds
    assert_eq!(config.unhealthy_threshold, 3);
    assert_eq!(config.healthy_threshold, 2);
}

#[test]
fn test_failover_config_default() {
    let config = FailoverConfig::default();

    assert!(config.enabled);
    assert_eq!(config.max_attempts, 3);
    assert_eq!(config.retry_delay_secs, 5);
}

#[test]
fn test_health_status_variants() {
    let healthy = HealthStatus::Healthy;
    let degraded = HealthStatus::Degraded;
    let unhealthy = HealthStatus::Unhealthy;
    let unknown = HealthStatus::Unknown;

    assert_eq!(healthy, HealthStatus::Healthy);
    assert_eq!(degraded, HealthStatus::Degraded);
    assert_eq!(unhealthy, HealthStatus::Unhealthy);
    assert_eq!(unknown, HealthStatus::Unknown);
}

#[test]
fn test_load_metrics_creation() {
    let metrics = LoadMetrics {
        target_id: "server1".to_string(),
        active_count: 50,
        cpu_usage: 0.75,
        memory_usage: 0.60,
        avg_response_time_ms: 125.5,
        request_rate: 100.0,
        error_rate: 0.02,
        health: HealthStatus::Healthy,
        timestamp: std::time::SystemTime::now(),
    };

    assert_eq!(metrics.active_count, 50);
    assert_eq!(metrics.cpu_usage, 0.75);
    assert_eq!(metrics.error_rate, 0.02);
    assert_eq!(metrics.health, HealthStatus::Healthy);
}

#[test]
fn test_load_balancing_advice_creation() {
    let advice = LoadBalancingAdvice {
        target_id: "node1".to_string(),
        confidence: 0.95,
        reason: "Lowest latency and best resource availability".to_string(),
        alternatives: vec!["node2".to_string(), "node3".to_string()],
    };

    assert_eq!(advice.confidence, 0.95);
    assert_eq!(advice.alternatives.len(), 2);
}

#[test]
fn test_load_balancing_advice_high_confidence() {
    let advice = LoadBalancingAdvice {
        target_id: "best-node".to_string(),
        confidence: 0.99,
        reason: "Clear winner".to_string(),
        alternatives: vec![],
    };

    assert!(advice.confidence > 0.9);
    assert!(advice.alternatives.is_empty());
}

#[test]
fn test_load_metrics_high_error_rate() {
    let metrics = LoadMetrics {
        target_id: "problematic-server".to_string(),
        active_count: 10,
        cpu_usage: 0.5,
        memory_usage: 0.5,
        avg_response_time_ms: 500.0,
        request_rate: 20.0,
        error_rate: 0.25,
        health: HealthStatus::Degraded,
        timestamp: std::time::SystemTime::now(),
    };

    assert!(metrics.error_rate > 0.2);
    assert_eq!(metrics.health, HealthStatus::Degraded);
}

#[test]
fn test_serialization_distribution_strategy() {
    let strategy = DistributionStrategy::Equal;
    let serialized = serde_json::to_string(&strategy).expect("Failed to serialize");
    let deserialized: DistributionStrategy =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(strategy, deserialized);
}

#[test]
fn test_serialization_load_balancing_strategy() {
    let strategy = LoadBalancingStrategy::RoundRobin;
    let serialized = serde_json::to_string(&strategy).expect("Failed to serialize");
    let deserialized: LoadBalancingStrategy =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(strategy, deserialized);
}

#[test]
fn test_serialization_health_status() {
    let status = HealthStatus::Healthy;
    let serialized = serde_json::to_string(&status).expect("Failed to serialize");
    let deserialized: HealthStatus =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
    assert_eq!(status, deserialized);
}
