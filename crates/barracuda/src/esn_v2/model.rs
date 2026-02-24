//! ESN model - training, prediction, and weight management

use crate::device::{Auto, Device, WgpuDevice, WorkloadHint};
use crate::error::{BarracudaError, Result as BarracudaResult};
use crate::linalg::solve_f64_cpu;
use crate::tensor::Tensor;
use rand::{Rng, SeedableRng};
use std::sync::Arc;

use super::config::{expect_size, validate_config, ESNConfig};
use super::npu::{quantize_affine_i8_f64, NpuReadoutWeights};

/// Result of [`ESN::export_weights`]: `(w_in, w_res, w_out)` as flat f32 vectors.
pub type ExportedWeights = (Vec<f32>, Vec<f32>, Option<Vec<f32>>);

/// Hardware-Agnostic Echo State Network
///
/// **Uses BarraCuda Tensors** - Works on CPU, GPU, NPU!
pub struct ESN {
    pub(super) config: ESNConfig,

    pub(super) w_in: Tensor,
    pub(super) w_res: Tensor,
    pub(super) w_out: Option<Tensor>,
    pub(super) state: Tensor,
    pub(super) device: Arc<WgpuDevice>,
    pub(super) trained: bool,
}

impl ESN {
    /// Create a new Echo State Network
    ///
    /// **Hardware-agnostic** - Auto-detects best device!
    pub async fn new(config: ESNConfig) -> BarracudaResult<Self> {
        validate_config(&config)?;

        let device = Auto::new().await?;

        let w_res = Self::init_reservoir(&config, &device).await?;
        let w_in = Self::init_input_weights(&config, &device).await?;
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
    async fn init_reservoir(
        config: &ESNConfig,
        device: &Arc<WgpuDevice>,
    ) -> BarracudaResult<Tensor> {
        let size = config.reservoir_size;

        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);
        let mut matrix = vec![0.0; size * size];

        for i in 0..size {
            for j in 0..size {
                if rng.gen::<f32>() < config.connectivity {
                    matrix[i * size + j] = rng.gen_range(-1.0..1.0);
                }
            }
        }

        let approx_radius = (config.connectivity * size as f32).sqrt();
        let scale = config.spectral_radius / approx_radius;

        for val in &mut matrix {
            *val *= scale;
        }

        Tensor::from_vec_on(matrix, vec![size, size], device.clone()).await
    }

    /// Initialize input weights
    async fn init_input_weights(
        config: &ESNConfig,
        device: &Arc<WgpuDevice>,
    ) -> BarracudaResult<Tensor> {
        let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed + 1);
        let weights: Vec<f32> = (0..(config.reservoir_size * config.input_size))
            .map(|_| rng.gen::<f32>() - 0.5)
            .collect();

