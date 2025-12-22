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
    use crate::workload::{AiFramework, AiMlWorkload, AiOperation, ModelSize};
    use crate::workload::{CudaLaunchConfig, CudaSource, CudaWorkload};

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
}
