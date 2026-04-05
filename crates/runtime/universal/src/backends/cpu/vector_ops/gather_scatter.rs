// SPDX-License-Identifier: AGPL-3.0-or-later
//! Indirect memory access: gather (read by index) and scatter (write by index).

use crate::types::*;

#[inline]
pub(crate) fn execute_gather(workload: Workload) -> Result<WorkloadData, ComputeError> {
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
pub(crate) fn execute_scatter(workload: Workload) -> Result<WorkloadData, ComputeError> {
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
