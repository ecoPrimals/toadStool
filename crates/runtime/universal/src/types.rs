//! Core types for universal compute
//!
//! This module defines the fundamental abstractions that unify CPU, GPU,
//! and neuromorphic processing under a single interface.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A compute unit represents any parallel processing resource.
///
/// This trait abstracts over different compute paradigms:
/// - CPU: Serial/parallel (1-64 cores typically)
/// - GPU: Massive parallel (1000s of cores)
/// - Neuromorphic: Event-driven (spike-based)
///
/// Key insight: They're all parallel compute with different profiles!
#[async_trait::async_trait]
pub trait ComputeUnit: Send + Sync {
    /// Get capabilities of this compute unit
    fn capabilities(&self) -> &Capabilities;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Execute a workload on this compute unit
    async fn execute(&self, workload: Workload) -> Result<Output, ComputeError>;

    /// Check if this unit can execute the given workload
    fn can_execute(&self, workload: &Workload) -> bool {
        self.capabilities().supports_workload(workload)
    }

    /// Get the optimal batch size for this unit
    fn optimal_batch_size(&self) -> usize {
        self.capabilities().optimal_batch_size
    }

    /// Estimate execution time for a workload (for scheduling)
    fn estimate_duration(&self, workload: &Workload) -> std::time::Duration {
        self.capabilities().estimate_duration(workload)
    }
}

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
        // Check operation type
        if !self.supported_ops.contains(&workload.operation) {
            return false;
        }

        // Check data type
        if !self.supported_types.contains(&workload.data_type) {
            return false;
        }

        // Check memory requirements
        if workload.required_memory > self.memory_capacity {
            return false;
        }

        true
    }

    /// Estimate execution duration
    pub fn estimate_duration(&self, workload: &Workload) -> std::time::Duration {
        // Simple model: time = ops / throughput + latency
        let compute_time = workload.num_operations as f64 / self.compute_throughput;
        let total_time = compute_time + self.latency.typical_ms as f64 / 1000.0;
        std::time::Duration::from_secs_f64(total_time)
    }

    /// Calculate a score for this unit given a workload (higher is better)
    pub fn score_for_workload(&self, workload: &Workload) -> f64 {
        if !self.supports_workload(workload) {
            return 0.0;
        }

        // Balance throughput, latency, and power
        let throughput_score = self.compute_throughput / 1e9; // Normalize
        let latency_score = 1.0 / (self.latency.typical_ms as f64 + 1.0);
        let power_score = match self.power_profile {
            PowerProfile::UltraLow => 2.0,
            PowerProfile::Low => 1.5,
            PowerProfile::Medium => 1.0,
            PowerProfile::High => 0.5,
        };

        // Weighted average
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
    F32,
    F64,
    I32,
    I64,
    U32,
    U64,
}

/// A workload to be executed
#[derive(Debug, Clone)]
pub struct Workload {
    /// Operation type
    pub operation: OperationType,

    /// Data type
    pub data_type: DataType,

    /// Number of operations
    pub num_operations: usize,

    /// Required memory (bytes)
    pub required_memory: usize,

    /// Input data (opaque, interpreted by backend)
    pub input: WorkloadData,

    /// Operation-specific parameters
    pub params: WorkloadParams,
}

/// Workload data (type-erased for flexibility)
#[derive(Debug, Clone)]
pub enum WorkloadData {
    /// Single vector
    F32Vec(Vec<f32>),
    F64Vec(Vec<f64>),
    I32Vec(Vec<i32>),
    I64Vec(Vec<i64>),
    /// Dual vectors (for binary operations)
    F32VecPair(Vec<f32>, Vec<f32>),
    F64VecPair(Vec<f64>, Vec<f64>),
    I32VecPair(Vec<i32>, Vec<i32>),
    /// Data + Indices (for gather/scatter operations)
    F32VecIndexed(Vec<f32>, Vec<usize>),
    F64VecIndexed(Vec<f64>, Vec<usize>),
    I32VecIndexed(Vec<i32>, Vec<usize>),
    /// 2D Matrix (data, rows, cols)
    F32Matrix(Vec<f32>, usize, usize),
    F64Matrix(Vec<f64>, usize, usize),
    I32Matrix(Vec<i32>, usize, usize),
    /// Pair of matrices (for MatMul: A * B)
    /// (A_data, A_rows, A_cols, B_data, B_rows, B_cols)
    F32MatrixPair(Vec<f32>, usize, usize, Vec<f32>, usize, usize),
    F64MatrixPair(Vec<f64>, usize, usize, Vec<f64>, usize, usize),
    I32MatrixPair(Vec<i32>, usize, usize, Vec<i32>, usize, usize),
    /// Conv2D data (input, kernel, bias, batch, in_channels, height, width, out_channels, kernel_h, kernel_w, stride, padding)
    F32Conv2D {
        input: Vec<f32>,
        kernel: Vec<f32>,
        bias: Option<Vec<f32>>,
        batch_size: usize,
        in_channels: usize,
        height: usize,
        width: usize,
        out_channels: usize,
        kernel_h: usize,
        kernel_w: usize,
        stride: usize,
        padding: usize,
    },
    /// Pooling2D data (input, batch, channels, height, width, pool_h, pool_w, stride, padding)
    F32Pool2D {
        input: Vec<f32>,
        batch_size: usize,
        channels: usize,
        height: usize,
        width: usize,
        pool_h: usize,
        pool_w: usize,
        stride: usize,
        padding: usize,
    },
    /// Custom data
    Custom(Vec<u8>),
}

