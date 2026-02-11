//! Workload analysis and characterization
//!
//! Analyzes workload characteristics to enable intelligent backend selection.
//! This is the core of ToadStool's workload-centric (not hardware-centric) approach.

use std::fmt;

use super::{AiMlWorkload, AiOperation, CudaWorkload, ModelSize, WorkloadSpec};

/// Workload analysis result containing characteristics
#[derive(Debug, Clone, PartialEq)]
pub struct WorkloadCharacteristics {
    /// How compute-intensive is this workload?
    pub compute_intensity: ComputeIntensity,

    /// Memory footprint requirement
    pub memory_requirement: MemoryRequirement,

    /// Level of parallelism in the workload
    pub parallelism_level: ParallelismLevel,

    /// How much does GPU help vs CPU?
    pub gpu_advantage: GpuAdvantage,

    /// Can this workload reasonably run on CPU?
    pub cpu_viable: bool,

    /// Estimated FLOPS (floating point operations)
    pub estimated_flops: Option<u64>,
}

impl Default for WorkloadCharacteristics {
    fn default() -> Self {
        Self {
            compute_intensity: ComputeIntensity::Medium,
            memory_requirement: MemoryRequirement::Medium,
            parallelism_level: ParallelismLevel::Medium,
            gpu_advantage: GpuAdvantage::Moderate,
            cpu_viable: true,
            estimated_flops: None,
        }
    }
}

/// Compute intensity classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ComputeIntensity {
    /// Minimal computation (<1 GFLOP)
    Minimal,

    /// Low computation (1-10 GFLOP)
    Low,

    /// Medium computation (10-100 GFLOP)
    Medium,

    /// High computation (100 GFLOP - 1 TFLOP)
    High,

    /// Very high computation (1-10 TFLOP)
    VeryHigh,

    /// Extreme computation (>10 TFLOP)
    Extreme,
}

impl fmt::Display for ComputeIntensity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minimal => write!(f, "Minimal (<1 GFLOP)"),
            Self::Low => write!(f, "Low (1-10 GFLOP)"),
            Self::Medium => write!(f, "Medium (10-100 GFLOP)"),
            Self::High => write!(f, "High (100 GFLOP - 1 TFLOP)"),
            Self::VeryHigh => write!(f, "Very High (1-10 TFLOP)"),
            Self::Extreme => write!(f, "Extreme (>10 TFLOP)"),
        }
    }
}

/// Memory footprint classification
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryRequirement {
    /// Tiny (<100MB)
    Tiny,

    /// Small (100MB-1GB)
    Small,

    /// Medium (1-10GB)
    Medium,

    /// Large (10-100GB)
    Large,

    /// Huge (>100GB)
    Huge,
}

impl fmt::Display for MemoryRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tiny => write!(f, "Tiny (<100MB)"),
            Self::Small => write!(f, "Small (100MB-1GB)"),
            Self::Medium => write!(f, "Medium (1-10GB)"),
            Self::Large => write!(f, "Large (10-100GB)"),
            Self::Huge => write!(f, "Huge (>100GB)"),
        }
    }
}

/// Parallelism level in workload
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParallelismLevel {
    /// Sequential (no parallelism)
    Sequential,

    /// Low parallelism (<10x speedup from parallelization)
    Low,

    /// Medium parallelism (10-100x speedup)
    Medium,

    /// High parallelism (100-1000x speedup)
    High,

    /// Very high parallelism (>1000x speedup)
    VeryHigh,
}

impl fmt::Display for ParallelismLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Sequential => write!(f, "Sequential"),
            Self::Low => write!(f, "Low (<10x)"),
            Self::Medium => write!(f, "Medium (10-100x)"),
            Self::High => write!(f, "High (100-1000x)"),
            Self::VeryHigh => write!(f, "Very High (>1000x)"),
        }
    }
}

/// GPU performance advantage over CPU
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpuAdvantage {
    /// Minimal (<2x speedup on GPU)
    Minimal,

    /// Moderate (2-5x speedup)
    Moderate,

    /// Significant (5-10x speedup)
    Significant,

    /// High (10-100x speedup)
    High,

    /// Very high (100-1000x speedup)
    VeryHigh,

    /// Critical (>1000x speedup, GPU essentially required)
    Critical,
}

