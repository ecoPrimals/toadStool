//! Reduction operations: Sum, Mean, Max, Min, Var, Std, Cumsum, Prod

use crate::error::{BarracudaError, Result};

pub struct Sum;

impl Sum {
    pub fn execute(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>> {
        if axis.is_none() {
            return Ok(vec![data.iter().sum()]);
        }

        let axis = axis.unwrap();
        if axis >= shape.len() {
            return Err(BarracudaError::invalid_params(
                "Sum",
                format!("Axis {axis} out of bounds for shape {shape:?}"),
            ));
        }

        Self::reduce_along_axis(data, shape, axis, |acc, val| acc + val, 0.0)
    }

    fn reduce_along_axis<F>(
        data: &[f32],
        shape: &[usize],
        axis: usize,
        op: F,
        init: f32,
    ) -> Result<Vec<f32>>
    where
        F: Fn(f32, f32) -> f32 + Copy,
    {
        let outer_size: usize = shape[..axis].iter().product();
        let axis_size = shape[axis];
        let inner_size: usize = shape[axis + 1..].iter().product();
        let output_size = outer_size * inner_size;

        let mut output = vec![init; output_size];

        for outer in 0..outer_size {
            for inner in 0..inner_size {
                let out_idx = outer * inner_size + inner;
                for ax in 0..axis_size {
                    let in_idx = outer * axis_size * inner_size + ax * inner_size + inner;
                    output[out_idx] = op(output[out_idx], data[in_idx]);
                }
            }
        }

        Ok(output)
    }
}

/// Mean Reduction Operation
pub struct Mean;

impl Mean {
    pub fn execute(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>> {
        let sum_result = Sum::execute(data, shape, axis)?;

        let count = if axis.is_none() {
            data.len() as f32
        } else {
            shape[axis.unwrap()] as f32
        };

        Ok(sum_result.iter().map(|&x| x / count).collect())
    }
}

/// Max Reduction Operation
pub struct Max;

impl Max {
    pub fn execute(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>> {
        if data.is_empty() {
            return Err(BarracudaError::invalid_params(
                "Max",
                "Cannot find max of empty array",
            ));
        }

        if axis.is_none() {
            return Ok(vec![data.iter().copied().fold(f32::NEG_INFINITY, f32::max)]);
        }

        let axis = axis.unwrap();
        if axis >= shape.len() {
            return Err(BarracudaError::invalid_params(
                "Max",
                format!("Axis {axis} out of bounds"),
            ));
        }

        Sum::reduce_along_axis(data, shape, axis, f32::max, f32::NEG_INFINITY)
    }
}

/// Min Reduction Operation
pub struct Min;

impl Min {
    pub fn execute(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>> {
        if data.is_empty() {
            return Err(BarracudaError::invalid_params(
                "Min",
                "Cannot find min of empty array",
            ));
        }

        if axis.is_none() {
            return Ok(vec![data.iter().copied().fold(f32::INFINITY, f32::min)]);
        }

        let axis = axis.unwrap();
        if axis >= shape.len() {
            return Err(BarracudaError::invalid_params(
                "Min",
                format!("Axis {axis} out of bounds"),
            ));
        }

        Sum::reduce_along_axis(data, shape, axis, f32::min, f32::INFINITY)
    }
}

/// Variance Reduction Operation
pub struct Var;

impl Var {
    pub fn execute(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>> {
        let mean = Mean::execute(data, shape, axis)?;

        if axis.is_none() {
            let mean_val = mean[0];
            let variance =
                data.iter().map(|&x| (x - mean_val).powi(2)).sum::<f32>() / data.len() as f32;
            return Ok(vec![variance]);
        }

        let axis = axis.unwrap();
        let axis_size = shape[axis];
        let mut squared_diffs = Vec::with_capacity(data.len());

        let outer_size: usize = shape[..axis].iter().product();
        let inner_size: usize = shape[axis + 1..].iter().product();

        for outer in 0..outer_size {
            for ax in 0..axis_size {
                for inner in 0..inner_size {
                    let mean_idx = outer * inner_size + inner;
                    let data_idx = outer * axis_size * inner_size + ax * inner_size + inner;
                    let diff = data[data_idx] - mean[mean_idx];
                    squared_diffs.push(diff * diff);
                }
            }
        }

        Mean::execute(&squared_diffs, shape, Some(axis))
    }
}

/// Standard Deviation Operation
pub struct Std;

impl Std {
    pub fn execute(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>> {
        let variance = Var::execute(data, shape, axis)?;
        Ok(variance.iter().map(|&v| v.sqrt()).collect())
    }
}

/// ReLU Activation

pub struct Cumsum;

impl Cumsum {
    pub fn execute(data: &[f32]) -> Vec<f32> {
        let mut result = Vec::with_capacity(data.len());
        let mut sum = 0.0;
        for &val in data {
            sum += val;
            result.push(sum);
        }
        result
    }
}

/// Prod Reduction Operation
pub struct Prod;

impl Prod {
    pub fn execute(data: &[f32], shape: &[usize], axis: Option<usize>) -> Result<Vec<f32>> {
        if axis.is_none() {
            return Ok(vec![data.iter().product()]);
        }

        let axis = axis.unwrap();
        if axis >= shape.len() {
            return Err(BarracudaError::invalid_params(
                "Prod",
                format!("Axis {axis} out of bounds"),
            ));
        }

        Sum::reduce_along_axis(data, shape, axis, |acc, val| acc * val, 1.0)
    }
}