/// Operation parameters
#[derive(Debug, Clone, Default)]
pub struct WorkloadParams {
    /// Generic key-value parameters
    pub params: std::collections::HashMap<String, ParamValue>,
}

/// Parameter values
#[derive(Debug, Clone)]
pub enum ParamValue {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
}

/// Execution output
#[derive(Debug, Clone)]
pub struct Output {
    /// Result data
    pub data: WorkloadData,

    /// Execution metadata
    pub metadata: OutputMetadata,
}

/// Execution metadata
#[derive(Debug, Clone)]
pub struct OutputMetadata {
    /// Which unit executed this
    pub unit_name: String,

    /// Unit type
    pub unit_type: ComputeUnitType,

    /// Actual execution time
    pub duration: std::time::Duration,

    /// Power consumed (if measurable)
    pub power_consumed_mw: Option<f64>,
}

/// Compute errors
#[derive(Debug, thiserror::Error)]
pub enum ComputeError {
    #[error("Workload not supported by this compute unit")]
    UnsupportedWorkload,

    #[error("Memory allocation failed: {0}")]
    MemoryAllocationFailed(String),

    #[error("Execution failed: {0}")]
    ExecutionFailed(String),

    #[error("Backend error: {0}")]
    BackendError(String),

    #[error("No suitable compute unit found for workload")]
    NoSuitableUnit,
}

/// Workload builder for convenience
pub struct WorkloadBuilder {
    operation: Option<OperationType>,
    data_type: Option<DataType>,
    num_operations: usize,
    required_memory: usize,
    input: Option<WorkloadData>,
    params: WorkloadParams,
}

impl WorkloadBuilder {
    pub fn new() -> Self {
        Self {
            operation: None,
            data_type: None,
            num_operations: 0,
            required_memory: 0,
            input: None,
            params: WorkloadParams::default(),
        }
    }

    pub fn operation(mut self, op: OperationType) -> Self {
        self.operation = Some(op);
        self
    }

    pub fn data_f32(mut self, data: Vec<f32>) -> Self {
        self.num_operations = data.len();
        self.required_memory = data.len() * std::mem::size_of::<f32>();
        self.data_type = Some(DataType::F32);
        self.input = Some(WorkloadData::F32Vec(data));
        self
    }

    pub fn param<S: Into<String>>(mut self, key: S, value: ParamValue) -> Self {
        self.params.params.insert(key.into(), value);
        self
    }

    pub fn build(self) -> Result<Workload, ComputeError> {
        Ok(Workload {
            operation: self.operation.ok_or_else(|| {
                ComputeError::ExecutionFailed("Operation not specified".to_string())
            })?,
            data_type: self.data_type.ok_or_else(|| {
                ComputeError::ExecutionFailed("Data type not specified".to_string())
            })?,
            num_operations: self.num_operations,
            required_memory: self.required_memory,
            input: self.input.ok_or_else(|| {
                ComputeError::ExecutionFailed("Input data not specified".to_string())
            })?,
            params: self.params,
        })
    }
}

