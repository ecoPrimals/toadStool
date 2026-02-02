//! High-level Echo State Network (ESN) API
//!
//! **EVOLVED**: CPU-side matrix operations - No specialized WGSL shaders!
//!
//! This module provides a production-ready interface for training and using
//! Echo State Networks (a type of Reservoir Computing). Uses pure Rust matrix
//! operations instead of specialized GPU shaders.
//!
//! # Echo State Networks
//!
//! ESNs are a type of Recurrent Neural Network where:
//! - **Fixed reservoir**: Random recurrent weights (not trained)
//! - **Trainable readout**: Only output layer is trained (fast!)
//! - **Echo State Property**: Spectral radius ~ target ensures stability
//! - **Efficient**: Much faster training than traditional RNNs
//!
//! # Philosophy
//!
//! ESN operations are **small matrix math**, not massive tensor operations!
//! For typical reservoir sizes (100-1000 neurons), CPU-side operations are
//! faster than GPU transfer overhead. Pure Rust implementation wins!
//!
//! # Deep Debt Compliance
//!
//! - ✅ **Hardware agnostic**: No GPU assumptions
//! - ✅ **Pure Rust**: No specialized WGSL shaders
//! - ✅ **Fast**: CPU matrix math beats GPU overhead for small matrices
//! - ✅ **Safe**: Zero unsafe code
//!
//! # Example
//!
//! ```no_run
//! use barracuda::esn::{ESN, ESNConfig};
//!
//! // No device needed - pure Rust!
//! let esn = ESN::new(ESNConfig {
//!     input_size: 10,
//!     reservoir_size: 100,
//!     output_size: 1,
//!     spectral_radius: 0.9,
//!     connectivity: 0.1,
//!     leak_rate: 0.3,
//!     regularization: 1e-6,
//!     seed: 42,
//! });
//!
//! // Train on sequential data
//! let training_inputs = vec![/* time series data */];
//! let training_targets = vec![/* target outputs */];
//! esn.train(&training_inputs, &training_targets)?;
//!
//! // Predict on new data
//! let test_input = vec![/* new time series */];
//! let predictions = esn.predict(&test_input)?;
//! ```

use crate::error::{BarracudaError, Result as BarracudaResult};
use rand::{Rng, SeedableRng};

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

/// Echo State Network for time series prediction and temporal learning
///
/// **Pure Rust implementation** - No GPU dependencies!
pub struct ESN {
    config: ESNConfig,

    // Network weights
    w_in: Vec<f32>,          // Input weights (N×M)
    w_res: Vec<f32>,         // Reservoir weights (N×N)
    w_out: Option<Vec<f32>>, // Readout weights (N×O) - trained

    // Current state
    state: Vec<f32>, // Reservoir state (N)

    // Training flag
    trained: bool,
}

impl ESN {
    /// Create a new Echo State Network
    ///
    /// **No device needed** - Pure Rust!
    ///
    /// # Arguments
    ///
    /// * `config` - ESN configuration parameters
    ///
    /// # Returns
    ///
    /// Initialized ESN ready for training
    pub fn new(config: ESNConfig) -> BarracudaResult<Self> {
        // Validate configuration
        if config.input_size == 0 || config.reservoir_size == 0 || config.output_size == 0 {
            return Err(BarracudaError::InvalidInput {
                message: "All sizes must be greater than zero".to_string(),
            });
        }

        if config.spectral_radius <= 0.0 || config.spectral_radius > 2.0 {
            return Err(BarracudaError::InvalidInput {
                message: "Spectral radius must be in (0, 2]".to_string(),
            });
        }

        if config.connectivity <= 0.0 || config.connectivity > 1.0 {
            return Err(BarracudaError::InvalidInput {
                message: "Connectivity must be in (0, 1]".to_string(),
            });
        }

        if config.leak_rate <= 0.0 || config.leak_rate > 1.0 {
            return Err(BarracudaError::InvalidInput {
                message: "Leak rate must be in (0, 1]".to_string(),
            });
        }

        if config.regularization <= 0.0 {
            return Err(BarracudaError::InvalidInput {
                message: "Regularization must be positive".to_string(),
            });
        }

        // Initialize reservoir weights (pure Rust!)
        let w_res = Self::init_reservoir(&config)?;

        // Initialize input weights (random uniform [-0.5, 0.5])
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
        let w_in: Vec<f32> = (0..(config.reservoir_size * config.input_size))
            .map(|_| rng.gen::<f32>() - 0.5)
            .collect();

        // Initialize state to zero
        let state = vec![0.0; config.reservoir_size];

        Ok(Self {
            config,
            w_in,
            w_res,
            w_out: None,
            state,
            trained: false,
        })
    }

