// Copyright (C) 2024-2025 ToadStool Project
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Multi-Workload Compositor
//!
//! This module implements simultaneous composition of multiple workloads with
//! priority-based resource allocation and conflict resolution.
//!
//! # Philosophy
//!
//! **Compose, Don't Fight**: When resources are limited, use priorities and
//! soft constraints to find a harmonious composition rather than failing.
//!
//! # Example
//!
//! ```rust,no_run
//! use toadstool::multi_workload_compositor::MultiWorkloadCompositor;
//! use toadstool::composition_constraints::{Constraint, CompositionRequest, ConstraintPriority};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut compositor = MultiWorkloadCompositor::from_runtime().await?;
//!
//! // Gaming: Critical priority, needs GPU
//! let gaming = CompositionRequest::new("gaming")
//!     .with_constraint(Constraint::requires_gpu())
//!     .with_constraint(Constraint::max_latency_ms(16))
//!     .with_priority(ConstraintPriority::Critical);
//!
//! // OpenFold: High priority, prefers GPU
//! let openfold = CompositionRequest::new("openfold")
//!     .with_constraint(Constraint::prefers_gpu())
//!     .with_constraint(Constraint::min_bandwidth_gbps(10.0))
//!     .with_priority(ConstraintPriority::High);
//!
//! // Add workloads
//! compositor.add_request(gaming);
//! compositor.add_request(openfold);
//!
//! // Compose simultaneously
//! let plan = compositor.compose().await?;
//!
//! for placement in &plan.placements {
//!     println!("{}: {} (score: {})",
//!         placement.request.name,
//!         if placement.is_feasible { "✅ Feasible" } else { "❌ Infeasible" },
//!         placement.score
//!     );
//! }
//! # Ok(())
//! # }
//! ```

use crate::composition_constraints::*;
use crate::composition_engine::{CompositionEngine, EngineStats};
use crate::layer_adaptation::GpuAccess;
use crate::ToadStoolResult;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Multi-workload compositor
///
/// Composes multiple workloads simultaneously with priority-based allocation.
pub struct MultiWorkloadCompositor {
    /// Composition engine for evaluating requests
    engine: Arc<CompositionEngine>,

    /// Pending workload requests
    requests: Vec<CompositionRequest>,
}

impl MultiWorkloadCompositor {
    /// Create compositor from current runtime
    pub async fn from_runtime() -> ToadStoolResult<Self> {
        let engine = CompositionEngine::from_runtime().await?;
        Self::new(Arc::new(engine))
    }

    /// Create compositor with specific engine
    pub fn new(engine: Arc<CompositionEngine>) -> ToadStoolResult<Self> {
        Ok(Self {
            engine,
            requests: Vec::new(),
        })
    }

    /// Add a workload request
    pub fn add_request(&mut self, request: CompositionRequest) {
        info!("➕ Added workload request: {}", request);
        self.requests.push(request);
    }

    /// Add multiple workload requests
    pub fn add_requests(&mut self, requests: Vec<CompositionRequest>) {
        for request in requests {
            self.add_request(request);
        }
    }

    /// Remove all requests
    pub fn clear_requests(&mut self) {
        self.requests.clear();
    }

    /// Get number of pending requests
    pub fn request_count(&self) -> usize {
        self.requests.len()
    }

    /// Compose all pending workloads
    ///
    /// Returns a composition plan showing how each workload should be placed.
    pub async fn compose(&self) -> ToadStoolResult<CompositionPlan> {
        info!("🎼 Composing {} workloads...", self.requests.len());

        if self.requests.is_empty() {
            return Ok(CompositionPlan {
                placements: Vec::new(),
                conflicts: Vec::new(),
                overall_feasibility: true,
                resource_utilization: ResourceUtilization::default(),
            });
        }

        // Step 1: Sort by priority using indices (avoids cloning entire requests vec)
        let mut indices: Vec<usize> = (0..self.requests.len()).collect();
        indices.sort_by(|&a, &b| self.requests[b].priority.cmp(&self.requests[a].priority));

        debug!("📋 Sorted by priority:");
        for &idx in &indices {
            let req = &self.requests[idx];
            debug!("  - {} (priority: {})", req.name, req.priority);
        }

        // Step 2: Evaluate each request
        let mut placements = Vec::new();
        let mut conflicts = Vec::new();

        for &idx in &indices {
            let request = &self.requests[idx];
            let evaluation = self.engine.evaluate(request).await?;

            let placement = WorkloadPlacement {
                request: request.clone(),
                evaluation: evaluation.clone(),
                is_feasible: evaluation.is_feasible,
                score: evaluation.overall_score,
                allocated_resources: self.estimate_allocated_resources(request),
            };

            placements.push(placement);

            // Check for conflicts with higher-priority workloads
            if !evaluation.is_feasible {
                let conflict = self.detect_conflict(request, &placements);
                if let Some(c) = conflict {
                    conflicts.push(c);
                }
            }
        }

        // Step 3: Calculate overall feasibility
        let overall_feasibility = placements.iter().all(|p| p.is_feasible);

        // Step 4: Calculate resource utilization
        let resource_utilization = self.calculate_resource_utilization(&placements);

        let plan = CompositionPlan {
            placements,
            conflicts,
            overall_feasibility,
            resource_utilization,
        };

        if plan.overall_feasibility {
            info!("✅ All {} workloads are feasible!", self.requests.len());
        } else {
            warn!("⚠️  {} conflicts detected", plan.conflicts.len());
        }

        Ok(plan)
    }

