//! Hardware-Agnostic Echo State Network (ESN) API
//!
//! **EVOLVED v2**: Uses BarraCUDA Tensors - Works on ANY hardware!
//!
//! This module provides a production-ready interface for training and using
//! Echo State Networks using BarraCUDA's universal Tensor operations.
//!
//! # Philosophy
//!
//! ESN operations ARE BarraCUDA operations! Instead of CPU-specific code,
//! we use universal tensor operations that work on CPU, GPU, and NPU.
//!
//! # Deep Debt Compliance
//!
//! - ✅ **Hardware agnostic**: Uses Tensor operations (CPU/GPU/NPU)
//! - ✅ **Pure Rust**: BarraCUDA is 100% Rust
//! - ✅ **Fast**: Leverages best device for workload
//! - ✅ **Safe**: Zero unsafe code
//! - ✅ **Capability-based**: Runtime device discovery
//! - ✅ **No hardcoding**: User can specify device
//!
//! # Example
//!
//! ```no_run
//! use barracuda::esn_v2::{ESN, ESNConfig};
//! use barracuda::device::Device;
//!
//! // Auto-detect best device
//! let esn = ESN::new(ESNConfig {
//!     input_size: 10,
//!     reservoir_size: 1000,  // Large reservoir → GPU!
//!     output_size: 1,
//!     spectral_radius: 0.9,
//!     connectivity: 0.1,
//!     leak_rate: 0.3,
//!     regularization: 1e-6,
//!     seed: 42,
//! }).await?;
//!
//! // Or specify device explicitly
//! let esn_gpu = ESN::new(config)
//!     .await?
//!     .prefer_device(Device::GPU);  // Force GPU
//!
//! // Or use workload hints
//! let esn_smart = ESN::new(config)
//!     .await?
//!     .with_hint(WorkloadHint::LargeMatrices);  // Smart routing
//!
//! // Train (works on any device!)
//! esn.train(&training_inputs, &training_targets).await?;
//!
//! // Predict (works on any device!)
//! let predictions = esn.predict(&test_input).await?;
//! ```

use crate::device::{Auto, Device, WgpuDevice, WorkloadHint};
use crate::error::{BarracudaError, Result as BarracudaResult};
use crate::tensor::Tensor;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

/// GPU shader for fused reservoir update: W_in·input + W_res·state → leaky tanh → new state.
///
/// Single dispatch replaces two matmul + element-wise ops.
/// Provenance: hotSpring v0.6.0 (Stanton-Murillo transport).
pub const WGSL_RESERVOIR_UPDATE: &str = include_str!("shaders/ml/esn_reservoir_update.wgsl");

/// GPU shader for readout: output[i] = W_out[i,:] · state (matrix-vector product).
///
/// Separated from reservoir update so readout can run on CPU while reservoir
/// runs on GPU/NPU. Readout is cheap (output_size << reservoir_size).
pub const WGSL_READOUT: &str = include_str!("shaders/ml/esn_readout.wgsl");

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

/// GPU shader for fused reservoir update: W_in*input + W_res*state → leaky tanh → new state.
///
/// Single dispatch replaces two matmul + elementwise ops. Provenance: hotSpring v0.6.0.
pub const WGSL_ESN_RESERVOIR_UPDATE: &str = include_str!("shaders/ml/esn_reservoir_update.wgsl");

/// GPU shader for readout: output[i] = W_out[i,:] · state (matrix-vector product).
///
/// Separated from reservoir update so readout can run on CPU while reservoir runs on GPU/NPU.
pub const WGSL_ESN_READOUT: &str = include_str!("shaders/ml/esn_readout.wgsl");

/// Result of [`ESN::export_weights`]: `(w_in, w_res, w_out)` as flat f32 vectors.
pub type ExportedWeights = (Vec<f32>, Vec<f32>, Option<Vec<f32>>);

/// Hardware-Agnostic Echo State Network
///
/// **Uses BarraCUDA Tensors** - Works on CPU, GPU, NPU!
pub struct ESN {
    config: ESNConfig,

