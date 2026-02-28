//! Unary and elementwise operations: Clamp, Abs, Sqrt, Pow, Exp

use crate::error::{BarracudaError, Result};

pub struct Clamp;

impl Clamp {
    /// Clamp values to [min, max]
    pub fn execute(data: &[f32], min: f32, max: f32) -> Vec<f32> {
        data.iter().map(|&x| x.clamp(min, max)).collect()
    }
}

/// Abs Operation (Absolute Value)
///
/// Computes element-wise absolute value.
///
/// ## Use Cases
///
/// - Distance calculations
/// - L1 loss
/// - Feature normalization
/// - Signal processing
pub struct Abs;

impl Abs {
    /// Compute absolute value
    pub fn execute(data: &[f32]) -> Vec<f32> {
        data.iter().map(|&x| x.abs()).collect()
    }
}

/// Sqrt Operation (Square Root)
///
/// Computes element-wise square root.
///
/// ## Use Cases
///
/// - Standard deviation
/// - Euclidean distance
/// - LayerNorm operations
/// - Gradient computations
pub struct Sqrt;

impl Sqrt {
    /// Compute square root
    ///
    /// # Errors
    ///
    /// Returns error if any value is negative
    pub fn execute(data: &[f32]) -> Result<Vec<f32>> {
        // Check for negative values
        if let Some(&neg) = data.iter().find(|&&x| x < 0.0) {
            return Err(BarracudaError::InvalidParameters {
                operation: "Sqrt".to_string(),
                reason: format!("Cannot take sqrt of negative value: {neg}"),
            });
        }

        Ok(data.iter().map(|&x| x.sqrt()).collect())
    }
}

/// Pow Operation (Exponentiation)
///
/// Raises elements to a power.
///
/// ## Use Cases
///
/// - Variance calculation (x²)
/// - Polynomial operations
/// - Custom activations
/// - MSE loss
pub struct Pow;

impl Pow {
    /// Raise to power
    pub fn execute(data: &[f32], exponent: f32) -> Vec<f32> {
        data.iter().map(|&x| x.powf(exponent)).collect()
    }
}

/// Exp Operation (Exponential)
///
/// Computes e^x element-wise.
///
/// ## Use Cases
///
/// - Softmax activation
/// - Gaussian functions
/// - Probability calculations
/// - Neuromorphic activations
pub struct Exp;

impl Exp {
    /// Compute exponential
    pub fn execute(data: &[f32]) -> Vec<f32> {
        data.iter().map(|&x| x.exp()).collect()
    }
}