    /// Estimate resources that would be allocated to a workload
    fn estimate_allocated_resources(&self, request: &CompositionRequest) -> AllocatedResources {
        let mut resources = AllocatedResources::default();

        // Check constraints to estimate allocations
        for constraint in &request.constraints {
            match constraint {
                Constraint::RequiresGPU | Constraint::PrefersGPU => {
                    resources.gpu_allocation = Some(1.0); // Full GPU
                }
                Constraint::MinMemoryGB(gb) => {
                    resources.memory_gb = Some(*gb);
                }
                Constraint::MinCPUCores(cores) => {
                    resources.cpu_cores = Some(*cores);
                }
                Constraint::MinBandwidthGbps(gbps) => {
                    resources.bandwidth_gbps = Some(*gbps);
                }
                _ => {}
            }
        }

        resources
    }

    /// Detect conflict between a failing request and existing placements
    fn detect_conflict(
        &self,
        failing_request: &CompositionRequest,
        placements: &[WorkloadPlacement],
    ) -> Option<WorkloadConflict> {
        // Find which hard constraint failed
        let hard_constraints = failing_request.hard_constraints();
        if hard_constraints.is_empty() {
            return None;
        }

        // Check if any higher-priority workload is using resources we need
        let conflicting_workloads: Vec<_> = placements
            .iter()
            .filter(|p| {
                p.is_feasible
                    && p.request.priority > failing_request.priority
                    && self.would_conflict(&p.request, failing_request)
            })
            .map(|p| p.request.name.clone())
            .collect();

        if conflicting_workloads.is_empty() {
            Some(WorkloadConflict {
                workload: failing_request.name.clone(),
                reason: "Insufficient resources available".to_string(),
                conflicting_workloads: Vec::new(),
                resolution: ConflictResolution::InsufficientResources,
            })
        } else {
            Some(WorkloadConflict {
                workload: failing_request.name.clone(),
                reason: "Resources allocated to higher-priority workloads".to_string(),
                conflicting_workloads,
                resolution: ConflictResolution::PriorityPreemption,
            })
        }
    }

    /// Check if two workloads would conflict for resources
    fn would_conflict(&self, req1: &CompositionRequest, req2: &CompositionRequest) -> bool {
        // Both require GPU
        let both_need_gpu = req1
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::RequiresGPU))
            && req2
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::RequiresGPU));

        // Add more conflict checks as needed
        both_need_gpu
    }

    /// Calculate overall resource utilization
    fn calculate_resource_utilization(
        &self,
        placements: &[WorkloadPlacement],
    ) -> ResourceUtilization {
        let mut gpu_used = 0.0;
        let mut memory_gb_used = 0.0;
        let mut cpu_cores_used = 0;
        let mut bandwidth_gbps_used = 0.0;

        for placement in placements {
            if placement.is_feasible {
                if let Some(gpu) = placement.allocated_resources.gpu_allocation {
                    gpu_used += gpu;
                }
                if let Some(mem) = placement.allocated_resources.memory_gb {
                    memory_gb_used += mem;
                }
                if let Some(cpu) = placement.allocated_resources.cpu_cores {
                    cpu_cores_used += cpu;
                }
                if let Some(bw) = placement.allocated_resources.bandwidth_gbps {
                    bandwidth_gbps_used += bw;
                }
            }
        }

        // Get total available resources
        let caps = self.engine.capabilities();
        let total_gpu = if !matches!(caps.compute.gpu_access, GpuAccess::None) {
            1.0
        } else {
            0.0
        };
        let total_memory_gb = caps
            .compute
            .memory_bytes
            .map(|b| b as f64 / 1_073_741_824.0)
            .unwrap_or(0.0);
        let cpu_total = caps.compute.cpu_cores.unwrap_or_default();

        ResourceUtilization {
            gpu_used,
            gpu_total: total_gpu,
            memory_gb_used,
            memory_gb_total: total_memory_gb,
            cpu_cores_used,
            cpu_cores_total: cpu_total,
            bandwidth_gbps_used,
        }
    }

    /// Get engine statistics
    pub async fn stats(&self) -> EngineStats {
        self.engine.stats().await
    }
}

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

impl CompositionPlan {
    /// Get feasible placements only
    pub fn feasible_placements(&self) -> Vec<&WorkloadPlacement> {
        self.placements.iter().filter(|p| p.is_feasible).collect()
    }

    /// Get infeasible placements only
    pub fn infeasible_placements(&self) -> Vec<&WorkloadPlacement> {
        self.placements.iter().filter(|p| !p.is_feasible).collect()
    }

