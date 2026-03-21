// SPDX-License-Identifier: AGPL-3.0-only
//! Universal Capability-Based Compute Abstractions
//!
//! This module provides hardware-agnostic compute abstractions where:
//! - Workloads describe WHAT they need (capabilities)
//! - Resources describe WHAT they can do (capabilities)
//! - Scheduler matches workloads to resources
//!
//! This enables:
//! - GPU, CPU, TPU, FPGA, Quantum, etc. as equal compute resources
//! - Automatic resource selection based on capabilities
//! - Future-proof architecture for unknown compute paradigms

use crate::types::*;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use toadstool::error::ToadStoolResult;
use uuid::Uuid;

/// Parallelism model supported by compute resource.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ParallelismModel {
    /// SIMD — Single Instruction Multiple Data (CPU vectors, AVX, NEON).
    Simd {
        /// Vector width in elements.
        width: u32,
    },

    /// SIMT — Single Instruction Multiple Threads (GPU threads).
    Simt {
        /// Maximum concurrent threads.
        max_threads: u64,
    },

    /// Task-based parallelism (CPU cores, thread pools).
    Task {
        /// Maximum concurrent tasks.
        max_tasks: u32,
    },

    /// Dataflow/Stream processing (specialized accelerators)
    Dataflow,

    /// Custom/future parallelism models
    Custom(String),
}

/// Parallelism capabilities of a compute resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParallelismCapabilities {
    /// Maximum parallel threads/work items
    pub max_parallel_threads: u64,

    /// Parallelism model
    pub model: ParallelismModel,

    /// Maximum work group/block size (for SIMT)
    pub max_work_group_size: Option<u32>,

    /// SIMD vector width (for SIMD)
    pub simd_width: Option<u32>,

    /// Supports nested parallelism
    pub nested_parallelism: bool,
}

/// Memory characteristics of a compute resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryCapabilities {
    /// Total memory available (bytes)
    pub total_bytes: u64,

    /// Memory bandwidth (bytes/second)
    pub bandwidth_bytes_per_sec: u64,

    /// Unified memory with host
    pub unified_memory: bool,

    /// Supports zero-copy operations
    pub zero_copy: bool,

    /// Cache hierarchy (L1, L2, L3, etc.)
    pub cache_levels: Vec<CacheLevel>,

    /// Memory access pattern optimization
    pub access_patterns: Vec<MemoryAccessPattern>,
}

/// Cache level description.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheLevel {
    /// Cache level (1, 2, 3, etc.).
    pub level: u8,
    /// Size in bytes.
    pub size_bytes: u64,
    /// Cache line size in bytes.
    pub line_size_bytes: u32,
    /// Cache associativity (ways); 0 = fully associative or unknown.
    pub associativity: u32,
}

/// Optimized memory access patterns.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MemoryAccessPattern {
    /// Sequential access.
    Sequential,
    /// Strided access.
    Strided,
    /// Random access.
    Random,
    /// Coalesced access (GPU-friendly).
    Coalesced,
}

/// Precision support capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrecisionCapabilities {
    /// 16-bit float (half precision)
    pub fp16: bool,

    /// 32-bit float (single precision)
    pub fp32: bool,

    /// 64-bit float (double precision)
    pub fp64: bool,

    /// 8-bit integer
    pub int8: bool,

    /// 16-bit integer
    pub int16: bool,

    /// 32-bit integer
    pub int32: bool,

    /// 64-bit integer
    pub int64: bool,

    /// Mixed precision operations
    pub mixed_precision: bool,
}

/// Specialized operation support
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationCapabilities {
    /// General compute operations
    pub general_compute: bool,

    /// Matrix multiplication
    pub matrix_multiply: bool,

    /// Tensor operations (reshape, transpose, etc.)
    pub tensor_ops: bool,

    /// Convolution operations
    pub convolution: bool,

    /// FFT (Fast Fourier Transform)
    pub fft: bool,

    /// Sorting and reduction operations
    pub reduction_ops: bool,

    /// Atomic operations
    pub atomic_ops: bool,

    /// Branching/control flow efficiency
    pub branching_efficiency: BranchingEfficiency,

    /// Custom operations
    pub custom_ops: Vec<String>,
}

/// How well the resource handles branching
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchingEfficiency {
    /// Excellent (CPU-like)
    High,

    /// Good (modern GPUs)
    Medium,

    /// Poor (SIMT divergence)
    Low,
}

