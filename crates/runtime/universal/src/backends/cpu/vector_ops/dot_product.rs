// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dot-product reduction over vector pairs (f32/f64).

use crate::types::*;
use rayon::prelude::*;

#[inline]
pub(crate) fn execute_dot_product(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32VecPair(a, b) => {
            if a.len() != b.len() {
                return Err(ComputeError::ExecutionFailed(
                    "DotProduct: vectors must have same length".to_string(),
                ));
            }
            let sum: f32 = a.par_iter().zip(b.par_iter()).map(|(x, y)| x * y).sum();
            Ok(WorkloadData::F32Vec(vec![sum]))
        }
        WorkloadData::F64VecPair(a, b) => {
            if a.len() != b.len() {
                return Err(ComputeError::ExecutionFailed(
                    "DotProduct: vectors must have same length".to_string(),
                ));
            }
            let sum: f64 = a.par_iter().zip(b.par_iter()).map(|(x, y)| x * y).sum();
            Ok(WorkloadData::F64Vec(vec![sum]))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}
