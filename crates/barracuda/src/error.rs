//! Error types for barraCUDA
//!
//! **Deep Debt Excellence**: Rich error context, zero panic paths

use thiserror::Error;

pub type Result<T> = std::result::Result<T, BarracudaError>;

#[derive(Error, Debug)]
pub enum BarracudaError {
    #[error("Device error: {0}")]
    Device(String),

    #[error("Shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch {
        expected: Vec<usize>,
        actual: Vec<usize>,
    },

    #[error("Invalid operation: {op} - {reason}")]
    InvalidOperation { op: String, reason: String },

    #[error("GPU error: {0}")]
    Gpu(String),

    #[error("Shader compilation error: {0}")]
    ShaderCompilation(String),

    #[error("Out of memory: {0}")]
    OutOfMemory(String),

    #[error("Operation not supported on device: {op} on {device}")]
    UnsupportedOperation { op: String, device: String },

    #[error("Invalid input: {message}")]
    InvalidInput { message: String },

    #[error("Execution error: {message}")]
    ExecutionError { message: String },

    #[error("Device not available: {device} - {reason}")]
    DeviceNotAvailable { device: String, reason: String },

    #[error("Internal error: {0}")]
    Internal(String),
}

impl BarracudaError {
    pub fn device(msg: impl Into<String>) -> Self {
        Self::Device(msg.into())
    }

    pub fn device_not_found(msg: impl Into<String>) -> Self {
        Self::Device(msg.into())
    }

    pub fn execution_failed(msg: impl Into<String>) -> Self {
        Self::ExecutionError {
            message: msg.into(),
        }
    }

    pub fn shape_mismatch(expected: Vec<usize>, actual: Vec<usize>) -> Self {
        Self::ShapeMismatch { expected, actual }
    }

    pub fn invalid_op(op: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidOperation {
            op: op.into(),
            reason: reason.into(),
        }
    }

    pub fn gpu(msg: impl Into<String>) -> Self {
        Self::Gpu(msg.into())
    }

    pub fn shader_compilation(msg: impl Into<String>) -> Self {
        Self::ShaderCompilation(msg.into())
    }

    pub fn oom(msg: impl Into<String>) -> Self {
        Self::OutOfMemory(msg.into())
    }

    pub fn unsupported(op: impl Into<String>, device: impl Into<String>) -> Self {
        Self::UnsupportedOperation {
            op: op.into(),
            device: device.into(),
        }
    }
}