        Tensor::from_vec_on(
            weights,
            vec![config.reservoir_size, config.input_size],
            device.clone(),
        )
        .await
    }

    /// Set device preference
    pub fn prefer_device(self, _device: Device) -> Self {
        log::debug!("Device preference set; migration not yet implemented");
        self
    }

    /// Set workload hint for smart routing
    #[must_use]
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

        let input_contrib = self.w_in.clone().matmul(input)?;
        let recurrent_contrib = self.w_res.clone().matmul(&self.state)?;
        let combined = input_contrib.add(&recurrent_contrib)?;
        let activated = combined.tanh()?;

        let old_state_scaled = self.state.mul_scalar(1.0 - leak)?;
        let activated_scaled = activated.mul_scalar(leak)?;
        let new_state = old_state_scaled.add(&activated_scaled)?;

        self.state = new_state.clone();
        Ok(new_state)
    }

    /// Train the ESN readout layer
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

            let state = self.update(&input_tensor).await?;

            all_states.push(state.to_vec()?);
            all_targets.extend_from_slice(target_seq);
        }

        let n_samples = all_states.len();
        let n = self.config.reservoir_size;
        let m = self.config.output_size;

        let states_flat: Vec<f32> = all_states.into_iter().flatten().collect();

        let states_tensor =
            Tensor::from_vec_on(states_flat, vec![n_samples, n], self.device.clone()).await?;

        let targets_tensor =
            Tensor::from_vec_on(all_targets, vec![n_samples, m], self.device.clone()).await?;

        let w_out = self
            .ridge_regression_solve(&states_tensor, &targets_tensor)
            .await?;

        let predictions = states_tensor.clone().matmul(&w_out)?;
        let diff = predictions.sub(&targets_tensor)?;
        let error_vec = diff.to_vec()?;
        let error: f32 = error_vec.iter().map(|x| x * x).sum::<f32>() / n_samples as f32;

        self.w_out = Some(w_out);
        self.trained = true;

        Ok(error.sqrt())
    }

    /// Train the readout layer using matrix ridge regression (closed-form).
    ///
    /// Implements W_out = Y * X^T * (X * X^T + lambda * I)^{-1} using CPU solve.
    /// Uses `solve_f64_cpu` from linalg for the matrix solve (ESN matrices are small).
    ///
    /// # Arguments
    ///
    /// * `states` - State matrix (reservoir_size × n_samples), row-major
    /// * `targets` - Target matrix (output_size × n_samples), row-major
    /// * `lambda` - Ridge regularization parameter (> 0)
    pub fn train_ridge_regression(
        &mut self,
        states: &[f64],
        targets: &[f64],
        lambda: f64,
    ) -> BarracudaResult<()> {
        let n = self.config.reservoir_size;
        let m = self.config.output_size;

        if states.is_empty() {
            return Err(BarracudaError::InvalidInput {
                message: "States cannot be empty".to_string(),
            });
        }
        if !states.len().is_multiple_of(n) {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "States length {} must be divisible by reservoir_size {}",
                    states.len(),
                    n
                ),
            });
        }
        let n_samples = states.len() / n;
        expect_size("Targets", m * n_samples, targets.len())?;

        if lambda <= 0.0 {
            return Err(BarracudaError::InvalidInput {
                message: "Lambda must be positive".to_string(),
            });
        }

        let x = states;
        let y = targets;

        let mut m_mat = vec![0.0; n * n];
        for i in 0..n {
            for j in 0..n {
                let mut sum = 0.0;
                for k in 0..n_samples {
                    sum += x[i * n_samples + k] * x[j * n_samples + k];
                }
                m_mat[i * n + j] = sum;
            }
            m_mat[i * n + i] += lambda;
        }

        let mut b_mat = vec![0.0; n * m];
        for i in 0..n {
            for j in 0..m {
                let mut sum = 0.0;
                for k in 0..n_samples {
                    sum += x[i * n_samples + k] * y[j * n_samples + k];
                }
                b_mat[i * m + j] = sum;
            }
        }

        let mut w_out_t = vec![0.0; n * m];
        for j in 0..m {
            let b_col: Vec<f64> = (0..n).map(|i| b_mat[i * m + j]).collect();
            let w_col = solve_f64_cpu(&m_mat, &b_col, n)?;
            for (i, &w) in w_col.iter().enumerate() {
                w_out_t[i * m + j] = w;
            }
        }

        let w_out_f32: Vec<f32> = w_out_t.iter().map(|&x| x as f32).collect();
        self.w_out = Some(Tensor::from_data(
            &w_out_f32,
            vec![n, m],
            self.device.clone(),
        )?);
        self.trained = true;

        Ok(())
    }

    /// Solve ridge regression using gradient descent
    async fn ridge_regression_solve(
        &self,
        states: &Tensor,
        targets: &Tensor,
    ) -> BarracudaResult<Tensor> {
        let n = self.config.reservoir_size;
        let m = self.config.output_size;

        let mut w_out = Tensor::zeros_on(vec![n, m], self.device.clone()).await?;

        let learning_rate = 0.01;
        let iterations = 1000;
        let lambda = self.config.regularization;

        for _iter in 0..iterations {
            let predictions = states.clone().matmul(&w_out)?;
            let diff = predictions.sub(targets)?;

            let states_t = states.transpose()?;
            let grad = states_t.clone().matmul(&diff)?;
            let reg_term = w_out.mul_scalar(lambda)?;
            let total_grad = grad.add(&reg_term)?;
            let scaled_grad = total_grad.mul_scalar(learning_rate)?;

            w_out = w_out.sub(&scaled_grad)?;
        }

        Ok(w_out)
    }

    /// Predict on new input sequence
    pub async fn predict(&mut self, input: &[f32]) -> BarracudaResult<Vec<f32>> {
        let (output, _state) = self.predict_return_state(input).await?;
        Ok(output)
    }

    /// Predict and return both output AND raw reservoir state.
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

    /// Export readout weights as int8-quantized NPU format.
    ///
    /// Requires the ESN to be trained. Converts f32 readout weights to f64,
    /// applies affine quantization, and returns `NpuReadoutWeights` suitable
    /// for NPU deployment (e.g. Akida AKD1000 FC layer).
    pub fn to_npu_weights(&self) -> BarracudaResult<NpuReadoutWeights> {
        let w_out =
            self.w_out
                .as_ref()
                .ok_or_else(|| crate::error::BarracudaError::InvalidOperation {
                    op: "ESN::to_npu_weights".to_string(),
                    reason: "ESN has not been trained yet — call train() first".to_string(),
                })?;

        let w_out_f32 = w_out.to_vec()?;
        let w_out_f64: Vec<f64> = w_out_f32.iter().map(|&x| f64::from(x)).collect();

        let (weights_i8, scale, zero_point) = quantize_affine_i8_f64(&w_out_f64);

        Ok(NpuReadoutWeights {
            weights_i8,
            scale,
            zero_point,
            input_dim: self.config.reservoir_size,
            output_dim: self.config.output_size,
        })
    }

    /// Export all ESN weights as flat f32 vectors for cross-device deployment.
    pub fn export_weights(&self) -> BarracudaResult<ExportedWeights> {
        let w_in_data = self.w_in.to_vec()?;
        let w_res_data = self.w_res.to_vec()?;
        let w_out_data = match &self.w_out {
            Some(w) => Some(w.to_vec()?),
            None => None,
        };
        Ok((w_in_data, w_res_data, w_out_data))
    }

    /// Import pre-trained weights
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
    use crate::device::{Device, WorkloadHint};

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
    }

    #[tokio::test]
    async fn test_esn_workload_hint() {
        let config = ESNConfig::default();
        let esn = ESN::new(config).await.unwrap();
        let _esn_large = esn.with_hint(WorkloadHint::LargeMatrices);
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

        let inputs = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0], vec![4.0]];
        let targets = vec![vec![1.0], vec![2.0], vec![3.0], vec![4.0], vec![5.0]];

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

        let inputs = vec![vec![0.0], vec![1.0], vec![2.0], vec![3.0]];
        let targets = vec![vec![2.0], vec![3.0], vec![4.0], vec![5.0]];

        esn.train(&inputs, &targets).await.unwrap();

        let prediction = esn.predict(&[10.0]).await.unwrap();
        assert_eq!(prediction.len(), 1);
        assert!(prediction[0] > 5.0 && prediction[0] < 20.0);
    }

    #[tokio::test]
    async fn test_esn_train_mismatched_lengths() {
        let config = ESNConfig::default();
        let mut esn = ESN::new(config).await.unwrap();

        let inputs = vec![vec![0.0], vec![1.0]];
        let targets = vec![vec![1.0]];

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

        let result = esn.predict(&[1.0]).await;
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

        let inputs = vec![vec![0.0, 0.0], vec![1.0, 1.0]];
        let targets = vec![vec![1.0], vec![2.0]];
        esn.train(&inputs, &targets).await.unwrap();

        let result = esn.predict(&[1.0]).await;
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

        let prediction = esn.predict(&[0.5, 0.5]).await.unwrap();
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

        let inputs = vec![vec![1.0], vec![2.0], vec![3.0]];
        let targets = vec![vec![2.0], vec![3.0], vec![4.0]];
        esn.train(&inputs, &targets).await.unwrap();

        let pred1 = esn.predict(&[5.0]).await.unwrap();
        let pred2 = esn.predict(&[5.0]).await.unwrap();

        assert_ne!(pred1, pred2, "State should evolve between predictions");
    }

    #[tokio::test]
    async fn test_esn_to_npu_weights() {
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

        let npu = esn.to_npu_weights().unwrap();
        assert_eq!(npu.input_dim, 20);
        assert_eq!(npu.output_dim, 1);
        assert_eq!(npu.weights_i8.len(), 20);
        assert!(npu.scale > 0.0);
    }

    #[tokio::test]
    async fn test_esn_to_npu_weights_untrained() {
        let config = ESNConfig::default();
        let esn = ESN::new(config).await.unwrap();
        assert!(esn.to_npu_weights().is_err());
    }

    #[tokio::test]
    async fn test_esn_train_ridge_regression_linear() {
        let config = ESNConfig {
            input_size: 1,
            reservoir_size: 4,
            output_size: 1,
            ..Default::default()
        };

        let mut esn = ESN::new(config).await.unwrap();

        let n_samples = 10;
        let mut states = vec![0.0; 4 * n_samples];
        let mut targets = vec![0.0; 1 * n_samples];
        for k in 0..n_samples {
            let x = k as f64 * 0.5;
            states[k] = 1.0;
            states[1 * n_samples + k] = x;
            states[2 * n_samples + k] = x * x;
            states[3 * n_samples + k] = x * x * x;
            targets[k] = 2.0 + 3.0 * x;
        }

        esn.train_ridge_regression(&states, &targets, 1e-6).unwrap();
        assert!(esn.is_trained());

        let w = esn.w_out.as_ref().unwrap().to_vec().unwrap();
        assert_eq!(w.len(), 4);
        assert!((w[0] - 2.0).abs() < 0.1);
        assert!((w[1] - 3.0).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_esn_train_ridge_regression_regularization() {
        let config = ESNConfig {
            input_size: 1,
            reservoir_size: 5,
            output_size: 1,
            ..Default::default()
        };

        let mut esn_small = ESN::new(config.clone()).await.unwrap();
        let mut esn_large = ESN::new(config).await.unwrap();

        let n_samples = 8;
        let mut states = vec![0.0; 5 * n_samples];
        let mut targets = vec![0.0; n_samples];
        for k in 0..n_samples {
            for i in 0..5 {
                states[i * n_samples + k] = (k as f64 + i as f64) * 0.1;
            }
            targets[k] = (k as f64) * 0.2;
        }

        esn_small
            .train_ridge_regression(&states, &targets, 1e-6)
            .unwrap();
        esn_large
            .train_ridge_regression(&states, &targets, 10.0)
            .unwrap();

        let w_small = esn_small.w_out.as_ref().unwrap().to_vec().unwrap();
        let w_large = esn_large.w_out.as_ref().unwrap().to_vec().unwrap();

        let norm_small: f32 = w_small.iter().map(|x| x * x).sum::<f32>().sqrt();
        let norm_large: f32 = w_large.iter().map(|x| x * x).sum::<f32>().sqrt();

        assert!(
            norm_large < norm_small,
            "Larger lambda should produce smaller weights: {} < {}",
            norm_large,
            norm_small
        );
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

        let esn = ESN::new(config).await.unwrap();
        assert_eq!(esn.state().shape(), &[200, 1]);
    }
}