impl Default for WorkloadBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_caps(
        supported_ops: Vec<OperationType>,
        supported_types: Vec<DataType>,
        memory: usize,
    ) -> Capabilities {
        Capabilities {
            unit_type: ComputeUnitType::Cpu,
            parallelism: Parallelism {
                num_units: 4,
                model: ExecutionModel::Mimd,
            },
            power_profile: PowerProfile::Medium,
            latency: LatencyProfile {
                typical_ms: 1,
                deterministic: true,
            },
            memory_capacity: memory,
            memory_bandwidth: 50_000_000_000,
            compute_throughput: 400e9,
            optimal_batch_size: 100,
            supported_ops,
            supported_types,
        }
    }

    fn make_workload(op: OperationType, dtype: DataType, mem: usize) -> Workload {
        Workload {
            operation: op,
            data_type: dtype,
            num_operations: 100,
            required_memory: mem,
            input: WorkloadData::F32Vec(vec![]),
            params: WorkloadParams::default(),
        }
    }

    #[test]
    fn test_capabilities_supports_workload_ok() {
        let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], 1_000_000);
        let w = make_workload(OperationType::Map, DataType::F32, 512);
        assert!(caps.supports_workload(&w));
    }

    #[test]
    fn test_capabilities_unsupported_op() {
        let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], 1_000_000);
        let w = make_workload(OperationType::Reduce, DataType::F32, 0);
        assert!(!caps.supports_workload(&w));
    }

    #[test]
    fn test_capabilities_unsupported_dtype() {
        let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], 1_000_000);
        let w = make_workload(OperationType::Map, DataType::F64, 0);
        assert!(!caps.supports_workload(&w));
    }

    #[test]
    fn test_capabilities_insufficient_memory() {
        let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], 100);
        let w = make_workload(OperationType::Map, DataType::F32, 200);
        assert!(!caps.supports_workload(&w));
    }

    #[test]
    fn test_capabilities_estimate_duration_positive() {
        let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], usize::MAX);
        let w = make_workload(OperationType::Map, DataType::F32, 0);
        let dur = caps.estimate_duration(&w);
        assert!(dur.as_nanos() > 0);
    }

    #[test]
    fn test_capabilities_score_unsupported_returns_zero() {
        let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], usize::MAX);
        let w = make_workload(OperationType::Reduce, DataType::F32, 0);
        assert_eq!(caps.score_for_workload(&w), 0.0);
    }

    #[test]
    fn test_capabilities_score_supported_positive() {
        let caps = make_caps(vec![OperationType::Map], vec![DataType::F32], usize::MAX);
        let w = make_workload(OperationType::Map, DataType::F32, 0);
        assert!(caps.score_for_workload(&w) > 0.0);
    }

    #[test]
    fn test_compute_unit_type_display() {
        assert_eq!(format!("{}", ComputeUnitType::Cpu), "CPU");
        assert_eq!(format!("{}", ComputeUnitType::GpuOpenCl), "GPU (OpenCL)");
        assert_eq!(format!("{}", ComputeUnitType::GpuWgpu), "GPU (wgpu)");
        assert_eq!(format!("{}", ComputeUnitType::GpuVulkan), "GPU (Vulkan)");
        assert_eq!(format!("{}", ComputeUnitType::Neuromorphic), "Neuromorphic");
        assert_eq!(format!("{}", ComputeUnitType::Custom(42)), "Custom(42)");
    }

    #[test]
    fn test_power_profile_variants_exist() {
        let _ = PowerProfile::UltraLow;
        let _ = PowerProfile::Low;
        let _ = PowerProfile::Medium;
        let _ = PowerProfile::High;
    }

    #[test]
    fn test_workload_builder_builds_ok() {
        let w = WorkloadBuilder::new()
            .operation(OperationType::Map)
            .data_f32(vec![1.0, 2.0, 3.0])
            .build()
            .unwrap();
        assert_eq!(w.operation, OperationType::Map);
        assert_eq!(w.data_type, DataType::F32);
        assert_eq!(w.num_operations, 3);
    }

    #[test]
    fn test_workload_builder_missing_op_fails() {
        let result = WorkloadBuilder::new().data_f32(vec![1.0]).build();
        assert!(matches!(result, Err(ComputeError::ExecutionFailed(_))));
    }

    #[test]
    fn test_workload_builder_missing_data_fails() {
        let result = WorkloadBuilder::new().operation(OperationType::Map).build();
        assert!(result.is_err());
    }

    #[test]
    fn test_workload_builder_with_param() {
        let w = WorkloadBuilder::new()
            .operation(OperationType::ElementwiseBinary)
            .data_f32(vec![1.0])
            .param("op", ParamValue::String("add".into()))
            .build()
            .unwrap();
        assert!(w.params.params.contains_key("op"));
    }

    #[test]
    fn test_compute_error_display() {
        let e = ComputeError::UnsupportedWorkload;
        assert!(!format!("{e}").is_empty());
        let e2 = ComputeError::ExecutionFailed("oops".into());
        assert!(format!("{e2}").contains("oops"));
    }
}
