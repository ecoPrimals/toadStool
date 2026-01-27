// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: Apache-2.0

//! Composition Constraint System
//!
//! This module implements constraint-based dynamic workload composition.
//! It enables "impossible" workload stacks by evaluating hard and soft
//! constraints across multiple dimensions.
//!
//! # Philosophy
//!
//! **Constraint Over Prescription**: We describe what we NEED, not HOW to achieve it.
//! The composition engine figures out HOW based on available capabilities.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::composition_constraints::{Constraint, ConstraintPriority, CompositionRequest};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Gaming workload: GPU required, low latency critical
//! let gaming = CompositionRequest::new("gaming")
//!     .with_constraint(Constraint::requires_gpu())
//!     .with_constraint(Constraint::max_latency_ms(16)) // 60 FPS
//!     .with_priority(ConstraintPriority::Critical);
//!
//! // OpenFold: GPU preferred, high bandwidth needed
//! let openfold = CompositionRequest::new("openfold")
//!     .with_constraint(Constraint::prefers_gpu())
//!     .with_constraint(Constraint::min_bandwidth_gbps(10.0))
//!     .with_priority(ConstraintPriority::High);
//!
//! // Compose both simultaneously
//! // Engine will figure out if/how this is possible
//! # Ok(())
//! # }
//! ```

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A composition constraint
///
/// Constraints are declarative requirements. They describe WHAT is needed,
/// not HOW to achieve it. The composition engine figures out HOW.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Constraint {
    /// Hard constraint: GPU required (workload fails without it)
    RequiresGPU,

    /// Soft constraint: GPU preferred (workload works without it, but slower)
    PrefersGPU,

    /// Hard constraint: Minimum memory (GB)
    MinMemoryGB(f64),

    /// Hard constraint: Minimum CPU cores
    MinCPUCores(usize),

    /// Hard constraint: Maximum latency (milliseconds)
    MaxLatencyMs(u64),

    /// Soft constraint: Preferred latency (milliseconds)
    PreferredLatencyMs(u64),

    /// Hard constraint: Minimum bandwidth (Gbps)
    MinBandwidthGbps(f64),

    /// Soft constraint: Preferred bandwidth (Gbps)
    PreferredBandwidthGbps(f64),

    /// Hard constraint: Requires specific capability
    RequiresCapability(String),

    /// Soft constraint: Prefers specific capability
    PrefersCapability(String),

    /// Hard constraint: Must run locally (no cloud)
    MustBeLocal,

    /// Soft constraint: Prefer local (but cloud OK)
    PreferLocal,

    /// Hard constraint: Must use specific deployment layer
    RequiresLayer(String),

    /// Soft constraint: Prefer specific deployment layer
    PrefersLayer(String),

    /// Hard constraint: Requires persistent storage
    RequiresPersistentStorage,

    /// Hard constraint: Maximum cost per hour (dollars)
    MaxCostPerHour(f64),

    /// Soft constraint: Minimize cost
    MinimizeCost,

    /// Custom constraint (for extensibility)
    Custom {
        name: String,
        hard: bool,
        value: String,
    },
}

impl Constraint {
    /// Create a "requires GPU" hard constraint
    pub fn requires_gpu() -> Self {
        Self::RequiresGPU
    }

    /// Create a "prefers GPU" soft constraint
    pub fn prefers_gpu() -> Self {
        Self::PrefersGPU
    }

    /// Create a "max latency" hard constraint
    pub fn max_latency_ms(ms: u64) -> Self {
        Self::MaxLatencyMs(ms)
    }

    /// Create a "preferred latency" soft constraint
    pub fn preferred_latency_ms(ms: u64) -> Self {
        Self::PreferredLatencyMs(ms)
    }

    /// Create a "min bandwidth" hard constraint
    pub fn min_bandwidth_gbps(gbps: f64) -> Self {
        Self::MinBandwidthGbps(gbps)
    }

    /// Create a "min memory" hard constraint
    pub fn min_memory_gb(gb: f64) -> Self {
        Self::MinMemoryGB(gb)
    }

