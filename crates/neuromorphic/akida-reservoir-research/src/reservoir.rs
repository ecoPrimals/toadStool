// SPDX-License-Identifier: AGPL-3.0-only
//! Reservoir generation for echo state networks
//!
//! Creates random, fixed-weight reservoirs with the echo state property.

use crate::error::Result;
use ndarray::{Array1, Array2};
use rand::SeedableRng;
use rand_distr::{Distribution, Normal};
use tracing::{debug, info};

/// Configuration for reservoir generation
#[derive(Debug, Clone)]
pub struct ReservoirConfig {
    /// Input dimension
    pub input_size: usize,

    /// Reservoir size (number of neurons)
    pub reservoir_size: usize,

    /// Output dimension
    pub output_size: usize,

    /// Random seed for reproducibility
    pub seed: u64,

    /// Input scaling factor
    pub input_scaling: f32,

    /// Spectral radius (should be < 1.0 for echo state property)
    pub spectral_radius: f32,

    /// Sparsity (fraction of zero weights in reservoir)
    pub sparsity: f32,
}

impl Default for ReservoirConfig {
    fn default() -> Self {
        Self {
            input_size: 784,      // MNIST default
            reservoir_size: 1000, // 1000 neurons
            output_size: 10,      // 10 classes
            seed: 42,
            input_scaling: 0.1,
            spectral_radius: 0.9, // < 1.0 for echo state
            sparsity: 0.0,        // Fully connected
        }
    }
}

/// Reservoir generator
pub struct ReservoirGenerator {
    config: ReservoirConfig,
}

impl ReservoirGenerator {
    /// Create generator with configuration
    pub fn new(config: ReservoirConfig) -> Self {
        info!("Creating reservoir generator (seed={})", config.seed);
        Self { config }
    }

    /// Generate reservoir weights
    ///
    /// Returns (`W_in`, `W_res`) where:
    /// - `W_in`: Input → Reservoir weights (`reservoir_size` × `input_size`)
    /// - `W_res`: Reservoir → Reservoir weights (`reservoir_size` × `reservoir_size`)
    ///
    /// # Errors
    ///
    /// Returns an error if the normal distribution cannot be created or array creation fails.
    pub fn generate_weights(&self) -> Result<(Array2<f32>, Array2<f32>)> {
        info!("Generating reservoir weights...");

        let mut rng = rand::rngs::StdRng::seed_from_u64(self.config.seed);
        let normal = Normal::new(0.0, 1.0).map_err(|e| {
            crate::error::ReservoirError::Numerical(format!(
                "Failed to create normal distribution: {e}"
            ))
        })?;

        // Generate input weights: W_in (reservoir_size × input_size)
        debug!(
            "Generating input weights ({} × {})",
            self.config.reservoir_size, self.config.input_size
        );

        let w_in_shape = (self.config.reservoir_size, self.config.input_size);
        // Precision is sufficient for neuromorphic computation
        #[allow(clippy::cast_possible_truncation)]
        let w_in_vec: Vec<f32> = (0..w_in_shape.0 * w_in_shape.1)
            .map(|_| normal.sample(&mut rng) as f32 * self.config.input_scaling)
            .collect();

        let w_in = Array2::from_shape_vec(w_in_shape, w_in_vec).map_err(|e| {
            crate::error::ReservoirError::Numerical(format!("Failed to create W_in array: {e}"))
        })?;

        // Generate reservoir weights: W_res (reservoir_size × reservoir_size)
        debug!(
            "Generating reservoir weights ({} × {})",
            self.config.reservoir_size, self.config.reservoir_size
        );

        let w_res_shape = (self.config.reservoir_size, self.config.reservoir_size);
        // Precision is sufficient for neuromorphic computation
        #[allow(clippy::cast_possible_truncation)]
        let w_res_vec: Vec<f32> = (0..w_res_shape.0 * w_res_shape.1)
            .map(|_| {
                // Apply sparsity
                if rand::random::<f32>() < self.config.sparsity {
                    0.0
                } else {
                    normal.sample(&mut rng) as f32
                }
            })
            .collect();

        let mut w_res = Array2::from_shape_vec(w_res_shape, w_res_vec).map_err(|e| {
            crate::error::ReservoirError::Numerical(format!("Failed to create W_res array: {e}"))
        })?;

        // Scale to desired spectral radius (enforce echo state property)
        self.scale_spectral_radius(&mut w_res);

        info!("✅ Reservoir weights generated successfully");
        Ok((w_in, w_res))
    }

