// SPDX-License-Identifier: AGPL-3.0-only
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DataType, OperationType, WorkloadParams};

    fn make_workload(input: WorkloadData) -> Workload {
        Workload {
            operation: OperationType::LayerNorm,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input,
            params: WorkloadParams::default(),
        }
    }

    fn workload_with_norm_size(input: WorkloadData, n: i64) -> Workload {
        let mut params = WorkloadParams::default();
        params
            .params
            .insert("normalized_size".into(), crate::types::ParamValue::Int(n));
        Workload {
            operation: OperationType::LayerNorm,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input,
            params,
        }
    }

    #[test]
    fn test_layernorm_f32_vec_zero_mean_unit_var() {
        // [1,2,3]: mean=2, var=2/3, std=sqrt(2/3+eps)
        let w = make_workload(WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]));
        match execute_layernorm(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                let mean: f32 = v.iter().sum::<f32>() / v.len() as f32;
                assert!(mean.abs() < 1e-5, "mean should be ~0, got {mean}");
                // all equal spacing means first and last are negatives of each other
                assert!(v[0] < 0.0);
                assert!(v[2] > 0.0);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_layernorm_with_explicit_normalized_size() {
        // 2 rows of 2, each normalized independently
        let w = workload_with_norm_size(WorkloadData::F32Vec(vec![1.0, 3.0, 5.0, 7.0]), 2);
        match execute_layernorm(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert_eq!(v.len(), 4);
                // Row 0: [1,3] -> mean=2, each row normalized to ~[-1, 1]
                let row0_mean = f32::midpoint(v[0], v[1]);
                assert!(row0_mean.abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_layernorm_bad_normalized_size() {
        // normalized_size=3 does not divide len=4
        let w = workload_with_norm_size(WorkloadData::F32Vec(vec![1.0, 2.0, 3.0, 4.0]), 3);
        assert!(matches!(
            execute_layernorm(w),
            Err(ComputeError::ExecutionFailed(_))
        ));
    }

    #[test]
    fn test_layernorm_matrix() {
        let w = make_workload(WorkloadData::F32Matrix(vec![1.0, 2.0, 3.0, 4.0], 2, 2));
        match execute_layernorm(w).unwrap() {
            WorkloadData::F32Matrix(v, rows, cols) => {
                assert_eq!(rows, 2);
                assert_eq!(cols, 2);
                assert_eq!(v.len(), 4);
            }
            other => panic!("expected F32Matrix, got {other:?}"),
        }
    }

    #[test]
    fn test_layernorm_matrix_size_mismatch() {
        let w = make_workload(WorkloadData::F32Matrix(vec![1.0, 2.0, 3.0], 2, 2));
        assert!(matches!(
            execute_layernorm(w),
            Err(ComputeError::ExecutionFailed(_))
        ));
    }

    #[test]
    fn test_layernorm_unsupported() {
        let w = make_workload(WorkloadData::I32Vec(vec![1]));
        assert!(matches!(
            execute_layernorm(w),
            Err(ComputeError::UnsupportedWorkload)
        ));
    }

    #[test]
    fn test_batchnorm_matrix_normalizes_columns() {
        // 4 rows x 2 cols; each column normalized to ~zero mean
        let input = vec![1.0f32, 10.0, 2.0, 20.0, 3.0, 30.0, 4.0, 40.0];
        let w = Workload {
            operation: OperationType::BatchNorm,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input: WorkloadData::F32Matrix(input, 4, 2),
            params: WorkloadParams::default(),
        };
        match execute_batchnorm(w).unwrap() {
            WorkloadData::F32Matrix(v, rows, cols) => {
                assert_eq!(rows, 4);
                assert_eq!(cols, 2);
                // Column 0 mean should be ~0
                let col0_mean: f32 = (0..rows).map(|r| v[r * cols]).sum::<f32>() / rows as f32;
                assert!(col0_mean.abs() < 1e-4, "col0 mean should be ~0");
            }
            other => panic!("expected F32Matrix, got {other:?}"),
        }
    }

    #[test]
    fn test_batchnorm_matrix_size_mismatch() {
        let w = Workload {
            operation: OperationType::BatchNorm,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input: WorkloadData::F32Matrix(vec![1.0, 2.0, 3.0], 2, 2),
            params: WorkloadParams::default(),
        };
        assert!(matches!(
            execute_batchnorm(w),
            Err(ComputeError::ExecutionFailed(_))
        ));
    }

    #[test]
    fn test_batchnorm_vec_identity() {
        // Single-batch batchnorm over a vec is identity
        let w = Workload {
            operation: OperationType::BatchNorm,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input: WorkloadData::F32Vec(vec![1.0, 2.0, 3.0]),
            params: WorkloadParams::default(),
        };
        match execute_batchnorm(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 1.0).abs() < 1e-5);
                assert!((v[1] - 2.0).abs() < 1e-5);
                assert!((v[2] - 3.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_batchnorm_unsupported() {
        let w = Workload {
            operation: OperationType::BatchNorm,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input: WorkloadData::I32Vec(vec![1]),
            params: WorkloadParams::default(),
        };
        assert!(matches!(
            execute_batchnorm(w),
            Err(ComputeError::UnsupportedWorkload)
        ));
    }
}