    /// Create a "min CPU cores" hard constraint
    pub fn min_cpu_cores(cores: usize) -> Self {
        Self::MinCPUCores(cores)
    }

    /// Create a "must be local" hard constraint
    pub fn must_be_local() -> Self {
        Self::MustBeLocal
    }

    /// Create a "prefer local" soft constraint
    pub fn prefer_local() -> Self {
        Self::PreferLocal
    }

    /// Create a "requires capability" hard constraint
    pub fn requires_capability(cap: impl Into<String>) -> Self {
        Self::RequiresCapability(cap.into())
    }

    /// Create a "prefers capability" soft constraint
    pub fn prefers_capability(cap: impl Into<String>) -> Self {
        Self::PrefersCapability(cap.into())
    }

    /// Check if this is a hard constraint (must be satisfied)
    pub fn is_hard(&self) -> bool {
        matches!(
            self,
            Self::RequiresGPU
                | Self::MinMemoryGB(_)
                | Self::MinCPUCores(_)
                | Self::MaxLatencyMs(_)
                | Self::MinBandwidthGbps(_)
                | Self::RequiresCapability(_)
                | Self::MustBeLocal
                | Self::RequiresLayer(_)
                | Self::RequiresPersistentStorage
                | Self::MaxCostPerHour(_)
                | Self::Custom { hard: true, .. }
        )
    }

    /// Check if this is a soft constraint (nice to have)
    pub fn is_soft(&self) -> bool {
        !self.is_hard()
    }

    /// Get constraint name for logging/debugging
    pub fn name(&self) -> &str {
        match self {
            Self::RequiresGPU => "requires_gpu",
            Self::PrefersGPU => "prefers_gpu",
            Self::MinMemoryGB(_) => "min_memory_gb",
            Self::MinCPUCores(_) => "min_cpu_cores",
            Self::MaxLatencyMs(_) => "max_latency_ms",
            Self::PreferredLatencyMs(_) => "preferred_latency_ms",
            Self::MinBandwidthGbps(_) => "min_bandwidth_gbps",
            Self::PreferredBandwidthGbps(_) => "preferred_bandwidth_gbps",
            Self::RequiresCapability(_) => "requires_capability",
            Self::PrefersCapability(_) => "prefers_capability",
            Self::MustBeLocal => "must_be_local",
            Self::PreferLocal => "prefer_local",
            Self::RequiresLayer(_) => "requires_layer",
            Self::PrefersLayer(_) => "prefers_layer",
            Self::RequiresPersistentStorage => "requires_persistent_storage",
            Self::MaxCostPerHour(_) => "max_cost_per_hour",
            Self::MinimizeCost => "minimize_cost",
            Self::Custom { name, .. } => name,
        }
    }
}

impl fmt::Display for Constraint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::RequiresGPU => write!(f, "RequiresGPU [HARD]"),
            Self::PrefersGPU => write!(f, "PrefersGPU [SOFT]"),
            Self::MinMemoryGB(gb) => write!(f, "MinMemory: {}GB [HARD]", gb),
            Self::MinCPUCores(cores) => write!(f, "MinCPU: {} cores [HARD]", cores),
            Self::MaxLatencyMs(ms) => write!(f, "MaxLatency: {}ms [HARD]", ms),
            Self::PreferredLatencyMs(ms) => write!(f, "PreferredLatency: {}ms [SOFT]", ms),
            Self::MinBandwidthGbps(gbps) => write!(f, "MinBandwidth: {}Gbps [HARD]", gbps),
            Self::PreferredBandwidthGbps(gbps) => {
                write!(f, "PreferredBandwidth: {}Gbps [SOFT]", gbps)
            }
            Self::RequiresCapability(cap) => write!(f, "RequiresCap: {} [HARD]", cap),
            Self::PrefersCapability(cap) => write!(f, "PrefersCap: {} [SOFT]", cap),
            Self::MustBeLocal => write!(f, "MustBeLocal [HARD]"),
            Self::PreferLocal => write!(f, "PreferLocal [SOFT]"),
            Self::RequiresLayer(layer) => write!(f, "RequiresLayer: {} [HARD]", layer),
            Self::PrefersLayer(layer) => write!(f, "PrefersLayer: {} [SOFT]", layer),
            Self::RequiresPersistentStorage => write!(f, "RequiresPersistentStorage [HARD]"),
            Self::MaxCostPerHour(cost) => write!(f, "MaxCost: ${}/hr [HARD]", cost),
            Self::MinimizeCost => write!(f, "MinimizeCost [SOFT]"),
            Self::Custom { name, hard, value } => {
                write!(
                    f,
                    "Custom({}={})[{}]",
                    name,
                    value,
                    if *hard { "HARD" } else { "SOFT" }
                )
            }
        }
    }
}