    /// Get average satisfaction score
    pub fn average_score(&self) -> f64 {
        if self.placements.is_empty() {
            return 0.0;
        }
        let total: f64 = self.placements.iter().map(|p| p.score).sum();
        let len = self.placements.len();
        #[allow(clippy::cast_precision_loss)]
        let result = total / len as f64;
        result
    }
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

impl ResourceUtilization {
    /// Get GPU utilization percentage (0.0-100.0)
    pub fn gpu_utilization_percent(&self) -> f64 {
        if self.gpu_total == 0.0 {
            0.0
        } else {
            (self.gpu_used / self.gpu_total) * 100.0
        }
    }

    /// Get memory utilization percentage (0.0-100.0)
    pub fn memory_utilization_percent(&self) -> f64 {
        if self.memory_gb_total == 0.0 {
            0.0
        } else {
            (self.memory_gb_used / self.memory_gb_total) * 100.0
        }
    }

    /// Get CPU utilization percentage (0.0-100.0)
    pub fn cpu_utilization_percent(&self) -> f64 {
        if self.cpu_cores_total == 0 {
            0.0
        } else {
            let used = self.cpu_cores_used;
            let total = self.cpu_cores_total;
            #[allow(clippy::cast_precision_loss)]
            let pct = (used as f64 / total as f64) * 100.0;
            pct
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_compositor_initialization() {
        let result = MultiWorkloadCompositor::from_runtime().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_add_requests() {
        let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

        assert_eq!(compositor.request_count(), 0);

        let req1 = CompositionRequest::new("test1");
        compositor.add_request(req1);

        assert_eq!(compositor.request_count(), 1);

        let req2 = CompositionRequest::new("test2");
        let req3 = CompositionRequest::new("test3");
        compositor.add_requests(vec![req2, req3]);

        assert_eq!(compositor.request_count(), 3);
    }

    #[tokio::test]
    async fn test_empty_composition() {
        let compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();
        let plan = compositor.compose().await.unwrap();

        assert!(plan.overall_feasibility);
        assert_eq!(plan.placements.len(), 0);
        assert_eq!(plan.conflicts.len(), 0);
    }

    #[tokio::test]
    async fn test_single_workload_composition() {
        let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

        let request =
            CompositionRequest::new("test").with_constraint(Constraint::min_memory_gb(0.1));

        compositor.add_request(request);

        let plan = compositor.compose().await.unwrap();

        assert_eq!(plan.placements.len(), 1);
        assert!(plan.overall_feasibility);
    }

    #[tokio::test]
    async fn test_multi_workload_composition() {
        let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

        let req1 = CompositionRequest::new("workload1")
            .with_constraint(Constraint::min_memory_gb(0.1))
            .with_priority(ConstraintPriority::High);

        let req2 = CompositionRequest::new("workload2")
            .with_constraint(Constraint::min_cpu_cores(1))
            .with_priority(ConstraintPriority::Normal);

        compositor.add_request(req1);
        compositor.add_request(req2);

        let plan = compositor.compose().await.unwrap();

        assert_eq!(plan.placements.len(), 2);

        // Both should be feasible (low requirements)
        assert_eq!(plan.feasible_placements().len(), 2);
        assert!(plan.overall_feasibility);
    }

    #[tokio::test]
    async fn test_priority_ordering() {
        let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

        let critical =
            CompositionRequest::new("critical").with_priority(ConstraintPriority::Critical);

        let background =
            CompositionRequest::new("background").with_priority(ConstraintPriority::Background);

        let high = CompositionRequest::new("high").with_priority(ConstraintPriority::High);

        // Add in random order
        compositor.add_request(background);
        compositor.add_request(high);
        compositor.add_request(critical);

        let plan = compositor.compose().await.unwrap();

        // Should be evaluated in priority order (critical first)
        assert_eq!(plan.placements[0].request.name, "critical");
        assert_eq!(plan.placements[1].request.name, "high");
        assert_eq!(plan.placements[2].request.name, "background");
    }

    #[tokio::test]
    async fn test_resource_utilization() {
        let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

        let req = CompositionRequest::new("test")
            .with_constraint(Constraint::min_memory_gb(1.0))
            .with_constraint(Constraint::min_cpu_cores(2));

        compositor.add_request(req);

        let plan = compositor.compose().await.unwrap();

        // Should have resource utilization info
        assert!(plan.resource_utilization.memory_gb_total > 0.0);
        assert!(plan.resource_utilization.cpu_cores_total > 0);
    }

    #[tokio::test]
    async fn test_plan_statistics() {
        let mut compositor = MultiWorkloadCompositor::from_runtime().await.unwrap();

        let req1 = CompositionRequest::new("test1").with_constraint(Constraint::min_memory_gb(0.1));
        let req2 = CompositionRequest::new("test2").with_constraint(Constraint::prefer_local());

        compositor.add_request(req1);
        compositor.add_request(req2);

        let plan = compositor.compose().await.unwrap();

        // Average score should be between 0 and 1
        let avg_score = plan.average_score();
        assert!((0.0..=1.0).contains(&avg_score));

        // Should have 2 feasible placements
        assert_eq!(plan.feasible_placements().len(), 2);
        assert_eq!(plan.infeasible_placements().len(), 0);
    }
}