impl fmt::Display for GpuAdvantage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Minimal => write!(f, "Minimal (<2x)"),
            Self::Moderate => write!(f, "Moderate (2-5x)"),
            Self::Significant => write!(f, "Significant (5-10x)"),
            Self::High => write!(f, "High (10-100x)"),
            Self::VeryHigh => write!(f, "Very High (100-1000x)"),
            Self::Critical => write!(f, "Critical (>1000x, GPU required)"),
        }
    }
}

/// Workload analyzer for characterizing workloads
pub struct WorkloadAnalyzer;

impl WorkloadAnalyzer {
    /// Create new workload analyzer
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Analyze workload and return characteristics
    #[must_use]
    pub fn analyze(&self, workload: &WorkloadSpec) -> WorkloadCharacteristics {
        match workload {
            WorkloadSpec::AiMl { workload } => self.analyze_aiml(workload),
            WorkloadSpec::Cuda { workload } => self.analyze_cuda(workload),
            WorkloadSpec::Gpu { .. } => self.analyze_gpu(),
            _ => WorkloadCharacteristics::default(),
        }
    }

    /// Analyze AI/ML workload
    fn analyze_aiml(&self, workload: &AiMlWorkload) -> WorkloadCharacteristics {
        // Compute intensity based on operation and model size
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

        // Memory requirement from model size and batch
        let memory_bytes = workload.estimate_total_memory_bytes();
        let memory_requirement = Self::classify_memory(memory_bytes);

        // Parallelism level
        let parallelism_level = match (&workload.operation, workload.batch_size) {
            (AiOperation::Training, bs) if bs >= 64 => ParallelismLevel::VeryHigh,
            (AiOperation::Training, bs) if bs >= 16 => ParallelismLevel::High,
            (AiOperation::Training, _) => ParallelismLevel::Medium,

            (AiOperation::Inference, bs) if bs >= 32 => ParallelismLevel::High,
            (AiOperation::Inference, bs) if bs >= 8 => ParallelismLevel::Medium,
            (AiOperation::Inference, _) => ParallelismLevel::Low,

            _ => ParallelismLevel::Medium,
        };

        // GPU advantage
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

        // CPU viability
        let cpu_viable = workload.is_cpu_viable();

        WorkloadCharacteristics {
            compute_intensity,
            memory_requirement,
            parallelism_level,
            gpu_advantage,
            cpu_viable,
            estimated_flops: None, // Could estimate from model architecture
        }
    }

