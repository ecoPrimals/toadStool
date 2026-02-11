//! Vector Operations - Memory-Bound Pattern
//!
//! Operations with memory access patterns:
//! - Gather/Scatter: Indirect memory access
//! - Dot Product: Reduction with multiply
//! - Elementwise Binary: SIMD pairwise operations

use crate::types::*;
use rayon::prelude::*;

#[inline]
pub(super) fn execute_dot_product(workload: Workload) -> Result<WorkloadData, ComputeError> {
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
pub(super) fn execute_elementwise_binary(workload: Workload) -> Result<WorkloadData, ComputeError> {
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

#[inline]
pub(super) fn execute_gather(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32VecIndexed(data, indices) => {
            let output: Vec<f32> = indices
                .iter()
                .map(|&i| *data.get(i).unwrap_or(&0.0))
                .collect();
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64VecIndexed(data, indices) => {
            let output: Vec<f64> = indices
                .iter()
                .map(|&i| *data.get(i).unwrap_or(&0.0))
                .collect();
            Ok(WorkloadData::F64Vec(output))
        }
        WorkloadData::I32VecIndexed(data, indices) => {
            let output: Vec<i32> = indices
                .iter()
                .map(|&i| *data.get(i).unwrap_or(&0))
                .collect();
            Ok(WorkloadData::I32Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

#[inline]
pub(super) fn execute_scatter(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32VecIndexed(data, indices) => {
            let size = indices
                .iter()
                .copied()
                .max()
                .map(|m| m + 1)
                .unwrap_or(0)
                .max(data.len());
            let mut output = vec![0.0f32; size];
            for (i, &idx) in indices.iter().enumerate() {
                if let Some(val) = data.get(i) {
                    if idx < output.len() {
                        output[idx] = *val;
                    }
                }
            }
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64VecIndexed(data, indices) => {
            let size = indices
                .iter()
                .copied()
                .max()
                .map(|m| m + 1)
                .unwrap_or(0)
                .max(data.len());
            let mut output = vec![0.0f64; size];
            for (i, &idx) in indices.iter().enumerate() {
                if let Some(val) = data.get(i) {
                    if idx < output.len() {
                        output[idx] = *val;
                    }
                }
            }
            Ok(WorkloadData::F64Vec(output))
        }
        WorkloadData::I32VecIndexed(data, indices) => {
            let size = indices
                .iter()
                .copied()
                .max()
                .map(|m| m + 1)
                .unwrap_or(0)
                .max(data.len());
            let mut output = vec![0i32; size];
            for (i, &idx) in indices.iter().enumerate() {
                if let Some(val) = data.get(i) {
                    if idx < output.len() {
                        output[idx] = *val;
                    }
                }
            }
            Ok(WorkloadData::I32Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}
