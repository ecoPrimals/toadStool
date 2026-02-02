//! High-level Echo State Network (ESN) API
//!
//! This module provides a production-ready interface for training and using
//! Echo State Networks (a type of Reservoir Computing). It wraps the low-level
//! reservoir operations (`reservoir_init`, `reservoir_update`, `spectral_radius`,
//! `ridge_regression`) into an ergonomic, easy-to-use API.
//!
//! # Echo State Networks
//!
//! ESNs are a type of Recurrent Neural Network where:
//! - **Fixed reservoir**: Random recurrent weights (not trained)
//! - **Trainable readout**: Only output layer is trained (fast!)
//! - **Echo State Property**: Spectral radius < 1.0 ensures stability
//! - **Efficient**: Much faster training than traditional RNNs
//!
//! # Example
//!
//! ```no_run
//! use barracuda::esn::{ESN, ESNConfig};
//! use barracuda::WgpuDevice;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let device = WgpuDevice::new().await?;
//!
//! // Create and configure ESN
//! let esn = ESN::new(
//!     &device,
//!     ESNConfig {
//!         input_size: 10,
//!         reservoir_size: 100,
//!         output_size: 1,
//!         spectral_radius: 0.9,
//!         connectivity: 0.1,
//!         leak_rate: 0.3,
//!         regularization: 1e-6,
//!         seed: 42,
//!     }
//! ).await?;
//!
//! // Train on sequential data
//! let training_inputs = vec![/* time series data */];
//! let training_targets = vec![/* target outputs */];
//! esn.train(&training_inputs, &training_targets).await?;
//!
//! // Predict on new data
//! let test_input = vec![/* new time series */];
//! let predictions = esn.predict(&test_input).await?;
//! # Ok(())
//! # }
//! ```

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result as BarracudaResult};
use crate::ops::reservoir_init::reservoir_init;
use crate::ops::reservoir_update::reservoir_update;
use crate::ops::ridge_regression::ridge_regression;
use crate::ops::spectral_radius::spectral_radius;

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
    pub seed: u32,
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
pub struct ESN {
    device: WgpuDevice,
    config: ESNConfig,

    // Network weights
    w_in: Vec<f32>,          // Input weights (N×M)
    w_res: Vec<f32>,         // Reservoir weights (N×N)
    w_out: Option<Vec<f32>>, // Readout weights (N×M) - trained

    // Current state
    state: Vec<f32>, // Reservoir state (N)

    // Training flag
    trained: bool,
}

impl ESN {
    /// Create a new Echo State Network
    ///
    /// # Arguments
    ///
    /// * `device` - WGPU device for GPU computation
    /// * `config` - ESN configuration parameters
    ///
    /// # Returns
    ///
    /// Initialized ESN ready for training
    pub async fn new(device: &WgpuDevice, config: ESNConfig) -> BarracudaResult<Self> {
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

        // Initialize reservoir weights
        let w_res = reservoir_init(
            &device.device,
            &device.queue,
            config.reservoir_size as u32,
            config.spectral_radius,
            config.connectivity,
            config.seed,
        )
        .await?;

        // Verify spectral radius
        let actual_spectral_radius: f32 = spectral_radius(
            &device.device,
            &device.queue,
            &w_res,
            config.reservoir_size as u32,
            50, // iterations
        )
        .await?;

        // Warn if spectral radius is significantly different
        if (actual_spectral_radius - config.spectral_radius).abs() > 0.2_f32 {
            eprintln!(
                "Warning: Actual spectral radius ({:.3}) differs from target ({:.3})",
                actual_spectral_radius, config.spectral_radius
            );
        }

        // Initialize input weights (random uniform [-0.5, 0.5])
        let w_in = (0..(config.reservoir_size * config.input_size))
            .map(|i| {
                let seed = config
                    .seed
                    .wrapping_add(i as u32)
                    .wrapping_mul(1664525)
                    .wrapping_add(1013904223);
                (seed as f32 / u32::MAX as f32) - 0.5
            })
            .collect();

        // Initialize state to zero
        let state = vec![0.0; config.reservoir_size];

        Ok(Self {
            device: device.clone(),
            config,
            w_in,
            w_res,
            w_out: None,
            state,
            trained: false,
        })
    }

    /// Reset reservoir state to zero
    pub fn reset_state(&mut self) {
        self.state.fill(0.0);
    }

