// SPDX-License-Identifier: AGPL-3.0-only
//! Common Scheduling Abstractions
//!
//! Capability-based workload scheduling for distributed execution.
//! Scheduling decisions are driven by runtime-discovered capabilities,
//! not hardcoded node lists.

/// Scheduling priority for workloads.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum SchedulingPriority {
    /// Background tasks — run when resources are idle.
    Background,
    /// Normal priority — default for most workloads.
    #[default]
    Normal,
    /// High priority — preempts background tasks.
    High,
    /// Critical — must be scheduled immediately or rejected.
    Critical,
}

/// A scheduling constraint for capability-based placement.
#[derive(Debug, Clone)]
pub struct PlacementConstraint {
    /// Required capability (e.g. "gpu.f64", "npu.inference").
    pub required_capability: String,
    /// Minimum resource amount (e.g. memory in bytes, TFLOPS).
    pub minimum_resource: u64,
}

/// Result of a scheduling decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulingDecision {
    /// Execute locally — this node has the required capabilities.
    ExecuteLocal,
    /// Delegate to a discovered peer with the given endpoint.
    Delegate {
        /// Peer endpoint URL.
        endpoint: String,
    },
    /// Reject — no node can satisfy the constraints.
    Reject {
        /// Rejection reason.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_priority_ordering() {
        assert!(SchedulingPriority::Background < SchedulingPriority::Normal);
        assert!(SchedulingPriority::Normal < SchedulingPriority::High);
        assert!(SchedulingPriority::High < SchedulingPriority::Critical);
    }

    #[test]
    fn test_priority_default() {
        assert_eq!(SchedulingPriority::default(), SchedulingPriority::Normal);
    }

    #[test]
    fn test_scheduling_decision_variants() {
        let local = SchedulingDecision::ExecuteLocal;
        let delegate = SchedulingDecision::Delegate {
            endpoint: "http://peer:8080".to_string(),
        };
        let reject = SchedulingDecision::Reject {
            reason: "no GPU available".to_string(),
        };

        assert_eq!(local, SchedulingDecision::ExecuteLocal);
        assert_ne!(local, delegate);
        assert_ne!(delegate, reject);
    }

    #[test]
    fn test_placement_constraint() {
        let constraint = PlacementConstraint {
            required_capability: "gpu.f64".to_string(),
            minimum_resource: 4_000_000_000,
        };
        assert_eq!(constraint.required_capability, "gpu.f64");
        assert_eq!(constraint.minimum_resource, 4_000_000_000);
    }
}