    /// Scale reservoir weights to desired spectral radius
    ///
    /// This ensures the echo state property: reservoir dynamics decay over time
    fn scale_spectral_radius(&self, w_res: &mut Array2<f32>) {
        debug!("Scaling to spectral radius {}", self.config.spectral_radius);

        // For simplicity, we use Frobenius norm as a cheap approximation.
        // Pending: exact spectral radius via eigenvalue decomposition; current ndarray-based
        // code uses Frobenius norm as a cheap approximation.
        let frobenius_norm = w_res.iter().map(|&x| x * x).sum::<f32>().sqrt();

        let scaling_factor = self.config.spectral_radius / frobenius_norm;

        // Scale all weights
        w_res.mapv_inplace(|x| x * scaling_factor);

        debug!("Scaled weights by factor {:.4}", scaling_factor);
    }

    /// Generate multiple reservoirs with different seeds
    ///
    /// # Errors
    ///
    /// Returns an error if weight generation fails for any reservoir in the ensemble.
    pub fn generate_ensemble(
        &self,
        num_reservoirs: usize,
    ) -> Result<Vec<(Array2<f32>, Array2<f32>)>> {
        info!("Generating ensemble of {} reservoirs", num_reservoirs);

        let mut reservoirs = Vec::with_capacity(num_reservoirs);

        for i in 0..num_reservoirs {
            let mut config = self.config.clone();
            config.seed = self.config.seed + i as u64;

            let generator = Self::new(config);
            let weights = generator.generate_weights()?;
            reservoirs.push(weights);
        }

        info!("✅ Generated {} reservoirs", num_reservoirs);
        Ok(reservoirs)
    }
}

/// Reservoir state updater (for testing dynamics)
pub struct ReservoirSimulator {
    w_in: Array2<f32>,
    w_res: Array2<f32>,
    state: Array1<f32>,
}

impl ReservoirSimulator {
    /// Create simulator with weights
    pub fn new(w_in: Array2<f32>, w_res: Array2<f32>) -> Self {
        let reservoir_size = w_res.nrows();
        let state = Array1::zeros(reservoir_size);

        Self { w_in, w_res, state }
    }

    /// Update reservoir state with new input
    ///
    /// state(t) = `tanh(W_in` * input(t) + `W_res` * state(t-1))
    ///
    /// # Errors
    ///
    /// Returns an error if matrix dimensions are incompatible.
    pub fn update(&mut self, input: &Array1<f32>) -> Result<Array1<f32>> {
        // Compute W_in * input
        let input_contrib = self.w_in.dot(input);

        // Compute W_res * state
        let recurrent_contrib = self.w_res.dot(&self.state);

        // Combine and apply activation (tanh)
        self.state = (&input_contrib + &recurrent_contrib).mapv(f32::tanh);

        Ok(self.state.clone())
    }

    /// Reset state to zero
    pub fn reset(&mut self) {
        self.state.fill(0.0);
    }

    /// Get current state
    pub const fn state(&self) -> &Array1<f32> {
        &self.state
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reservoir_generation() {
        let config = ReservoirConfig::default();
        let generator = ReservoirGenerator::new(config);

        let result = generator.generate_weights();
        assert!(result.is_ok());

        let (w_in, w_res) = result.unwrap();
        assert_eq!(w_in.shape(), &[1000, 784]);
        assert_eq!(w_res.shape(), &[1000, 1000]);
    }

    #[test]
    fn test_spectral_radius_scaling() {
        let config = ReservoirConfig {
            spectral_radius: 0.5,
            ..Default::default()
        };

        let generator = ReservoirGenerator::new(config);
        let (_w_in, w_res) = generator.generate_weights().unwrap();

        // Check that weights are non-zero and reasonably scaled
        #[allow(clippy::cast_precision_loss)] // weight vector length is small (<1000)
        let mean_abs = w_res.iter().map(|&x| x.abs()).sum::<f32>() / (w_res.len() as f32);
        assert!(mean_abs > 0.0 && mean_abs < 1.0);
    }

    #[test]
    fn test_reservoir_simulator() {
        let config = ReservoirConfig {
            input_size: 10,
            reservoir_size: 100,
            ..Default::default()
        };

        let generator = ReservoirGenerator::new(config);
        let (w_in, w_res) = generator.generate_weights().unwrap();

        let mut simulator = ReservoirSimulator::new(w_in, w_res);

        // Test state update
        let input = Array1::from_vec(vec![0.5; 10]);
        let state = simulator.update(&input).unwrap();

        assert_eq!(state.len(), 100);
        assert!(state.iter().all(|&x| x.abs() <= 1.0)); // tanh bounds
    }
}
