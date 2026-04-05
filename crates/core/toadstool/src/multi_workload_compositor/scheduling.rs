// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (C) 2024-2025 ToadStool Project

use super::types::{ConflictResolution, WorkloadConflict, WorkloadPlacement};
use crate::composition_constraints::{CompositionRequest, Constraint};

/// Sort request indices by descending priority (highest priority first).
pub(crate) fn sort_indices_by_priority(requests: &[CompositionRequest]) -> Vec<usize> {
    let mut indices: Vec<usize> = (0..requests.len()).collect();
    indices.sort_by(|&a, &b| requests[b].priority.cmp(&requests[a].priority));
    indices
}

/// Detect conflict between a failing request and existing placements.
pub(crate) fn detect_conflict(
    failing_request: &CompositionRequest,
    placements: &[WorkloadPlacement],
) -> Option<WorkloadConflict> {
    let hard_constraints = failing_request.hard_constraints();
    if hard_constraints.is_empty() {
        return None;
    }

    let conflicting_workloads: Vec<_> = placements
        .iter()
        .filter(|p| {
            p.is_feasible
                && p.request.priority > failing_request.priority
                && would_conflict(&p.request, failing_request)
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

/// Check if two workloads would conflict for resources.
pub(crate) fn would_conflict(req1: &CompositionRequest, req2: &CompositionRequest) -> bool {
    req1.constraints
        .iter()
        .any(|c| matches!(c, Constraint::RequiresGPU))
        && req2
            .constraints
            .iter()
            .any(|c| matches!(c, Constraint::RequiresGPU))
}
