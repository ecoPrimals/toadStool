// SPDX-License-Identifier: AGPL-3.0-only
//! Resource optimizer types

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Optimization suggestions for an execution graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestions {
    pub graph_id: String,
    pub bottlenecks: Vec<Bottleneck>,
    pub opportunities: Vec<Opportunity>,
    pub estimated_improvement: ImprovementEstimate,
    pub priority_order: Vec<String>,
}

/// A performance bottleneck in the graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bottleneck {
    pub bottleneck_type: BottleneckType,
    pub affected_nodes: Vec<String>,
    pub severity: f32,
    pub description: String,
    pub time_impact_secs: u64,
}

/// Type of bottleneck
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BottleneckType {
    SequentialExecution,
    ResourceContention,
    InefficientAllocation,
    LongCriticalPath,
    MemoryBottleneck,
    GpuUnderutilization,
}

/// An optimization opportunity
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Opportunity {
    pub opportunity_type: OpportunityType,
    pub affected_nodes: Vec<String>,
    pub benefit: f32,
    pub description: String,
    pub recommendation: String,
    pub time_savings_secs: u64,
    pub resource_savings: HashMap<String, u64>,
}

/// Type of optimization opportunity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OpportunityType {
    Parallelization,
    GpuAcceleration,
    MemoryStreaming,
    Batching,
    Caching,
    Reordering,
    NodeSplitting,
    NodeMerging,
}

/// Estimated improvement from optimizations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImprovementEstimate {
    pub current_duration_secs: u64,
    pub optimized_duration_secs: u64,
    pub time_savings_secs: u64,
    pub speedup_factor: f32,
    pub current_resources: HashMap<String, u64>,
    pub optimized_resources: HashMap<String, u64>,
}
