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
    use std::collections::HashMap;

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
    fn test_constraint_factory_methods() {
        assert!(Constraint::max_latency_ms(16).is_hard());
        assert!(Constraint::preferred_latency_ms(16).is_soft());
        assert!(Constraint::min_bandwidth_gbps(10.0).is_hard());
        assert!(Constraint::min_memory_gb(8.0).is_hard());
        assert!(Constraint::min_cpu_cores(4).is_hard());
        assert!(Constraint::must_be_local().is_hard());
        assert!(Constraint::prefer_local().is_soft());
        assert!(Constraint::requires_capability("cuda").is_hard());
        assert!(Constraint::prefers_capability("akida").is_soft());

        assert_eq!(Constraint::min_memory_gb(16.0).name(), "min_memory_gb");
        assert_eq!(Constraint::min_cpu_cores(8).name(), "min_cpu_cores");
        assert_eq!(
            Constraint::requires_capability("fp16").name(),
            "requires_capability"
        );
    }

    #[test]
    fn test_constraint_is_hard_soft_all_variants() {
        // Hard constraints
        assert!(Constraint::RequiresGPU.is_hard());
        assert!(Constraint::MinMemoryGB(8.0).is_hard());
        assert!(Constraint::MinCPUCores(4).is_hard());
        assert!(Constraint::MaxLatencyMs(100).is_hard());
        assert!(Constraint::MinBandwidthGbps(10.0).is_hard());
        assert!(Constraint::RequiresCapability("cuda".into()).is_hard());
        assert!(Constraint::MustBeLocal.is_hard());
        assert!(Constraint::RequiresLayer("edge".into()).is_hard());
        assert!(Constraint::RequiresPersistentStorage.is_hard());
        assert!(Constraint::MaxCostPerHour(1.5).is_hard());
        assert!(Constraint::Custom {
            name: "x".into(),
            hard: true,
            value: "v".into(),
        }
        .is_hard());

        // Soft constraints
        assert!(Constraint::PrefersGPU.is_soft());
        assert!(Constraint::PreferredLatencyMs(16).is_soft());
        assert!(Constraint::PreferredBandwidthGbps(5.0).is_soft());
        assert!(Constraint::PrefersCapability("fp16".into()).is_soft());
        assert!(Constraint::PreferLocal.is_soft());
        assert!(Constraint::PrefersLayer("cloud".into()).is_soft());
        assert!(Constraint::MinimizeCost.is_soft());
        assert!(Constraint::Custom {
            name: "x".into(),
            hard: false,
            value: "v".into(),
        }
        .is_soft());

        assert!(!Constraint::PrefersGPU.is_hard());
        assert!(!Constraint::RequiresGPU.is_soft());
    }

    #[test]
    fn test_constraint_name_all_variants() {
        assert_eq!(Constraint::RequiresGPU.name(), "requires_gpu");
        assert_eq!(Constraint::PrefersGPU.name(), "prefers_gpu");
        assert_eq!(Constraint::MinMemoryGB(8.0).name(), "min_memory_gb");
        assert_eq!(Constraint::MinCPUCores(4).name(), "min_cpu_cores");
        assert_eq!(Constraint::MaxLatencyMs(100).name(), "max_latency_ms");
        assert_eq!(
            Constraint::PreferredLatencyMs(16).name(),
            "preferred_latency_ms"
        );
        assert_eq!(
            Constraint::MinBandwidthGbps(10.0).name(),
            "min_bandwidth_gbps"
        );
        assert_eq!(
            Constraint::PreferredBandwidthGbps(5.0).name(),
            "preferred_bandwidth_gbps"
        );
        assert_eq!(
            Constraint::RequiresCapability("cuda".into()).name(),
            "requires_capability"
        );
        assert_eq!(
            Constraint::PrefersCapability("akida".into()).name(),
            "prefers_capability"
        );
        assert_eq!(Constraint::MustBeLocal.name(), "must_be_local");
        assert_eq!(Constraint::PreferLocal.name(), "prefer_local");
        assert_eq!(
            Constraint::RequiresLayer("edge".into()).name(),
            "requires_layer"
        );
        assert_eq!(
            Constraint::PrefersLayer("cloud".into()).name(),
            "prefers_layer"
        );
        assert_eq!(
            Constraint::RequiresPersistentStorage.name(),
            "requires_persistent_storage"
        );
        assert_eq!(Constraint::MaxCostPerHour(1.5).name(), "max_cost_per_hour");
        assert_eq!(Constraint::MinimizeCost.name(), "minimize_cost");
        assert_eq!(
            Constraint::Custom {
                name: "my_custom".into(),
                hard: false,
                value: "val".into(),
            }
            .name(),
            "my_custom"
        );
    }

    #[test]
    fn test_constraint_display() {
        let c = Constraint::max_latency_ms(100);
        assert_eq!(format!("{}", c), "MaxLatency: 100ms [HARD]");
    }

    #[test]
    fn test_constraint_display_all_variants() {
        assert_eq!(format!("{}", Constraint::RequiresGPU), "RequiresGPU [HARD]");
        assert_eq!(format!("{}", Constraint::PrefersGPU), "PrefersGPU [SOFT]");
        assert_eq!(
            format!("{}", Constraint::MinMemoryGB(16.0)),
            "MinMemory: 16GB [HARD]"
        );
        assert_eq!(
            format!("{}", Constraint::MinCPUCores(8)),
            "MinCPU: 8 cores [HARD]"
        );
        assert_eq!(
            format!("{}", Constraint::PreferredLatencyMs(16)),
            "PreferredLatency: 16ms [SOFT]"
        );
        assert_eq!(
            format!("{}", Constraint::MinBandwidthGbps(10.0)),
            "MinBandwidth: 10Gbps [HARD]"
        );
        assert_eq!(
            format!("{}", Constraint::PreferredBandwidthGbps(5.0)),
            "PreferredBandwidth: 5Gbps [SOFT]"
        );
        assert_eq!(
            format!("{}", Constraint::RequiresCapability("cuda".into())),
            "RequiresCap: cuda [HARD]"
        );
        assert_eq!(
            format!("{}", Constraint::PrefersCapability("akida".into())),
            "PrefersCap: akida [SOFT]"
        );
        assert_eq!(format!("{}", Constraint::MustBeLocal), "MustBeLocal [HARD]");
        assert_eq!(format!("{}", Constraint::PreferLocal), "PreferLocal [SOFT]");
        assert_eq!(
            format!("{}", Constraint::RequiresLayer("edge".into())),
            "RequiresLayer: edge [HARD]"
        );
        assert_eq!(
            format!("{}", Constraint::PrefersLayer("cloud".into())),
            "PrefersLayer: cloud [SOFT]"
        );
        assert_eq!(
            format!("{}", Constraint::RequiresPersistentStorage),
            "RequiresPersistentStorage [HARD]"
        );
        assert_eq!(
            format!("{}", Constraint::MaxCostPerHour(2.5)),
            "MaxCost: $2.5/hr [HARD]"
        );
        assert_eq!(
            format!("{}", Constraint::MinimizeCost),
            "MinimizeCost [SOFT]"
        );
        assert_eq!(
            format!(
                "{}",
                Constraint::Custom {
                    name: "custom".into(),
                    hard: true,
                    value: "123".into(),
                }
            ),
            "Custom(custom=123)[HARD]"
        );
        assert_eq!(
            format!(
                "{}",
                Constraint::Custom {
                    name: "soft_custom".into(),
                    hard: false,
                    value: "x".into(),
                }
            ),
            "Custom(soft_custom=x)[SOFT]"
        );
    }

    #[test]
    fn test_priority_display_and_default() {
        assert_eq!(format!("{}", ConstraintPriority::Background), "Background");
        assert_eq!(format!("{}", ConstraintPriority::Normal), "Normal");
        assert_eq!(format!("{}", ConstraintPriority::High), "High");
        assert_eq!(format!("{}", ConstraintPriority::Critical), "Critical");
        assert_eq!(ConstraintPriority::default(), ConstraintPriority::Normal);
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
    fn test_composition_request_empty() {
        let request = CompositionRequest::new("empty");
        assert_eq!(request.name, "empty");
        assert!(request.constraints.is_empty());
        assert_eq!(request.priority, ConstraintPriority::Normal);
        assert!(request.metadata.is_empty());
        let (hard, soft) = request.constraint_count();
        assert_eq!(hard, 0);
        assert_eq!(soft, 0);
        assert!(request.hard_constraints().is_empty());
        assert!(request.soft_constraints().is_empty());
    }

    #[test]
    fn test_composition_request_all_hard_constraints() {
        let request = CompositionRequest::new("all_hard")
            .with_constraint(Constraint::requires_gpu())
            .with_constraint(Constraint::min_memory_gb(8.0))
            .with_constraint(Constraint::max_latency_ms(100));
        assert_eq!(request.hard_constraints().len(), 3);
        assert!(request.soft_constraints().is_empty());
        let (hard, soft) = request.constraint_count();
        assert_eq!(hard, 3);
        assert_eq!(soft, 0);
    }

    #[test]
    fn test_composition_request_all_soft_constraints() {
        let request = CompositionRequest::new("all_soft")
            .with_constraint(Constraint::prefers_gpu())
            .with_constraint(Constraint::prefer_local())
            .with_constraint(Constraint::MinimizeCost);
        assert!(request.hard_constraints().is_empty());
        assert_eq!(request.soft_constraints().len(), 3);
        let (hard, soft) = request.constraint_count();
        assert_eq!(hard, 0);
        assert_eq!(soft, 3);
    }

    #[test]
    fn test_composition_request_display() {
        let request = CompositionRequest::new("gaming")
            .with_constraint(Constraint::requires_gpu())
            .with_constraint(Constraint::prefer_local());
        let s = format!("{}", request);
        assert!(s.contains("gaming"));
        assert!(s.contains("Normal"));
        assert!(s.contains("1 hard"));
        assert!(s.contains("1 soft"));
    }

    #[test]
    fn test_composition_request_chained_builder() {
        let request = CompositionRequest::new("chained")
            .with_constraint(Constraint::requires_gpu())
            .with_constraint(Constraint::min_bandwidth_gbps(10.0))
            .with_constraint(Constraint::preferred_latency_ms(16))
            .with_priority(ConstraintPriority::Critical)
            .with_metadata("k1", "v1")
            .with_metadata("k2", "v2");
        assert_eq!(request.name, "chained");
        assert_eq!(request.constraints.len(), 3);
        assert_eq!(request.priority, ConstraintPriority::Critical);
        assert_eq!(request.metadata.get("k1"), Some(&"v1".to_string()));
        assert_eq!(request.metadata.get("k2"), Some(&"v2".to_string()));
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
    fn test_constraint_satisfaction_edge_cases() {
        let partial_zero = ConstraintSatisfaction::Partial(0.0);
        assert_eq!(partial_zero.score(), 0.0);
        assert!(partial_zero.is_satisfied());
        assert!(!partial_zero.is_fully_satisfied());

        let partial_one = ConstraintSatisfaction::Partial(1.0);
        assert_eq!(partial_one.score(), 1.0);
        assert!(partial_one.is_satisfied());
        assert!(!partial_one.is_fully_satisfied());

        let unsatisfied_with_reason = ConstraintSatisfaction::Unsatisfied {
            reason: "GPU unavailable".to_string(),
        };
        assert!(!unsatisfied_with_reason.is_satisfied());
        assert!(!unsatisfied_with_reason.is_fully_satisfied());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(ConstraintPriority::Critical > ConstraintPriority::High);
        assert!(ConstraintPriority::High > ConstraintPriority::Normal);
        assert!(ConstraintPriority::Normal > ConstraintPriority::Background);
    }

    #[test]
    fn test_priority_equality() {
        assert_eq!(ConstraintPriority::Normal, ConstraintPriority::Normal);
        assert_ne!(ConstraintPriority::High, ConstraintPriority::Normal);
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

    #[test]
    fn test_custom_constraint_soft() {
        let custom = Constraint::Custom {
            name: "prefer_fp16".to_string(),
            hard: false,
            value: "true".to_string(),
        };
        assert!(custom.is_soft());
        assert_eq!(custom.name(), "prefer_fp16");
    }

    #[test]
    fn test_constraint_evaluation_get_satisfaction() {
        let request = CompositionRequest::new("eval")
            .with_constraint(Constraint::requires_gpu())
            .with_constraint(Constraint::prefer_local());
        let mut results = HashMap::new();
        results.insert(
            "requires_gpu".to_string(),
            ConstraintSatisfaction::Satisfied,
        );
        results.insert(
            "prefer_local".to_string(),
            ConstraintSatisfaction::Partial(0.5),
        );

        let eval = ConstraintEvaluation {
            request: request.clone(),
            results: results.clone(),
            overall_score: 0.75,
            is_feasible: true,
        };

        assert_eq!(
            eval.get_satisfaction("requires_gpu"),
            Some(&ConstraintSatisfaction::Satisfied)
        );
        assert_eq!(
            eval.get_satisfaction("prefer_local"),
            Some(&ConstraintSatisfaction::Partial(0.5))
        );
        assert_eq!(eval.get_satisfaction("nonexistent"), None);
    }

    #[test]
    fn test_constraint_evaluation_unsatisfied_hard_constraints() {
        let request = CompositionRequest::new("eval")
            .with_constraint(Constraint::requires_gpu())
            .with_constraint(Constraint::min_memory_gb(8.0));
        let mut results = HashMap::new();
        results.insert(
            "requires_gpu".to_string(),
            ConstraintSatisfaction::Satisfied,
        );
        results.insert(
            "min_memory_gb".to_string(),
            ConstraintSatisfaction::Unsatisfied {
                reason: "only 4GB".to_string(),
            },
        );

        let eval = ConstraintEvaluation {
            request,
            results,
            overall_score: 0.5,
            is_feasible: false,
        };

        let unsatisfied = eval.unsatisfied_hard_constraints();
        assert_eq!(unsatisfied.len(), 1);
        assert_eq!(unsatisfied[0].0, "min_memory_gb");
    }

    #[test]
    fn test_constraint_evaluation_soft_constraint_score() {
        let request = CompositionRequest::new("eval")
            .with_constraint(Constraint::prefers_gpu())
            .with_constraint(Constraint::prefer_local());
        let mut results = HashMap::new();
        results.insert("prefers_gpu".to_string(), ConstraintSatisfaction::Satisfied);
        results.insert(
            "prefer_local".to_string(),
            ConstraintSatisfaction::Partial(0.6),
        );

        let eval = ConstraintEvaluation {
            request,
            results,
            overall_score: 0.8,
            is_feasible: true,
        };

        let soft_score = eval.soft_constraint_score();
        assert!((soft_score - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_constraint_evaluation_soft_constraint_score_empty() {
        let request =
            CompositionRequest::new("no_soft").with_constraint(Constraint::requires_gpu());
        let mut results = HashMap::new();
        results.insert(
            "requires_gpu".to_string(),
            ConstraintSatisfaction::Satisfied,
        );

        let eval = ConstraintEvaluation {
            request,
            results,
            overall_score: 1.0,
            is_feasible: true,
        };

        assert_eq!(eval.soft_constraint_score(), 1.0);
    }

    #[test]
    fn test_constraint_evaluation_soft_constraint_score_missing_results() {
        let request = CompositionRequest::new("eval")
            .with_constraint(Constraint::prefers_gpu())
            .with_constraint(Constraint::prefer_local());
        let mut results = HashMap::new();
        results.insert(
            "prefers_gpu".to_string(),
            ConstraintSatisfaction::Partial(0.5),
        );
        results.insert(
            "prefer_local".to_string(),
            ConstraintSatisfaction::Partial(0.5),
        );

        let eval = ConstraintEvaluation {
            request,
            results,
            overall_score: 0.5,
            is_feasible: true,
        };

        let soft_score = eval.soft_constraint_score();
        assert!((soft_score - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_serialization_constraint_roundtrip() {
        let constraints = [
            Constraint::RequiresGPU,
            Constraint::PrefersGPU,
            Constraint::MinMemoryGB(16.0),
            Constraint::MinCPUCores(8),
            Constraint::MaxLatencyMs(16),
            Constraint::PreferredLatencyMs(33),
            Constraint::MinBandwidthGbps(10.0),
            Constraint::PreferredBandwidthGbps(5.0),
            Constraint::RequiresCapability("cuda".into()),
            Constraint::PrefersCapability("akida".into()),
            Constraint::MustBeLocal,
            Constraint::PreferLocal,
            Constraint::RequiresLayer("edge".into()),
            Constraint::PrefersLayer("cloud".into()),
            Constraint::RequiresPersistentStorage,
            Constraint::MaxCostPerHour(2.5),
            Constraint::MinimizeCost,
            Constraint::Custom {
                name: "custom".into(),
                hard: true,
                value: "val".into(),
            },
        ];

        for c in &constraints {
            let json = serde_json::to_string(c).unwrap();
            let deserialized: Constraint = serde_json::from_str(&json).unwrap();
            assert_eq!(c, &deserialized);
        }
    }

    #[test]
    fn test_serialization_priority_roundtrip() {
        let priorities = [
            ConstraintPriority::Background,
            ConstraintPriority::Normal,
            ConstraintPriority::High,
            ConstraintPriority::Critical,
        ];

        for p in &priorities {
            let json = serde_json::to_string(p).unwrap();
            let deserialized: ConstraintPriority = serde_json::from_str(&json).unwrap();
            assert_eq!(p, &deserialized);
        }
    }

    #[test]
    fn test_serialization_composition_request_roundtrip() {
        let request = CompositionRequest::new("gaming")
            .with_constraint(Constraint::requires_gpu())
            .with_constraint(Constraint::max_latency_ms(16))
            .with_constraint(Constraint::prefer_local())
            .with_priority(ConstraintPriority::Critical)
            .with_metadata("workload_type", "gaming")
            .with_metadata("version", "1.0");

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CompositionRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.name, deserialized.name);
        assert_eq!(request.constraints.len(), deserialized.constraints.len());
        assert_eq!(request.priority, deserialized.priority);
        assert_eq!(request.metadata, deserialized.metadata);
        for (a, b) in request
            .constraints
            .iter()
            .zip(deserialized.constraints.iter())
        {
            assert_eq!(a, b);
        }
    }

    #[test]
    fn test_constraint_clone_eq() {
        let c = Constraint::requires_gpu();
        let c2 = c.clone();
        assert_eq!(c, c2);

        let c3 = Constraint::MinMemoryGB(8.0);
        assert_ne!(c, c3);
    }

    #[test]
    fn test_composition_request_with_string_name() {
        let request = CompositionRequest::new("my_workload".to_string());
        assert_eq!(request.name, "my_workload");
    }

    #[test]
    fn test_composition_request_with_str_name() {
        let request = CompositionRequest::new("str_name");
        assert_eq!(request.name, "str_name");
    }
}
