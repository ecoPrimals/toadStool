// SPDX-License-Identifier: AGPL-3.0-or-later
//! Resource optimizer types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Optimization suggestions for an execution graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestions {
    /// Graph identifier.
    pub graph_id: String,
    /// Identified bottlenecks.
    pub bottlenecks: Vec<Bottleneck>,
    /// Optimization opportunities.
    pub opportunities: Vec<Opportunity>,
    /// Estimated improvement metrics.
    pub estimated_improvement: ImprovementEstimate,
    /// Recommended priority order for addressing issues.
    pub priority_order: Vec<String>,
}

/// A performance bottleneck in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    /// Bottleneck type.
    pub bottleneck_type: BottleneckType,
    /// Affected node IDs.
    pub affected_nodes: Vec<String>,
    /// Severity score (0-1).
    pub severity: f32,
    /// Human-readable description.
    pub description: String,
    /// Estimated time impact in seconds.
    pub time_impact_secs: u64,
}

/// Type of bottleneck
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottleneckType {
    /// Sequential execution limiting parallelism.
    SequentialExecution,
    /// Resource contention.
    ResourceContention,
    /// Inefficient resource allocation.
    InefficientAllocation,
    /// Long critical path.
    LongCriticalPath,
    /// Memory bottleneck.
    MemoryBottleneck,
    /// GPU underutilization.
    GpuUnderutilization,
}

/// An optimization opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    /// Opportunity type.
    pub opportunity_type: OpportunityType,
    /// Affected node IDs.
    pub affected_nodes: Vec<String>,
    /// Benefit score (0-1).
    pub benefit: f32,
    /// Human-readable description.
    pub description: String,
    /// Recommended action.
    pub recommendation: String,
    /// Estimated time savings in seconds.
    pub time_savings_secs: u64,
    /// Resource savings by type.
    pub resource_savings: HashMap<String, u64>,
}

/// Type of optimization opportunity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpportunityType {
    /// Parallelize execution.
    Parallelization,
    /// Use GPU acceleration.
    GpuAcceleration,
    /// Use memory streaming.
    MemoryStreaming,
    /// Batch operations.
    Batching,
    /// Add caching.
    Caching,
    /// Reorder operations.
    Reordering,
    /// Split nodes.
    NodeSplitting,
    /// Merge nodes.
    NodeMerging,
}

/// Estimated improvement from optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementEstimate {
    /// Current duration in seconds.
    pub current_duration_secs: u64,
    /// Optimized duration in seconds.
    pub optimized_duration_secs: u64,
    /// Time savings in seconds.
    pub time_savings_secs: u64,
    /// Speedup factor.
    pub speedup_factor: f32,
    /// Current resource usage.
    pub current_resources: HashMap<String, u64>,
    /// Optimized resource usage.
    pub optimized_resources: HashMap<String, u64>,
}
