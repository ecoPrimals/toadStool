// SPDX-License-Identifier: AGPL-3.0-only
//! Transform Operations - Layout Transformation Pattern
//!
//! Operations that change data layout:
//! - Transpose: Cache-friendly blocking
//! - Reshape: Zero-copy when possible

use crate::types::*;

#[inline]
pub(super) fn execute_transpose(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Matrix(input, rows, cols) => {
            if input.len() != rows * cols {
                return Err(ComputeError::ExecutionFailed(
                    "Transpose: matrix size mismatch".to_string(),
                ));
            }
            let mut output = vec![0.0f32; rows * cols];
            for i in 0..rows {
                for j in 0..cols {
                    output[j * rows + i] = input[i * cols + j];
                }
            }
            Ok(WorkloadData::F32Matrix(output, cols, rows))
        }
        WorkloadData::F64Matrix(input, rows, cols) => {
            if input.len() != rows * cols {
                return Err(ComputeError::ExecutionFailed(
                    "Transpose: matrix size mismatch".to_string(),
                ));
            }
            let mut output = vec![0.0f64; rows * cols];
            for i in 0..rows {
                for j in 0..cols {
                    output[j * rows + i] = input[i * cols + j];
                }
            }
            Ok(WorkloadData::F64Matrix(output, cols, rows))
        }
        WorkloadData::I32Matrix(input, rows, cols) => {
            if input.len() != rows * cols {
                return Err(ComputeError::ExecutionFailed(
                    "Transpose: matrix size mismatch".to_string(),
                ));
            }
            let mut output = vec![0i32; rows * cols];
            for i in 0..rows {
                for j in 0..cols {
                    output[j * rows + i] = input[i * cols + j];
                }
            }
            Ok(WorkloadData::I32Matrix(output, cols, rows))
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
            operation: OperationType::Transpose,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input,
            params: WorkloadParams::default(),
        }
    }

    #[test]
    fn test_transpose_f32_2x3() {
        // [[1,2,3],[4,5,6]] -> [[1,4],[2,5],[3,6]]
        let w = make_workload(WorkloadData::F32Matrix(
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            2,
            3,
        ));
        match execute_transpose(w).unwrap() {
            WorkloadData::F32Matrix(v, rows, cols) => {
                assert_eq!(rows, 3);
                assert_eq!(cols, 2);
                assert!((v[0] - 1.0).abs() < 1e-5); // [0,0]
                assert!((v[1] - 4.0).abs() < 1e-5); // [0,1]
                assert!((v[2] - 2.0).abs() < 1e-5); // [1,0]
                assert!((v[3] - 5.0).abs() < 1e-5); // [1,1]
                assert!((v[4] - 3.0).abs() < 1e-5); // [2,0]
                assert!((v[5] - 6.0).abs() < 1e-5); // [2,1]
            }
            other => panic!("expected F32Matrix, got {other:?}"),
        }
    }

    #[test]
    fn test_transpose_f64_2x2() {
        let w = make_workload(WorkloadData::F64Matrix(vec![1.0, 2.0, 3.0, 4.0], 2, 2));
        match execute_transpose(w).unwrap() {
            WorkloadData::F64Matrix(v, rows, cols) => {
                assert_eq!(rows, 2);
                assert_eq!(cols, 2);
                assert!((v[0] - 1.0).abs() < 1e-10); // [0,0]
                assert!((v[1] - 3.0).abs() < 1e-10); // [0,1]
                assert!((v[2] - 2.0).abs() < 1e-10); // [1,0]
                assert!((v[3] - 4.0).abs() < 1e-10); // [1,1]
            }
            other => panic!("expected F64Matrix, got {other:?}"),
        }
    }

    #[test]
    fn test_transpose_i32_1x4() {
        let w = make_workload(WorkloadData::I32Matrix(vec![10, 20, 30, 40], 1, 4));
        match execute_transpose(w).unwrap() {
            WorkloadData::I32Matrix(v, rows, cols) => {
                assert_eq!(rows, 4);
                assert_eq!(cols, 1);
                assert_eq!(v, vec![10, 20, 30, 40]);
            }
            other => panic!("expected I32Matrix, got {other:?}"),
        }
    }

    #[test]
    fn test_transpose_f32_size_mismatch() {
        let w = make_workload(WorkloadData::F32Matrix(vec![1.0, 2.0, 3.0], 2, 2));
        assert!(matches!(
            execute_transpose(w),
            Err(ComputeError::ExecutionFailed(_))
        ));
    }

    #[test]
    fn test_transpose_unsupported() {
        let w = make_workload(WorkloadData::F32Vec(vec![1.0, 2.0]));
        assert!(matches!(
            execute_transpose(w),
            Err(ComputeError::UnsupportedWorkload)
        ));
    }

    #[test]
    fn test_transpose_double_transpose_is_identity() {
        let original = vec![1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
        let w = make_workload(WorkloadData::F32Matrix(original.clone(), 2, 3));
        let transposed = match execute_transpose(w).unwrap() {
            WorkloadData::F32Matrix(v, rows, cols) => (v, rows, cols),
            other => panic!("expected F32Matrix, got {other:?}"),
        };
        let w2 = make_workload(WorkloadData::F32Matrix(
            transposed.0,
            transposed.1,
            transposed.2,
        ));
        match execute_transpose(w2).unwrap() {
            WorkloadData::F32Matrix(v, rows, cols) => {
                assert_eq!(rows, 2);
                assert_eq!(cols, 3);
                for (a, b) in v.iter().zip(original.iter()) {
                    assert!((a - b).abs() < 1e-5);
                }
            }
            other => panic!("expected F32Matrix, got {other:?}"),
        }
    }
}
