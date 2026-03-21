// SPDX-License-Identifier: AGPL-3.0-only
//! Pairwise elementwise binary ops (SIMD-friendly parallel zip).

use crate::types::*;
use rayon::prelude::*;

fn get_binary_op(params: &WorkloadParams) -> &str {
    params
        .params
        .get("op")
        .and_then(|v| {
            if let ParamValue::String(s) = v {
                Some(s.as_str())
            } else {
                None
            }
        })
        .unwrap_or("add")
}

#[inline]
pub(crate) fn execute_elementwise_binary(workload: Workload) -> Result<WorkloadData, ComputeError> {
    let op = get_binary_op(&workload.params);
    match workload.input {
        WorkloadData::F32VecPair(a, b) => {
            if a.len() != b.len() {
                return Err(ComputeError::ExecutionFailed(
                    "ElementwiseBinary: vectors must have same length".to_string(),
                ));
            }
            let output: Vec<f32> = match op {
                "add" | "+" => a.par_iter().zip(b.par_iter()).map(|(x, y)| x + y).collect(),
                "sub" | "-" => a.par_iter().zip(b.par_iter()).map(|(x, y)| x - y).collect(),
                "mul" | "*" => a.par_iter().zip(b.par_iter()).map(|(x, y)| x * y).collect(),
                "div" | "/" => a.par_iter().zip(b.par_iter()).map(|(x, y)| x / y).collect(),
                "max" => a
                    .par_iter()
                    .zip(b.par_iter())
                    .map(|(x, y)| x.max(*y))
                    .collect(),
                "min" => a
                    .par_iter()
                    .zip(b.par_iter())
                    .map(|(x, y)| x.min(*y))
                    .collect(),
                _ => a.par_iter().zip(b.par_iter()).map(|(x, y)| x + y).collect(),
            };
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64VecPair(a, b) => {
            if a.len() != b.len() {
                return Err(ComputeError::ExecutionFailed(
                    "ElementwiseBinary: vectors must have same length".to_string(),
                ));
            }
            let output: Vec<f64> = match op {
                "add" | "+" => a.par_iter().zip(b.par_iter()).map(|(x, y)| x + y).collect(),
                "sub" | "-" => a.par_iter().zip(b.par_iter()).map(|(x, y)| x - y).collect(),
                "mul" | "*" => a.par_iter().zip(b.par_iter()).map(|(x, y)| x * y).collect(),
                "div" | "/" => a.par_iter().zip(b.par_iter()).map(|(x, y)| x / y).collect(),
                "max" => a
                    .par_iter()
                    .zip(b.par_iter())
                    .map(|(x, y)| x.max(*y))
                    .collect(),
                "min" => a
                    .par_iter()
                    .zip(b.par_iter())
                    .map(|(x, y)| x.min(*y))
                    .collect(),
                _ => a.par_iter().zip(b.par_iter()).map(|(x, y)| x + y).collect(),
            };
            Ok(WorkloadData::F64Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}
