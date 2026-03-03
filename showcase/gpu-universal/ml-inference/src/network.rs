// SPDX-License-Identifier: AGPL-3.0-or-later
//! Simple neural network for MNIST (real implementation, no mocks)

use anyhow::Result;
use ndarray::{Array1, Array2};
use rand::Rng;

/// Simple 2-layer neural network for MNIST
/// 784 -> 128 -> 10
#[derive(Clone)]
pub struct SimpleNetwork {
    pub w1: Array2<f32>, // (784, 128)
    pub b1: Array1<f32>, // (128,)
    pub w2: Array2<f32>, // (128, 10)
    pub b2: Array1<f32>, // (10,)
}

impl SimpleNetwork {
    /// Create new network with random weights
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();

        // He initialization
        let w1_scale = (2.0 / 784.0_f32).sqrt();
        let w1: Vec<f32> = (0..784 * 128)
            .map(|_| rng.gen::<f32>() * w1_scale - w1_scale / 2.0)
            .collect();

        let w2_scale = (2.0 / 128.0_f32).sqrt();
        let w2: Vec<f32> = (0..128 * 10)
            .map(|_| rng.gen::<f32>() * w2_scale - w2_scale / 2.0)
            .collect();

        Self {
            // SAFETY: We just created vectors with exact lengths (784*128 and 128*10)
            // These from_shape_vec calls cannot fail as dimensions match perfectly
            w1: Array2::from_shape_vec((784, 128), w1)
                .expect("w1 shape mismatch - this is a bug in SimpleNetwork::new"),
            b1: Array1::zeros(128),
            w2: Array2::from_shape_vec((128, 10), w2)
                .expect("w2 shape mismatch - this is a bug in SimpleNetwork::new"),
            b2: Array1::zeros(10),
        }
    }

    /// Load pre-trained weights
    pub fn load_pretrained() -> Result<Self> {
        // TODO: Load actual trained weights
        // For now, use initialized network (will get ~10% accuracy - random)
        Ok(Self::new())
    }

    /// Forward pass (CPU)
    pub fn forward_cpu(&self, input: &Array1<f32>) -> Result<Array1<f32>> {
        // Layer 1: input @ w1 + b1
        let z1 = input.dot(&self.w1) + &self.b1;

        // ReLU activation
        let a1 = z1.mapv(|x| x.max(0.0));

        // Layer 2: a1 @ w2 + b2
        let z2 = a1.dot(&self.w2) + &self.b2;

        // Softmax
        let exp_z2 = z2.mapv(|x| x.exp());
        let sum_exp = exp_z2.sum();
        let output = exp_z2 / sum_exp;

        Ok(output)
    }

    /// Forward pass batch (CPU)
    pub fn forward_batch_cpu(&self, inputs: &Array2<f32>) -> Result<Array2<f32>> {
        let batch_size = inputs.nrows();
        let mut outputs = Array2::zeros((batch_size, 10));

        for i in 0..batch_size {
            let input = inputs.row(i).to_owned();
            let output = self.forward_cpu(&input)?;
            outputs.row_mut(i).assign(&output);
        }

        Ok(outputs)
    }

    /// Get predicted class
    pub fn predict(&self, output: &Array1<f32>) -> (usize, f32) {
        let mut max_idx = 0;
        let mut max_val = output[0];

        for (i, &val) in output.iter().enumerate() {
            if val > max_val {
                max_val = val;
                max_idx = i;
            }
        }

        (max_idx, max_val)
    }

    /// Calculate accuracy on dataset
    pub fn accuracy(&self, images: &Array2<f32>, labels: &Array1<u8>) -> Result<f32> {
        let mut correct = 0;
        let total = images.nrows();

        for i in 0..total {
            let image = images.row(i).to_owned();
            let output = self.forward_cpu(&image)?;
            let (predicted, _) = self.predict(&output);

            if predicted == labels[i] as usize {
                correct += 1;
            }
        }

        Ok(correct as f32 / total as f32)
    }

    /// Forward pass on GPU using OpenCL
    #[cfg(feature = "opencl")]
    pub fn forward_gpu_opencl(
        &self,
        input: &Array1<f32>,
        executor: &crate::gpu_kernels::OpenCLExecutor,
    ) -> Result<Array1<f32>> {
        use anyhow::Context;

        // Prepare data for GPU
        let input_vec = input.to_vec();
        let w1_vec = self.w1.as_slice().context("w1 not contiguous")?.to_vec();
        let b1_vec = self.b1.to_vec();
        let w2_vec = self.w2.as_slice().context("w2 not contiguous")?.to_vec();
        let b2_vec = self.b2.to_vec();

        // Execute on GPU (batch size = 1)
        let output_vec =
            executor.forward_pass(&input_vec, &w1_vec, &b1_vec, &w2_vec, &b2_vec, 1)?;

        // Convert back to ndarray
        Ok(Array1::from_vec(output_vec))
    }

    /// Forward pass batch on GPU using OpenCL
    #[cfg(feature = "opencl")]
    pub fn forward_batch_gpu_opencl(
        &self,
        inputs: &Array2<f32>,
        executor: &crate::gpu_kernels::OpenCLExecutor,
    ) -> Result<Array2<f32>> {
        use anyhow::Context;

        let batch_size = inputs.nrows();

        // Prepare data for GPU
        let input_vec = inputs.as_slice().context("inputs not contiguous")?.to_vec();
        let w1_vec = self.w1.as_slice().context("w1 not contiguous")?.to_vec();
        let b1_vec = self.b1.to_vec();
        let w2_vec = self.w2.as_slice().context("w2 not contiguous")?.to_vec();
        let b2_vec = self.b2.to_vec();

        // Execute on GPU
        let output_vec =
            executor.forward_pass(&input_vec, &w1_vec, &b1_vec, &w2_vec, &b2_vec, batch_size)?;

        // Convert back to ndarray
        Array2::from_shape_vec((batch_size, 10), output_vec).context("Failed to reshape output")
    }

    /// Forward pass batch on GPU using Vulkan
    #[cfg(feature = "vulkan")]
    pub fn forward_batch_gpu_vulkan(
        &self,
        inputs: &Array2<f32>,
        executor: &crate::vulkan_executor::VulkanExecutor,
    ) -> Result<Array2<f32>> {
        use anyhow::Context;

        let batch_size = inputs.nrows();
        let input_size = 784;
        let hidden_size = 128;
        let output_size = 10;

        // Prepare data
        let input_vec = inputs.as_slice().context("inputs not contiguous")?.to_vec();
        let w1_vec = self.w1.as_slice().context("w1 not contiguous")?.to_vec();
        let b1_vec = self.b1.to_vec();
        let w2_vec = self.w2.as_slice().context("w2 not contiguous")?.to_vec();
        let b2_vec = self.b2.to_vec();

        // Layer 1: input @ w1 + b1
        let mut hidden =
            executor.matrix_multiply(&input_vec, &w1_vec, batch_size, input_size, hidden_size)?;

        // Add bias (broadcast across batch)
        for i in 0..batch_size {
            for j in 0..hidden_size {
                hidden[i * hidden_size + j] += b1_vec[j];
            }
        }

        // ReLU activation
        executor.relu(&mut hidden)?;

        // Layer 2: hidden @ w2 + b2
        let mut output =
            executor.matrix_multiply(&hidden, &w2_vec, batch_size, hidden_size, output_size)?;

        // Add bias (broadcast across batch)
        for i in 0..batch_size {
            for j in 0..output_size {
                output[i * output_size + j] += b2_vec[j];
            }
        }

        // Softmax per sample
        for i in 0..batch_size {
            let start = i * output_size;
            let end = start + output_size;
            executor.softmax(&mut output[start..end])?;
        }

        // Convert back to ndarray
        Array2::from_shape_vec((batch_size, 10), output).context("Failed to reshape output")
    }
}

impl Default for SimpleNetwork {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_pass() {
        let network = SimpleNetwork::new();
        let input = Array1::from_vec(vec![0.5; 784]);

        let output = network.forward_cpu(&input).unwrap();

        // Output should be 10 probabilities
        assert_eq!(output.len(), 10);

        // Should sum to ~1.0 (softmax)
        let sum: f32 = output.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5);

        // All probabilities should be in [0, 1]
        assert!(output.iter().all(|&x| x >= 0.0 && x <= 1.0));
    }

    #[test]
    fn test_predict() {
        let network = SimpleNetwork::new();
        let output = Array1::from_vec(vec![
            0.1, 0.05, 0.6, 0.05, 0.1, 0.05, 0.02, 0.01, 0.01, 0.01,
        ]);

        let (predicted, confidence) = network.predict(&output);

        assert_eq!(predicted, 2);
        assert_eq!(confidence, 0.6);
    }
}
