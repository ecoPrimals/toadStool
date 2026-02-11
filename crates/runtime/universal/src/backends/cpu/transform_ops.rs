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
