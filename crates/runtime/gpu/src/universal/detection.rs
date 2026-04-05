// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability descriptor types for universal compute resources.

use serde::{Deserialize, Serialize};

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