    /// Analyze CUDA workload
    fn analyze_cuda(&self, workload: &CudaWorkload) -> WorkloadCharacteristics {
        let total_threads = workload.launch_config.total_threads();

        // Compute intensity from thread count (heuristic)
        let compute_intensity = match total_threads {
            0..=10_000 => ComputeIntensity::Low,
            10_001..=100_000 => ComputeIntensity::Medium,
            100_001..=1_000_000 => ComputeIntensity::High,
            1_000_001..=10_000_000 => ComputeIntensity::VeryHigh,
            _ => ComputeIntensity::Extreme,
        };

        // Memory requirement (rough estimate)
        let shared_mem = workload.launch_config.shared_mem_bytes;
        let blocks = workload.launch_config.total_blocks();
        let total_shared = shared_mem.saturating_mul(blocks as usize);
        let memory_requirement = Self::classify_memory(total_shared as u64);

        // Parallelism level
        let parallelism_level = match total_threads {
            0..=1_000 => ParallelismLevel::Low,
            1_001..=10_000 => ParallelismLevel::Medium,
            10_001..=100_000 => ParallelismLevel::High,
            _ => ParallelismLevel::VeryHigh,
        };

        // GPU advantage (CUDA is GPU-native, so high advantage)
        let gpu_advantage = if workload.has_memory_dependencies {
            GpuAdvantage::Moderate // Memory dependencies reduce GPU benefit
        } else if total_threads > 100_000 {
            GpuAdvantage::VeryHigh
        } else {
            GpuAdvantage::High
        };

        // CPU viability
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

    /// Analyze generic GPU workload
    fn analyze_gpu(&self) -> WorkloadCharacteristics {
        // Generic GPU workload - assume moderate characteristics
        WorkloadCharacteristics {
            compute_intensity: ComputeIntensity::High,
            memory_requirement: MemoryRequirement::Medium,
            parallelism_level: ParallelismLevel::High,
            gpu_advantage: GpuAdvantage::High,
            cpu_viable: false,
            estimated_flops: None,
        }
    }

    /// Classify memory requirement from bytes
    fn classify_memory(bytes: u64) -> MemoryRequirement {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workload::{
        AiFramework, AiMlWorkload, AiOperation, CudaLaunchConfig, CudaSource, CudaWorkload,
        GpuProgramSource, ModelSize, PythonSource, WasmModuleSource, WorkloadSpec,
    };
    use std::collections::HashMap;

    // --- WorkloadCharacteristics ---

    #[test]
    fn test_workload_characteristics_default() {
        let default = WorkloadCharacteristics::default();
        assert_eq!(default.compute_intensity, ComputeIntensity::Medium);
        assert_eq!(default.memory_requirement, MemoryRequirement::Medium);
        assert_eq!(default.parallelism_level, ParallelismLevel::Medium);
        assert_eq!(default.gpu_advantage, GpuAdvantage::Moderate);
        assert!(default.cpu_viable);
        assert_eq!(default.estimated_flops, None);
    }

    #[test]
    fn test_workload_characteristics_partial_eq() {
        let a = WorkloadCharacteristics::default();
        let b = WorkloadCharacteristics::default();
        assert_eq!(a, b);
        let c = WorkloadCharacteristics {
            cpu_viable: false,
            ..a.clone()
        };
        assert_ne!(a, c);
    }

    // --- ComputeIntensity Display & ordering ---

    #[test]
    fn test_compute_intensity_display() {
        assert_eq!(ComputeIntensity::Minimal.to_string(), "Minimal (<1 GFLOP)");
        assert_eq!(ComputeIntensity::Low.to_string(), "Low (1-10 GFLOP)");
        assert_eq!(
            ComputeIntensity::Medium.to_string(),
            "Medium (10-100 GFLOP)"
        );
        assert_eq!(
            ComputeIntensity::High.to_string(),
            "High (100 GFLOP - 1 TFLOP)"
        );
        assert_eq!(
            ComputeIntensity::VeryHigh.to_string(),
            "Very High (1-10 TFLOP)"
        );
        assert_eq!(ComputeIntensity::Extreme.to_string(), "Extreme (>10 TFLOP)");
    }

    #[test]
    fn test_compute_intensity_ordering() {
        assert!(ComputeIntensity::Minimal < ComputeIntensity::Extreme);
        assert!(ComputeIntensity::Low < ComputeIntensity::High);
        assert!(ComputeIntensity::Medium >= ComputeIntensity::Low);
    }

    // --- MemoryRequirement Display & ordering ---

    #[test]
    fn test_memory_requirement_display() {
        assert_eq!(MemoryRequirement::Tiny.to_string(), "Tiny (<100MB)");
        assert_eq!(MemoryRequirement::Small.to_string(), "Small (100MB-1GB)");
        assert_eq!(MemoryRequirement::Medium.to_string(), "Medium (1-10GB)");
        assert_eq!(MemoryRequirement::Large.to_string(), "Large (10-100GB)");
        assert_eq!(MemoryRequirement::Huge.to_string(), "Huge (>100GB)");
    }

    #[test]
    fn test_memory_requirement_ordering() {
        assert!(MemoryRequirement::Tiny < MemoryRequirement::Huge);
    }

    // --- ParallelismLevel Display ---

    #[test]
    fn test_parallelism_level_display() {
        assert_eq!(ParallelismLevel::Sequential.to_string(), "Sequential");
        assert_eq!(ParallelismLevel::Low.to_string(), "Low (<10x)");
        assert_eq!(ParallelismLevel::Medium.to_string(), "Medium (10-100x)");
        assert_eq!(ParallelismLevel::High.to_string(), "High (100-1000x)");
        assert_eq!(ParallelismLevel::VeryHigh.to_string(), "Very High (>1000x)");
    }

    // --- GpuAdvantage Display ---

    #[test]
    fn test_gpu_advantage_display() {
        assert_eq!(GpuAdvantage::Minimal.to_string(), "Minimal (<2x)");
        assert_eq!(GpuAdvantage::Moderate.to_string(), "Moderate (2-5x)");
        assert_eq!(GpuAdvantage::Significant.to_string(), "Significant (5-10x)");
        assert_eq!(GpuAdvantage::High.to_string(), "High (10-100x)");
        assert_eq!(GpuAdvantage::VeryHigh.to_string(), "Very High (100-1000x)");
        assert_eq!(
            GpuAdvantage::Critical.to_string(),
            "Critical (>1000x, GPU required)"
        );
    }

    // --- WorkloadAnalyzer ---

    #[test]
    fn test_workload_analyzer_new() {
        let analyzer = WorkloadAnalyzer::new();
        let _ = analyzer.analyze(&WorkloadSpec::default());
    }

    #[test]
    fn test_workload_analyzer_default() {
        let analyzer = WorkloadAnalyzer::default();
        let _chars = analyzer.analyze(&WorkloadSpec::default());
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
        assert!(matches!(
            chars.parallelism_level,
            ParallelismLevel::VeryHigh
        ));
    }

    #[test]
    fn test_aiml_inference_small_model() {
        let workload = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Inference,
            ModelSize::Small,
            8,
        );

        let analyzer = WorkloadAnalyzer::new();
        let chars = analyzer.analyze(&WorkloadSpec::AiMl {
            workload: workload.clone(),
        });

        assert_eq!(chars.compute_intensity, ComputeIntensity::Low);
        assert!(chars.cpu_viable);
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
        assert!(matches!(
            chars.parallelism_level,
            ParallelismLevel::VeryHigh
        ));
    }

    #[test]
    fn test_cuda_small_workload() {
        let launch = CudaLaunchConfig::linear(5_000, 256);
        let source = CudaSource::CudaCpp {
            source: "...".to_string(),
            entry_point: "kernel".to_string(),
        };
        let workload = CudaWorkload::new(source, launch);

        let analyzer = WorkloadAnalyzer::new();
        let chars = analyzer.analyze(&WorkloadSpec::Cuda {
            workload: workload.clone(),
        });

        assert!(chars.cpu_viable);
    }

    #[test]
    fn test_memory_classification() {
        assert_eq!(
            WorkloadAnalyzer::classify_memory(50_000_000),
            MemoryRequirement::Tiny
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(500_000_000),
            MemoryRequirement::Small
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(5_000_000_000),
            MemoryRequirement::Medium
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(50_000_000_000),
            MemoryRequirement::Large
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(500_000_000_000),
            MemoryRequirement::Huge
        );
    }

    #[test]
    fn test_classify_memory_boundaries() {
        assert_eq!(
            WorkloadAnalyzer::classify_memory(0),
            MemoryRequirement::Tiny
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(99_999_999),
            MemoryRequirement::Tiny
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(100_000_000),
            MemoryRequirement::Small
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(999_999_999),
            MemoryRequirement::Small
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(1_000_000_000),
            MemoryRequirement::Medium
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(9_999_999_999),
            MemoryRequirement::Medium
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(10_000_000_000),
            MemoryRequirement::Large
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(99_999_999_999),
            MemoryRequirement::Large
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(100_000_000_000),
            MemoryRequirement::Huge
        );
        assert_eq!(
            WorkloadAnalyzer::classify_memory(u64::MAX),
            MemoryRequirement::Huge
        );
    }

    // --- AI/ML comprehensive compute intensity ---

    #[test]
    fn test_aiml_compute_intensity_training_all_sizes() {
        let analyzer = WorkloadAnalyzer::new();
        let cases = [
            (ModelSize::XXLarge, ComputeIntensity::Extreme),
            (ModelSize::XLarge, ComputeIntensity::VeryHigh),
            (ModelSize::Large, ComputeIntensity::High),
            (ModelSize::Medium, ComputeIntensity::Medium),
            (ModelSize::Small, ComputeIntensity::Medium),
        ];
        for (model_size, expected) in cases {
            let w = AiMlWorkload::new(AiFramework::PyTorch, AiOperation::Training, model_size, 8);
            let c = analyzer.analyze(&WorkloadSpec::AiMl { workload: w });
            assert_eq!(c.compute_intensity, expected, "Training + {:?}", model_size);
        }
    }

    #[test]
    fn test_aiml_compute_intensity_finetuning() {
        let analyzer = WorkloadAnalyzer::new();
        for model_size in [ModelSize::XLarge, ModelSize::XXLarge] {
            let w = AiMlWorkload::new(AiFramework::PyTorch, AiOperation::FineTuning, model_size, 8);
            let c = analyzer.analyze(&WorkloadSpec::AiMl { workload: w });
            assert_eq!(c.compute_intensity, ComputeIntensity::VeryHigh);
        }
        for model_size in [ModelSize::Small, ModelSize::Medium, ModelSize::Large] {
            let w = AiMlWorkload::new(AiFramework::PyTorch, AiOperation::FineTuning, model_size, 8);
            let c = analyzer.analyze(&WorkloadSpec::AiMl { workload: w });
            assert_eq!(c.compute_intensity, ComputeIntensity::High);
        }
    }

    #[test]
    fn test_aiml_compute_intensity_inference() {
        let analyzer = WorkloadAnalyzer::new();
        assert_eq!(
            analyzer
                .analyze(&WorkloadSpec::AiMl {
                    workload: AiMlWorkload::new(
                        AiFramework::ONNX,
                        AiOperation::Inference,
                        ModelSize::XXLarge,
                        1,
                    ),
                })
                .compute_intensity,
            ComputeIntensity::High
        );
        assert_eq!(
            analyzer
                .analyze(&WorkloadSpec::AiMl {
                    workload: AiMlWorkload::new(
                        AiFramework::ONNX,
                        AiOperation::Inference,
                        ModelSize::XLarge,
                        1,
                    ),
                })
                .compute_intensity,
            ComputeIntensity::Medium
        );
        for size in [ModelSize::Small, ModelSize::Medium, ModelSize::Large] {
            let c = analyzer.analyze(&WorkloadSpec::AiMl {
                workload: AiMlWorkload::new(AiFramework::ONNX, AiOperation::Inference, size, 1),
            });
            assert_eq!(c.compute_intensity, ComputeIntensity::Low);
        }
    }

    #[test]
    fn test_aiml_compute_intensity_evaluation_quantization() {
        let analyzer = WorkloadAnalyzer::new();
        for op in [AiOperation::Evaluation, AiOperation::Quantization] {
            let w = AiMlWorkload::new(AiFramework::PyTorch, op, ModelSize::Large, 16);
            let c = analyzer.analyze(&WorkloadSpec::AiMl { workload: w });
            assert_eq!(c.compute_intensity, ComputeIntensity::Medium);
        }
    }

    // --- AI/ML parallelism ---

    #[test]
    fn test_aiml_parallelism_training() {
        let analyzer = WorkloadAnalyzer::new();
        assert_eq!(
            analyzer
                .analyze(&WorkloadSpec::AiMl {
                    workload: AiMlWorkload::new(
                        AiFramework::PyTorch,
                        AiOperation::Training,
                        ModelSize::Large,
                        64,
                    ),
                })
                .parallelism_level,
            ParallelismLevel::VeryHigh
        );
        assert_eq!(
            analyzer
                .analyze(&WorkloadSpec::AiMl {
                    workload: AiMlWorkload::new(
                        AiFramework::PyTorch,
                        AiOperation::Training,
                        ModelSize::Large,
                        16,
                    ),
                })
                .parallelism_level,
            ParallelismLevel::High
        );
        assert_eq!(
            analyzer
                .analyze(&WorkloadSpec::AiMl {
                    workload: AiMlWorkload::new(
                        AiFramework::PyTorch,
                        AiOperation::Training,
                        ModelSize::Large,
                        8,
                    ),
                })
                .parallelism_level,
            ParallelismLevel::Medium
        );
    }

    #[test]
    fn test_aiml_parallelism_inference() {
        let analyzer = WorkloadAnalyzer::new();
        assert_eq!(
            analyzer
                .analyze(&WorkloadSpec::AiMl {
                    workload: AiMlWorkload::new(
                        AiFramework::ONNX,
                        AiOperation::Inference,
                        ModelSize::Small,
                        32,
                    ),
                })
                .parallelism_level,
            ParallelismLevel::High
        );
        assert_eq!(
            analyzer
                .analyze(&WorkloadSpec::AiMl {
                    workload: AiMlWorkload::new(
                        AiFramework::ONNX,
                        AiOperation::Inference,
                        ModelSize::Small,
                        8,
                    ),
                })
                .parallelism_level,
            ParallelismLevel::Medium
        );
        assert_eq!(
            analyzer
                .analyze(&WorkloadSpec::AiMl {
                    workload: AiMlWorkload::new(
                        AiFramework::ONNX,
                        AiOperation::Inference,
                        ModelSize::Small,
                        4,
                    ),
                })
                .parallelism_level,
            ParallelismLevel::Low
        );
    }

    #[test]
    fn test_aiml_parallelism_finetuning_medium() {
        let analyzer = WorkloadAnalyzer::new();
        let w = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::FineTuning,
            ModelSize::Medium,
            128,
        );
        let c = analyzer.analyze(&WorkloadSpec::AiMl { workload: w });
        assert_eq!(c.parallelism_level, ParallelismLevel::Medium);
    }

