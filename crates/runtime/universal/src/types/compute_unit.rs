// SPDX-License-Identifier: AGPL-3.0-or-later
//! [`ComputeUnit`] trait — unified interface for parallel compute resources.

use super::capabilities::Capabilities;
use super::error::ComputeError;
use super::output::Output;
use super::workload::Workload;

/// A compute unit represents any parallel processing resource.
///
/// This trait abstracts over different compute paradigms:
/// - CPU: Serial/parallel (1-64 cores typically)
/// - GPU: Massive parallel (1000s of cores)
/// - Neuromorphic: Event-driven (spike-based)
///
/// Key insight: They're all parallel compute with different profiles!
#[async_trait::async_trait]
pub trait ComputeUnit: Send + Sync {
    /// Get capabilities of this compute unit
    fn capabilities(&self) -> &Capabilities;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Execute a workload on this compute unit
    async fn execute(&self, workload: Workload) -> Result<Output, ComputeError>;

    /// Check if this unit can execute the given workload
    fn can_execute(&self, workload: &Workload) -> bool {
        self.capabilities().supports_workload(workload)
    }

    /// Get the optimal batch size for this unit
    fn optimal_batch_size(&self) -> usize {
        self.capabilities().optimal_batch_size
    }

    /// Estimate execution time for a workload (for scheduling)
    fn estimate_duration(&self, workload: &Workload) -> std::time::Duration {
        self.capabilities().estimate_duration(workload)
    }
}