/// Performance characteristics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceCapabilities {
    /// Peak FLOPS (floating point operations per second)
    pub peak_flops: f64,

    /// Peak integer ops per second
    pub peak_iops: f64,

    /// Typical power consumption (watts)
    pub power_watts: f32,

    /// Latency to start execution (microseconds)
    pub startup_latency_us: u64,

    /// Sustained performance as percentage of peak
    pub sustained_performance_percent: f32,
}

/// Complete capability description for a compute resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeCapabilities {
    /// Parallelism characteristics
    pub parallelism: ParallelismCapabilities,

    /// Memory characteristics
    pub memory: MemoryCapabilities,

    /// Precision support
    pub precision: PrecisionCapabilities,

    /// Operation support
    pub operations: OperationCapabilities,

    /// Performance characteristics
    pub performance: PerformanceCapabilities,

    /// Resource type hint (for debugging/logging)
    pub resource_type: String,
}

impl ComputeCapabilities {
    /// Check if capabilities meet requirements
    pub fn meets_requirements(&self, requirements: &ComputeRequirements) -> bool {
        // Check parallelism
        if self.parallelism.max_parallel_threads < requirements.min_parallel_threads {
            return false;
        }

        // Check memory
        if self.memory.total_bytes < requirements.memory_bytes {
            return false;
        }

        // Check precision
        if !self.precision.supports(requirements.precision) {
            return false;
        }

        // Check operations
        for op in &requirements.operations {
            if !self.operations.supports(op) {
                return false;
            }
        }

        true
    }

    /// Calculate capability score for a workload (0.0 = poor, 1.0 = perfect)
    pub fn score_for_workload(&self, requirements: &ComputeRequirements) -> f64 {
        let mut score = 0.0;
        let mut factors = 0;

        // Parallelism score (more is better, but diminishing returns)
        let parallelism_ratio =
            self.parallelism.max_parallel_threads as f64 / requirements.min_parallel_threads as f64;
        score += (parallelism_ratio.min(2.0) / 2.0).min(1.0);
        factors += 1;

        // Memory score (exact match is best)
        let memory_ratio = self.memory.total_bytes as f64 / requirements.memory_bytes as f64;
        score += if (1.0..=2.0).contains(&memory_ratio) {
            1.0
        } else if memory_ratio < 1.0 {
            memory_ratio
        } else {
            0.5 // Too much memory wastes resources
        };
        factors += 1;

        // Precision score (exact match or better)
        score += if self.precision.supports(requirements.precision) {
            1.0
        } else {
            0.0
        };
        factors += 1;

        // Operation score (all required ops supported)
        let supported_ops = requirements
            .operations
            .iter()
            .filter(|op| self.operations.supports(op))
            .count();
        score += supported_ops as f64 / requirements.operations.len().max(1) as f64;
        factors += 1;

        score / factors as f64
    }
}

impl PrecisionCapabilities {
    /// Check if precision is supported
    pub const fn supports(&self, precision: Precision) -> bool {
        match precision {
            Precision::Fp16 => self.fp16,
            Precision::Fp32 => self.fp32,
            Precision::Fp64 => self.fp64,
            Precision::Int8 => self.int8,
            Precision::Int16 => self.int16,
            Precision::Int32 => self.int32,
            Precision::Int64 => self.int64,
            Precision::Mixed => self.mixed_precision,
        }
    }
}

impl OperationCapabilities {
    /// Check if operation is supported
    pub fn supports(&self, operation: &Operation) -> bool {
        match operation {
            Operation::GeneralCompute => self.general_compute,
            Operation::MatrixMultiply => self.matrix_multiply,
            Operation::TensorOps => self.tensor_ops,
            Operation::Convolution => self.convolution,
            Operation::Fft => self.fft,
            Operation::Reduction => self.reduction_ops,
            Operation::Atomic => self.atomic_ops,
            Operation::BranchHeavy => matches!(
                self.branching_efficiency,
                BranchingEfficiency::High | BranchingEfficiency::Medium
            ),
            Operation::Custom(name) => self.custom_ops.contains(name),
        }
    }
}

/// Required precision for computation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Precision {
    /// 16-bit float.
    Fp16,
    /// 32-bit float.
    Fp32,
    /// 64-bit float.
    Fp64,
    /// 8-bit integer.
    Int8,
    /// 16-bit integer.
    Int16,
    /// 32-bit integer.
    Int32,
    /// 64-bit integer.
    Int64,
    /// Mixed precision.
    Mixed,
}

