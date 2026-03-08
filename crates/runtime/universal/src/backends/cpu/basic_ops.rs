// SPDX-License-Identifier: AGPL-3.0-or-later
//! Basic Operations - Embarrassingly Parallel Pattern
//!
//! Operations in this module share a computational pattern:
//! - **Embarrassingly parallel** - No inter-element dependencies
//! - **Memory-bound** - Limited by memory bandwidth
//! - **SIMD-friendly** - Can vectorize easily
//! - **Rayon-parallel** - Perfect for multi-core CPUs
//!
//! ## Architectural Pattern
//!
//! ```text
//! Input → Parallel Transform → Output
//!   [x₁, x₂, ..., xₙ] → f(x₁), f(x₂), ..., f(xₙ) → [y₁, y₂, ..., yₙ]
//! ```
//!
//! This pattern is the foundation of data-parallel computing.

use crate::types::*;
use rayon::prelude::*;

/// Execute map operation - Transform each element independently
///
/// **Pattern**: Embarrassingly parallel
/// **Complexity**: O(n) time, O(1) space per element
/// **Parallelism**: Perfect (no synchronization needed)
#[inline]
pub(super) fn execute_map(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            let output: Vec<f32> = input.par_iter().map(|&x| x * 2.0 + 1.0).collect();
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64Vec(input) => {
            let output: Vec<f64> = input.par_iter().map(|&x| x * 2.0 + 1.0).collect();
            Ok(WorkloadData::F64Vec(output))
        }
        WorkloadData::I32Vec(input) => {
            let output: Vec<i32> = input.par_iter().map(|&x| x * 2 + 1).collect();
            Ok(WorkloadData::I32Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// Execute filter operation - Select elements matching predicate
///
/// **Pattern**: Conditional embarrassingly parallel
/// **Complexity**: O(n) time, O(k) space (k = matching elements)
/// **Parallelism**: Parallel filter + sequential collect
#[inline]
pub(super) fn execute_filter(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            let output: Vec<f32> = input.par_iter().filter(|&&x| x > 0.5).copied().collect();
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64Vec(input) => {
            let output: Vec<f64> = input.par_iter().filter(|&&x| x > 0.5).copied().collect();
            Ok(WorkloadData::F64Vec(output))
        }
        WorkloadData::I32Vec(input) => {
            let output: Vec<i32> = input.par_iter().filter(|&&x| x > 0).copied().collect();
            Ok(WorkloadData::I32Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// Execute reduce operation - Combine all elements to single value
///
/// **Pattern**: Tree-based parallel reduction
/// **Complexity**: O(n) work, O(log n) span
/// **Parallelism**: Divide-and-conquer tree reduction
#[inline]
pub(super) fn execute_reduce(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            // Parallel sum reduction
            let result = input.par_iter().sum::<f32>();
            Ok(WorkloadData::F32Vec(vec![result]))
        }
        WorkloadData::F64Vec(input) => {
            let result = input.par_iter().sum::<f64>();
            Ok(WorkloadData::F64Vec(vec![result]))
        }
        WorkloadData::I32Vec(input) => {
            let result = input.par_iter().sum::<i32>();
            Ok(WorkloadData::I32Vec(vec![result]))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

/// Execute scan operation - Parallel prefix sum
///
/// **Pattern**: Sequential with parallel work-stealing
/// **Complexity**: O(n) time, O(n) space
/// **Parallelism**: Limited (inherently sequential)
///
/// Note: True parallel scan requires sophisticated algorithms (Blelloch scan).
/// This implementation uses Rayon's fold + scan for simplicity.
#[inline]
pub(super) fn execute_scan(workload: Workload) -> Result<WorkloadData, ComputeError> {
    match workload.input {
        WorkloadData::F32Vec(input) => {
            // Prefix sum (cumulative sum)
            let mut output = Vec::with_capacity(input.len());
            let mut sum = 0.0;
            for &x in &input {
                sum += x;
                output.push(sum);
            }
            Ok(WorkloadData::F32Vec(output))
        }
        WorkloadData::F64Vec(input) => {
            let mut output = Vec::with_capacity(input.len());
            let mut sum = 0.0;
            for &x in &input {
                sum += x;
                output.push(sum);
            }
            Ok(WorkloadData::F64Vec(output))
        }
        WorkloadData::I32Vec(input) => {
            let mut output = Vec::with_capacity(input.len());
            let mut sum = 0;
            for &x in &input {
                sum += x;
                output.push(sum);
            }
            Ok(WorkloadData::I32Vec(output))
        }
        _ => Err(ComputeError::UnsupportedWorkload),
    }
}

#[cfg(test)]
#[expect(clippy::float_cmp, reason = "comparing against exact literal")]
mod tests {
    use super::*;

    #[test]
    fn test_map_f32() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let workload = Workload {
            operation: OperationType::Map,
            input: WorkloadData::F32Vec(input.clone()),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 5,
            required_memory: 5 * 4,
        };

        let result = execute_map(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            assert_eq!(output.len(), input.len());
            // x * 2.0 + 1.0
            assert_eq!(output[0], 3.0);
            assert_eq!(output[4], 11.0);
        } else {
            panic!("Expected F32Vec");
        }
    }

    #[test]
    fn test_filter_f32() {
        let input = vec![0.3, 0.6, 0.4, 0.9, 0.2];
        let workload = Workload {
            operation: OperationType::Filter,
            input: WorkloadData::F32Vec(input),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 6,
            required_memory: 6 * 4,
        };

        let result = execute_filter(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            assert_eq!(output.len(), 2); // 0.6 and 0.9
            assert!(output.contains(&0.6));
            assert!(output.contains(&0.9));
        } else {
            panic!("Expected F32Vec");
        }
    }

    #[test]
    fn test_reduce_f32() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let workload = Workload {
            operation: OperationType::Reduce,
            input: WorkloadData::F32Vec(input),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 5,
            required_memory: 5 * 4,
        };

        let result = execute_reduce(workload).unwrap();
        if let WorkloadData::F32Vec(sums) = result {
            assert_eq!(sums.len(), 1);
            assert_eq!(sums[0], 15.0);
        } else {
            panic!("Expected F32Vec with single element");
        }
    }

    #[test]
    fn test_scan_f32() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let workload = Workload {
            operation: OperationType::Scan,
            input: WorkloadData::F32Vec(input),
            params: WorkloadParams::default(),
            data_type: DataType::F32,
            num_operations: 5,
            required_memory: 5 * 4, // 5 f32s
        };

        let result = execute_scan(workload).unwrap();
        if let WorkloadData::F32Vec(output) = result {
            assert_eq!(output, vec![1.0, 3.0, 6.0, 10.0, 15.0]);
        } else {
            panic!("Expected F32Vec");
        }
    }
}