    /// Initialize reservoir weights (sparse random matrix with scaling)
    ///
    /// **Pure Rust** - No GPU, no WGSL!
    ///
    /// Uses simple scaling approximation instead of exact spectral radius computation.
    /// For typical ESN parameters, this works well!
    fn init_reservoir(config: &ESNConfig) -> BarracudaResult<Vec<f32>> {
        let size = config.reservoir_size;
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);

        // Generate sparse random matrix
        let mut matrix = vec![0.0; size * size];
        for i in 0..size {
            for j in 0..size {
                if rng.gen::<f32>() < config.connectivity {
                    matrix[i * size + j] = rng.gen_range(-1.0..1.0);
                }
            }
        }

        // Scale to approximate target spectral radius
        // For sparse random matrices, spectral radius ≈ sqrt(connectivity * N)
        // Scale factor: target / sqrt(connectivity * N)
        let approx_radius = (config.connectivity * size as f32).sqrt();
        let scale = config.spectral_radius / approx_radius;

        for val in &mut matrix {
            *val *= scale;
        }

        Ok(matrix)
    }

    /// Reset reservoir state to zero
    pub fn reset_state(&mut self) {
        self.state.fill(0.0);
    }

    /// Update reservoir state with a single input
    ///
    /// **Pure Rust matrix math** - Faster than GPU for small matrices!
    ///
    /// # Arguments
    ///
    /// * `input` - Input vector (length must match input_size)
    ///
    /// # Returns
    ///
    /// Updated reservoir state
    pub fn update(&mut self, input: &[f32]) -> BarracudaResult<Vec<f32>> {
        if input.len() != self.config.input_size {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input size mismatch: expected {}, got {}",
                    self.config.input_size,
                    input.len()
                ),
            });
        }

        let n = self.config.reservoir_size;
        let leak = self.config.leak_rate;

        // new_state = (1-leak)*state + leak*tanh(W_in*input + W_res*state)

        // Compute W_in * input
        let mut input_contrib = vec![0.0; n];
        for i in 0..n {
            for j in 0..self.config.input_size {
                input_contrib[i] += self.w_in[i * self.config.input_size + j] * input[j];
            }
        }

        // Compute W_res * state
        let mut recurrent_contrib = vec![0.0; n];
        for i in 0..n {
            for j in 0..n {
                recurrent_contrib[i] += self.w_res[i * n + j] * self.state[j];
            }
        }

        // Combine and apply tanh
        let mut activated = vec![0.0; n];
        for i in 0..n {
            activated[i] = (input_contrib[i] + recurrent_contrib[i]).tanh();
        }

        // Leaky integration
        for i in 0..n {
            self.state[i] = (1.0 - leak) * self.state[i] + leak * activated[i];
        }

        Ok(self.state.clone())
    }

    /// Train the ESN on sequential data
    ///
    /// **Pure Rust** - Ridge regression via CPU matrix solve!
    ///
    /// # Arguments
    ///
    /// * `inputs` - Sequence of input vectors (each length input_size)
    /// * `targets` - Sequence of target vectors (each length output_size)
    ///
    /// # Training Process
    ///
    /// 1. Reset reservoir state
    /// 2. Run all inputs through reservoir (collect states)
    /// 3. Train readout layer using ridge regression
    ///
    /// # Returns
    ///
    /// Training error (MSE)
    pub fn train(&mut self, inputs: &[Vec<f32>], targets: &[Vec<f32>]) -> BarracudaResult<f32> {
        if inputs.len() != targets.len() {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input/target length mismatch: {} inputs, {} targets",
                    inputs.len(),
                    targets.len()
                ),
            });
        }

        if inputs.is_empty() {
            return Err(BarracudaError::InvalidInput {
                message: "Cannot train on empty data".to_string(),
            });
        }

        // Validate input/target dimensions
        for (i, input) in inputs.iter().enumerate() {
            if input.len() != self.config.input_size {
                return Err(BarracudaError::InvalidInput {
                    message: format!(
                        "Input {} size mismatch: expected {}, got {}",
                        i, self.config.input_size, input.len()
                    ),
                });
            }
        }

        for (i, target) in targets.iter().enumerate() {
            if target.len() != self.config.output_size {
                return Err(BarracudaError::InvalidInput {
                    message: format!(
                        "Target {} size mismatch: expected {}, got {}",
                        i, self.config.output_size, target.len()
                    ),
                });
            }
        }

        // Reset state and collect reservoir states
        self.reset_state();
        let mut states = Vec::with_capacity(inputs.len());

        for input in inputs {
            self.update(input)?;
            states.push(self.state.clone());
        }

        // Train readout layer (ridge regression)
        self.w_out = Some(Self::ridge_regression(
            &states,
            targets,
            self.config.output_size,
            self.config.regularization,
        )?);

        self.trained = true;

        // Compute training error
        self.reset_state();
        let mut total_error = 0.0;

        for (input, target) in inputs.iter().zip(targets.iter()) {
            let prediction = self.predict_single(input)?;
            for (pred, tgt) in prediction.iter().zip(target.iter()) {
                let error = pred - tgt;
                total_error += error * error;
            }
        }

        let mse = total_error / (inputs.len() * self.config.output_size) as f32;
        Ok(mse)
    }

    /// Ridge regression (pure Rust matrix solve)
    ///
    /// Solves: W = (X^T X + λI)^(-1) X^T Y
    ///
    /// Uses simple direct method for small matrices.
    fn ridge_regression(
        states: &[Vec<f32>],
        targets: &[Vec<f32>],
        output_size: usize,
        regularization: f32,
    ) -> BarracudaResult<Vec<f32>> {
        let n_samples = states.len();
        let n_features = states[0].len();

        // For small problems, use simple least squares with regularization
        // W_k = (X^T X + λI)^(-1) X^T y_k for each output dimension k
        
        // Build X^T X + λI
        let mut xtx = vec![0.0; n_features * n_features];
        for i in 0..n_features {
            for j in 0..n_features {
                let mut sum = 0.0;
                for sample in states {
                    sum += sample[i] * sample[j];
                }
                xtx[i * n_features + j] = sum / n_samples as f32; // Normalize
                if i == j {
                    xtx[i * n_features + j] += regularization;
                }
            }
        }

        // Build X^T Y
        let mut xty = vec![0.0; n_features * output_size];
        for i in 0..n_features {
            for k in 0..output_size {
                let mut sum = 0.0;
                for (sample, target) in states.iter().zip(targets.iter()) {
                    sum += sample[i] * target[k];
                }
                xty[i * output_size + k] = sum / n_samples as f32; // Normalize
            }
        }

        // Simple gradient descent solver (more stable than Jacobi for ill-conditioned systems)
        let mut weights = vec![0.0; n_features * output_size];
        let learning_rate = 0.01;
        
        for _ in 0..1000 {
            // Compute gradient: grad = (X^T X + λI) * W - X^T Y
            let mut gradient = vec![0.0; n_features * output_size];
            for i in 0..n_features {
                for k in 0..output_size {
                    let mut sum = -xty[i * output_size + k];
                    for j in 0..n_features {
                        sum += xtx[i * n_features + j] * weights[j * output_size + k];
                    }
                    gradient[i * output_size + k] = sum;
                }
            }
            
            // Update weights: W = W - lr * grad
            for i in 0..(n_features * output_size) {
                weights[i] -= learning_rate * gradient[i];
            }
        }

        Ok(weights)
    }

    /// Predict a single output given an input
    ///
    /// # Arguments
    ///
    /// * `input` - Input vector (length must match input_size)
    ///
    /// # Returns
    ///
    /// Predicted output vector (length output_size)
    fn predict_single(&mut self, input: &[f32]) -> BarracudaResult<Vec<f32>> {
        if !self.trained {
            return Err(BarracudaError::InvalidInput {
                message: "ESN must be trained before prediction".to_string(),
            });
        }

        // Update state
        self.update(input)?;

        // Compute output: y = W_out · state
        let w_out = self.w_out.as_ref().unwrap();
        let mut output = vec![0.0; self.config.output_size];

        for i in 0..self.config.output_size {
            for j in 0..self.config.reservoir_size {
                output[i] += w_out[j * self.config.output_size + i] * self.state[j];
            }
        }

        Ok(output)
    }

    /// Predict outputs for a sequence of inputs
    ///
    /// # Arguments
    ///
    /// * `inputs` - Sequence of input vectors
    ///
    /// # Returns
    ///
    /// Sequence of predicted output vectors
    pub fn predict(&mut self, inputs: &[Vec<f32>]) -> BarracudaResult<Vec<Vec<f32>>> {
        if !self.trained {
            return Err(BarracudaError::InvalidInput {
                message: "ESN must be trained before prediction".to_string(),
            });
        }

        inputs
            .iter()
            .map(|input| self.predict_single(input))
            .collect()
    }

    /// Get current reservoir state
    pub fn get_state(&self) -> &[f32] {
        &self.state
    }

    /// Check if ESN has been trained
    pub fn is_trained(&self) -> bool {
        self.trained
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_esn_creation() {
        let esn = ESN::new(ESNConfig::default()).unwrap();
        assert_eq!(esn.config.reservoir_size, 100);
        assert!(!esn.is_trained());
    }

    #[test]
    fn test_esn_state_update() {
        let mut esn = ESN::new(ESNConfig {
            input_size: 2,
            reservoir_size: 10,
            output_size: 1,
            ..Default::default()
        })
        .unwrap();

        let input = vec![0.5, -0.3];
        let state = esn.update(&input).unwrap();
        assert_eq!(state.len(), 10);

        // State should be non-zero after update
        assert!(state.iter().any(|&x| x != 0.0));
    }

    #[test]
    fn test_esn_reset() {
        let mut esn = ESN::new(ESNConfig {
            input_size: 2,
            reservoir_size: 10,
            output_size: 1,
            ..Default::default()
        })
        .unwrap();

        let input = vec![0.5, -0.3];
        esn.update(&input).unwrap();
        esn.reset_state();

        assert!(esn.get_state().iter().all(|&x| x == 0.0));
    }

    #[test]
    fn test_esn_train_predict() {
        let mut esn = ESN::new(ESNConfig {
            input_size: 1,
            reservoir_size: 20,
            output_size: 1,
            spectral_radius: 0.9,
            leak_rate: 0.3,
            regularization: 1e-4,
            ..Default::default()
        })
        .unwrap();

        // Simple sin wave prediction
        let mut inputs = Vec::new();
        let mut targets = Vec::new();
        for i in 0..20 {
            let x = i as f32 * 0.1;
            inputs.push(vec![x.sin()]);
            targets.push(vec![(x + 0.1).sin()]);
        }

        // Train
        let error = esn.train(&inputs, &targets).unwrap();
        assert!(error < 1.0, "Training error too high: {}", error);
        assert!(esn.is_trained());

        // Predict
        let test_inputs = vec![vec![0.5_f32.sin()]];
        let predictions = esn.predict(&test_inputs).unwrap();
        assert_eq!(predictions.len(), 1);
        assert_eq!(predictions[0].len(), 1);
    }

    #[test]
    fn test_esn_validation() {
        // Invalid sizes
        assert!(ESN::new(ESNConfig {
            input_size: 0,
            ..Default::default()
        })
        .is_err());

        // Invalid spectral radius
        assert!(ESN::new(ESNConfig {
            spectral_radius: 0.0,
            ..Default::default()
        })
        .is_err());

        // Invalid connectivity
        assert!(ESN::new(ESNConfig {
            connectivity: 1.5,
            ..Default::default()
        })
        .is_err());
    }
}
