//! ESN configuration, validation, and builder

use crate::error::{BarracudaError, Result as BarracudaResult};

/// Configuration for Echo State Network
#[derive(Debug, Clone)]
pub struct ESNConfig {
    /// Number of input features
    pub input_size: usize,

    /// Number of reservoir neurons
    pub reservoir_size: usize,

    /// Number of output features
    pub output_size: usize,

    /// Target spectral radius (typically 0.9-0.99)
    pub spectral_radius: f32,

    /// Fraction of non-zero reservoir weights (0.0-1.0)
    pub connectivity: f32,

    /// Leak rate for temporal integration (0.0-1.0)
    pub leak_rate: f32,

    /// Ridge regression regularization parameter (> 0)
    pub regularization: f32,

    /// Random seed for reproducibility
    pub seed: u64,
}

impl Default for ESNConfig {
    fn default() -> Self {
        Self {
            input_size: 1,
            reservoir_size: 100,
            output_size: 1,
            spectral_radius: 0.9,
            connectivity: 0.1,
            leak_rate: 0.3,
            regularization: 1e-6,
            seed: 42,
        }
    }
}

/// Validate ESN configuration parameters
pub fn validate_config(config: &ESNConfig) -> BarracudaResult<()> {
    let check = |cond: bool, msg: &str| -> BarracudaResult<()> {
        if cond {
            Ok(())
        } else {
            Err(BarracudaError::InvalidInput {
                message: msg.to_string(),
            })
        }
    };
    check(
        config.input_size > 0 && config.reservoir_size > 0 && config.output_size > 0,
        "All sizes must be greater than zero",
    )?;
    check(
        config.spectral_radius > 0.0 && config.spectral_radius <= 2.0,
        "Spectral radius must be in (0, 2]",
    )?;
    check(
        config.connectivity > 0.0 && config.connectivity <= 1.0,
        "Connectivity must be in (0, 1]",
    )?;
    check(
        config.leak_rate > 0.0 && config.leak_rate <= 1.0,
        "Leak rate must be in (0, 1]",
    )?;
    check(
        config.regularization > 0.0,
        "Regularization must be positive",
    )?;
    Ok(())
}

/// Check that a dimension matches expected size
pub fn expect_size(label: &str, expected: usize, actual: usize) -> BarracudaResult<()> {
    if actual == expected {
        return Ok(());
    }
    Err(BarracudaError::InvalidInput {
        message: format!("{label} size mismatch: expected {expected}, got {actual}"),
    })
}