/// Priority level for composition requests
///
/// Higher priority workloads get preferential resource allocation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Default)]
pub enum ConstraintPriority {
    /// Background task (lowest priority)
    Background = 0,

    /// Normal priority
    #[default]
    Normal = 1,

    /// High priority (important workload)
    High = 2,

    /// Critical priority (system-critical, real-time)
    Critical = 3,
}

impl fmt::Display for ConstraintPriority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Background => write!(f, "Background"),
            Self::Normal => write!(f, "Normal"),
            Self::High => write!(f, "High"),
            Self::Critical => write!(f, "Critical"),
        }
    }
}

/// A composition request with constraints
///
/// This represents a workload that wants to be composed/placed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositionRequest {
    /// Workload name/ID
    pub name: String,

    /// List of constraints
    pub constraints: Vec<Constraint>,

    /// Priority level
    pub priority: ConstraintPriority,

    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

impl CompositionRequest {
    /// Create a new composition request
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            constraints: Vec::new(),
            priority: ConstraintPriority::default(),
            metadata: HashMap::new(),
        }
    }

    /// Add a constraint to this request
    pub fn with_constraint(mut self, constraint: Constraint) -> Self {
        self.constraints.push(constraint);
        self
    }

    /// Set priority level
    pub fn with_priority(mut self, priority: ConstraintPriority) -> Self {
        self.priority = priority;
        self
    }

    /// Add metadata
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }

    /// Get all hard constraints
    pub fn hard_constraints(&self) -> Vec<&Constraint> {
        self.constraints.iter().filter(|c| c.is_hard()).collect()
    }

    /// Get all soft constraints
    pub fn soft_constraints(&self) -> Vec<&Constraint> {
        self.constraints.iter().filter(|c| c.is_soft()).collect()
    }

    /// Count constraints by type
    pub fn constraint_count(&self) -> (usize, usize) {
        let hard = self.hard_constraints().len();
        let soft = self.soft_constraints().len();
        (hard, soft)
    }
}

impl fmt::Display for CompositionRequest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (hard, soft) = self.constraint_count();
        write!(
            f,
            "Request('{}', priority={}, constraints={} hard + {} soft)",
            self.name, self.priority, hard, soft
        )
    }
}

/// Constraint satisfaction result
///
/// Indicates whether a constraint was satisfied and by how much.
#[derive(Debug, Clone, PartialEq)]
pub enum ConstraintSatisfaction {
    /// Constraint fully satisfied
    Satisfied,

    /// Constraint partially satisfied (only for soft constraints)
    /// Value 0.0-1.0 indicates how well satisfied
    Partial(f64),

    /// Constraint not satisfied
    Unsatisfied { reason: String },
}

impl ConstraintSatisfaction {
    /// Check if satisfied (fully or partially)
    pub fn is_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied | Self::Partial(_))
    }

    /// Check if fully satisfied
    pub fn is_fully_satisfied(&self) -> bool {
        matches!(self, Self::Satisfied)
    }

    /// Get satisfaction score (0.0 = unsatisfied, 1.0 = fully satisfied)
    pub fn score(&self) -> f64 {
        match self {
            Self::Satisfied => 1.0,
            Self::Partial(s) => *s,
            Self::Unsatisfied { .. } => 0.0,
        }
    }
}

