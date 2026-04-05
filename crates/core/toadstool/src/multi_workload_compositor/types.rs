// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

use crate::composition_constraints::{CompositionRequest, ConstraintEvaluation};

/// Composition plan
///
/// The result of composing multiple workloads.
#[derive(Debug, Clone)]
pub struct CompositionPlan {
    /// Placement for each workload
    pub placements: Vec<WorkloadPlacement>,

    /// Detected conflicts
    pub conflicts: Vec<WorkloadConflict>,

    /// Can all workloads run?
    pub overall_feasibility: bool,

    /// Resource utilization summary
    pub resource_utilization: ResourceUtilization,
}

/// Workload placement
///
/// Describes how a single workload should be placed.
#[derive(Debug, Clone)]
pub struct WorkloadPlacement {
    /// Original request
    pub request: CompositionRequest,

    /// Constraint evaluation results
    pub evaluation: ConstraintEvaluation,

    /// Is this placement feasible?
    pub is_feasible: bool,

    /// Overall satisfaction score (0.0-1.0)
    pub score: f64,

    /// Resources allocated
    pub allocated_resources: AllocatedResources,
}

/// Allocated resources for a workload
#[derive(Debug, Clone, Default)]
pub struct AllocatedResources {
    /// GPU allocation (0.0-1.0, None if not allocated)
    pub gpu_allocation: Option<f64>,

    /// Memory in GB
    pub memory_gb: Option<f64>,

    /// CPU cores
    pub cpu_cores: Option<usize>,

    /// Bandwidth in Gbps
    pub bandwidth_gbps: Option<f64>,
}

/// Workload conflict
///
/// Describes why a workload cannot be placed.
#[derive(Debug, Clone)]
pub struct WorkloadConflict {
    /// Workload that cannot be placed
    pub workload: String,

    /// Reason for conflict
    pub reason: String,

    /// Other workloads involved in conflict
    pub conflicting_workloads: Vec<String>,

    /// Suggested resolution
    pub resolution: ConflictResolution,
}

/// Conflict resolution strategy
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConflictResolution {
    /// Insufficient resources (no resolution possible)
    InsufficientResources,

    /// Higher-priority workload has resources (preemption needed)
    PriorityPreemption,

    /// Could work with degraded performance
    DegradedPerformance,

    /// Move to cloud/different layer
    AlternativePlacement,
}

/// Resource utilization summary
#[derive(Debug, Clone, Default)]
pub struct ResourceUtilization {
    /// GPU used (0.0-1.0)
    pub gpu_used: f64,

    /// GPU total available
    pub gpu_total: f64,

    /// Memory used (GB)
    pub memory_gb_used: f64,

    /// Memory total (GB)
    pub memory_gb_total: f64,

    /// CPU cores used
    pub cpu_cores_used: usize,

    /// CPU cores total
    pub cpu_cores_total: usize,

    /// Bandwidth used (Gbps)
    pub bandwidth_gbps_used: f64,
}