/// Required operations for computation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Operation {
    /// General compute.
    GeneralCompute,
    /// Matrix multiply.
    MatrixMultiply,
    /// Tensor operations.
    TensorOps,
    /// Convolution.
    Convolution,
    /// FFT.
    Fft,
    /// Reduction.
    Reduction,
    /// Atomic operations.
    Atomic,
    /// Branch-heavy code.
    BranchHeavy,
    /// Custom operation.
    Custom(String),
}

/// Compute workload requirements (WHAT the workload needs)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComputeRequirements {
    /// Minimum parallel threads needed
    pub min_parallel_threads: u64,

    /// Memory required (bytes)
    pub memory_bytes: u64,

    /// Required precision
    pub precision: Precision,

    /// Required operations
    pub operations: Vec<Operation>,

    /// Estimated number of operations (for performance prediction)
    /// If None, scheduler will estimate based on workload characteristics
    pub estimated_operations: Option<u64>,

    /// Maximum acceptable execution time
    pub max_execution_time: Option<Duration>,

    /// Preferred memory access pattern
    pub preferred_access_pattern: Option<MemoryAccessPattern>,
}

impl Default for ComputeRequirements {
    fn default() -> Self {
        Self {
            min_parallel_threads: 1,
            memory_bytes: 1024,
            precision: Precision::Fp32,
            operations: vec![Operation::GeneralCompute],
            estimated_operations: None,
            max_execution_time: None,
            preferred_access_pattern: None,
        }
    }
}

/// Universal compute resource trait - GPU, CPU, TPU, anything!
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait UniversalComputeResource: Send + Sync {
    /// Get capabilities of this resource
    fn capabilities(&self) -> &ComputeCapabilities;

    /// Get unique identifier for this resource
    fn resource_id(&self) -> &str;

    /// Check if can execute this workload
    fn can_execute(&self, requirements: &ComputeRequirements) -> bool {
        self.capabilities().meets_requirements(requirements)
    }

    /// Score how well this resource matches workload (0.0-1.0)
    fn score_workload(&self, requirements: &ComputeRequirements) -> f64 {
        if !self.can_execute(requirements) {
            return 0.0;
        }
        self.capabilities().score_for_workload(requirements)
    }

    /// Create execution context for this resource
    async fn create_context(&self) -> ToadStoolResult<Box<dyn ComputeContext>>;

    /// Get current utilization (0.0 = idle, 1.0 = fully utilized)
    async fn utilization(&self) -> f32;

    /// Estimate execution time for workload
    fn estimate_execution_time(&self, requirements: &ComputeRequirements) -> Duration;
}

/// Compute execution context (session on a specific resource)
// NOTE(async-dyn): #[async_trait] required — native async fn in trait is not dyn-compatible
#[async_trait]
pub trait ComputeContext: Send + Sync {
    /// Get context ID
    fn context_id(&self) -> Uuid;

    /// Get resource this context is on
    fn resource_id(&self) -> &str;

    /// Execute workload in this context
    async fn execute(&mut self, workload: &UniversalWorkload) -> ToadStoolResult<WorkloadResult>;

    /// Close context and cleanup resources
    async fn close(self: Box<Self>) -> ToadStoolResult<()>;
}

/// Universal workload description (hardware-agnostic)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UniversalWorkload {
    /// Unique workload ID
    pub id: String,

    /// What capabilities does this workload need?
    pub requirements: ComputeRequirements,

    /// The compute kernel
    pub kernel: UniversalKernel,

    /// Input buffers
    pub inputs: Vec<ComputeBuffer>,

    /// Expected output size
    pub output_size: usize,

    /// Optimization hints
    pub hints: OptimizationHints,
}

/// Buffer for compute data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ComputeBuffer {
    /// Buffer name.
    pub name: String,
    /// Buffer data.
    pub data: bytes::Bytes,
    /// Element data type.
    pub element_type: DataType,
}

/// Universal kernel representation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum UniversalKernel {
    /// Source code in a universal language.
    Source {
        /// Kernel language.
        language: KernelLanguage,
        /// Source code.
        code: String,
        /// Entry point name.
        entry_point: String,
    },

    /// Pre-compiled binary.
    Binary {
        /// Binary format.
        format: BinaryFormat,
        /// Binary data.
        data: bytes::Bytes,
    },

    /// High-level operation description.
    Operation {
        /// Operation type.
        operation: Operation,
        /// Operation parameters.
        parameters: HashMap<String, serde_json::Value>,
    },

    /// Reference to standard library function.
    Library {
        /// Function name.
        name: String,
        /// Version string.
        version: String,
    },
}