/// Constraint evaluation result for a composition request
#[derive(Debug, Clone)]
pub struct ConstraintEvaluation {
    /// Request being evaluated
    pub request: CompositionRequest,

    /// Per-constraint satisfaction results
    pub results: HashMap<String, ConstraintSatisfaction>,

    /// Overall satisfaction score (0.0-1.0)
    pub overall_score: f64,

    /// Is this request feasible? (all hard constraints satisfied)
    pub is_feasible: bool,
}

impl ConstraintEvaluation {
    /// Get satisfaction for a specific constraint
    pub fn get_satisfaction(&self, constraint_name: &str) -> Option<&ConstraintSatisfaction> {
        self.results.get(constraint_name)
    }

    /// Get all unsatisfied hard constraints
    pub fn unsatisfied_hard_constraints(&self) -> Vec<(&String, &ConstraintSatisfaction)> {
        self.results
            .iter()
            .filter(|(_, sat)| !sat.is_satisfied())
            .collect()
    }

    /// Get soft constraint satisfaction score (0.0-1.0)
    pub fn soft_constraint_score(&self) -> f64 {
        let soft_results: Vec<_> = self
            .request
            .soft_constraints()
            .iter()
            .filter_map(|c| self.results.get(c.name()))
            .collect();

        if soft_results.is_empty() {
            return 1.0; // No soft constraints = perfect score
        }

        let total_score: f64 = soft_results.iter().map(|s| s.score()).sum();
        total_score / soft_results.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constraint_creation() {
        let c1 = Constraint::requires_gpu();
        assert!(c1.is_hard());
        assert_eq!(c1.name(), "requires_gpu");

        let c2 = Constraint::prefers_gpu();
        assert!(c2.is_soft());
        assert_eq!(c2.name(), "prefers_gpu");
    }

    #[test]
    fn test_constraint_display() {
        let c = Constraint::max_latency_ms(100);
        assert_eq!(format!("{}", c), "MaxLatency: 100ms [HARD]");
    }

    #[test]
    fn test_composition_request_builder() {
        let request = CompositionRequest::new("test")
            .with_constraint(Constraint::requires_gpu())
            .with_constraint(Constraint::prefer_local())
            .with_priority(ConstraintPriority::High)
            .with_metadata("workload_type", "gaming");

        assert_eq!(request.name, "test");
        assert_eq!(request.constraints.len(), 2);
        assert_eq!(request.priority, ConstraintPriority::High);
        assert_eq!(
            request.metadata.get("workload_type"),
            Some(&"gaming".to_string())
        );

        let (hard, soft) = request.constraint_count();
        assert_eq!(hard, 1);
        assert_eq!(soft, 1);
    }

    #[test]
    fn test_constraint_satisfaction_score() {
        let satisfied = ConstraintSatisfaction::Satisfied;
        assert_eq!(satisfied.score(), 1.0);
        assert!(satisfied.is_satisfied());
        assert!(satisfied.is_fully_satisfied());

        let partial = ConstraintSatisfaction::Partial(0.7);
        assert_eq!(partial.score(), 0.7);
        assert!(partial.is_satisfied());
        assert!(!partial.is_fully_satisfied());

        let unsatisfied = ConstraintSatisfaction::Unsatisfied {
            reason: "no GPU".to_string(),
        };
        assert_eq!(unsatisfied.score(), 0.0);
        assert!(!unsatisfied.is_satisfied());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(ConstraintPriority::Critical > ConstraintPriority::High);
        assert!(ConstraintPriority::High > ConstraintPriority::Normal);
        assert!(ConstraintPriority::Normal > ConstraintPriority::Background);
    }

    #[test]
    fn test_custom_constraint() {
        let custom = Constraint::Custom {
            name: "needs_akida".to_string(),
            hard: true,
            value: "true".to_string(),
        };

        assert!(custom.is_hard());
        assert_eq!(custom.name(), "needs_akida");
    }
}