    // --- AI/ML GPU advantage ---

    #[test]
    fn test_aiml_gpu_advantage_training() {
        let analyzer = WorkloadAnalyzer::new();
        for size in [ModelSize::XLarge, ModelSize::XXLarge] {
            let c = analyzer.analyze(&WorkloadSpec::AiMl {
                workload: AiMlWorkload::new(AiFramework::PyTorch, AiOperation::Training, size, 8),
            });
            assert_eq!(c.gpu_advantage, GpuAdvantage::Critical);
        }
        let c = analyzer.analyze(&WorkloadSpec::AiMl {
            workload: AiMlWorkload::new(
                AiFramework::PyTorch,
                AiOperation::Training,
                ModelSize::Large,
                8,
            ),
        });
        assert_eq!(c.gpu_advantage, GpuAdvantage::VeryHigh);
        for size in [ModelSize::Small, ModelSize::Medium] {
            let c = analyzer.analyze(&WorkloadSpec::AiMl {
                workload: AiMlWorkload::new(AiFramework::PyTorch, AiOperation::Training, size, 8),
            });
            assert_eq!(c.gpu_advantage, GpuAdvantage::High);
        }
    }

    #[test]
    fn test_aiml_gpu_advantage_inference() {
        let analyzer = WorkloadAnalyzer::new();
        for size in [ModelSize::XLarge, ModelSize::XXLarge] {
            let c = analyzer.analyze(&WorkloadSpec::AiMl {
                workload: AiMlWorkload::new(AiFramework::ONNX, AiOperation::Inference, size, 1),
            });
            assert_eq!(c.gpu_advantage, GpuAdvantage::High);
        }
        let c = analyzer.analyze(&WorkloadSpec::AiMl {
            workload: AiMlWorkload::new(
                AiFramework::ONNX,
                AiOperation::Inference,
                ModelSize::Small,
                1,
            ),
        });
        assert_eq!(c.gpu_advantage, GpuAdvantage::Moderate);
    }

