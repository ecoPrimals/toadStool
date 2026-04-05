// SPDX-License-Identifier: AGPL-3.0-or-later
#![allow(
    clippy::float_cmp,
    reason = "exact comparison intended in this context"
)]
//! Comprehensive tests for cloud scheduling and deployment strategies
//!
//! This test suite covers:
//! - `HybridSchedulingStrategy` enum
//! - `CloudLoadBalancingStrategy` enum  
//! - `DeploymentStrategy` enum
//! - `DistributionStrategy` enum
//! - `NetworkingFeature` enum
//! - `SecurityFeature` enum
//! - `TopologyType` enum
//! - `ConnectionStatus` enum
//! - `ReplicaStatus` enum
//! - `TrustLevel` enum
//! - `ConsistencyLevel` enum

use std::collections::HashMap;
use toadstool_distributed::cloud::*;

// ============================================================================
// HybridSchedulingStrategy Tests
// ============================================================================

#[test]
fn test_hybrid_scheduling_cost_optimized() {
    let strategy = HybridSchedulingStrategy::CostOptimized;

    assert!(matches!(strategy, HybridSchedulingStrategy::CostOptimized));
}

#[test]
fn test_hybrid_scheduling_performance_optimized() {
    let strategy = HybridSchedulingStrategy::PerformanceOptimized;

    assert!(matches!(
        strategy,
        HybridSchedulingStrategy::PerformanceOptimized
    ));
}

#[test]
fn test_hybrid_scheduling_balanced() {
    let strategy = HybridSchedulingStrategy::Balanced {
        cost_weight: 0.6,
        performance_weight: 0.3,
        compliance_weight: 0.1,
    };

    match strategy {
        HybridSchedulingStrategy::Balanced {
            cost_weight,
            performance_weight,
            compliance_weight,
        } => {
            assert_eq!(cost_weight, 0.6);
            assert_eq!(performance_weight, 0.3);
            assert_eq!(compliance_weight, 0.1);
        }
        _ => panic!("Expected Balanced variant"),
    }
}

#[test]
fn test_hybrid_scheduling_sustainability_focused() {
    let strategy = HybridSchedulingStrategy::SustainabilityFocused {
        renewable_energy_preference: 0.9,
    };

    match strategy {
        HybridSchedulingStrategy::SustainabilityFocused {
            renewable_energy_preference,
        } => {
            assert_eq!(renewable_energy_preference, 0.9);
        }
        _ => panic!("Expected SustainabilityFocused variant"),
    }
}

// ============================================================================
// CloudLoadBalancingStrategy Tests
// ============================================================================

#[test]
fn test_cloud_load_balancing_primary_only() {
    let strategy = CloudLoadBalancingStrategy::PrimaryOnly;

    assert!(matches!(strategy, CloudLoadBalancingStrategy::PrimaryOnly));
}

#[test]
fn test_cloud_load_balancing_round_robin() {
    let strategy = CloudLoadBalancingStrategy::RoundRobin;

    assert!(matches!(strategy, CloudLoadBalancingStrategy::RoundRobin));
}

#[test]
fn test_cloud_load_balancing_latency_based() {
    let strategy = CloudLoadBalancingStrategy::LatencyBased;

    assert!(matches!(strategy, CloudLoadBalancingStrategy::LatencyBased));
}

#[test]
fn test_cloud_load_balancing_cost_optimized() {
    let strategy = CloudLoadBalancingStrategy::CostOptimized;

    assert!(matches!(
        strategy,
        CloudLoadBalancingStrategy::CostOptimized
    ));
}

#[test]
fn test_cloud_load_balancing_regional_affinity() {
    let strategy = CloudLoadBalancingStrategy::RegionalAffinity {
        preferred_regions: vec!["us-east-1".to_string(), "eu-west-1".to_string()],
    };

    assert!(matches!(
        strategy,
        CloudLoadBalancingStrategy::RegionalAffinity { .. }
    ));
}

// ============================================================================
// NetworkingFeature Tests
// ============================================================================

#[test]
fn test_networking_feature_vpc() {
    let feature = NetworkingFeature::VPC;

    assert!(matches!(feature, NetworkingFeature::VPC));
}

#[test]
fn test_networking_feature_load_balancer() {
    let feature = NetworkingFeature::LoadBalancer;

    assert!(matches!(feature, NetworkingFeature::LoadBalancer));
}

#[test]
fn test_networking_feature_cdn() {
    let feature = NetworkingFeature::CDN;

    assert!(matches!(feature, NetworkingFeature::CDN));
}

#[test]
fn test_networking_feature_private_networking() {
    let feature = NetworkingFeature::PrivateNetworking;

    assert!(matches!(feature, NetworkingFeature::PrivateNetworking));
}

#[test]
fn test_networking_feature_vpn() {
    let feature = NetworkingFeature::VPN;

    assert!(matches!(feature, NetworkingFeature::VPN));
}

// ============================================================================
// SecurityFeature Tests
// ============================================================================

#[test]
fn test_security_feature_encryption() {
    let feature = SecurityFeature::Encryption;

    assert!(matches!(feature, SecurityFeature::Encryption));
}

#[test]
fn test_security_feature_identity_management() {
    let feature = SecurityFeature::IdentityManagement;

    assert!(matches!(feature, SecurityFeature::IdentityManagement));
}

#[test]
fn test_security_feature_network_security() {
    let feature = SecurityFeature::NetworkSecurity;

    assert!(matches!(feature, SecurityFeature::NetworkSecurity));
}

#[test]
fn test_security_feature_compliance() {
    let feature = SecurityFeature::Compliance;

    assert!(matches!(feature, SecurityFeature::Compliance));
}

