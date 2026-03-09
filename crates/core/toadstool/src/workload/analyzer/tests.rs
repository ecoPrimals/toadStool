// SPDX-License-Identifier: AGPL-3.0-only
//! Workload analyzer tests.

use super::super::*;
use crate::workload::{
    AiFramework, AiMlWorkload, AiOperation, CudaLaunchConfig, CudaSource, CudaWorkload,
    GpuProgramSource, ModelSize, WorkloadSpec,
};

#[test]
fn test_workload_characteristics_default() {
    let default = WorkloadCharacteristics::default();
    assert_eq!(default.compute_intensity, ComputeIntensity::Medium);
    assert_eq!(default.memory_requirement, MemoryRequirement::Medium);
    assert!(default.cpu_viable);
}

#[test]
fn test_workload_analyzer_new() {
    let analyzer = WorkloadAnalyzer::new();
    let _ = analyzer.analyze(&WorkloadSpec::default());
}

#[test]
fn test_aiml_training_large_model() {
    let workload = AiMlWorkload::new(
        AiFramework::PyTorch,
        AiOperation::Training,
        ModelSize::Large,
        64,
    );
    let analyzer = WorkloadAnalyzer::new();
    let chars = analyzer.analyze(&WorkloadSpec::AiMl {
        workload: workload.clone(),
    });
    assert_eq!(chars.compute_intensity, ComputeIntensity::High);
    assert!(matches!(chars.gpu_advantage, GpuAdvantage::VeryHigh));
}

#[test]
fn test_memory_classification() {
    assert_eq!(
        WorkloadAnalyzer::classify_memory(50_000_000),
        MemoryRequirement::Tiny
    );
    assert_eq!(
        WorkloadAnalyzer::classify_memory(5_000_000_000),
        MemoryRequirement::Medium
    );
}

#[test]
fn test_cuda_large_workload() {
    let launch = CudaLaunchConfig::linear(10_000_000, 256);
    let source = CudaSource::CudaCpp {
        source: "...".to_string(),
        entry_point: "kernel".to_string(),
    };
    let workload = CudaWorkload::new(source, launch);
    let analyzer = WorkloadAnalyzer::new();
    let chars = analyzer.analyze(&WorkloadSpec::Cuda {
        workload: workload.clone(),
    });
    assert!(matches!(
        chars.compute_intensity,
        ComputeIntensity::VeryHigh | ComputeIntensity::Extreme
    ));
}

#[test]
fn test_gpu_workload_analysis() {
    let analyzer = WorkloadAnalyzer::new();
    let spec = WorkloadSpec::Gpu {
        program: GpuProgramSource::OpenCL {
            source: "kernel void k() {}".to_string(),
        },
        kernel_name: "k".to_string(),
        work_group_size: Some((16, 16, 1)),
        global_work_size: (1024, 1024, 1),
        args: vec![],
    };
    let c = analyzer.analyze(&spec);
    assert_eq!(c.compute_intensity, ComputeIntensity::High);
    assert!(!c.cpu_viable);
}

#[test]
fn test_native_workload_returns_default() {
    let analyzer = WorkloadAnalyzer::new();
    let c = analyzer.analyze(&WorkloadSpec::default());
    assert!(c.cpu_viable);
}