    // --- CUDA comprehensive ---

    #[test]
    fn test_cuda_compute_intensity_by_threads() {
        let analyzer = WorkloadAnalyzer::new();
        let src = CudaSource::CudaCpp {
            source: "x".to_string(),
            entry_point: "k".to_string(),
        };
        // linear(num_threads, block_size): total = ceil(num/block)*block. Use block divides num for exact totals.
        let cases: &[(u32, u32, ComputeIntensity)] = &[
            (5_000, 256, ComputeIntensity::Low),           // ~5120
            (10_000, 100, ComputeIntensity::Low),          // exactly 10_000
            (10_001, 256, ComputeIntensity::Medium),       // ~10240
            (100_000, 100, ComputeIntensity::Medium),      // exactly 100_000
            (100_001, 256, ComputeIntensity::High),        // ~100096
            (1_000_000, 1000, ComputeIntensity::High),     // exactly 1M
            (1_000_001, 256, ComputeIntensity::VeryHigh),  // ~1000192
            (10_000_000, 100, ComputeIntensity::VeryHigh), // exactly 10M
            (10_000_001, 256, ComputeIntensity::Extreme),  // ~10M -> Extreme
        ];
        for (num_threads, block_size, expected) in cases {
            let launch = CudaLaunchConfig::linear(*num_threads, *block_size);
            let total = launch.total_threads();
            let w = CudaWorkload::new(src.clone(), launch);
            let c = analyzer.analyze(&WorkloadSpec::Cuda { workload: w });
            assert_eq!(c.compute_intensity, *expected, "total_threads={}", total);
        }
    }

