// SPDX-License-Identifier: AGPL-3.0-or-later
//! Common Distribution Types
//!
//! Generic distribution abstractions used across Songbird, Cloud, and other distributed systems.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Generic distribution strategy
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub enum DistributionStrategy {
    /// Execute on a single node/provider
    #[default]
    Single,
    /// Distribute equally across all available nodes/providers
    Equal,
    /// Distribute based on weighted allocation (weights can be percentages 0.0-1.0 or absolute values)
    Weighted { weights: HashMap<String, f64> },
    /// Optimize for cost (cheapest nodes/providers first)
    CostOptimized,
    /// Optimize for performance (fastest nodes/providers first)
    PerformanceOptimized,
    /// Optimize for latency (closest nodes/providers first)
    LatencyOptimized,
    /// Regional affinity (prefer specific regions)
    RegionalAffinity { preferred_regions: Vec<String> },
    /// Replicate across multiple nodes for redundancy
    Replicated { replication_factor: u32 },
    /// Burst to additional resources when primary is saturated
    Burst {
        primary_target: String,
        burst_targets: Vec<String>,
    },
}

/// Distribution plan for a workload
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionPlan {
    /// Unique identifier for this plan
    pub plan_id: Uuid,
    /// Strategy to use
    pub strategy: DistributionStrategy,
    /// Target nodes/providers for distribution
    pub targets: Vec<DistributionTarget>,
    /// Total work units to distribute
    pub total_units: u64,
    /// Expected completion time (seconds)
    pub estimated_duration_secs: u64,
}

/// A target for distribution (node, cloud provider, edge device, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionTarget {
    /// Target identifier (node ID, provider name, etc.)
    pub id: String,
    /// Target type (for logging/monitoring)
    pub target_type: TargetType,
    /// Allocated work units for this target
    pub allocated_units: u64,
    /// Weight/priority (higher = more work)
    pub weight: f64,
    /// Estimated capacity
    pub capacity: ResourceCapacity,
}

/// Type of distribution target
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TargetType {
    /// Local node
    LocalNode,
    /// Remote node via Songbird
    SongbirdNode,
    /// Cloud provider
    CloudProvider,
    /// Kubernetes cluster
    Kubernetes,
    /// Edge/IoT device
    EdgeDevice,
    /// Self-hosted infrastructure
    SelfHosted,
}

/// Resource capacity for a distribution target
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCapacity {
    /// CPU cores available
    pub cpu_cores: f64,
    /// Memory available (bytes)
    pub memory_bytes: u64,
    /// Storage available (bytes)
    pub storage_bytes: u64,
    /// Network bandwidth (bytes/sec)
    pub network_bandwidth_bps: u64,
    /// GPU units available (if applicable)
    pub gpu_units: Option<u32>,
}

/// Distribution algorithm
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DistributionAlgorithm {
    /// Round-robin distribution
    RoundRobin,
    /// Least-loaded target first
    LeastLoaded,
    /// Load-based distribution (considers current load)
    LoadBased,
    /// Weighted round-robin
    WeightedRoundRobin,
    /// Random selection
    Random,
    /// Consistent hashing
    ConsistentHashing,
    /// Power of two choices
    PowerOfTwoChoices,
    /// Match based on target capabilities
    CapabilityMatched,
    /// Optimize based on geographic proximity
    GeographicOptimized,
    /// Custom algorithm (identified by name)
    Custom(String),
}

/// Distribution result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionResult {
    /// Original request ID
    pub request_id: Uuid,
    /// Distribution plan used
    pub plan: DistributionPlan,
    /// Actual targets used
    pub targets_used: Vec<String>,
    /// Start time
    #[serde(with = "toadstool_common::system_time_serde")]
    pub started_at: std::time::SystemTime,
    /// Success status
    pub success: bool,
    /// Error message (if failed)
    pub error: Option<String>,
}

/// Distribution configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistributionConfig {
    /// Default strategy
    pub default_strategy: DistributionStrategy,
    /// Maximum number of targets to use
    pub max_targets: usize,
    /// Minimum work units per target (don't split too small)
    pub min_units_per_target: u64,
    /// Enable automatic rebalancing
    pub auto_rebalance: bool,
    /// Rebalance threshold (percentage of imbalance before rebalancing)
    pub rebalance_threshold_percent: f64,
}

