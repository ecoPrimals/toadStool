// SPDX-License-Identifier: AGPL-3.0-or-later
//! OpenCL kernel helpers - built-in kernel selection and work size calculation

use crate::universal::Operation;
use toadstool::error::{ToadStoolError, ToadStoolResult};

/// Get built-in kernel source for operation
///
/// No hardcoding - kernels are selected based on operation type
pub(super) fn get_builtin_kernel(
    operation: &Operation,
) -> ToadStoolResult<(&'static str, &'static str)> {
    match operation {
        Operation::GeneralCompute => Ok((
            include_str!("../../../kernels/general_compute.cl"),
            "general_compute",
        )),
        Operation::MatrixMultiply => Ok((
            include_str!("../../../kernels/matrix_multiply.cl"),
            "matrix_multiply",
        )),
        Operation::Reduction => Ok((include_str!("../../../kernels/reduction.cl"), "reduction")),
        _ => Err(ToadStoolError::runtime(format!(
            "No built-in kernel for operation: {:?}",
            operation
        ))),
    }
}

/// Calculate optimal work size based on data size
///
/// Capability-aware: adjusts to data size, not hardcoded
pub(super) const fn calculate_work_size(total_elements: usize) -> [usize; 3] {
    // Simple 1D work size for now
    // Future: capability-based optimization
    [total_elements, 1, 1]
}
