// SPDX-License-Identifier: AGPL-3.0-or-later
//! AI/ML workload analysis

use super::characteristics::{
    ComputeIntensity, GpuAdvantage, ParallelismLevel, WorkloadCharacteristics,
};
use super::WorkloadAnalyzer;
use crate::workload::{AiMlWorkload, AiOperation, ModelSize};

impl WorkloadAnalyzer {
    pub(super) fn analyze_aiml(&self, workload: &AiMlWorkload) -> WorkloadCharacteristics {
        let compute_intensity = match (&workload.operation, &workload.model_size) {
            (AiOperation::Training, ModelSize::XXLarge) => ComputeIntensity::Extreme,
            (AiOperation::Training, ModelSize::XLarge) => ComputeIntensity::VeryHigh,
            (AiOperation::Training, ModelSize::Large) => ComputeIntensity::High,
            (AiOperation::Training, _) => ComputeIntensity::Medium,
            (AiOperation::FineTuning, ModelSize::XLarge | ModelSize::XXLarge) => {
                ComputeIntensity::VeryHigh
            }
            (AiOperation::FineTuning, _) => ComputeIntensity::High,
            (AiOperation::Inference, ModelSize::XXLarge) => ComputeIntensity::High,
            (AiOperation::Inference, ModelSize::XLarge) => ComputeIntensity::Medium,
            (AiOperation::Inference, _) => ComputeIntensity::Low,
            _ => ComputeIntensity::Medium,
        };

        let memory_bytes = workload.estimate_total_memory_bytes();
        let memory_requirement = WorkloadAnalyzer::classify_memory(memory_bytes);

        let parallelism_level = match (&workload.operation, workload.batch_size) {
            (AiOperation::Training, bs) if bs >= 64 => ParallelismLevel::VeryHigh,
            (AiOperation::Training, bs) if bs >= 16 => ParallelismLevel::High,
            (AiOperation::Training, _) => ParallelismLevel::Medium,
            (AiOperation::Inference, bs) if bs >= 32 => ParallelismLevel::High,
            (AiOperation::Inference, bs) if bs >= 8 => ParallelismLevel::Medium,
            (AiOperation::Inference, _) => ParallelismLevel::Low,
            _ => ParallelismLevel::Medium,
        };

        let gpu_advantage = match (&workload.operation, &workload.model_size) {
            (AiOperation::Training, ModelSize::XLarge | ModelSize::XXLarge) => {
                GpuAdvantage::Critical
            }
            (AiOperation::Training, ModelSize::Large) => GpuAdvantage::VeryHigh,
            (AiOperation::Training, _) => GpuAdvantage::High,
            (AiOperation::Inference, ModelSize::XLarge | ModelSize::XXLarge) => GpuAdvantage::High,
            (AiOperation::Inference, _) => GpuAdvantage::Moderate,
            _ => GpuAdvantage::Moderate,
        };

        let cpu_viable = workload.is_cpu_viable();

        WorkloadCharacteristics {
            compute_intensity,
            memory_requirement,
            parallelism_level,
            gpu_advantage,
            cpu_viable,
            estimated_flops: None,
        }
    }
}
