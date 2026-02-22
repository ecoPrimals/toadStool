//! Comprehensive Error Type Hierarchy for barraCuda
//!
//! **Design Philosophy**:
//! - Follow ToadStool's A+ error handling pattern
//! - Use `thiserror` for ergonomic error definitions
//! - Provide rich context for debugging
//! - Domain-specific error variants
//! - No panics in production (Result<T, E> everywhere)
//!
//! **Error Categories**:
//! - GPU/Device errors (initialization, allocation, execution)
//! - Shader errors (compilation, binding)
//! - Tensor errors (shape mismatch, dimension errors)
//! - Operation errors (unsupported, invalid parameters)
//! - IO errors (buffer operations, data transfer)

use thiserror::Error;

/// Main error type for barraCuda operations
#[derive(Error, Debug)]
pub enum BarracudaError {
    /// GPU device initialization failed
    #[error("GPU device initialization failed: {0}")]
    DeviceInitialization(String),

    /// GPU device not found or unavailable
    #[error("No suitable GPU device found: {0}")]
    DeviceNotFound(String),

    /// Buffer allocation failed
    #[error("Failed to allocate GPU buffer of size {size} bytes: {reason}")]
    BufferAllocation { size: usize, reason: String },

    /// Buffer operation failed (map, unmap, copy)
    #[error("Buffer operation failed: {operation}: {reason}")]
    BufferOperation { operation: String, reason: String },

    /// Shader compilation failed
    #[error("Shader compilation failed for '{shader_name}': {reason}")]
    ShaderCompilation { shader_name: String, reason: String },

    /// Shader binding error
    #[error("Failed to bind shader resources: {0}")]
    ShaderBinding(String),

    /// Compute pipeline creation failed
    #[error("Failed to create compute pipeline: {0}")]
    PipelineCreation(String),

    /// Compute execution failed
    #[error("GPU compute execution failed: {operation}: {reason}")]
    ComputeExecution { operation: String, reason: String },

    /// Tensor shape mismatch
    #[error("Tensor shape mismatch: expected {expected:?}, got {actual:?}")]
    ShapeMismatch {
        expected: Vec<usize>,
        actual: Vec<usize>,
    },

    /// Invalid tensor dimensions
    #[error("Invalid tensor dimensions: {0}")]
    InvalidDimensions(String),

    /// Invalid operation parameters
    #[error("Invalid operation parameters for '{operation}': {reason}")]
    InvalidParameters { operation: String, reason: String },

    /// Unsupported operation
    #[error("Unsupported operation: {operation}: {reason}")]
    UnsupportedOperation { operation: String, reason: String },

    /// Data type mismatch or conversion error
    #[error("Data type error: {0}")]
    DataType(String),

    /// Random number generation error
    #[error("Random number generation failed: {0}")]
    RandomGeneration(String),

    /// Normalization error
    #[error("Normalization operation failed: {operation}: {reason}")]
    Normalization { operation: String, reason: String },

    /// Convolution error
    #[error("Convolution operation failed: {0}")]
    Convolution(String),

    /// Matrix operation error
    #[error("Matrix operation failed: {operation}: {reason}")]
    MatrixOperation { operation: String, reason: String },

    /// Pooling operation error
    #[error("Pooling operation failed: {0}")]
    Pooling(String),

    /// Activation function error
    #[error("Activation function '{function}' failed: {reason}")]
    Activation { function: String, reason: String },

    /// Training operation error
    #[error("Training operation failed: {operation}: {reason}")]
    Training { operation: String, reason: String },

    /// Loss function error
    #[error("Loss function '{function}' failed: {reason}")]
    LossFunction { function: String, reason: String },

    /// Optimizer error
    #[error("Optimizer '{optimizer}' failed: {reason}")]
    Optimizer { optimizer: String, reason: String },

    /// Timeout waiting for GPU operation
    #[error("GPU operation timed out after {duration_ms}ms: {operation}")]
    Timeout { operation: String, duration_ms: u64 },

    /// Out of GPU memory
    #[error("Out of GPU memory: requested {requested} bytes, available {available} bytes")]
    OutOfMemory { requested: usize, available: usize },

    /// Synchronization error
    #[error("GPU synchronization failed: {0}")]
    Synchronization(String),

    /// Substrate selection error
    #[error("Failed to select GPU substrate: {0}")]
    SubstrateSelection(String),

