//! Common types for adaptive optimization system

use serde::{Deserialize, Serialize};

/// GPU operation type
///
/// Enum representing different GPU operations that can be optimized.
/// Each operation may have different optimal workgroup sizes depending
/// on hardware and input size.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum OpType {
    /// Matrix multiplication
    MatMul,
    /// Layer normalization
    LayerNorm,
    /// GELU activation
    GELU,
    /// Softmax normalization
    Softmax,
    /// Element-wise addition
    Add,
    /// Element-wise multiplication
    Mul,
    /// Element-wise division
    Div,
    /// `ReLU` activation
    ReLU,
    /// Sigmoid activation
    Sigmoid,
    /// Tanh activation
    Tanh,
    /// Transpose operation
    Transpose,
    /// Reduce operations
    Reduce,
    /// Convolution 2D
    Conv2D,
    /// Max pooling 2D
    MaxPool2D,
    /// Average pooling 2D
    AvgPool2D,
    /// Batch normalization
    BatchNorm,
    /// Dropout
    Dropout,
    /// Embedding lookup
    Embedding,
}

impl OpType {
    /// Get all operation types
    #[must_use]
    pub fn all() -> Vec<Self> {
        vec![
            Self::MatMul,
            Self::LayerNorm,
            Self::GELU,
            Self::Softmax,
            Self::Add,
            Self::Mul,
            Self::Div,
            Self::ReLU,
            Self::Sigmoid,
            Self::Tanh,
            Self::Transpose,
            Self::Reduce,
            Self::Conv2D,
            Self::MaxPool2D,
            Self::AvgPool2D,
            Self::BatchNorm,
            Self::Dropout,
            Self::Embedding,
        ]
    }
}

/// Size class for input data
///
/// Categorizes input sizes to reduce profiling overhead.
/// Different size classes may have different optimal configurations.
#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
pub enum SizeClass {
    /// < 1K elements
    Tiny,
    /// 1K - 100K elements
    Small,
    /// 100K - 1M elements
    Medium,
    /// 1M - 10M elements
    Large,
    /// > 10M elements
    Huge,
}

impl SizeClass {
    /// Determine size class from element count
    #[must_use]
    pub const fn from_size(size: usize) -> Self {
        if size < 1_000 {
            Self::Tiny
        } else if size < 100_000 {
            Self::Small
        } else if size < 1_000_000 {
            Self::Medium
        } else if size < 10_000_000 {
            Self::Large
        } else {
            Self::Huge
        }
    }

    /// Get representative size for benchmarking
    #[must_use]
    pub const fn representative_size(self) -> usize {
        match self {
            Self::Tiny => 512,
            Self::Small => 10_000,
            Self::Medium => 500_000,
            Self::Large => 5_000_000,
            Self::Huge => 20_000_000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_size_class_from_size() {
        assert_eq!(SizeClass::from_size(500), SizeClass::Tiny);
        assert_eq!(SizeClass::from_size(5_000), SizeClass::Small);
        assert_eq!(SizeClass::from_size(500_000), SizeClass::Medium);
        assert_eq!(SizeClass::from_size(5_000_000), SizeClass::Large);
        assert_eq!(SizeClass::from_size(50_000_000), SizeClass::Huge);
    }

    #[test]
    fn test_op_type_all() {
        let all_ops = OpType::all();
        assert!(all_ops.len() >= 18);
        assert!(all_ops.contains(&OpType::MatMul));
        assert!(all_ops.contains(&OpType::LayerNorm));
    }
}