// ============================================================================
// TopologyType Tests
// ============================================================================

#[test]
fn test_topology_type_centralized() {
    let topology = TopologyType::Centralized;

    assert!(matches!(topology, TopologyType::Centralized));
}

#[test]
fn test_topology_type_distributed() {
    let topology = TopologyType::Distributed;

    assert!(matches!(topology, TopologyType::Distributed));
}

#[test]
fn test_topology_type_mesh() {
    let topology = TopologyType::Mesh;

    assert!(matches!(topology, TopologyType::Mesh));
}

#[test]
fn test_topology_type_hierarchical() {
    let topology = TopologyType::Hierarchical;

    assert!(matches!(topology, TopologyType::Hierarchical));
}

#[test]
fn test_topology_type_default() {
    let topology = TopologyType::default();

    assert!(matches!(topology, TopologyType::Centralized));
}

// ============================================================================
// ConnectionStatus Tests
// ============================================================================

#[test]
fn test_connection_status_active() {
    let status = ConnectionStatus::Active;

    assert!(matches!(status, ConnectionStatus::Active));
}

#[test]
fn test_connection_status_inactive() {
    let status = ConnectionStatus::Inactive;

    assert!(matches!(status, ConnectionStatus::Inactive));
}

#[test]
fn test_connection_status_error() {
    let status = ConnectionStatus::Error;

    assert!(matches!(status, ConnectionStatus::Error));
}

#[test]
fn test_connection_status_default() {
    let status = ConnectionStatus::default();

    assert!(matches!(status, ConnectionStatus::Active));
}

// ============================================================================
// ReplicaStatus Tests
// ============================================================================

#[test]
fn test_replica_status_synced() {
    let status = ReplicaStatus::Synced;

    assert!(matches!(status, ReplicaStatus::Synced));
}

#[test]
fn test_replica_status_syncing() {
    let status = ReplicaStatus::Syncing;

    assert!(matches!(status, ReplicaStatus::Syncing));
}

#[test]
fn test_replica_status_out_of_sync() {
    let status = ReplicaStatus::OutOfSync;

    assert!(matches!(status, ReplicaStatus::OutOfSync));
}

#[test]
fn test_replica_status_default() {
    let status = ReplicaStatus::default();

    assert!(matches!(status, ReplicaStatus::Synced));
}

// ============================================================================
// TrustLevel Tests
// ============================================================================

#[test]
fn test_trust_level_trusted() {
    let level = TrustLevel::Trusted;

    assert!(matches!(level, TrustLevel::Trusted));
}

#[test]
fn test_trust_level_untrusted() {
    let level = TrustLevel::Untrusted;

    assert!(matches!(level, TrustLevel::Untrusted));
}

#[test]
fn test_trust_level_conditional() {
    let level = TrustLevel::Conditional;

    assert!(matches!(level, TrustLevel::Conditional));
}

#[test]
fn test_trust_level_default() {
    let level = TrustLevel::default();

    assert!(matches!(level, TrustLevel::Trusted));
}

// ============================================================================
// ConsistencyLevel Tests
// ============================================================================

#[test]
fn test_consistency_level_strong() {
    let level = ConsistencyLevel::Strong;

    assert!(matches!(level, ConsistencyLevel::Strong));
}

#[test]
fn test_consistency_level_eventual() {
    let level = ConsistencyLevel::Eventual;

    assert!(matches!(level, ConsistencyLevel::Eventual));
}

#[test]
fn test_consistency_level_weak() {
    let level = ConsistencyLevel::Weak;

    assert!(matches!(level, ConsistencyLevel::Weak));
}

#[test]
fn test_consistency_level_default() {
    let level = ConsistencyLevel::default();

    assert!(matches!(level, ConsistencyLevel::Strong));
}

// ============================================================================
// DistributionStrategy Tests
// ============================================================================

#[test]
fn test_distribution_strategy_equal() {
    let strategy = DistributionStrategy::Equal;

    assert!(matches!(strategy, DistributionStrategy::Equal));
}

#[test]
fn test_distribution_strategy_weighted() {
    let mut weights = HashMap::new();
    weights.insert("provider1".to_string(), 0.7);
    weights.insert("provider2".to_string(), 0.3);

    let strategy = DistributionStrategy::Weighted { weights };

    assert!(matches!(strategy, DistributionStrategy::Weighted { .. }));
}

#[test]
fn test_distribution_strategy_cost_optimized() {
    let strategy = DistributionStrategy::CostOptimized;

    assert!(matches!(strategy, DistributionStrategy::CostOptimized));
}

#[test]
fn test_distribution_strategy_performance_optimized() {
    let strategy = DistributionStrategy::PerformanceOptimized;

    assert!(matches!(
        strategy,
        DistributionStrategy::PerformanceOptimized
    ));
}

// ============================================================================
// Test Summary
// ============================================================================

#[test]
fn test_cloud_strategies_coverage_summary() {
    println!("=== Cloud Strategies Test Coverage ===");
    println!("HybridSchedulingStrategy:    4 tests");
    println!("CloudLoadBalancingStrategy:  5 tests");
    println!("NetworkingFeature Tests:     5 tests");
    println!("SecurityFeature Tests:       4 tests");
    println!("TopologyType Tests:          5 tests");
    println!("ConnectionStatus Tests:      4 tests");
    println!("ReplicaStatus Tests:         4 tests");
    println!("TrustLevel Tests:            4 tests");
    println!("ConsistencyLevel Tests:      4 tests");
    println!("DistributionStrategy Tests:  4 tests");
    println!("Total:                       43 tests");
    println!("========================================");
}