impl Default for DistributionConfig {
    fn default() -> Self {
        Self {
            default_strategy: DistributionStrategy::Single,
            max_targets: 100,
            min_units_per_target: 1,
            auto_rebalance: true,
            rebalance_threshold_percent: 20.0,
        }
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_distribution_strategy_default() {
        let s = DistributionStrategy::default();
        assert!(matches!(s, DistributionStrategy::Single));
    }

    #[test]
    fn test_distribution_strategy_variants() {
        let _equal = DistributionStrategy::Equal;
        let _weighted = DistributionStrategy::Weighted {
            weights: {
                let mut m = HashMap::new();
                m.insert("node1".to_string(), 0.6);
                m.insert("node2".to_string(), 0.4);
                m
            },
        };
        let _replicated = DistributionStrategy::Replicated {
            replication_factor: 3,
        };
        let _burst = DistributionStrategy::Burst {
            primary_target: "primary".to_string(),
            burst_targets: vec!["burst1".to_string()],
        };
    }

    #[test]
    fn test_distribution_config_default() {
        let config = DistributionConfig::default();
        assert!(matches!(
            config.default_strategy,
            DistributionStrategy::Single
        ));
        assert_eq!(config.max_targets, 100);
        assert_eq!(config.min_units_per_target, 1);
        assert!(config.auto_rebalance);
        assert!((config.rebalance_threshold_percent - 20.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_distribution_plan_construction() {
        let plan = DistributionPlan {
            plan_id: Uuid::new_v4(),
            strategy: DistributionStrategy::Equal,
            targets: vec![DistributionTarget {
                id: "node-1".to_string(),
                target_type: TargetType::LocalNode,
                allocated_units: 50,
                weight: 1.0,
                capacity: ResourceCapacity {
                    cpu_cores: 4.0,
                    memory_bytes: 8 * 1024 * 1024 * 1024,
                    storage_bytes: 100 * 1024 * 1024 * 1024,
                    network_bandwidth_bps: 1_000_000_000,
                    gpu_units: None,
                },
            }],
            total_units: 100,
            estimated_duration_secs: 60,
        };
        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.total_units, 100);
        assert_eq!(plan.targets[0].id, "node-1");
    }

    #[test]
    fn test_target_type_variants() {
        assert!(TargetType::LocalNode != TargetType::CloudProvider);
        assert_eq!(TargetType::SongbirdNode, TargetType::SongbirdNode);
    }

    #[test]
    fn test_distribution_algorithm_variants() {
        assert_eq!(
            DistributionAlgorithm::RoundRobin,
            DistributionAlgorithm::RoundRobin
        );
        let custom = DistributionAlgorithm::Custom("my_algo".to_string());
        assert_eq!(custom, DistributionAlgorithm::Custom("my_algo".to_string()));
    }

    #[test]
    fn test_distribution_strategy_serialization_roundtrip() {
        let strategy = DistributionStrategy::Replicated {
            replication_factor: 5,
        };
        let json = serde_json::to_string(&strategy).unwrap();
        let parsed: DistributionStrategy = serde_json::from_str(&json).unwrap();
        assert_eq!(strategy, parsed);
    }

    #[test]
    fn test_distribution_config_serialization_roundtrip() {
        let config = DistributionConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let parsed: DistributionConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(config.default_strategy, parsed.default_strategy);
        assert_eq!(config.max_targets, parsed.max_targets);
        assert_eq!(config.auto_rebalance, parsed.auto_rebalance);
    }

    #[test]
    fn test_resource_capacity_serialization_roundtrip() {
        let cap = ResourceCapacity {
            cpu_cores: 8.0,
            memory_bytes: 16 * 1024 * 1024 * 1024,
            storage_bytes: 500 * 1024 * 1024 * 1024,
            network_bandwidth_bps: 10_000_000_000,
            gpu_units: Some(2),
        };
        let json = serde_json::to_string(&cap).unwrap();
        let parsed: ResourceCapacity = serde_json::from_str(&json).unwrap();
        assert_eq!(cap.cpu_cores, parsed.cpu_cores);
        assert_eq!(cap.gpu_units, parsed.gpu_units);
    }
}
