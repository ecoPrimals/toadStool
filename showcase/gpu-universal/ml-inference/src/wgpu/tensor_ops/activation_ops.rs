//! Activation operations: ReLU, GELU, Sigmoid, Softmax, LogSoftmax

use crate::error::{BarracudaError, Result};

pub struct ReLU;

impl ReLU {
    pub fn execute(data: &[f32]) -> Vec<f32> {
        data.iter().map(|&x| x.max(0.0)).collect()
    }
}

/// GELU Activation (Gaussian Error Linear Unit)
pub struct GELU;

impl GELU {
    pub fn execute(data: &[f32]) -> Vec<f32> {
        data.iter()
            .map(|&x| {
                // Approximation: 0.5 * x * (1 + tanh(sqrt(2/π) * (x + 0.044715 * x^3)))
                let sqrt_2_over_pi = 0.797_884_6;
                let coeff = sqrt_2_over_pi * (x + 0.044715 * x.powi(3));
                0.5 * x * (1.0 + coeff.tanh())
            })
            .collect()
    }
}

/// Sigmoid Activation
pub struct Sigmoid;

impl Sigmoid {
    pub fn execute(data: &[f32]) -> Vec<f32> {
        data.iter().map(|&x| 1.0 / (1.0 + (-x).exp())).collect()
    }
}

/// Softmax Operation
pub struct Softmax;

impl Softmax {
    pub fn execute(data: &[f32], shape: &[usize], axis: usize) -> Result<Vec<f32>> {
        if axis >= shape.len() {
            return Err(BarracudaError::invalid_params(
                "Softmax",
                format!("Axis {} out of bounds", axis),
            ));
        }

        let axis_size = shape[axis];
        let outer_size: usize = shape[..axis].iter().product();
        let inner_size: usize = shape[axis + 1..].iter().product();

        let mut output = data.to_vec();

        for outer in 0..outer_size {
            for inner in 0..inner_size {
                let start = outer * axis_size * inner_size + inner;

                // Find max for numerical stability
                let mut max_val = f32::NEG_INFINITY;
                for ax in 0..axis_size {
                    let idx = start + ax * inner_size;
                    max_val = max_val.max(output[idx]);
                }

                // Compute exp and sum
                let mut sum = 0.0;
                for ax in 0..axis_size {
                    let idx = start + ax * inner_size;
                    output[idx] = (output[idx] - max_val).exp();
                    sum += output[idx];
                }

                // Normalize
                for ax in 0..axis_size {
                    let idx = start + ax * inner_size;
                    output[idx] /= sum;
                }
            }
        }

        Ok(output)
    }
}

/// LogSoftmax Operation
pub struct LogSoftmax;

impl LogSoftmax {
    pub fn execute(data: &[f32], shape: &[usize], axis: usize) -> Result<Vec<f32>> {
        let softmax = Softmax::execute(data, shape, axis)?;
        Ok(softmax.iter().map(|&x| x.ln()).collect())
    }
}