    /// Update reservoir state with a single input
    ///
    /// # Arguments
    ///
    /// * `input` - Input vector (length must match input_size)
    ///
    /// # Returns
    ///
    /// Updated reservoir state
    pub async fn update(&mut self, input: &[f32]) -> BarracudaResult<Vec<f32>> {
        if input.len() != self.config.input_size {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input size mismatch: expected {}, got {}",
                    self.config.input_size,
                    input.len()
                ),
            });
        }

        self.state = reservoir_update(
            &self.device.device,
            &self.device.queue,
            &self.state,
            input,
            &self.w_in,
            &self.w_res,
            self.config.leak_rate,
        )
        .await?;

        Ok(self.state.clone())
    }

    /// Train the ESN on sequential data
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
    pub async fn train(
        &mut self,
        inputs: &[Vec<f32>],
        targets: &[Vec<f32>],
    ) -> BarracudaResult<f32> {
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
                        i,
                        self.config.input_size,
                        input.len()
                    ),
                });
            }
        }

        for (i, target) in targets.iter().enumerate() {
            if target.len() != self.config.output_size {
                return Err(BarracudaError::InvalidInput {
                    message: format!(
                        "Target {} size mismatch: expected {}, got {}",
                        i,
                        self.config.output_size,
                        target.len()
                    ),
                });
            }
        }

        // Reset state and collect reservoir states
        self.reset_state();
        let mut states = Vec::with_capacity(inputs.len() * self.config.reservoir_size);

        for input in inputs {
            self.update(input).await?;
            states.extend_from_slice(&self.state);
        }

        // Flatten targets
        let flat_targets: Vec<f32> = targets.iter().flatten().copied().collect();

        // Train readout layer
        self.w_out = Some(
            ridge_regression(
                &self.device.device,
                &self.device.queue,
                &states,
                &flat_targets,
                self.config.reservoir_size as u32,
                inputs.len() as u32,
                self.config.output_size as u32,
                self.config.regularization,
            )
            .await?,
        );

        self.trained = true;

        // Compute training error
        self.reset_state();
        let mut total_error = 0.0;

        for (input, target) in inputs.iter().zip(targets.iter()) {
            let prediction = self.predict_single(input).await?;
            for (pred, tgt) in prediction.iter().zip(target.iter()) {
                let error = pred - tgt;
                total_error += error * error;
            }
        }

        let mse = total_error / (inputs.len() * self.config.output_size) as f32;
        Ok(mse)
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
    async fn predict_single(&mut self, input: &[f32]) -> BarracudaResult<Vec<f32>> {
        if !self.trained {
            return Err(BarracudaError::InvalidInput {
                message: "ESN must be trained before prediction".to_string(),
            });
        }

        // Update state
        self.update(input).await?;

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
    pub async fn predict(&mut self, inputs: &[Vec<f32>]) -> BarracudaResult<Vec<Vec<f32>>> {
        if !self.trained {
            return Err(BarracudaError::InvalidInput {
                message: "ESN must be trained before prediction".to_string(),
            });
        }

        let mut predictions = Vec::with_capacity(inputs.len());

        for input in inputs {
            predictions.push(self.predict_single(input).await?);
        }

        Ok(predictions)
    }

    /// Check if ESN has been trained
    pub fn is_trained(&self) -> bool {
        self.trained
    }

    /// Get current reservoir state
    pub fn state(&self) -> &[f32] {
        &self.state
    }

    /// Get ESN configuration
    pub fn config(&self) -> &ESNConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_esn_creation() {
        let device = WgpuDevice::new().await.unwrap();
        let esn = ESN::new(&device, ESNConfig::default()).await.unwrap();
        assert!(!esn.is_trained());
        assert_eq!(esn.state().len(), 100);
    }

    #[tokio::test]
    async fn test_esn_update() {
        let device = WgpuDevice::new().await.unwrap();
        let mut esn = ESN::new(
            &device,
            ESNConfig {
                input_size: 2,
                reservoir_size: 10,
                ..Default::default()
            },
        )
        .await
        .unwrap();

        let input = vec![1.0, 0.5];
        let state = esn.update(&input).await.unwrap();
        assert_eq!(state.len(), 10);
        assert!(state.iter().any(|&x| x != 0.0));
    }

    #[tokio::test]
    async fn test_esn_train_predict() {
        let device = WgpuDevice::new().await.unwrap();
        let mut esn = ESN::new(
            &device,
            ESNConfig {
                input_size: 1,
                reservoir_size: 50,
                output_size: 1,
                spectral_radius: 0.5,
                connectivity: 0.2,
                leak_rate: 0.5,
                regularization: 1e-6,
                seed: 42,
            },
        )
        .await
        .unwrap();

        // Simple pattern: output = 2 * input
        let inputs: Vec<Vec<f32>> = (0..10).map(|i| vec![i as f32 / 10.0]).collect();
        let targets: Vec<Vec<f32>> = (0..10).map(|i| vec![2.0 * i as f32 / 10.0]).collect();

        // Train
        let mse = esn.train(&inputs, &targets).await.unwrap();
        assert!(esn.is_trained());
        assert!(mse.is_finite());

        // Predict
        esn.reset_state();
        let predictions = esn.predict(&inputs).await.unwrap();
        assert_eq!(predictions.len(), inputs.len());
        assert_eq!(predictions[0].len(), 1);
    }

    #[tokio::test]
    async fn test_esn_reset_state() {
        let device = WgpuDevice::new().await.unwrap();
        let mut esn = ESN::new(&device, ESNConfig::default()).await.unwrap();

        esn.update(&vec![1.0]).await.unwrap();
        assert!(esn.state().iter().any(|&x| x != 0.0));

        esn.reset_state();
        assert!(esn.state().iter().all(|&x| x == 0.0));
    }

    #[tokio::test]
    async fn test_esn_validation() {
        let device = WgpuDevice::new().await.unwrap();

        // Invalid spectral radius
        assert!(ESN::new(
            &device,
            ESNConfig {
                spectral_radius: 0.0,
                ..Default::default()
            }
        )
        .await
        .is_err());

        // Invalid connectivity
        assert!(ESN::new(
            &device,
            ESNConfig {
                connectivity: 1.5,
                ..Default::default()
            }
        )
        .await
        .is_err());

        // Invalid leak rate
        assert!(ESN::new(
            &device,
            ESNConfig {
                leak_rate: 0.0,
                ..Default::default()
            }
        )
        .await
        .is_err());
    }
}