    #[test]
    fn test_cuda_parallelism_by_threads() {
        let analyzer = WorkloadAnalyzer::new();
        let src = CudaSource::CudaCpp {
            source: "x".to_string(),
            entry_point: "k".to_string(),
        };
        // Use block_size equal to num_threads for exact totals; otherwise linear rounds up
        let cases: &[(u32, u32, ParallelismLevel)] = &[
            (500, 500, ParallelismLevel::Low),          // total=500
            (1_000, 1000, ParallelismLevel::Low),       // total=1000
            (1_001, 256, ParallelismLevel::Medium),     // total=1024
            (10_000, 100, ParallelismLevel::Medium),    // total=10000
            (10_001, 256, ParallelismLevel::High),      // total=~10240
            (100_000, 100, ParallelismLevel::High),     // total=100000
            (100_001, 256, ParallelismLevel::VeryHigh), // total=~100352
        ];
        for (num_threads, block_size, expected) in cases {
            let launch = CudaLaunchConfig::linear(*num_threads, *block_size);
            let total = launch.total_threads();
            let w = CudaWorkload::new(src.clone(), launch);
            let c = analyzer.analyze(&WorkloadSpec::Cuda { workload: w });
            assert_eq!(c.parallelism_level, *expected, "total_threads={}", total);
        }
    }

