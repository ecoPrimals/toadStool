//! Normalization operations: LayerNorm, Norm

use super::reduction_ops::{Mean, Std};
use crate::error::{BarracudaError, Result};

pub struct LayerNorm;

impl LayerNorm {
    pub fn execute(data: &[f32], shape: &[usize], eps: f32) -> Result<Vec<f32>> {
        if shape.is_empty() {
            return Err(BarracudaError::invalid_params(
                "LayerNorm",
                "Shape cannot be empty",
            ));
        }

        let last_axis = shape.len() - 1;
        let mean = Mean::execute(data, shape, Some(last_axis))?;
        let std = Std::execute(data, shape, Some(last_axis))?;

        let feature_size = shape[last_axis];
        let batch_size = data.len() / feature_size;

        let mut output = Vec::with_capacity(data.len());

        for batch in 0..batch_size {
            let m = mean[batch];
            let s = std[batch];
            for feat in 0..feature_size {
                let idx = batch * feature_size + feat;
                output.push((data[idx] - m) / (s + eps));
            }
        }

        Ok(output)
    }
}

/// Norm Operation (L1, L2)
pub struct Norm;

impl Norm {
    pub fn l1(data: &[f32]) -> f32 {
        data.iter().map(|&x| x.abs()).sum()
    }

    pub fn l2(data: &[f32]) -> f32 {
        data.iter().map(|&x| x * x).sum::<f32>().sqrt()
    }

    pub fn execute(data: &[f32], p: f32) -> f32 {
        if (p - 1.0).abs() < f32::EPSILON {
            Self::l1(data)
        } else if (p - 2.0).abs() < f32::EPSILON {
            Self::l2(data)
        } else {
            data.iter()
                .map(|&x| x.abs().powf(p))
                .sum::<f32>()
                .powf(1.0 / p)
        }
    }
}
