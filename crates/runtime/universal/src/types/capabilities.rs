// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability and hardware descriptor types for compute units.

use serde::{Deserialize, Serialize};
use std::fmt;

use super::workload::Workload;

/// Capabilities describe what a compute unit can do
///
/// This is discovered at runtime, not hardcoded!
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capabilities {
    /// Type of compute unit
    pub unit_type: ComputeUnitType,

    /// Parallelism model
    pub parallelism: Parallelism,

    /// Power consumption profile
    pub power_profile: PowerProfile,

    /// Latency characteristics
    pub latency: LatencyProfile,

    /// Memory capacity (bytes)
    pub memory_capacity: usize,

    /// Memory bandwidth (bytes/sec)
    pub memory_bandwidth: usize,

    /// Compute throughput (ops/sec)
    pub compute_throughput: f64,

    /// Optimal batch size for this unit
    pub optimal_batch_size: usize,

    /// Supported operation types
    pub supported_ops: Vec<OperationType>,

    /// Supported data types
    pub supported_types: Vec<DataType>,
}

impl Capabilities {
    /// Check if this unit supports a workload
    pub fn supports_workload(&self, workload: &Workload) -> bool {
        if !self.supported_ops.contains(&workload.operation) {
            return false;
        }

        if !self.supported_types.contains(&workload.data_type) {
            return false;
        }

        if workload.required_memory > self.memory_capacity {
            return false;
        }

        true
    }

    /// Estimate execution duration
    pub fn estimate_duration(&self, workload: &Workload) -> std::time::Duration {
        let compute_time = workload.num_operations as f64 / self.compute_throughput;
        let total_time = compute_time + self.latency.typical_ms as f64 / 1000.0;
        std::time::Duration::from_secs_f64(total_time)
    }

    /// Calculate a score for this unit given a workload (higher is better)
    pub fn score_for_workload(&self, workload: &Workload) -> f64 {
        if !self.supports_workload(workload) {
            return 0.0;
        }

        let throughput_score = self.compute_throughput / 1e9;
        let latency_score = 1.0 / (self.latency.typical_ms as f64 + 1.0);
        let power_score = match self.power_profile {
            PowerProfile::UltraLow => 2.0,
            PowerProfile::Low => 1.5,
            PowerProfile::Medium => 1.0,
            PowerProfile::High => 0.5,
        };

        throughput_score * 0.5 + latency_score * 0.3 + power_score * 0.2
    }
}

/// Types of compute units
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComputeUnitType {
    /// CPU (x86, ARM, etc.)
    Cpu,
    /// GPU (via OpenCL)
    GpuOpenCl,
    /// GPU (via wgpu/WebGPU)
    GpuWgpu,
    /// GPU (via Vulkan)
    GpuVulkan,
    /// Neuromorphic (Akida, etc.)
    Neuromorphic,
    /// Future extension point
    Custom(u32),
}

impl fmt::Display for ComputeUnitType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cpu => write!(f, "CPU"),
            Self::GpuOpenCl => write!(f, "GPU (OpenCL)"),
            Self::GpuWgpu => write!(f, "GPU (wgpu)"),
            Self::GpuVulkan => write!(f, "GPU (Vulkan)"),
            Self::Neuromorphic => write!(f, "Neuromorphic"),
            Self::Custom(id) => write!(f, "Custom({id})"),
        }
    }
}

/// Parallelism models
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Parallelism {
    /// Number of parallel units (cores, threads, etc.)
    pub num_units: usize,

    /// Execution model
    pub model: ExecutionModel,
}

/// Execution models
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionModel {
    /// Serial execution (1 operation at a time)
    Serial,
    /// SIMD/vector (same operation on multiple data)
    Simd,
    /// MIMD (multiple operations on multiple data)
    Mimd,
    /// Event-driven (spike-based, neuromorphic)
    EventDriven,
}

/// Power consumption profiles
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PowerProfile {
    /// < 1W
    UltraLow,
    /// 1-10W
    Low,
    /// 10-100W
    Medium,
    /// > 100W
    High,
}

/// Latency characteristics
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LatencyProfile {
    /// Typical latency in milliseconds
    pub typical_ms: u32,
    /// Whether latency is deterministic
    pub deterministic: bool,
}

/// Operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OperationType {
    /// Element-wise map (apply function to each element)
    Map,
    /// Filter (select elements matching predicate)
    Filter,
    /// Reduction (fold/aggregate)
    Reduce,
    /// Scan (prefix sum / cumulative operation)
    Scan,
    /// Dot product (vector inner product)
    DotProduct,
    /// Element-wise binary operation (add, multiply, etc.)
    ElementwiseBinary,
    /// Gather (select elements by indices)
    Gather,
    /// Scatter (place elements by indices)
    Scatter,
    /// Transpose (2D matrix transpose)
    Transpose,
    /// Softmax (normalization: exp + reduce + map)
    Softmax,
    /// ReLU activation (max(0, x))
    ReLU,
    /// GELU activation (Gaussian Error Linear Unit)
    GELU,
    /// Tanh activation (hyperbolic tangent)
    Tanh,
    /// Sigmoid activation (logistic function)
    Sigmoid,
    /// Dropout (random masking for regularization)
    Dropout,
    /// Layer normalization (mean, variance, normalize)
    LayerNorm,
    /// Batch normalization (mean, variance, normalize per batch)
    BatchNorm,
    /// Matrix multiplication
    MatMul,
    /// Convolution (2D/3D)
    Conv,
    /// Max Pooling (2D)
    MaxPool2D,
    /// Average Pooling (2D)
    AvgPool2D,
    /// Custom operation
    Custom,
}

/// Data types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DataType {
    /// 32-bit float.
    F32,
    /// 64-bit float.
    F64,
    /// 32-bit signed integer.
    I32,
    /// 64-bit signed integer.
    I64,
    /// 32-bit unsigned integer.
    U32,
    /// 64-bit unsigned integer.
    U64,
}
