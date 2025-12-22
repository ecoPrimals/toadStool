//! CUDA workload types for compatibility layer
//!
//! Defines CUDA-specific workload characteristics to enable intelligent
//! backend selection and universal CUDA compatibility.

use serde::{Deserialize, Serialize};
use std::fmt;

/// CUDA kernel source representation
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CudaSource {
    /// CUDA C++ source code
    CudaCpp { source: String, entry_point: String },

    /// PTX (NVIDIA's assembly-like IR)
    Ptx { source: String, entry_point: String },

    /// Compiled CUDA binary (cubin)
    CuBin {
        binary: Vec<u8>,
        entry_point: String,
    },

    /// Path to CUDA source file
    File {
        path: std::path::PathBuf,
        entry_point: String,
    },
}

impl fmt::Display for CudaSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CudaCpp { entry_point, .. } => write!(f, "CUDA C++ ({})", entry_point),
            Self::Ptx { entry_point, .. } => write!(f, "PTX ({})", entry_point),
            Self::CuBin { entry_point, .. } => write!(f, "CuBin ({})", entry_point),
            Self::File { path, entry_point } => {
                write!(f, "File: {} ({})", path.display(), entry_point)
            }
        }
    }
}

/// Preferred execution backend for CUDA workload
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CudaBackend {
    /// Native NVIDIA CUDA (100% compatibility, best performance)
    NativeNvidia,

    /// GPU translation via ToadStool (80-95% performance, any GPU)
    TranslatedGpu,

    /// CPU parallel execution (50-70% performance on high-core systems)
    CpuParallel,

    /// CPU sequential fallback (5-10% performance, always works)
    CpuSequential,

    /// Automatic selection based on available hardware
    Automatic,
}

impl fmt::Display for CudaBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NativeNvidia => write!(f, "Native NVIDIA CUDA"),
            Self::TranslatedGpu => write!(f, "Translated GPU"),
            Self::CpuParallel => write!(f, "CPU Parallel"),
            Self::CpuSequential => write!(f, "CPU Sequential"),
            Self::Automatic => write!(f, "Automatic"),
        }
    }
}

impl Default for CudaBackend {
    fn default() -> Self {
        Self::Automatic
    }
}

/// CUDA launch configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CudaLaunchConfig {
    /// Grid dimensions (blocks in each dimension)
    pub grid_dim: (u32, u32, u32),

    /// Block dimensions (threads per block in each dimension)
    pub block_dim: (u32, u32, u32),

    /// Shared memory per block (bytes)
    pub shared_mem_bytes: usize,
}

impl CudaLaunchConfig {
    /// Create new launch configuration
    #[must_use]
    pub const fn new(grid_dim: (u32, u32, u32), block_dim: (u32, u32, u32)) -> Self {
        Self {
            grid_dim,
            block_dim,
            shared_mem_bytes: 0,
        }
    }

    /// Create 1D launch configuration
    #[must_use]
    pub const fn linear(num_threads: u32, threads_per_block: u32) -> Self {
        let blocks = num_threads.div_ceil(threads_per_block);
        Self::new((blocks, 1, 1), (threads_per_block, 1, 1))
    }

    /// Set shared memory requirement
    #[must_use]
    pub const fn with_shared_mem(mut self, bytes: usize) -> Self {
        self.shared_mem_bytes = bytes;
        self
    }

    /// Calculate total number of threads
    #[must_use]
    pub const fn total_threads(&self) -> u64 {
        (self.grid_dim.0 as u64 * self.grid_dim.1 as u64 * self.grid_dim.2 as u64)
            * (self.block_dim.0 as u64 * self.block_dim.1 as u64 * self.block_dim.2 as u64)
    }

    /// Calculate total number of blocks
    #[must_use]
    pub const fn total_blocks(&self) -> u64 {
        self.grid_dim.0 as u64 * self.grid_dim.1 as u64 * self.grid_dim.2 as u64
    }
}

/// Complete CUDA workload specification
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CudaWorkload {
    /// CUDA kernel source
    pub source: CudaSource,

    /// Launch configuration
    pub launch_config: CudaLaunchConfig,

    /// CUDA compute capability requirement (e.g., "7.5", "8.0")
    pub compute_capability: Option<String>,

    /// Preferred execution backend
    pub preferred_backend: CudaBackend,

    /// Estimated FLOPS for this kernel (for scheduling)
    pub estimated_flops: Option<u64>,

    /// Whether this kernel has memory dependencies (affects parallelism)
    pub has_memory_dependencies: bool,
}

