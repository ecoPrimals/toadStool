// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

use super::types::{AllocatedResources, CompositionPlan, ResourceUtilization, WorkloadPlacement};
use crate::composition_constraints::{CompositionRequest, Constraint};
use crate::composition_engine::CompositionEngine;
use crate::layer_adaptation::GpuAccess;

/// Estimate resources that would be allocated to a workload from its constraints.
pub(crate) fn estimate_allocated_resources(request: &CompositionRequest) -> AllocatedResources {
    let mut resources = AllocatedResources::default();

    for constraint in &request.constraints {
        match constraint {
            Constraint::RequiresGPU | Constraint::PrefersGPU => {
                resources.gpu_allocation = Some(1.0);
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

/// Calculate overall resource utilization across feasible placements.
#[expect(
    clippy::cast_precision_loss,
    reason = "precision loss acceptable for this conversion"
)] // GiB fraction from byte counts for utilization display
pub(crate) fn calculate_resource_utilization(
    engine: &CompositionEngine,
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

    let caps = engine.capabilities();
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
        #[expect(
            clippy::cast_precision_loss,
            reason = "precision loss acceptable for this conversion"
        )]
        let result = total / len as f64;
        result
    }
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
            #[expect(
                clippy::cast_precision_loss,
                reason = "precision loss acceptable for this conversion"
            )]
            let pct = (used as f64 / total as f64) * 100.0;
            pct
        }
    }
}
