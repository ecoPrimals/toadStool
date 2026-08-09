// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use crate::workload::analyzer::MemoryRequirement;

#[test]
fn test_hardware_detection() {
    let hw = HardwareCapabilities::detect();
    assert!(hw.cpu_cores > 0);
    assert!(hw.ram_bytes > 0);
}

#[test]
fn test_nvidia_gpu_native_cuda() {
    let hw = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024)
        .with_gpu(GpuDevice::new(
            "RTX 3090".to_string(),
            GpuVendor::Nvidia,
            24 * 1024 * 1024 * 1024,
            128,
        ))
        .with_cuda_capability("8.6".to_string());

    let selector = BackendSelector::with_hardware(hw);

    let chars = WorkloadCharacteristics {
        compute_intensity: ComputeIntensity::High,
        memory_requirement: MemoryRequirement::Medium,
        parallelism_level: ParallelismLevel::VeryHigh,
        gpu_advantage: GpuAdvantage::VeryHigh,
        cpu_viable: true,
        estimated_flops: Some(1_000_000_000),
    };

    let decision = selector.select_cuda_backend(&chars);
    assert_eq!(decision.cuda_backend, CudaBackend::NativeNvidia);
    assert!(decision.confidence > 0.9);
}

#[test]
fn test_amd_gpu_translation() {
    let hw = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024).with_gpu(GpuDevice::new(
        "RX 7900 XTX".to_string(),
        GpuVendor::Amd,
        24 * 1024 * 1024 * 1024,
        96,
    ));

    let selector = BackendSelector::with_hardware(hw);

    let chars = WorkloadCharacteristics {
        compute_intensity: ComputeIntensity::High,
        memory_requirement: MemoryRequirement::Medium,
        parallelism_level: ParallelismLevel::High,
        gpu_advantage: GpuAdvantage::High,
        cpu_viable: true,
        estimated_flops: Some(500_000_000),
    };

    let decision = selector.select_cuda_backend(&chars);
    assert_eq!(decision.cuda_backend, CudaBackend::TranslatedGpu);
    assert!(decision.confidence > 0.7);
    assert!(decision.reasoning.contains("AMD"));
}

#[test]
fn test_cpu_parallel_fallback() {
    let hw = HardwareCapabilities::new(16, 32 * 1024 * 1024 * 1024); // No GPU

    let selector = BackendSelector::with_hardware(hw);

    let chars = WorkloadCharacteristics {
        compute_intensity: ComputeIntensity::Medium,
        memory_requirement: MemoryRequirement::Small,
        parallelism_level: ParallelismLevel::Medium,
        gpu_advantage: GpuAdvantage::Moderate,
        cpu_viable: true,
        estimated_flops: Some(100_000_000),
    };

    let decision = selector.select_cuda_backend(&chars);
    assert_eq!(decision.cuda_backend, CudaBackend::CpuParallel);
    assert!(decision.reasoning.contains("16 cores"));
}

#[test]
fn test_cpu_sequential_last_resort() {
    let hw = HardwareCapabilities::new(2, 4 * 1024 * 1024 * 1024); // Limited CPU, no GPU

    let selector = BackendSelector::with_hardware(hw);

    let chars = WorkloadCharacteristics {
        compute_intensity: ComputeIntensity::Extreme,
        memory_requirement: MemoryRequirement::Huge,
        parallelism_level: ParallelismLevel::Sequential,
        gpu_advantage: GpuAdvantage::Critical,
        cpu_viable: false,
        estimated_flops: Some(10_000_000_000),
    };

    let decision = selector.select_cuda_backend(&chars);
    assert_eq!(decision.cuda_backend, CudaBackend::CpuSequential);
    assert!(decision.confidence < 0.5);
}

#[test]
fn test_apple_gpu_translation() {
    let hw = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024).with_gpu(GpuDevice::new(
        "Apple M2 Max".to_string(),
        GpuVendor::Apple,
        32 * 1024 * 1024 * 1024,
        38,
    ));

    let selector = BackendSelector::with_hardware(hw);

    let chars = WorkloadCharacteristics {
        compute_intensity: ComputeIntensity::High,
        memory_requirement: MemoryRequirement::Medium,
        parallelism_level: ParallelismLevel::High,
        gpu_advantage: GpuAdvantage::Significant,
        cpu_viable: true,
        estimated_flops: None,
    };

    let decision = selector.select_cuda_backend(&chars);
    assert_eq!(decision.cuda_backend, CudaBackend::TranslatedGpu);
    assert!(decision.reasoning.contains("Apple"));
}

#[test]
fn test_alternatives_provided() {
    let hw = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024)
        .with_gpu(GpuDevice::new(
            "RTX 3090".to_string(),
            GpuVendor::Nvidia,
            24 * 1024 * 1024 * 1024,
            128,
        ))
        .with_cuda_capability("8.6".to_string());

    let selector = BackendSelector::with_hardware(hw);

    let chars = WorkloadCharacteristics::default();
    let decision = selector.select_cuda_backend(&chars);

    // Should have alternatives
    assert!(!decision.alternatives.is_empty());
    assert!(decision.alternatives.contains(&CudaBackend::TranslatedGpu));
}
