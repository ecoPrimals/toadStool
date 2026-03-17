// SPDX-License-Identifier: AGPL-3.0-only
//! Workload analysis and characterization
//!
//! Analyzes workload characteristics to enable intelligent backend selection.

mod aiml;
mod characteristics;
mod cuda;

#[cfg(test)]
mod tests;

pub use characteristics::{
    ComputeIntensity, GpuAdvantage, MemoryRequirement, ParallelismLevel, WorkloadCharacteristics,
};

use crate::workload::WorkloadSpec;

/// Workload analyzer for characterizing workloads
pub struct WorkloadAnalyzer;

impl WorkloadAnalyzer {
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    #[must_use]
    pub fn analyze(&self, workload: &WorkloadSpec) -> WorkloadCharacteristics {
        match workload {
            WorkloadSpec::AiMl { workload } => self.analyze_aiml(workload),
            WorkloadSpec::Cuda { workload } => self.analyze_cuda(workload),
            WorkloadSpec::Gpu { .. } => self.analyze_gpu(),
            _ => WorkloadCharacteristics::default(),
        }
    }

    const fn analyze_gpu(&self) -> WorkloadCharacteristics {
        use characteristics::{
            ComputeIntensity, GpuAdvantage, MemoryRequirement, ParallelismLevel,
        };

        WorkloadCharacteristics {
            compute_intensity: ComputeIntensity::High,
            memory_requirement: MemoryRequirement::Medium,
            parallelism_level: ParallelismLevel::High,
            gpu_advantage: GpuAdvantage::High,
            cpu_viable: false,
            estimated_flops: None,
        }
    }

    pub const fn classify_memory(bytes: u64) -> characteristics::MemoryRequirement {
        use characteristics::MemoryRequirement;
        match bytes {
            0..=99_999_999 => MemoryRequirement::Tiny,
            100_000_000..=999_999_999 => MemoryRequirement::Small,
            1_000_000_000..=9_999_999_999 => MemoryRequirement::Medium,
            10_000_000_000..=99_999_999_999 => MemoryRequirement::Large,
            _ => MemoryRequirement::Huge,
        }
    }
}

impl Default for WorkloadAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
