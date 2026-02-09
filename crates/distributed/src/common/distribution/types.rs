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
    pub started_at: chrono::DateTime<chrono::Utc>,
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
