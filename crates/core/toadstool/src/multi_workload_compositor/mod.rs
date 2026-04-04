// SPDX-License-Identifier: AGPL-3.0-only
// Copyright (C) 2024-2025 ToadStool Project

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

mod merging;
mod scheduling;
mod types;

#[cfg(test)]
mod tests;

use crate::ToadStoolResult;
use crate::composition_constraints::CompositionRequest;
use crate::composition_engine::{CompositionEngine, EngineStats};
use std::sync::Arc;
use tracing::{debug, info, warn};

pub use types::{
    AllocatedResources, CompositionPlan, ConflictResolution, ResourceUtilization,
    WorkloadConflict, WorkloadPlacement,
};

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
    ///
    /// # Errors
    ///
    /// Returns error if the composition engine cannot be initialized.
    pub async fn from_runtime() -> ToadStoolResult<Self> {
        let engine = CompositionEngine::from_runtime().await?;
        Self::new(Arc::new(engine))
    }

    /// Create compositor with specific engine
    ///
    /// # Errors
    ///
    /// This function currently always returns `Ok`.
    pub const fn new(engine: Arc<CompositionEngine>) -> ToadStoolResult<Self> {
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
    ///
    /// # Errors
    ///
    /// Returns error if composition evaluation fails for a workload.
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

        let indices = scheduling::sort_indices_by_priority(&self.requests);

        debug!("📋 Sorted by priority:");
        for &idx in &indices {
            let req = &self.requests[idx];
            debug!("  - {} (priority: {})", req.name, req.priority);
        }

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
                allocated_resources: merging::estimate_allocated_resources(request),
            };

            placements.push(placement);

            if !evaluation.is_feasible {
                if let Some(c) = scheduling::detect_conflict(request, &placements) {
                    conflicts.push(c);
                }
            }
        }

        let overall_feasibility = placements.iter().all(|p| p.is_feasible);
        let resource_utilization =
            merging::calculate_resource_utilization(&self.engine, &placements);

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

    /// Get engine statistics
    pub async fn stats(&self) -> EngineStats {
        self.engine.stats().await
    }
}
