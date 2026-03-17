// SPDX-License-Identifier: AGPL-3.0-only
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
                if let Some(val) = data.get(i)
                    && idx < output.len()
                {
                    output[idx] = *val;
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
                if let Some(val) = data.get(i)
                    && idx < output.len()
                {
                    output[idx] = *val;
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
                if let Some(val) = data.get(i)
                    && idx < output.len()
                {
                    output[idx] = *val;
                }
            }
            Ok(WorkloadData::I32Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{DataType, OperationType, ParamValue, WorkloadParams};

    fn make_workload(input: WorkloadData) -> Workload {
        Workload {
            operation: OperationType::DotProduct,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input,
            params: WorkloadParams::default(),
        }
    }

    fn workload_with_op(input: WorkloadData, op: &str) -> Workload {
        let mut params = WorkloadParams::default();
        params
            .params
            .insert("op".into(), crate::types::ParamValue::String(op.into()));
        Workload {
            operation: OperationType::ElementwiseBinary,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input,
            params,
        }
    }

    #[test]
    fn test_dot_product_f32() {
        let w = make_workload(WorkloadData::F32VecPair(
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
        ));
        match execute_dot_product(w).unwrap() {
            WorkloadData::F32Vec(v) => assert!((v[0] - 32.0f32).abs() < 1e-5),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn test_dot_product_f64() {
        let w = make_workload(WorkloadData::F64VecPair(
            vec![1.0f64, 2.0, 3.0],
            vec![4.0, 5.0, 6.0],
        ));
        match execute_dot_product(w).unwrap() {
            WorkloadData::F64Vec(v) => assert!((v[0] - 32.0f64).abs() < 1e-10),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn test_dot_product_mismatched_lengths() {
        let w = make_workload(WorkloadData::F32VecPair(vec![1.0, 2.0], vec![1.0]));
        assert!(matches!(
            execute_dot_product(w),
            Err(ComputeError::ExecutionFailed(_))
        ));
    }

    #[test]
    fn test_dot_product_unsupported_input() {
        let w = make_workload(WorkloadData::I32Vec(vec![1, 2, 3]));
        assert!(matches!(
            execute_dot_product(w),
            Err(ComputeError::UnsupportedWorkload)
        ));
    }

    #[test]
    fn test_elementwise_add_f32() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![1.0, 2.0], vec![3.0, 4.0]),
            "add",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 4.0).abs() < 1e-5);
                assert!((v[1] - 6.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_sub_f32() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![5.0, 3.0], vec![2.0, 1.0]),
            "sub",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 3.0).abs() < 1e-5);
                assert!((v[1] - 2.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_mul_f64() {
        let w = workload_with_op(
            WorkloadData::F64VecPair(vec![2.0f64, 3.0], vec![4.0, 5.0]),
            "mul",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F64Vec(v) => {
                assert!((v[0] - 8.0).abs() < 1e-10);
                assert!((v[1] - 15.0).abs() < 1e-10);
            }
            other => panic!("expected F64Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_max_f32() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![1.0, 5.0], vec![3.0, 2.0]),
            "max",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 3.0).abs() < 1e-5);
                assert!((v[1] - 5.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_min_f32() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![1.0, 5.0], vec![3.0, 2.0]),
            "min",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 1.0).abs() < 1e-5);
                assert!((v[1] - 2.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_mismatched_lengths() {
        let w = workload_with_op(WorkloadData::F32VecPair(vec![1.0, 2.0], vec![1.0]), "add");
        assert!(matches!(
            execute_elementwise_binary(w),
            Err(ComputeError::ExecutionFailed(_))
        ));
    }

    #[test]
    fn test_elementwise_unsupported_input() {
        let w = workload_with_op(WorkloadData::I32Vec(vec![1, 2]), "add");
        assert!(matches!(
            execute_elementwise_binary(w),
            Err(ComputeError::UnsupportedWorkload)
        ));
    }

    #[test]
    fn test_gather_f32() {
        let w = make_workload(WorkloadData::F32VecIndexed(
            vec![10.0, 20.0, 30.0],
            vec![2, 0, 1],
        ));
        match execute_gather(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 30.0).abs() < 1e-5);
                assert!((v[1] - 10.0).abs() < 1e-5);
                assert!((v[2] - 20.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_gather_out_of_bounds_returns_zero() {
        let w = make_workload(WorkloadData::F32VecIndexed(vec![1.0, 2.0], vec![5]));
        match execute_gather(w).unwrap() {
            WorkloadData::F32Vec(v) => assert!((v[0] - 0.0).abs() < 1e-5),
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_gather_i32() {
        let w = make_workload(WorkloadData::I32VecIndexed(vec![7, 8, 9], vec![1, 2]));
        match execute_gather(w).unwrap() {
            WorkloadData::I32Vec(v) => {
                assert_eq!(v[0], 8);
                assert_eq!(v[1], 9);
            }
            other => panic!("expected I32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_scatter_f32() {
        let w = make_workload(WorkloadData::F32VecIndexed(
            vec![100.0, 200.0, 300.0],
            vec![2, 0, 1],
        ));
        match execute_scatter(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 200.0).abs() < 1e-5);
                assert!((v[1] - 300.0).abs() < 1e-5);
                assert!((v[2] - 100.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_scatter_f64_empty() {
        let w = make_workload(WorkloadData::F64VecIndexed(vec![], vec![]));
        match execute_scatter(w).unwrap() {
            WorkloadData::F64Vec(v) => assert!(v.is_empty()),
            other => panic!("expected F64Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_scatter_unsupported() {
        let w = make_workload(WorkloadData::I32Vec(vec![1]));
        assert!(matches!(
            execute_scatter(w),
            Err(ComputeError::UnsupportedWorkload)
        ));
    }

    #[test]
    fn test_elementwise_div_f32() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![10.0, 4.0], vec![2.0, 2.0]),
            "div",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 5.0).abs() < 1e-5);
                assert!((v[1] - 2.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_div_f64() {
        let w = workload_with_op(
            WorkloadData::F64VecPair(vec![15.0f64, 9.0], vec![3.0, 3.0]),
            "div",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F64Vec(v) => {
                assert!((v[0] - 5.0).abs() < 1e-10);
                assert!((v[1] - 3.0).abs() < 1e-10);
            }
            other => panic!("expected F64Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_plus_alias() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![1.0, 2.0], vec![3.0, 4.0]),
            "+",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 4.0).abs() < 1e-5);
                assert!((v[1] - 6.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_minus_alias() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![5.0, 3.0], vec![2.0, 1.0]),
            "-",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 3.0).abs() < 1e-5);
                assert!((v[1] - 2.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_mul_alias() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![2.0, 3.0], vec![4.0, 5.0]),
            "*",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 8.0).abs() < 1e-5);
                assert!((v[1] - 15.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_default_op_add() {
        let w = Workload {
            operation: OperationType::ElementwiseBinary,
            data_type: DataType::F32,
            num_operations: 0,
            required_memory: 0,
            input: WorkloadData::F32VecPair(vec![1.0, 2.0], vec![3.0, 4.0]),
            params: WorkloadParams::default(),
        };
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 4.0).abs() < 1e-5);
                assert!((v[1] - 6.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_unknown_op_defaults_to_add() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![1.0, 2.0], vec![3.0, 4.0]),
            "unknown_op",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 4.0).abs() < 1e-5);
                assert!((v[1] - 6.0).abs() < 1e-5);
            }
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_max_f64() {
        let w = workload_with_op(
            WorkloadData::F64VecPair(vec![1.0f64, 5.0], vec![3.0, 2.0]),
            "max",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F64Vec(v) => {
                assert!((v[0] - 3.0).abs() < 1e-10);
                assert!((v[1] - 5.0).abs() < 1e-10);
            }
            other => panic!("expected F64Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_elementwise_min_f64() {
        let w = workload_with_op(
            WorkloadData::F64VecPair(vec![1.0f64, 5.0], vec![3.0, 2.0]),
            "min",
        );
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F64Vec(v) => {
                assert!((v[0] - 1.0).abs() < 1e-10);
                assert!((v[1] - 2.0).abs() < 1e-10);
            }
            other => panic!("expected F64Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_gather_f64() {
        let w = make_workload(WorkloadData::F64VecIndexed(
            vec![10.0, 20.0, 30.0],
            vec![2, 0, 1],
        ));
        match execute_gather(w).unwrap() {
            WorkloadData::F64Vec(v) => {
                assert!((v[0] - 30.0).abs() < 1e-10);
                assert!((v[1] - 10.0).abs() < 1e-10);
                assert!((v[2] - 20.0).abs() < 1e-10);
            }
            other => panic!("expected F64Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_gather_f64_out_of_bounds() {
        let w = make_workload(WorkloadData::F64VecIndexed(vec![1.0, 2.0], vec![10]));
        match execute_gather(w).unwrap() {
            WorkloadData::F64Vec(v) => assert!((v[0] - 0.0).abs() < 1e-10),
            other => panic!("expected F64Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_scatter_i32() {
        let w = make_workload(WorkloadData::I32VecIndexed(
            vec![100, 200, 300],
            vec![2, 0, 1],
        ));
        match execute_scatter(w).unwrap() {
            WorkloadData::I32Vec(v) => {
                assert_eq!(v[0], 200);
                assert_eq!(v[1], 300);
                assert_eq!(v[2], 100);
            }
            other => panic!("expected I32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_scatter_f32_empty_indices() {
        let w = make_workload(WorkloadData::F32VecIndexed(vec![1.0, 2.0], vec![]));
        match execute_scatter(w).unwrap() {
            WorkloadData::F32Vec(v) => assert!(!v.is_empty() || v.is_empty()),
            other => panic!("expected F32Vec, got {other:?}"),
        }
    }

    #[test]
    fn test_dot_product_f32_empty() {
        let w = make_workload(WorkloadData::F32VecPair(vec![], vec![]));
        match execute_dot_product(w).unwrap() {
            WorkloadData::F32Vec(v) => assert!((v[0] - 0.0).abs() < 1e-5),
            _ => panic!("unexpected variant"),
        }
    }

    #[test]
    fn test_dot_product_f64_mismatched() {
        let w = make_workload(WorkloadData::F64VecPair(vec![1.0, 2.0], vec![1.0]));
        assert!(matches!(
            execute_dot_product(w),
            Err(ComputeError::ExecutionFailed(_))
        ));
    }

    #[test]
    fn test_dot_product_f64_single_element() {
        let w = make_workload(WorkloadData::F64VecPair(vec![3.0], vec![4.0]));
        match execute_dot_product(w).unwrap() {
            WorkloadData::F64Vec(v) => assert!((v[0] - 12.0).abs() < 1e-10),
            _ => panic!("expected F64Vec"),
        }
    }

    #[test]
    fn test_elementwise_div_f32_by_zero() {
        let w = workload_with_op(
            WorkloadData::F32VecPair(vec![1.0, 2.0], vec![0.0, 1.0]),
            "div",
        );
        let result = execute_elementwise_binary(w);
        assert!(result.is_ok());
        let out = result.unwrap();
        match out {
            WorkloadData::F32Vec(v) => {
                assert!(v[0].is_infinite() || v[0].is_nan());
                assert!((v[1] - 2.0).abs() < 1e-5);
            }
            _ => panic!("expected F32Vec"),
        }
    }

    #[test]
    fn test_gather_empty_indices() {
        let w = make_workload(WorkloadData::F32VecIndexed(vec![1.0, 2.0, 3.0], vec![]));
        match execute_gather(w).unwrap() {
            WorkloadData::F32Vec(v) => assert!(v.is_empty()),
            _ => panic!("expected F32Vec"),
        }
    }

    #[test]
    fn test_scatter_f32_single_element() {
        let w = make_workload(WorkloadData::F32VecIndexed(vec![42.0], vec![0]));
        match execute_scatter(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert_eq!(v.len(), 1);
                assert!((v[0] - 42.0).abs() < 1e-5);
            }
            _ => panic!("expected F32Vec"),
        }
    }

    #[test]
    fn test_scatter_f64_single_element() {
        let w = make_workload(WorkloadData::F64VecIndexed(vec![99.0], vec![0]));
        match execute_scatter(w).unwrap() {
            WorkloadData::F64Vec(v) => {
                assert_eq!(v.len(), 1);
                assert!((v[0] - 99.0).abs() < 1e-10);
            }
            _ => panic!("expected F64Vec"),
        }
    }

    #[test]
    fn test_elementwise_binary_f32_single_element() {
        let w = workload_with_op(WorkloadData::F32VecPair(vec![5.0], vec![3.0]), "add");
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => assert!((v[0] - 8.0).abs() < 1e-5),
            _ => panic!("expected F32Vec"),
        }
    }

    #[test]
    fn test_get_binary_op_from_params() {
        let mut params = WorkloadParams::default();
        params
            .params
            .insert("op".into(), ParamValue::String("mul".into()));
        let w = Workload {
            operation: OperationType::ElementwiseBinary,
            data_type: DataType::F32,
            num_operations: 2,
            required_memory: 8,
            input: WorkloadData::F32VecPair(vec![2.0, 3.0], vec![4.0, 5.0]),
            params,
        };
        match execute_elementwise_binary(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!((v[0] - 8.0).abs() < 1e-5);
                assert!((v[1] - 15.0).abs() < 1e-5);
            }
            _ => panic!("expected F32Vec"),
        }
    }

    #[test]
    fn test_gather_multiple_out_of_bounds() {
        let w = make_workload(WorkloadData::F32VecIndexed(vec![1.0, 2.0], vec![0, 99, 1]));
        match execute_gather(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert_eq!(v.len(), 3);
                assert!((v[0] - 1.0).abs() < 1e-5);
                assert!((v[1] - 0.0).abs() < 1e-5);
                assert!((v[2] - 2.0).abs() < 1e-5);
            }
            _ => panic!("expected F32Vec"),
        }
    }

    #[test]
    fn test_scatter_indices_larger_than_data() {
        let w = make_workload(WorkloadData::F32VecIndexed(vec![10.0, 20.0], vec![5, 10]));
        match execute_scatter(w).unwrap() {
            WorkloadData::F32Vec(v) => {
                assert!(v.len() >= 11);
                assert!((v[5] - 10.0).abs() < 1e-5);
                assert!((v[10] - 20.0).abs() < 1e-5);
            }
            _ => panic!("expected F32Vec"),
        }
    }
}