    #[test]
    fn test_cuda_memory_dependencies_reduces_gpu_advantage() {
        let analyzer = WorkloadAnalyzer::new();
        let src = CudaSource::CudaCpp {
            source: "x".to_string(),
            entry_point: "k".to_string(),
        };
        let launch = CudaLaunchConfig::linear(500_000, 256);
        let with_deps = CudaWorkload::new(src.clone(), launch).with_memory_dependencies();
        let without_deps = CudaWorkload::new(src, launch);

        let c_deps = analyzer.analyze(&WorkloadSpec::Cuda {
            workload: with_deps,
        });
        let c_no_deps = analyzer.analyze(&WorkloadSpec::Cuda {
            workload: without_deps,
        });

        assert_eq!(c_deps.gpu_advantage, GpuAdvantage::Moderate);
        assert_eq!(c_no_deps.gpu_advantage, GpuAdvantage::VeryHigh);
    }

    #[test]
    fn test_cuda_estimated_flops_propagated() {
        let analyzer = WorkloadAnalyzer::new();
        let src = CudaSource::CudaCpp {
            source: "x".to_string(),
            entry_point: "k".to_string(),
        };
        let launch = CudaLaunchConfig::linear(10_000, 256);
        let w = CudaWorkload::new(src, launch).with_estimated_flops(42_000_000);
        let c = analyzer.analyze(&WorkloadSpec::Cuda { workload: w });
        assert_eq!(c.estimated_flops, Some(42_000_000));
    }

    #[test]
    fn test_cuda_zero_threads() {
        let analyzer = WorkloadAnalyzer::new();
        let launch = CudaLaunchConfig::new((0, 1, 1), (256, 1, 1));
        let src = CudaSource::CudaCpp {
            source: "x".to_string(),
            entry_point: "k".to_string(),
        };
        let w = CudaWorkload::new(src, launch);
        let c = analyzer.analyze(&WorkloadSpec::Cuda { workload: w });
        assert_eq!(c.compute_intensity, ComputeIntensity::Low);
        assert_eq!(c.parallelism_level, ParallelismLevel::Low);
    }

    #[test]
    fn test_cuda_shared_memory_affects_classification() {
        let analyzer = WorkloadAnalyzer::new();
        let src = CudaSource::CudaCpp {
            source: "x".to_string(),
            entry_point: "k".to_string(),
        };
        let launch = CudaLaunchConfig::linear(1024, 256).with_shared_mem(100_000_000);
        let w = CudaWorkload::new(src, launch);
        let c = analyzer.analyze(&WorkloadSpec::Cuda { workload: w });
        assert_eq!(c.memory_requirement, MemoryRequirement::Small);
    }