/// Supported kernel languages.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KernelLanguage {
    /// WGSL (WebGPU Shading Language).
    Wgsl,
    /// SPIR-V.
    Spirv,
    /// OpenCL C.
    OpenClC,
    /// CUDA.
    Cuda,
    /// Metal Shading Language.
    Metal,
    /// Python (for high-level kernels).
    Python,
    /// Rust (for GPU kernels).
    Rust,
    /// Custom language.
    Custom(String),
}

/// Binary formats for pre-compiled kernels.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BinaryFormat {
    /// SPIR-V binary.
    SpirvBinary,
    /// Native binary.
    NativeBinary,
    /// Custom format.
    Custom(String),
}

/// Optimization hints for scheduler.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct OptimizationHints {
    /// Prefer latency over throughput.
    pub low_latency: bool,

    /// Prefer energy efficiency.
    pub energy_efficient: bool,

    /// Allow approximate results
    pub approximate: bool,

    /// Priority (0 = low, 10 = critical)
    pub priority: u8,
}

/// Result of workload execution
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkloadResult {
    /// Output buffers (zero-copy via refcounted `Bytes`)
    pub outputs: HashMap<String, bytes::Bytes>,

    /// Execution metrics
    pub metrics: ExecutionMetrics,

    /// Errors or warnings
    pub messages: Vec<String>,
}

/// Execution metrics
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutionMetrics {
    /// Actual execution time
    pub execution_time: Duration,

    /// Memory used
    pub memory_used: u64,

    /// Energy consumed (if available)
    pub energy_joules: Option<f64>,

    /// Resource utilization during execution
    pub utilization: f32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capability_matching() {
        let capabilities = ComputeCapabilities {
            parallelism: ParallelismCapabilities {
                max_parallel_threads: 4096,
                model: ParallelismModel::Simt { max_threads: 4096 },
                max_work_group_size: Some(256),
                simd_width: None,
                nested_parallelism: false,
            },
            memory: MemoryCapabilities {
                total_bytes: 8 * 1024 * 1024 * 1024,               // 8 GB
                bandwidth_bytes_per_sec: 300 * 1024 * 1024 * 1024, // 300 GB/s
                unified_memory: false,
                zero_copy: false,
                cache_levels: vec![],
                access_patterns: vec![MemoryAccessPattern::Coalesced],
            },
            precision: PrecisionCapabilities {
                fp16: true,
                fp32: true,
                fp64: false,
                int8: true,
                int16: true,
                int32: true,
                int64: false,
                mixed_precision: true,
            },
            operations: OperationCapabilities {
                general_compute: true,
                matrix_multiply: true,
                tensor_ops: true,
                convolution: true,
                fft: true,
                reduction_ops: true,
                atomic_ops: true,
                branching_efficiency: BranchingEfficiency::Medium,
                custom_ops: vec![],
            },
            performance: PerformanceCapabilities {
                peak_flops: 10_000_000_000_000.0, // 10 TFLOPS
                peak_iops: 20_000_000_000_000.0,
                power_watts: 250.0,
                startup_latency_us: 100,
                sustained_performance_percent: 80.0,
            },
            resource_type: "GPU".to_string(),
        };

        let requirements = ComputeRequirements {
            min_parallel_threads: 1024,
            memory_bytes: 1024 * 1024, // 1 MB
            precision: Precision::Fp32,
            operations: vec![Operation::MatrixMultiply],
            estimated_operations: Some(1024 * 1024),
            max_execution_time: None,
            preferred_access_pattern: None,
        };

        assert!(capabilities.meets_requirements(&requirements));

        let score = capabilities.score_for_workload(&requirements);
        assert!(score > 0.8); // Should be a good match
    }

    #[test]
    fn test_precision_support() {
        let precision = PrecisionCapabilities {
            fp16: false,
            fp32: true,
            fp64: true,
            int8: false,
            int16: true,
            int32: true,
            int64: true,
            mixed_precision: false,
        };

        assert!(!precision.supports(Precision::Fp16));
        assert!(precision.supports(Precision::Fp32));
        assert!(precision.supports(Precision::Fp64));
        assert!(precision.supports(Precision::Int32));
    }
}