impl CudaWorkload {
    /// Create new CUDA workload
    #[must_use]
    pub const fn new(source: CudaSource, launch_config: CudaLaunchConfig) -> Self {
        Self {
            source,
            launch_config,
            compute_capability: None,
            preferred_backend: CudaBackend::Automatic,
            estimated_flops: None,
            has_memory_dependencies: false,
        }
    }

    /// Set compute capability requirement
    #[must_use]
    pub fn with_compute_capability(mut self, capability: impl Into<String>) -> Self {
        self.compute_capability = Some(capability.into());
        self
    }

    /// Set preferred backend
    #[must_use]
    pub const fn with_preferred_backend(mut self, backend: CudaBackend) -> Self {
        self.preferred_backend = backend;
        self
    }

    /// Set estimated FLOPS
    #[must_use]
    pub const fn with_estimated_flops(mut self, flops: u64) -> Self {
        self.estimated_flops = Some(flops);
        self
    }

    /// Mark as having memory dependencies
    #[must_use]
    pub const fn with_memory_dependencies(mut self) -> Self {
        self.has_memory_dependencies = true;
        self
    }

    /// Check if this is a compute-intensive workload
    #[must_use]
    pub const fn is_compute_intensive(&self) -> bool {
        // Heuristic: >1M threads likely compute-bound
        self.launch_config.total_threads() > 1_000_000
    }

    /// Check if CPU execution is viable
    #[must_use]
    pub const fn is_cpu_viable(&self) -> bool {
        // Small workloads can run on CPU reasonably
        !self.has_memory_dependencies && self.launch_config.total_threads() < 100_000
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_launch_config_calculations() {
        let config = CudaLaunchConfig::linear(1024, 256);
        assert_eq!(config.grid_dim, (4, 1, 1));
        assert_eq!(config.block_dim, (256, 1, 1));
        assert_eq!(config.total_threads(), 1024);
        assert_eq!(config.total_blocks(), 4);
    }

    #[test]
    fn test_launch_config_with_shared_mem() {
        let config = CudaLaunchConfig::linear(512, 128).with_shared_mem(4096);
        assert_eq!(config.shared_mem_bytes, 4096);
    }

    #[test]
    fn test_workload_builder() {
        let source = CudaSource::CudaCpp {
            source: "...".to_string(),
            entry_point: "kernel".to_string(),
        };
        let launch = CudaLaunchConfig::linear(1000, 256);

        let workload = CudaWorkload::new(source, launch)
            .with_compute_capability("7.5")
            .with_preferred_backend(CudaBackend::TranslatedGpu)
            .with_estimated_flops(1_000_000);

        assert_eq!(workload.compute_capability.as_deref(), Some("7.5"));
        assert_eq!(workload.preferred_backend, CudaBackend::TranslatedGpu);
        assert_eq!(workload.estimated_flops, Some(1_000_000));
    }

    #[test]
    fn test_compute_intensity_detection() {
        let large_launch = CudaLaunchConfig::linear(10_000_000, 256);
        let large_workload = CudaWorkload::new(
            CudaSource::CudaCpp {
                source: "...".to_string(),
                entry_point: "kernel".to_string(),
            },
            large_launch,
        );
        assert!(large_workload.is_compute_intensive());

        let small_launch = CudaLaunchConfig::linear(1000, 256);
        let small_workload = CudaWorkload::new(
            CudaSource::CudaCpp {
                source: "...".to_string(),
                entry_point: "kernel".to_string(),
            },
            small_launch,
        );
        assert!(!small_workload.is_compute_intensive());
    }

    #[test]
    fn test_cpu_viability() {
        let viable_launch = CudaLaunchConfig::linear(10_000, 256);
        let viable = CudaWorkload::new(
            CudaSource::CudaCpp {
                source: "...".to_string(),
                entry_point: "kernel".to_string(),
            },
            viable_launch,
        );
        assert!(viable.is_cpu_viable());

        let not_viable = viable.with_memory_dependencies();
        assert!(!not_viable.is_cpu_viable());
    }
}
