// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload characteristic classifications

use std::fmt;

/// Workload analysis result containing characteristics
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkloadCharacteristics {
    /// FLOP intensity classification.
    pub compute_intensity: ComputeIntensity,
    /// Memory footprint classification.
    pub memory_requirement: MemoryRequirement,
    /// Parallelism / scalability classification.
    pub parallelism_level: ParallelismLevel,
    /// GPU speedup potential.
    pub gpu_advantage: GpuAdvantage,
    /// Whether CPU execution is viable.
    pub cpu_viable: bool,
    /// Estimated FLOPs if known.
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

/// Compute intensity (FLOP) classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ComputeIntensity {
    /// &lt;1 GFLOP.
    Minimal,
    /// 1–10 GFLOP.
    Low,
    /// 10–100 GFLOP.
    Medium,
    /// 100 GFLOP–1 TFLOP.
    High,
    /// 1–10 TFLOP.
    VeryHigh,
    /// &gt;10 TFLOP.
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

/// Memory footprint classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MemoryRequirement {
    /// &lt;100 MB.
    Tiny,
    /// 100 MB–1 GB.
    Small,
    /// 1–10 GB.
    Medium,
    /// 10–100 GB.
    Large,
    /// &gt;100 GB.
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

/// Parallelism / scalability classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ParallelismLevel {
    /// Single-threaded.
    Sequential,
    /// &lt;10x speedup.
    Low,
    /// 10–100x speedup.
    Medium,
    /// 100–1000x speedup.
    High,
    /// &gt;1000x speedup.
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

/// GPU speedup potential classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum GpuAdvantage {
    /// &lt;2x speedup.
    Minimal,
    /// 2–5x speedup.
    Moderate,
    /// 5–10x speedup.
    Significant,
    /// 10–100x speedup.
    High,
    /// 100–1000x speedup.
    VeryHigh,
    /// &gt;1000x; GPU required.
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

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_characteristics() {
        let wc = WorkloadCharacteristics::default();
        assert_eq!(wc.compute_intensity, ComputeIntensity::Medium);
        assert_eq!(wc.memory_requirement, MemoryRequirement::Medium);
        assert_eq!(wc.parallelism_level, ParallelismLevel::Medium);
        assert_eq!(wc.gpu_advantage, GpuAdvantage::Moderate);
        assert!(wc.cpu_viable);
        assert!(wc.estimated_flops.is_none());
    }

    #[test]
    fn test_compute_intensity_display() {
        assert!(ComputeIntensity::Minimal.to_string().contains("Minimal"));
        assert!(ComputeIntensity::Low.to_string().contains("Low"));
        assert!(ComputeIntensity::Medium.to_string().contains("Medium"));
        assert!(ComputeIntensity::High.to_string().contains("High"));
        assert!(ComputeIntensity::VeryHigh.to_string().contains("Very High"));
        assert!(ComputeIntensity::Extreme.to_string().contains("Extreme"));
    }

    #[test]
    fn test_memory_requirement_display() {
        assert!(MemoryRequirement::Tiny.to_string().contains("Tiny"));
        assert!(MemoryRequirement::Small.to_string().contains("Small"));
        assert!(MemoryRequirement::Large.to_string().contains("Large"));
        assert!(MemoryRequirement::Huge.to_string().contains("Huge"));
    }

    #[test]
    fn test_parallelism_level_display() {
        assert!(
            ParallelismLevel::Sequential
                .to_string()
                .contains("Sequential")
        );
        assert!(ParallelismLevel::High.to_string().contains("High"));
        assert!(ParallelismLevel::VeryHigh.to_string().contains("Very High"));
    }

    #[test]
    fn test_gpu_advantage_display() {
        assert!(GpuAdvantage::Minimal.to_string().contains("Minimal"));
        assert!(GpuAdvantage::Critical.to_string().contains("Critical"));
        assert!(GpuAdvantage::VeryHigh.to_string().contains("Very High"));
    }

    #[test]
    fn test_compute_intensity_ordering() {
        assert!(ComputeIntensity::Minimal < ComputeIntensity::Low);
        assert!(ComputeIntensity::Low < ComputeIntensity::Medium);
        assert!(ComputeIntensity::High < ComputeIntensity::VeryHigh);
        assert!(ComputeIntensity::VeryHigh < ComputeIntensity::Extreme);
    }

    #[test]
    fn test_gpu_advantage_ordering() {
        assert!(GpuAdvantage::Minimal < GpuAdvantage::Critical);
        assert!(GpuAdvantage::Moderate < GpuAdvantage::Significant);
    }

    #[test]
    fn test_characteristics_with_flops() {
        let wc = WorkloadCharacteristics {
            compute_intensity: ComputeIntensity::Extreme,
            memory_requirement: MemoryRequirement::Huge,
            parallelism_level: ParallelismLevel::VeryHigh,
            gpu_advantage: GpuAdvantage::Critical,
            cpu_viable: false,
            estimated_flops: Some(10_000_000_000_000),
        };
        assert_eq!(wc.compute_intensity, ComputeIntensity::Extreme);
        assert!(!wc.cpu_viable);
        assert_eq!(wc.estimated_flops, Some(10_000_000_000_000));
    }
}