    // --- GPU workload ---

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
        assert_eq!(c.memory_requirement, MemoryRequirement::Medium);
        assert_eq!(c.parallelism_level, ParallelismLevel::High);
        assert_eq!(c.gpu_advantage, GpuAdvantage::High);
        assert!(!c.cpu_viable);
        assert_eq!(c.estimated_flops, None);
    }

    // --- WorkloadSpec fallback (Native, Wasm, Container, Python) ---

    #[test]
    fn test_native_workload_returns_default() {
        let analyzer = WorkloadAnalyzer::new();
        let spec = WorkloadSpec::default();
        let c = analyzer.analyze(&spec);
        assert_eq!(
            c.compute_intensity,
            WorkloadCharacteristics::default().compute_intensity
        );
        assert!(c.cpu_viable);
    }

    #[test]
    fn test_wasm_workload_returns_default() {
        let analyzer = WorkloadAnalyzer::new();
        let spec = WorkloadSpec::Wasm {
            module: WasmModuleSource::Bytes {
                data: vec![0x00, 0x61, 0x73, 0x6d],
            },
            args: None,
            wasi_config: None,
            env_vars: HashMap::new(),
        };
        let c = analyzer.analyze(&spec);
        assert_eq!(c.compute_intensity, ComputeIntensity::Medium);
        assert!(c.cpu_viable);
    }

    #[test]
    fn test_container_workload_returns_default() {
        let analyzer = WorkloadAnalyzer::new();
        let spec = WorkloadSpec::Container {
            image: "alpine:latest".to_string(),
            command: None,
            args: None,
            env_vars: HashMap::new(),
            working_dir: None,
            volumes: vec![],
            ports: vec![],
            registry_auth: None,
        };
        let c = analyzer.analyze(&spec);
        assert_eq!(c.compute_intensity, ComputeIntensity::Medium);
    }

    #[test]
    fn test_python_workload_returns_default() {
        let analyzer = WorkloadAnalyzer::new();
        let spec = WorkloadSpec::Python {
            source: PythonSource::Code {
                code: "print('hi')".to_string(),
            },
            python_version: None,
            requirements: vec![],
            env_vars: HashMap::new(),
        };
        let c = analyzer.analyze(&spec);
        assert_eq!(c.compute_intensity, ComputeIntensity::Medium);
    }

    // --- Edge cases ---

    #[test]
    fn test_aiml_batch_size_zero() {
        let analyzer = WorkloadAnalyzer::new();
        let w = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            0,
        );
        let c = analyzer.analyze(&WorkloadSpec::AiMl { workload: w });
        assert_eq!(c.parallelism_level, ParallelismLevel::Medium);
    }

    #[test]
    fn test_aiml_batch_size_boundary_64() {
        let analyzer = WorkloadAnalyzer::new();
        let w = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            64,
        );
        let c = analyzer.analyze(&WorkloadSpec::AiMl { workload: w });
        assert_eq!(c.parallelism_level, ParallelismLevel::VeryHigh);
    }

    #[test]
    fn test_aiml_batch_size_boundary_63() {
        let analyzer = WorkloadAnalyzer::new();
        let w = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::Large,
            63,
        );
        let c = analyzer.analyze(&WorkloadSpec::AiMl { workload: w });
        assert_eq!(c.parallelism_level, ParallelismLevel::High);
    }

    #[test]
    fn test_aiml_cpu_viable_delegates_to_workload() {
        let analyzer = WorkloadAnalyzer::new();
        let viable = AiMlWorkload::new(
            AiFramework::ONNX,
            AiOperation::Inference,
            ModelSize::Small,
            16,
        );
        let not_viable = AiMlWorkload::new(
            AiFramework::PyTorch,
            AiOperation::Training,
            ModelSize::XLarge,
            64,
        );
        let c_ok = analyzer.analyze(&WorkloadSpec::AiMl { workload: viable });
        let c_no = analyzer.analyze(&WorkloadSpec::AiMl {
            workload: not_viable,
        });
        assert!(c_ok.cpu_viable);
        assert!(!c_no.cpu_viable);
    }
}
