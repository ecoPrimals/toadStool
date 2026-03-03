// SPDX-License-Identifier: AGPL-3.0-or-later
//! Activation Operations - Element-Wise Non-Linear Pattern
//!
//! Operations in this module share computational characteristics:
//! - **Element-wise** - Each output depends on single input
//! - **Non-linear** - Introduce non-linearity for neural networks
//! - **SIMD-friendly** - Can vectorize with CPU SIMD instructions
//! - **Memory-bound** - Limited by memory bandwidth, not compute
//!
//! ## Architectural Pattern
//!
//! ```text
//! Input → Non-Linear Transform → Output
//!   [x₁, x₂, ..., xₙ] → [f(x₁), f(x₂), ..., f(xₙ)]
//! ```
//!
//! ## Evolution of Activations
//!
//! - **Sigmoid/Tanh**: Classic, smooth, but vanishing gradients
//! - **ReLU**: Modern standard, dead neurons possible
//! - **LeakyReLU**: Fixes dead neurons
//! - **GELU**: State-of-art (used in BERT, GPT)
//!
//! This progression shows ML's evolution toward better optimization.

use crate::types::*;
use rayon::prelude::*;

/// Execute ReLU activation - Rectified Linear Unit
///
/// **Formula**: `f(x) = max(0, x)`
/// **Pattern**: Element-wise max operation
/// **Properties**:
/// - Zero-cost positive values (identity)
/// - Sparse activation (many zeros)
/// - Dead neurons possible (gradient = 0 for x < 0)
#[inline]
pub(super) fn execute_relu(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            let output: Vec<f32> = input.par_iter().map(|&x| x.max(0.0)).collect();
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64Vec(input) => {
            let output: Vec<f64> = input.par_iter().map(|&x| x.max(0.0)).collect();
            Ok(WorkloadData::F64Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// Execute GELU activation - Gaussian Error Linear Unit
///
/// **Formula**: `f(x) = x * Φ(x)` where Φ is cumulative distribution function
/// **Approximation**: `f(x) ≈ 0.5 * x * (1 + tanh(√(2/π) * (x + 0.044715 * x³)))`
/// **Pattern**: Smooth non-linearity
/// **Usage**: State-of-art (BERT, GPT-2, GPT-3)
/// **Properties**:
/// - Smooth activation (differentiable everywhere)
/// - Stochastic regularization interpretation
/// - Better than ReLU for transformers
#[inline]
pub(super) fn execute_gelu(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            let output: Vec<f32> = input
                .par_iter()
                .map(|&x| {
                    // GELU approximation
                    let sqrt_2_over_pi = 0.797_884_6_f32; // √(2/π)
                    let coeff = 0.044715_f32;
                    let inner = sqrt_2_over_pi * (x + coeff * x.powi(3));
                    0.5 * x * (1.0 + inner.tanh())
                })
                .collect();
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64Vec(input) => {
            let output: Vec<f64> = input
                .par_iter()
                .map(|&x| {
                    let sqrt_2_over_pi = 0.7978845608028654_f64;
                    let coeff = 0.044715_f64;
                    let inner = sqrt_2_over_pi * (x + coeff * x.powi(3));
                    0.5 * x * (1.0 + inner.tanh())
                })
                .collect();
            Ok(WorkloadData::F64Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// Execute Tanh activation - Hyperbolic Tangent
///
/// **Formula**: `f(x) = tanh(x) = (e^x - e^(-x)) / (e^x + e^(-x))`
/// **Range**: (-1, 1)
/// **Pattern**: Smooth S-curve
/// **Properties**:
/// - Zero-centered (better than sigmoid)
/// - Smooth gradients
/// - Vanishing gradient problem for large |x|
#[inline]
pub(super) fn execute_tanh(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            let output: Vec<f32> = input.par_iter().map(|&x| x.tanh()).collect();
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64Vec(input) => {
            let output: Vec<f64> = input.par_iter().map(|&x| x.tanh()).collect();
            Ok(WorkloadData::F64Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// Execute Sigmoid activation - Logistic Function
///
/// **Formula**: `f(x) = 1 / (1 + e^(-x))`
/// **Range**: (0, 1)
/// **Pattern**: Smooth S-curve
/// **Usage**: Binary classification, gates in LSTM/GRU
/// **Properties**:
/// - Outputs interpretable as probabilities
/// - NOT zero-centered (optimization issue)
/// - Vanishing gradient problem
#[inline]
pub(super) fn execute_sigmoid(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            let output: Vec<f32> = input
                .par_iter()
                .map(|&x| 1.0 / (1.0 + (-x).exp()))
                .collect();
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64Vec(input) => {
            let output: Vec<f64> = input
                .par_iter()
                .map(|&x| 1.0 / (1.0 + (-x).exp()))
                .collect();
            Ok(WorkloadData::F64Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// Execute Softmax activation - Normalized Exponential
///
/// **Formula**: `f(xᵢ) = exp(xᵢ) / Σⱼ exp(xⱼ)`
/// **Pattern**: Reduce (max) → Map (exp) → Reduce (sum) → Map (divide)
/// **Usage**: Multi-class classification, attention mechanisms
/// **Properties**:
/// - Outputs sum to 1 (probability distribution)
/// - Numerically stable with max-subtraction
/// - Differentiable
///
/// ## Computational Pattern
///
/// This is a **composite operation** built from simpler primitives:
/// 1. Find max (for numerical stability)
/// 2. Subtract max and exponentiate
/// 3. Sum all values
/// 4. Divide by sum
#[inline]
pub(super) fn execute_softmax(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            if input.is_empty() {
                return Ok(WorkloadData::F32Vec(vec![]));
            }

            // Step 1: Find max for numerical stability
            let max = input
                .par_iter()
                .copied()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);

            // Step 2: Compute exp(x - max) for numerical stability
            let exp_values: Vec<f32> = input.par_iter().map(|&x| (x - max).exp()).collect();

            // Step 3: Sum all exponentials
            let sum: f32 = exp_values.par_iter().sum();

            // Step 4: Normalize
            let output: Vec<f32> = exp_values.par_iter().map(|&x| x / sum).collect();

            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64Vec(input) => {
            if input.is_empty() {
                return Ok(WorkloadData::F64Vec(vec![]));
            }

            let max = input
                .par_iter()
                .copied()
                .max_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                .unwrap_or(0.0);

            let exp_values: Vec<f64> = input.par_iter().map(|&x| (x - max).exp()).collect();
            let sum: f64 = exp_values.par_iter().sum();
            let output: Vec<f64> = exp_values.par_iter().map(|&x| x / sum).collect();

            Ok(WorkloadData::F64Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// Execute Dropout - Stochastic Regularization
///
/// **Pattern**: Random masking with compensation
/// **Formula**:
/// - Training: `f(x) = x * mask / (1 - p)` where mask ~ Bernoulli(1 - p)
/// - Inference: `f(x) = x` (identity)
///   **Properties**:
///   - Prevents co-adaptation of neurons
/// - Ensemble effect
/// - Different behavior training vs inference
#[inline]
pub(super) fn execute_dropout(workload: Workload) -> Result<WorkloadData, ComputeError> {
    // For simplicity, return input unchanged (inference mode)
    // Full implementation would check training flag and apply dropout
    Ok(workload.input)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_relu_positive() {
        let input = vec![1.0, 2.0, 3.0];
        let workload = Workload {
            operation: OperationType::ReLU,
            input: WorkloadData::F32Vec(input.clone()),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 5,
            required_memory: 5 * 4,
        };

        let result = execute_relu(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            assert_eq!(output, input); // Positive values unchanged
        } else {
            panic!("Expected F32Vec");
        }
    }

    #[test]
    fn test_relu_negative() {
        let input = vec![-1.0, -2.0, -3.0];
        let workload = Workload {
            operation: OperationType::ReLU,
            input: WorkloadData::F32Vec(input),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 3,
            required_memory: 3 * 4,
        };

        let result = execute_relu(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            assert_eq!(output, vec![0.0, 0.0, 0.0]); // Negative values zeroed
        } else {
            panic!("Expected F32Vec");
        }
    }

    #[test]
    fn test_relu_mixed() {
        let input = vec![-1.0, 2.0, -3.0, 4.0];
        let workload = Workload {
            operation: OperationType::ReLU,
            input: WorkloadData::F32Vec(input),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 3,
            required_memory: 3 * 4,
        };

        let result = execute_relu(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            assert_eq!(output, vec![0.0, 2.0, 0.0, 4.0]);
        } else {
            panic!("Expected F32Vec");
        }
    }

    #[test]
    fn test_sigmoid_zero() {
        let input = vec![0.0];
        let workload = Workload {
            operation: OperationType::Sigmoid,
            input: WorkloadData::F32Vec(input),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 5,
            required_memory: 5 * 4,
        };

        let result = execute_sigmoid(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            assert!((output[0] - 0.5).abs() < 1e-6); // sigmoid(0) = 0.5
        } else {
            panic!("Expected F32Vec");
        }
    }

    #[test]
    fn test_softmax_sum() {
        let input = vec![1.0, 2.0, 3.0];
        let workload = Workload {
            operation: OperationType::Softmax,
            input: WorkloadData::F32Vec(input),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 4,
            required_memory: 4 * 4,
        };

        let result = execute_softmax(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            let sum: f32 = output.iter().sum();
            assert!((sum - 1.0).abs() < 1e-6); // Softmax outputs sum to 1
        } else {
            panic!("Expected F32Vec");
        }
    }

    #[test]
    fn test_softmax_monotonic() {
        let input = vec![1.0, 2.0, 3.0];
        let workload = Workload {
            operation: OperationType::Softmax,
            input: WorkloadData::F32Vec(input),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 4,
            required_memory: 4 * 4,
        };

        let result = execute_softmax(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            // Softmax preserves ordering
            assert!(output[0] < output[1]);
            assert!(output[1] < output[2]);
        } else {
            panic!("Expected F32Vec");
        }
    }
}
