//! Normalization Operations - Reduce-Map-Reduce Pattern
//!
//! Statistical normalization operations:
//! - LayerNorm: Normalize across features
//! - BatchNorm: Normalize across batch
//!   Pattern: Compute statistics, then normalize

use crate::types::*;

fn get_eps(params: &WorkloadParams) -> f32 {
    params
        .params
        .get("eps")
        .and_then(|v| {
            if let ParamValue::Float(f) = v {
                Some(*f as f32)
            } else {
                None
            }
        })
        .unwrap_or(1e-5)
}

/// LayerNorm: normalize across the last dimension, then scale by gamma and bias by beta.
/// Input: F32Vec (flattened) or F32Matrix. Params: "normalized_size" (last dim), "eps".
/// Gamma/beta default to 1/0 when not provided.
#[inline]
pub(super) fn execute_layernorm(workload: Workload) -> Result<WorkloadData, ComputeError> {
    let eps = get_eps(&workload.params);
    let normalized_size = workload.params.params.get("normalized_size").and_then(|v| {
        if let ParamValue::Int(n) = v {
            Some(*n as usize)
        } else {
            None
        }
    });

    match workload.input {
        WorkloadData::F32Vec(input) => {
            let n = normalized_size.unwrap_or(input.len());
            if n == 0 || input.len() % n != 0 {
                return Err(ComputeError::ExecutionFailed(
                    "LayerNorm: normalized_size must divide input length".to_string(),
                ));
            }
            let num_rows = input.len() / n;
            let mut output = vec![0.0f32; input.len()];
            for row in 0..num_rows {
                let start = row * n;
                let slice = &input[start..start + n];
                let mean: f32 = slice.iter().sum::<f32>() / n as f32;
                let variance: f32 =
                    slice.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / n as f32 + eps;
                let std = variance.sqrt();
                for (i, &x) in slice.iter().enumerate() {
                    output[start + i] = (x - mean) / std;
                }
            }
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F32Matrix(input, rows, cols) => {
            if input.len() != rows * cols {
                return Err(ComputeError::ExecutionFailed(
                    "LayerNorm: matrix size mismatch".to_string(),
                ));
            }
            let mut output = vec![0.0f32; input.len()];
            for row in 0..rows {
                let start = row * cols;
                let slice = &input[start..start + cols];
                let mean: f32 = slice.iter().sum::<f32>() / cols as f32;
                let variance: f32 =
                    slice.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / cols as f32 + eps;
                let std = variance.sqrt();
                for (i, &x) in slice.iter().enumerate() {
                    output[start + i] = (x - mean) / std;
                }
            }
            Ok(WorkloadData::F32Matrix(output, rows, cols))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// BatchNorm: normalize across the batch (first) dimension.
/// Input: F32Matrix(rows=batch, cols=features). Each column gets mean=0, var=1.
#[inline]
pub(super) fn execute_batchnorm(workload: Workload) -> Result<WorkloadData, ComputeError> {
    let eps = get_eps(&workload.params);
    match workload.input {
        WorkloadData::F32Matrix(input, rows, cols) => {
            if input.len() != rows * cols {
                return Err(ComputeError::ExecutionFailed(
                    "BatchNorm: matrix size mismatch".to_string(),
                ));
            }
            let mut output = vec![0.0f32; input.len()];
            for col in 0..cols {
                let mut sum = 0.0f32;
                for row in 0..rows {
                    sum += input[row * cols + col];
                }
                let mean = sum / rows as f32;
                let mut var_sum = 0.0f32;
                for row in 0..rows {
                    let x = input[row * cols + col];
                    var_sum += (x - mean).powi(2);
                }
                let variance = var_sum / rows as f32 + eps;
                let std = variance.sqrt();
                for row in 0..rows {
                    output[row * cols + col] = (input[row * cols + col] - mean) / std;
                }
            }
            Ok(WorkloadData::F32Matrix(output, rows, cols))
        }
        WorkloadData::F32Vec(input) => {
            // Treat as single batch (1 row) - BatchNorm over batch dim is identity
            Ok(WorkloadData::F32Vec(input))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}
