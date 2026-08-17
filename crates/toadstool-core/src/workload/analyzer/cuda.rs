// SPDX-License-Identifier: AGPL-3.0-or-later
//! CUDA workload analysis

use super::WorkloadAnalyzer;
use super::characteristics::{
    ComputeIntensity, GpuAdvantage, ParallelismLevel, WorkloadCharacteristics,
};
use crate::workload::CudaWorkload;

impl WorkloadAnalyzer {
    #[expect(
        clippy::unused_self,
        reason = "method pattern — analyzer may gain state"
    )]
    pub(super) const fn analyze_cuda(&self, workload: &CudaWorkload) -> WorkloadCharacteristics {
        let total_threads = workload.launch_config.total_threads();

        let compute_intensity = match total_threads {
            0..=10_000 => ComputeIntensity::Low,
            10_001..=100_000 => ComputeIntensity::Medium,
            100_001..=1_000_000 => ComputeIntensity::High,
            1_000_001..=10_000_000 => ComputeIntensity::VeryHigh,
            _ => ComputeIntensity::Extreme,
        };

        let shared_mem = workload.launch_config.shared_mem_bytes;
        let blocks = workload.launch_config.total_blocks();
        let total_shared = shared_mem.saturating_mul(blocks as usize);
        let memory_requirement = Self::classify_memory(total_shared as u64);

        let parallelism_level = match total_threads {
            0..=1_000 => ParallelismLevel::Low,
            1_001..=10_000 => ParallelismLevel::Medium,
            10_001..=100_000 => ParallelismLevel::High,
            _ => ParallelismLevel::VeryHigh,
        };

        let gpu_advantage = if workload.has_memory_dependencies {
            GpuAdvantage::Moderate
        } else if total_threads > 100_000 {
            GpuAdvantage::VeryHigh
        } else {
            GpuAdvantage::High
        };

        let cpu_viable = workload.is_cpu_viable();

        WorkloadCharacteristics {
            compute_intensity,
            memory_requirement,
            parallelism_level,
            gpu_advantage,
            cpu_viable,
            estimated_flops: workload.estimated_flops,
        }
    }
}
