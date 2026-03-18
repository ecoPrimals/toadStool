// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload Scheduler - Multi-substrate coordination
//!
//! **Deep Debt**: Intelligent scheduling across multiple substrates

use std::time::Duration;

/// Workload scheduler for parallel execution
///
/// **Deep Debt**: Load balancing, not hardcoded partitioning
#[derive(Debug)]
pub struct WorkloadScheduler {
    /// Scheduling strategy
    strategy: SchedulingStrategy,
}

impl WorkloadScheduler {
    /// Creates a new workload scheduler with default strategy.
    pub fn new() -> Self {
        Self {
            strategy: SchedulingStrategy::default(),
        }
    }

    /// Get scheduling strategy
    #[allow(clippy::missing_const_for_fn)] // Returns reference, Deref not const
    pub fn strategy(&self) -> &SchedulingStrategy {
        &self.strategy
    }
}

impl Default for WorkloadScheduler {
    fn default() -> Self {
        Self::new()
    }
}

/// Scheduling strategies
#[derive(Debug, Clone, Copy, Default)]
pub enum SchedulingStrategy {
    /// Single substrate (no parallelism)
    #[default]
    Single,

    /// Data parallelism (split data across substrates)
    DataParallel,

    /// Pipeline parallelism (stages on different substrates)
    Pipeline,

    /// Hybrid (mix of data and pipeline)
    Hybrid,
}

/// Schedule for workload execution
#[derive(Debug)]
pub struct ExecutionSchedule {
    /// Tasks to execute
    pub tasks: Vec<ScheduledTask>,

    /// Expected total duration
    pub estimated_duration: Duration,
}

/// A scheduled task
#[derive(Debug)]
pub struct ScheduledTask {
    /// Task ID
    pub id: usize,

    /// Substrate to execute on
    pub substrate_id: usize,

    /// Start time offset
    pub start_offset: Duration,

    /// Estimated duration
    pub duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_creation() {
        let scheduler = WorkloadScheduler::new();
        assert!(matches!(scheduler.strategy(), SchedulingStrategy::Single));
    }
}
