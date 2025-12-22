//! End-to-end integration tests for AI/ML and CUDA workload routing
//!
//! These tests validate the entire flow from workload specification through
//! analysis, backend selection, and runtime routing.

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::path::PathBuf;

    use crate::workload::*;
    use crate::WorkloadSpec;

    #[test]
    fn test_aiml_workload_analysis_training_large() {
        // Test AI/ML training workload analysis
        let workload = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            64, // batch size
        )
        .with_precision(Precision::FP16)
        .with_max_latency_ms(1000);

        let workload_spec = WorkloadSpec::AiMl {
            workload: workload.clone(),
        };

        let analyzer = WorkloadAnalyzer::new();
        let characteristics = analyzer.analyze(&workload_spec);

        // Verify characteristics
        assert!(matches!(
            characteristics.compute_intensity,
            ComputeIntensity::High
        ));
        assert!(matches!(
            characteristics.gpu_advantage,
            GpuAdvantage::VeryHigh
        ));
        assert!(matches!(
            characteristics.parallelism_level,
            ParallelismLevel::VeryHigh
        ));
        assert!(!characteristics.cpu_viable); // Large training is NOT CPU viable (too slow)
    }

    #[test]
    fn test_aiml_workload_analysis_inference_small() {
        // Test AI/ML inference workload analysis
        let workload = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Inference,
            ModelSize::Small,
            1, // batch size
        )
        .with_precision(Precision::INT8)
        .with_max_latency_ms(50);

        let workload_spec = WorkloadSpec::AiMl {
            workload: workload.clone(),
        };

        let analyzer = WorkloadAnalyzer::new();
        let characteristics = analyzer.analyze(&workload_spec);

        // Verify characteristics
        assert!(matches!(
            characteristics.compute_intensity,
            ComputeIntensity::Low
        ));
        assert!(matches!(
            characteristics.memory_requirement,
            MemoryRequirement::Tiny | MemoryRequirement::Small
        ));
        assert!(characteristics.cpu_viable); // Small inference is CPU-viable
    }

    #[test]
    fn test_cuda_workload_analysis_large_parallel() {
        // Test CUDA workload with high parallelism
        let launch = CudaLaunchConfig::linear(10_000_000, 256);
        let source = CudaSource::CudaCpp {
            source: "...".to_string(),
            entry_point: "vector_add".to_string(),
        };
        let workload = CudaWorkload::new(source, launch);

        let workload_spec = WorkloadSpec::Cuda {
            workload: workload.clone(),
        };

        let analyzer = WorkloadAnalyzer::new();
        let characteristics = analyzer.analyze(&workload_spec);

        // Verify characteristics
        assert!(matches!(
            characteristics.compute_intensity,
            ComputeIntensity::VeryHigh | ComputeIntensity::Extreme
        ));
        assert!(matches!(
            characteristics.parallelism_level,
            ParallelismLevel::VeryHigh
        ));
        assert!(matches!(
            characteristics.gpu_advantage,
            GpuAdvantage::VeryHigh | GpuAdvantage::Critical
        ));
    }

    #[test]
    fn test_cuda_workload_analysis_small_viable_cpu() {
        // Test CUDA workload that's viable on CPU
        let launch = CudaLaunchConfig::linear(1_000, 64);
        let source = CudaSource::CudaCpp {
            source: "...".to_string(),
            entry_point: "simple_kernel".to_string(),
        };
        let workload = CudaWorkload::new(source, launch);

        let workload_spec = WorkloadSpec::Cuda {
            workload: workload.clone(),
        };

        let analyzer = WorkloadAnalyzer::new();
        let characteristics = analyzer.analyze(&workload_spec);

        // Small CUDA workloads should be CPU-viable
        assert!(characteristics.cpu_viable);
    }

    #[test]
    fn test_backend_selection_nvidia_gpu() {
        // Test backend selection with NVIDIA GPU
        let hardware = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024)
            .with_gpu(GpuDevice::new(
                "RTX 3090".to_string(),
                GpuVendor::Nvidia,
                24 * 1024 * 1024 * 1024,
                128,
            ))
            .with_cuda_capability("8.6".to_string());

        let selector = BackendSelector::with_hardware(hardware);

        // Create high-compute CUDA workload
        let launch = CudaLaunchConfig::linear(1_000_000, 256);
        let source = CudaSource::CudaCpp {
            source: "...".to_string(),
            entry_point: "kernel".to_string(),
        };
        let workload = CudaWorkload::new(source, launch);
        let workload_spec = WorkloadSpec::Cuda {
            workload: workload.clone(),
        };

        let analyzer = WorkloadAnalyzer::new();
        let characteristics = analyzer.analyze(&workload_spec);
        let decision = selector.select_cuda_backend(&characteristics);

        // Should select native CUDA
        assert_eq!(decision.cuda_backend, CudaBackend::NativeNvidia);
        assert!(decision.confidence > 0.9);
        assert!(decision.reasoning.contains("NVIDIA") || decision.reasoning.contains("Optimal"));
        assert!(!decision.alternatives.is_empty()); // Should have fallback options
    }

    #[test]
    fn test_backend_selection_amd_gpu() {
        // Test backend selection with AMD GPU
        let hardware =
            HardwareCapabilities::new(16, 32 * 1024 * 1024 * 1024).with_gpu(GpuDevice::new(
                "RX 7900 XTX".to_string(),
                GpuVendor::Amd,
                24 * 1024 * 1024 * 1024,
                96,
            ));

        let selector = BackendSelector::with_hardware(hardware);

        // Create medium-compute workload
        let launch = CudaLaunchConfig::linear(100_000, 256);
        let source = CudaSource::CudaCpp {
            source: "...".to_string(),
            entry_point: "kernel".to_string(),
        };
        let workload = CudaWorkload::new(source, launch);
        let workload_spec = WorkloadSpec::Cuda {
            workload: workload.clone(),
        };

        let analyzer = WorkloadAnalyzer::new();
        let characteristics = analyzer.analyze(&workload_spec);
        let decision = selector.select_cuda_backend(&characteristics);

        // Should select GPU translation
        assert_eq!(decision.cuda_backend, CudaBackend::TranslatedGpu);
        assert!(decision.confidence > 0.7);
        assert!(decision.reasoning.contains("AMD"));
    }

    #[test]
    fn test_backend_selection_cpu_only() {
        // Test backend selection with CPU only (no GPU)
        let hardware = HardwareCapabilities::new(16, 32 * 1024 * 1024 * 1024);

        let selector = BackendSelector::with_hardware(hardware);

        // Create CPU-viable workload
        let launch = CudaLaunchConfig::linear(10_000, 128);
        let source = CudaSource::CudaCpp {
            source: "...".to_string(),
            entry_point: "kernel".to_string(),
        };
        let workload = CudaWorkload::new(source, launch);
        let workload_spec = WorkloadSpec::Cuda {
            workload: workload.clone(),
        };

        let analyzer = WorkloadAnalyzer::new();
        let characteristics = analyzer.analyze(&workload_spec);
        let decision = selector.select_cuda_backend(&characteristics);

        // Should select CPU parallel
        assert!(matches!(
            decision.cuda_backend,
            CudaBackend::CpuParallel | CudaBackend::CpuSequential
        ));
        assert!(decision.reasoning.contains("CPU") || decision.reasoning.contains("cores"));
    }

    #[test]
    fn test_backend_selection_graceful_degradation() {
        // Test that backend selector always provides alternatives
        let hardware = HardwareCapabilities::new(8, 16 * 1024 * 1024 * 1024)
            .with_gpu(GpuDevice::new(
                "RTX 3090".to_string(),
                GpuVendor::Nvidia,
                24 * 1024 * 1024 * 1024,
                128,
            ))
            .with_cuda_capability("8.6".to_string());

        let selector = BackendSelector::with_hardware(hardware);

        let launch = CudaLaunchConfig::linear(1_000_000, 256);
        let source = CudaSource::CudaCpp {
            source: "...".to_string(),
            entry_point: "kernel".to_string(),
        };
        let workload = CudaWorkload::new(source, launch);
        let workload_spec = WorkloadSpec::Cuda {
            workload: workload.clone(),
        };

        let analyzer = WorkloadAnalyzer::new();
        let characteristics = analyzer.analyze(&workload_spec);
        let decision = selector.select_cuda_backend(&characteristics);

        // Must have alternatives for graceful degradation
        assert!(!decision.alternatives.is_empty());

        // Alternatives should include different backend options
        let has_cpu_fallback = decision
            .alternatives
            .iter()
            .any(|b| matches!(b, CudaBackend::CpuParallel | CudaBackend::CpuSequential));
        assert!(has_cpu_fallback, "Should have CPU fallback option");
    }

    #[test]
    fn test_memory_estimation_accuracy() {
        // Test that memory estimation is reasonable
        let small = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Inference,
            ModelSize::Small,
            1,
        );
        let large = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            64,
        );
        let xlarge = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::XLarge,
            32,
        );

        let small_mem = small.estimate_total_memory_bytes();
        let large_mem = large.estimate_total_memory_bytes();
        let xlarge_mem = xlarge.estimate_total_memory_bytes();

        // Verify ordering
        assert!(small_mem < large_mem);
        assert!(large_mem < xlarge_mem);

        // Verify reasonable magnitudes (small should be < 1GB, large should be > 1GB)
        assert!(small_mem < 1024 * 1024 * 1024);
        assert!(large_mem > 1024 * 1024 * 1024);
        assert!(xlarge_mem > 8 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_cuda_thread_calculations() {
        // Test CUDA launch configuration calculations
        // Note: linear() rounds up to nearest block, so 1_000_000 / 256 = 3907 blocks * 256 = 1_000_192 threads
        let linear = CudaLaunchConfig::linear(1_000_000, 256);
        assert!(linear.total_threads() >= 1_000_000); // Should cover at least the requested threads
        assert!(linear.total_threads() < 1_000_000 + 256); // But not more than one block extra

        let grid2d = CudaLaunchConfig::new((100, 100, 1), (16, 16, 1));
        assert_eq!(grid2d.total_threads(), 100 * 100 * 16 * 16);

        let grid3d = CudaLaunchConfig::new((10, 10, 10), (8, 8, 8));
        assert_eq!(grid3d.total_threads(), 10 * 10 * 10 * 8 * 8 * 8);
    }

    #[test]
    fn test_workload_type_routing() {
        // Test that workload types are correctly identified for routing
        let aiml_workload = WorkloadSpec::AiMl {
            workload: AiMlWorkload::new(
                AiFramework::PyTorch,
                AiOperation::Training,
                ModelSize::Medium,
                32,
            ),
        };

        let cuda_workload = WorkloadSpec::Cuda {
            workload: CudaWorkload::new(
                CudaSource::CudaCpp {
                    source: "...".to_string(),
                    entry_point: "kernel".to_string(),
                },
                CudaLaunchConfig::linear(10_000, 256),
            ),
        };

        let native_workload = WorkloadSpec::Native {
            executable: crate::workload::ExecutableSource::File {
                path: PathBuf::from("/bin/echo"),
            },
            args: None,
            working_dir: None,
            env_vars: HashMap::new(),
            user: None,
        };

        assert_eq!(aiml_workload.workload_type(), crate::WorkloadType::AiMl);
        assert_eq!(cuda_workload.workload_type(), crate::WorkloadType::Cuda);
        assert_eq!(native_workload.workload_type(), crate::WorkloadType::Native);
    }
}