    /// Capability query error
    #[error("Failed to query GPU capabilities: {0}")]
    CapabilityQuery(String),

    /// IO error (file operations, etc.)
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error with context
    #[error("{context}: {source}")]
    WithContext {
        context: String,
        #[source]
        source: Box<BarracudaError>,
    },

    /// Internal error (should not happen in production)
    #[error("Internal error: {0}")]
    Internal(String),
}

impl BarracudaError {
    /// Add context to an error
    ///
    /// # Example
    /// ```ignore
    /// operation()
    ///     .map_err(|e| e.with_context("Failed to execute MatMul"))?;
    /// ```
    pub fn with_context<S: Into<String>>(self, context: S) -> Self {
        Self::WithContext {
            context: context.into(),
            source: Box::new(self),
        }
    }

    /// Create a device initialization error
    pub fn device_init<S: Into<String>>(reason: S) -> Self {
        Self::DeviceInitialization(reason.into())
    }

    /// Create a buffer allocation error
    pub fn buffer_alloc(size: usize, reason: impl Into<String>) -> Self {
        Self::BufferAllocation {
            size,
            reason: reason.into(),
        }
    }

    /// Create a compute execution error
    pub fn compute_exec(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::ComputeExecution {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a shape mismatch error
    pub fn shape_mismatch(expected: Vec<usize>, actual: Vec<usize>) -> Self {
        Self::ShapeMismatch { expected, actual }
    }

    /// Create an invalid parameters error
    pub fn invalid_params(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidParameters {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create a training operation error
    pub fn training(operation: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Training {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create an optimizer error
    pub fn optimizer(optimizer: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Optimizer {
            optimizer: optimizer.into(),
            reason: reason.into(),
        }
    }
}

/// Result type alias for barraCuda operations
pub type Result<T> = std::result::Result<T, BarracudaError>;

/// Extension trait for adding context to Results
pub trait ResultExt<T> {
    /// Add context to a Result
    fn context<S: Into<String>>(self, context: S) -> Result<T>;

    /// Add context lazily (only evaluated on error)
    fn with_context<S, F>(self, f: F) -> Result<T>
    where
        S: Into<String>,
        F: FnOnce() -> S;
}

impl<T> ResultExt<T> for Result<T> {
    fn context<S: Into<String>>(self, context: S) -> Result<T> {
        self.map_err(|e| e.with_context(context))
    }

    fn with_context<S, F>(self, f: F) -> Result<T>
    where
        S: Into<String>,
        F: FnOnce() -> S,
    {
        self.map_err(|e| e.with_context(f()))
    }
}

/// Convert wgpu errors to BarracudaError
impl From<wgpu::Error> for BarracudaError {
    fn from(err: wgpu::Error) -> Self {
        match err {
            wgpu::Error::OutOfMemory { source: _ } => Self::OutOfMemory {
                requested: 0, // Unknown from wgpu error
                available: 0, // Unknown from wgpu error
            },
            wgpu::Error::Validation { source, .. } => {
                Self::DeviceInitialization(format!("Validation error: {}", source))
            }
            wgpu::Error::Internal { source, .. } => {
                Self::Internal(format!("wgpu internal error: {}", source))
            }
        }
    }
}

/// Convert buffer async errors
impl From<wgpu::BufferAsyncError> for BarracudaError {
    fn from(err: wgpu::BufferAsyncError) -> Self {
        Self::BufferOperation {
            operation: "async buffer operation".to_string(),
            reason: format!("{:?}", err),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = BarracudaError::device_init("Test reason");
        assert!(matches!(err, BarracudaError::DeviceInitialization(_)));
    }

    #[test]
    fn test_error_context() {
        let err = BarracudaError::device_init("Original error");
        let with_ctx = err.with_context("Additional context");

        match with_ctx {
            BarracudaError::WithContext { context, .. } => {
                assert_eq!(context, "Additional context");
            }
            _ => panic!("Expected WithContext variant"),
        }
    }

    #[test]
    fn test_result_ext_context() {
        let result: Result<()> = Err(BarracudaError::device_init("Test"));
        let with_ctx = result.context("Operation failed");

        assert!(with_ctx.is_err());
    }

    #[test]
    fn test_shape_mismatch() {
        let err = BarracudaError::shape_mismatch(vec![2, 3, 4], vec![2, 3]);
        let msg = format!("{}", err);
        assert!(msg.contains("[2, 3, 4]"));
        assert!(msg.contains("[2, 3]"));
    }
}
