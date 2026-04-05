// SPDX-License-Identifier: AGPL-3.0-or-later
//! Workload data, parameters, and builder types.

use super::capabilities::{DataType, OperationType};
use super::error::ComputeError;

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
    /// Single vector (f32).
    F32Vec(Vec<f32>),
    /// Single vector (f64).
    F64Vec(Vec<f64>),
    /// Single vector (i32).
    I32Vec(Vec<i32>),
    /// Single vector (i64).
    I64Vec(Vec<i64>),
    /// Dual f32 vectors (for binary operations).
    F32VecPair(Vec<f32>, Vec<f32>),
    /// Dual f64 vectors (for binary operations).
    F64VecPair(Vec<f64>, Vec<f64>),
    /// Dual i32 vectors (for binary operations).
    I32VecPair(Vec<i32>, Vec<i32>),
    /// f32 data + indices (for gather/scatter).
    F32VecIndexed(Vec<f32>, Vec<usize>),
    /// f64 data + indices (for gather/scatter).
    F64VecIndexed(Vec<f64>, Vec<usize>),
    /// i32 data + indices (for gather/scatter).
    I32VecIndexed(Vec<i32>, Vec<usize>),
    /// 2D f32 matrix (data, rows, cols).
    F32Matrix(Vec<f32>, usize, usize),
    /// 2D f64 matrix (data, rows, cols).
    F64Matrix(Vec<f64>, usize, usize),
    /// 2D i32 matrix (data, rows, cols).
    I32Matrix(Vec<i32>, usize, usize),
    /// Pair of f32 matrices (for MatMul: A * B).
    F32MatrixPair(Vec<f32>, usize, usize, Vec<f32>, usize, usize),
    /// Pair of f64 matrices (for MatMul: A * B).
    F64MatrixPair(Vec<f64>, usize, usize, Vec<f64>, usize, usize),
    /// Pair of i32 matrices (for MatMul: A * B).
    I32MatrixPair(Vec<i32>, usize, usize, Vec<i32>, usize, usize),
    /// Conv2D data.
    F32Conv2D {
        /// Input tensor.
        input: Vec<f32>,
        /// Kernel weights.
        kernel: Vec<f32>,
        /// Optional bias.
        bias: Option<Vec<f32>>,
        /// Batch size.
        batch_size: usize,
        /// Input channels.
        in_channels: usize,
        /// Input height.
        height: usize,
        /// Input width.
        width: usize,
        /// Output channels.
        out_channels: usize,
        /// Kernel height.
        kernel_h: usize,
        /// Kernel width.
        kernel_w: usize,
        /// Stride.
        stride: usize,
        /// Padding.
        padding: usize,
    },
    /// Pooling2D data.
    F32Pool2D {
        /// Input tensor.
        input: Vec<f32>,
        /// Batch size.
        batch_size: usize,
        /// Channels.
        channels: usize,
        /// Input height.
        height: usize,
        /// Input width.
        width: usize,
        /// Pool height.
        pool_h: usize,
        /// Pool width.
        pool_w: usize,
        /// Stride.
        stride: usize,
        /// Padding.
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
    /// Integer parameter.
    Int(i64),
    /// Float parameter.
    Float(f64),
    /// String parameter.
    String(String),
    /// Boolean parameter.
    Bool(bool),
}

/// Workload builder for convenience
pub struct WorkloadBuilder {
    /// Operation type (set via `operation()`).
    operation: Option<OperationType>,
    /// Data type (inferred from `data_f32` etc.).
    data_type: Option<DataType>,
    /// Number of operations.
    num_operations: usize,
    /// Required memory in bytes.
    required_memory: usize,
    /// Input data.
    input: Option<WorkloadData>,
    /// Operation parameters.
    params: WorkloadParams,
}

impl WorkloadBuilder {
    /// Create a new workload builder.
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

    /// Set the operation type.
    pub const fn operation(mut self, op: OperationType) -> Self {
        self.operation = Some(op);
        self
    }

    /// Set f32 vector input data.
    pub fn data_f32(mut self, data: Vec<f32>) -> Self {
        self.num_operations = data.len();
        self.required_memory = data.len() * std::mem::size_of::<f32>();
        self.data_type = Some(DataType::F32);
        self.input = Some(WorkloadData::F32Vec(data));
        self
    }

    /// Add a parameter.
    pub fn param<S: Into<String>>(mut self, key: S, value: ParamValue) -> Self {
        self.params.params.insert(key.into(), value);
        self
    }

    /// Build the workload.
    ///
    /// # Errors
    ///
    /// Returns when operation, data type, or input was not set on the builder.
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