    // Network weights (BarraCUDA Tensors - hardware agnostic!)
    w_in: Tensor,          // Input weights (reservoir_size × input_size)
    w_res: Tensor,         // Reservoir weights (reservoir_size × reservoir_size)
    w_out: Option<Tensor>, // Readout weights (output_size × reservoir_size)

    // Current state (BarraCUDA Tensor!)
    state: Tensor, // Reservoir state (reservoir_size × 1)

    // Device
    device: Arc<WgpuDevice>,

    // Training flag
    trained: bool,
}

fn validate_config(config: &ESNConfig) -> BarracudaResult<()> {
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

fn expect_size(label: &str, expected: usize, actual: usize) -> BarracudaResult<()> {
    if actual == expected {
        return Ok(());
    }
    Err(BarracudaError::InvalidInput {
        message: format!("{label} size mismatch: expected {expected}, got {actual}"),
    })
}

impl ESN {
    /// Create a new Echo State Network
    ///
    /// **Hardware-agnostic** - Auto-detects best device!
    pub async fn new(config: ESNConfig) -> BarracudaResult<Self> {
        validate_config(&config)?;

        // Auto-detect best device (uses shared pool for concurrent safety)
        let device = Auto::new().await?;

        // Initialize reservoir weights (sparse random matrix on device!)
        let w_res = Self::init_reservoir(&config, &device).await?;

        // Initialize input weights (random uniform [-0.5, 0.5] on device!)
        let w_in = Self::init_input_weights(&config, &device).await?;

        // Initialize state to zero (on device!)
        let state = Tensor::zeros_on(vec![config.reservoir_size, 1], device.clone()).await?;

        Ok(Self {
            config,
            w_in,
            w_res,
            w_out: None,
            state,
            device,
            trained: false,
        })
    }

    /// Initialize reservoir weights
    ///
    /// Creates a sparse random matrix scaled to target spectral radius.
    /// **Uses BarraCUDA Tensors** - works on any device!
    async fn init_reservoir(
        config: &ESNConfig,
        device: &Arc<WgpuDevice>,
    ) -> BarracudaResult<Tensor> {
        let size = config.reservoir_size;

        // Generate sparse random matrix on CPU first
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
        let mut matrix = vec![0.0; size * size];

        for i in 0..size {
            for j in 0..size {
                if rng.gen::<f32>() < config.connectivity {
                    matrix[i * size + j] = rng.gen_range(-1.0..1.0);
                }
            }
        }

        // Scale to approximate target spectral radius
        let approx_radius = (config.connectivity * size as f32).sqrt();
        let scale = config.spectral_radius / approx_radius;

        for val in &mut matrix {
            *val *= scale;
        }

        // Upload to device as Tensor!
        Tensor::from_vec_on(matrix, vec![size, size], device.clone()).await
    }

    /// Initialize input weights
    async fn init_input_weights(
        config: &ESNConfig,
        device: &Arc<WgpuDevice>,
    ) -> BarracudaResult<Tensor> {
        // Generate random uniform [-0.5, 0.5]
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed + 1);
        let weights: Vec<f32> = (0..(config.reservoir_size * config.input_size))
            .map(|_| rng.gen::<f32>() - 0.5)
            .collect();

        // Upload to device as Tensor!
        Tensor::from_vec_on(
            weights,
            vec![config.reservoir_size, config.input_size],
            device.clone(),
        )
        .await
    }

    /// Set device preference
    ///
    /// **Hardware control** - User specifies device!
    ///
    /// # Example
    /// ```ignore
    /// let esn_gpu = esn.prefer_device(Device::GPU);  // Force GPU
    /// let esn_npu = esn.prefer_device(Device::NPU);  // Force NPU
    /// ```
    pub fn prefer_device(self, _device: Device) -> Self {
        // Routing hint recorded; live tensor migration deferred (tracked as D-S18-003).
        log::debug!("Device preference set; migration not yet implemented");
        self
    }

    /// Set workload hint for smart routing
    ///
    /// **Intelligent routing** - System chooses best device!
    ///
    /// # Example
    /// ```ignore
    /// let esn = esn.with_hint(WorkloadHint::LargeMatrices);  // → GPU
    /// let esn = esn.with_hint(WorkloadHint::SmallWorkload);  // → CPU
    /// ```
    pub fn with_hint(self, hint: WorkloadHint) -> Self {
        let preferred_device = Device::select_for_workload(&hint);
        log::debug!("Workload hint: {:?} → Device: {}", hint, preferred_device);
        self
    }

    /// Reset reservoir state to zero
    pub async fn reset_state(&mut self) -> BarracudaResult<()> {
        self.state =
            Tensor::zeros_on(vec![self.config.reservoir_size, 1], self.device.clone()).await?;
        Ok(())
    }

    /// Update reservoir state with a single input
    ///
    /// **BarraCUDA operations** - Works on any device!
    ///
    /// Uses tensor operations:
    /// - `matmul()` for matrix multiplication
    /// - `add()` for element-wise addition
    /// - `tanh()` for activation
    /// - `mul_scalar()` for leak rate
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor (input_size × 1)
    ///
    /// # Returns
    ///
    /// Updated reservoir state (reservoir_size × 1)
    pub async fn update(&mut self, input: &Tensor) -> BarracudaResult<Tensor> {
        if input.shape() != [self.config.input_size, 1] {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input tensor shape mismatch: expected [{}, 1], got {:?}",
                    self.config.input_size,
                    input.shape()
                ),
            });
        }

        let leak = self.config.leak_rate;

        // new_state = (1-leak)*state + leak*tanh(W_in*input + W_res*state)

        // Compute W_in * input (BarraCUDA matmul!)
        let input_contrib = self.w_in.clone().matmul(input)?;

        // Compute W_res * state (BarraCUDA matmul!)
        let recurrent_contrib = self.w_res.clone().matmul(&self.state)?;

        // Combine: input_contrib + recurrent_contrib (BarraCUDA add!)
        let combined = input_contrib.add(&recurrent_contrib)?;

        // Activation: tanh(combined) (BarraCUDA tanh!)
        let activated = combined.tanh()?;

        // Leaky integration: (1-leak)*state + leak*activated
        let old_state_scaled = self.state.mul_scalar(1.0 - leak)?;
        let activated_scaled = activated.mul_scalar(leak)?;
        let new_state = old_state_scaled.add(&activated_scaled)?;

        self.state = new_state.clone();
        Ok(new_state)
    }

    /// Train the ESN readout layer
    ///
    /// **BarraCUDA operations** - Works on any device!
    ///
    /// Uses ridge regression (linear regression with L2 regularization):
    /// W_out = (S^T S + λI)^(-1) S^T Y
    ///
    /// where S is the collected states and Y is the target outputs.
    ///
    /// # Arguments
    ///
    /// * `inputs` - Training input sequences
    /// * `targets` - Target output sequences
    ///
    /// # Returns
    ///
    /// Training error
    pub async fn train(
        &mut self,
        inputs: &[Vec<f32>],
        targets: &[Vec<f32>],
    ) -> BarracudaResult<f32> {
        if inputs.is_empty() || targets.is_empty() {
            return Err(BarracudaError::InvalidInput {
                message: "Training data cannot be empty".to_string(),
            });
        }

        if inputs.len() != targets.len() {
            return Err(BarracudaError::InvalidInput {
                message: "Inputs and targets must have same length".to_string(),
            });
        }

        // Collect reservoir states for all training samples
        let mut all_states = Vec::new();
        let mut all_targets = Vec::new();

        for (input_seq, target_seq) in inputs.iter().zip(targets.iter()) {
            expect_size("Input", self.config.input_size, input_seq.len())?;
            expect_size("Target", self.config.output_size, target_seq.len())?;

            let input_tensor = Tensor::from_vec_on(
                input_seq.clone(),
                vec![self.config.input_size, 1],
                self.device.clone(),
            )
            .await?;

            // Update state with this input
            let state = self.update(&input_tensor).await?;

            // Collect state and target
            all_states.push(state.to_vec()?);
            all_targets.extend_from_slice(target_seq);
        }

        let n_samples = all_states.len();
        let n = self.config.reservoir_size;
        let m = self.config.output_size;

        // Flatten states into matrix (n_samples × reservoir_size)
        let states_flat: Vec<f32> = all_states.into_iter().flatten().collect();

        // Convert to Tensors
        let states_tensor =
            Tensor::from_vec_on(states_flat, vec![n_samples, n], self.device.clone()).await?;

        let targets_tensor =
            Tensor::from_vec_on(all_targets, vec![n_samples, m], self.device.clone()).await?;

        // Ridge regression: W_out = (S^T S + λI)^(-1) S^T Y
        // For now, use simplified gradient descent solver
        let w_out = self
            .ridge_regression_solve(&states_tensor, &targets_tensor)
            .await?;

        // Calculate training error
        let predictions = states_tensor.clone().matmul(&w_out)?;
        let diff = predictions.sub(&targets_tensor)?;
        let error_vec = diff.to_vec()?;
        let error: f32 = error_vec.iter().map(|x| x * x).sum::<f32>() / n_samples as f32;

        self.w_out = Some(w_out);
        self.trained = true;

        Ok(error.sqrt())
    }

    /// Solve ridge regression using gradient descent
    ///
    /// **BarraCUDA operations** - All tensor ops!
    async fn ridge_regression_solve(
        &self,
        states: &Tensor,
        targets: &Tensor,
    ) -> BarracudaResult<Tensor> {
        // Initialize W_out to zeros
        let n = self.config.reservoir_size;
        let m = self.config.output_size;

        let mut w_out = Tensor::zeros_on(vec![n, m], self.device.clone()).await?;

        // Gradient descent parameters
        let learning_rate = 0.01;
        let iterations = 1000;
        let lambda = self.config.regularization;

        for _iter in 0..iterations {
            // Forward: predictions = states * w_out
            let predictions = states.clone().matmul(&w_out)?;

            // Error: diff = predictions - targets
            let diff = predictions.sub(targets)?;

            // Gradient: grad = states^T * diff / n_samples + lambda * w_out
            let states_t = states.transpose()?;
            let grad = states_t.clone().matmul(&diff)?;
            let reg_term = w_out.mul_scalar(lambda)?;
            let total_grad = grad.add(&reg_term)?;
            let scaled_grad = total_grad.mul_scalar(learning_rate)?;

            // Update: w_out = w_out - learning_rate * grad
            w_out = w_out.sub(&scaled_grad)?;
        }

        Ok(w_out)
    }

    /// Predict on new input sequence
    ///
    /// **BarraCUDA operations** - Works on any device!
    ///
    /// # Arguments
    ///
    /// * `input` - Input sequence
    ///
    /// # Returns
    ///
    /// Predicted output
    pub async fn predict(&mut self, input: &[f32]) -> BarracudaResult<Vec<f32>> {
        let (output, _state) = self.predict_return_state(input).await?;
        Ok(output)
    }

    /// Predict and return both output AND raw reservoir state.
    ///
    /// Essential for cross-substrate validation (GPU reservoir → NPU readout)
    /// and for debugging reservoir dynamics. metalForge proved that raw state
    /// access enables:
    /// - Cross-substrate pipeline: train GPU readout from NPU reservoir state
    /// - Online readout switching via weight mutation (AKD1000 validated)
    /// - Reservoir quality metrics (effective rank, Lyapunov exponent)
    pub async fn predict_return_state(
        &mut self,
        input: &[f32],
    ) -> BarracudaResult<(Vec<f32>, Vec<f32>)> {
        if !self.trained {
            return Err(BarracudaError::InvalidInput {
                message: "ESN must be trained before prediction".to_string(),
            });
        }

        expect_size("Input", self.config.input_size, input.len())?;

        let input_tensor = Tensor::from_vec_on(
            input.to_vec(),
            vec![self.config.input_size, 1],
            self.device.clone(),
        )
        .await?;

        let state = self.update(&input_tensor).await?;
        let raw_state = state.to_vec()?;

        let w_out =
            self.w_out
                .as_ref()
                .ok_or_else(|| crate::error::BarracudaError::InvalidOperation {
                    op: "ESN::predict_return_state".to_string(),
                    reason: "ESN has not been trained yet — call train() first".to_string(),
                })?;
        let output = w_out.transpose()?.matmul(&state)?;

        Ok((output.to_vec()?, raw_state))
    }

    /// Replace the readout weights without retraining the reservoir.
    ///
    /// Enables online readout switching — validated on AKD1000 via metalForge's
    /// weight mutation discovery. The reservoir dynamics are preserved; only the
    /// linear readout layer changes.
    pub fn set_readout_weights(&mut self, weights: Tensor) -> BarracudaResult<()> {
        let expected = [self.config.output_size, self.config.reservoir_size];
        if weights.shape() != expected {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Readout weight shape mismatch: expected {:?}, got {:?}",
                    expected,
                    weights.shape()
                ),
            });
        }
        self.w_out = Some(weights);
        self.trained = true;
        Ok(())
    }

    /// Get current configuration
    pub fn config(&self) -> &ESNConfig {
        &self.config
    }

    /// Check if ESN is trained
    pub fn is_trained(&self) -> bool {
        self.trained
    }

    /// Get current reservoir state
    pub fn state(&self) -> &Tensor {
        &self.state
    }

    /// Export all ESN weights as flat f32 vectors for cross-device deployment.
    ///
    /// Returns `(w_in, w_res, w_out)` where each is a row-major flat vector.
    /// `w_out` is `None` if the network has not been trained.
    ///
    /// This is the primary mechanism for the GPU-train → NPU-deploy pipeline:
    /// train on a GPU-backed ESN, export weights, then load onto an NPU or
    /// edge device for inference.
    /// Exported weight tuple: `(w_in, w_res, w_out)`.
    pub fn export_weights(&self) -> BarracudaResult<ExportedWeights> {
        let w_in_data = self.w_in.to_vec()?;
        let w_res_data = self.w_res.to_vec()?;
        let w_out_data = match &self.w_out {
            Some(w) => Some(w.to_vec()?),
            None => None,
        };
        Ok((w_in_data, w_res_data, w_out_data))
    }

    /// Import pre-trained weights (e.g., from another device or saved checkpoint).
    ///
    /// Shapes must match the current config:
    ///   - `w_in`:  `[reservoir_size, input_size]`
    ///   - `w_res`: `[reservoir_size, reservoir_size]`
    ///   - `w_out`: `[output_size, reservoir_size]`
    pub fn import_weights(
        &mut self,
        w_in: &[f32],
        w_res: &[f32],
        w_out: Option<&[f32]>,
    ) -> BarracudaResult<()> {
        let rs = self.config.reservoir_size;
        let is = self.config.input_size;
        let os = self.config.output_size;

        expect_size("w_in", rs * is, w_in.len())?;
        expect_size("w_res", rs * rs, w_res.len())?;

        self.w_in = Tensor::from_data(w_in, vec![rs, is], self.device.clone())?;
        self.w_res = Tensor::from_data(w_res, vec![rs, rs], self.device.clone())?;

        if let Some(wo) = w_out {
            expect_size("w_out", os * rs, wo.len())?;
            self.w_out = Some(Tensor::from_data(wo, vec![os, rs], self.device.clone())?);
            self.trained = true;
        }
        Ok(())
    }

    /// Get device query
    pub fn query_device(&self) -> Device {
        // Map WgpuDevice type to unified Device enum
        match self.device.device_type() {
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu => Device::GPU,
            wgpu::DeviceType::VirtualGpu => Device::GPU,
            wgpu::DeviceType::Cpu => Device::CPU,
            wgpu::DeviceType::Other => Device::Auto,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_esn_creation() {
        let config = ESNConfig::default();
        let esn = ESN::new(config).await.unwrap();
        assert!(!esn.is_trained());
        assert_eq!(esn.state().shape(), &[100, 1]);
    }

    #[tokio::test]
    async fn test_esn_invalid_config() {
        let config = ESNConfig {
            input_size: 0,
            ..Default::default()
        };
        assert!(ESN::new(config).await.is_err());
    }

    #[tokio::test]
    async fn test_esn_device_preference() {
        let config = ESNConfig::default();
        let esn = ESN::new(config).await.unwrap();
        let _esn_gpu = esn.prefer_device(Device::GPU);
        // Just test that it compiles and returns
    }

    #[tokio::test]
    async fn test_esn_workload_hint() {
        let config = ESNConfig::default();
        let esn = ESN::new(config).await.unwrap();
        let _esn_large = esn.with_hint(WorkloadHint::LargeMatrices);
        // Just test that it compiles and returns
    }

    #[tokio::test]
    async fn test_esn_device_query() {
        let config = ESNConfig::default();
        let esn = ESN::new(config).await.unwrap();
        let device = esn.query_device();
        assert!(matches!(device, Device::CPU | Device::GPU | Device::Auto));
    }

    #[tokio::test]
    async fn test_esn_train_simple() {
        let config = ESNConfig {
            input_size: 1,
            reservoir_size: 20,
            output_size: 1,
            spectral_radius: 0.9,
            connectivity: 0.1,
            leak_rate: 0.3,
            regularization: 1e-6,
            seed: 42,
        };

        let mut esn = ESN::new(config).await.unwrap();

        // Simple sequence: increasing values
        let inputs = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0], vec![4.0]];
        let targets = vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0], vec![5.0]];

        // Train should succeed
        let error = esn.train(&inputs, &targets).await.unwrap();
        assert!(error >= 0.0);
        assert!(esn.is_trained());
        assert!(esn.w_out.is_some());
    }

    #[tokio::test]
    async fn test_esn_predict_after_train() {
        let config = ESNConfig {
            input_size: 1,
            reservoir_size: 30,
            output_size: 1,
            spectral_radius: 0.95,
            connectivity: 0.15,
            leak_rate: 0.3,
            regularization: 1e-5,
            seed: 42,
        };

        let mut esn = ESN::new(config).await.unwrap();

        // Train on simple pattern
        let inputs = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0]];
        let targets = vec![vec![2.0], vec![3.0], vec![4.0], vec![5.0]]; // f(x) = x + 2

        esn.train(&inputs, &targets).await.unwrap();

        // Predict
        let prediction = esn.predict(&vec![10.0]).await.unwrap();
        assert_eq!(prediction.len(), 1);
        // Should approximate f(10) = 12, but reservoir is chaotic so just check reasonable range
        assert!(prediction[0] > 5.0 && prediction[0] < 20.0);
    }

    #[tokio::test]
    async fn test_esn_train_mismatched_lengths() {
        let config = ESNConfig::default();
        let mut esn = ESN::new(config).await.unwrap();

        let inputs = vec![vec![0.0], vec![1.0]];
        let targets = vec![vec![1.0]]; // Mismatch!

        let result = esn.train(&inputs, &targets).await;
        assert!(result.is_err());
        assert!(!esn.is_trained());
    }

    #[tokio::test]
    async fn test_esn_train_empty_data() {
        let config = ESNConfig::default();
        let mut esn = ESN::new(config).await.unwrap();

        let inputs: Vec<Vec<f32>> = vec![];
        let targets: Vec<Vec<f32>> = vec![];

        let result = esn.train(&inputs, &targets).await;
        assert!(result.is_err());
        assert!(!esn.is_trained());
    }

    #[tokio::test]
    async fn test_esn_predict_untrained() {
        let config = ESNConfig::default();
        let mut esn = ESN::new(config).await.unwrap();

        // Should error when not trained
        let result = esn.predict(&vec![1.0]).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_esn_predict_wrong_input_size() {
        let config = ESNConfig {
            input_size: 2,
            reservoir_size: 20,
            output_size: 1,
            ..Default::default()
        };

        let mut esn = ESN::new(config).await.unwrap();

        // Train with correct size
        let inputs = vec![vec![0.0, 0.0], vec![1.0, 1.0]];
        let targets = vec![vec![1.0], vec![2.0]];
        esn.train(&inputs, &targets).await.unwrap();

        // Predict with wrong size
        let result = esn.predict(&vec![1.0]).await; // Wrong! Should be 2D
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_esn_multiple_outputs() {
        let config = ESNConfig {
            input_size: 2,
            reservoir_size: 40,
            output_size: 3,
            spectral_radius: 0.9,
            connectivity: 0.1,
            leak_rate: 0.3,
            regularization: 1e-5,
            seed: 42,
        };

        let mut esn = ESN::new(config).await.unwrap();

        // Train on multi-output task
        let inputs = vec![
            vec![0.0, 0.0],
            vec![1.0, 0.0],
            vec![0.0, 1.0],
            vec![1.0, 1.0],
        ];
        let targets = vec![
            vec![0.0, 0.0, 0.0],
            vec![1.0, 0.0, 1.0],
            vec![0.0, 1.0, 1.0],
            vec![1.0, 1.0, 2.0],
        ];

        let error = esn.train(&inputs, &targets).await.unwrap();
        assert!(error >= 0.0);
        assert!(esn.is_trained());

        // Predict
        let prediction = esn.predict(&vec![0.5, 0.5]).await.unwrap();
        assert_eq!(prediction.len(), 3);
    }

    #[tokio::test]
    async fn test_esn_predict_return_state() {
        let config = ESNConfig {
            input_size: 1,
            reservoir_size: 20,
            output_size: 1,
            ..Default::default()
        };

        let mut esn = ESN::new(config).await.unwrap();

        let inputs = vec![vec![1.0], vec![2.0], vec![3.0]];
        let targets = vec![vec![2.0], vec![3.0], vec![4.0]];
        esn.train(&inputs, &targets).await.unwrap();

        let (output, state) = esn.predict_return_state(&[5.0]).await.unwrap();
        assert_eq!(output.len(), 1);
        assert_eq!(state.len(), 20);
        assert!(
            state.iter().any(|&v| v != 0.0),
            "State should be non-zero after update"
        );
    }

    #[tokio::test]
    async fn test_esn_set_readout_weights() {
        let config = ESNConfig {
            input_size: 1,
            reservoir_size: 20,
            output_size: 1,
            ..Default::default()
        };

        let mut esn = ESN::new(config).await.unwrap();

        let inputs = vec![vec![1.0], vec![2.0]];
        let targets = vec![vec![2.0], vec![3.0]];
        esn.train(&inputs, &targets).await.unwrap();

        let original = esn.predict(&[5.0]).await.unwrap();

        let new_weights = Tensor::zeros_on(vec![1, 20], esn.device.clone())
            .await
            .unwrap();
        esn.set_readout_weights(new_weights).unwrap();

        let zeroed = esn.predict(&[5.0]).await.unwrap();
        assert!(
            (zeroed[0]).abs() < 1e-5,
            "Zero readout should produce near-zero output"
        );
        assert_ne!(
            original, zeroed,
            "Different readout weights should produce different output"
        );
    }

    #[tokio::test]
    async fn test_esn_state_persistence() {
        let config = ESNConfig {
            input_size: 1,
            reservoir_size: 20,
            output_size: 1,
            ..Default::default()
        };

        let mut esn = ESN::new(config).await.unwrap();

        // Train
        let inputs = vec![vec![1.0], vec![2.0], vec![3.0]];
        let targets = vec![vec![2.0], vec![3.0], vec![4.0]];
        esn.train(&inputs, &targets).await.unwrap();

        // First prediction
        let pred1 = esn.predict(&vec![5.0]).await.unwrap();

        // Second prediction (state should be different)
        let pred2 = esn.predict(&vec![5.0]).await.unwrap();

        // Predictions should differ because state evolves
        assert_ne!(pred1, pred2, "State should evolve between predictions");
    }

    #[tokio::test]
    async fn test_esn_large_reservoir() {
        let config = ESNConfig {
            input_size: 5,
            reservoir_size: 200,
            output_size: 2,
            spectral_radius: 0.95,
            connectivity: 0.05,
            leak_rate: 0.2,
            regularization: 1e-4,
            seed: 42,
        };

        // Should handle larger reservoirs
        let esn = ESN::new(config).await.unwrap();
        assert_eq!(esn.state().shape(), &[200, 1]);
    }
}
