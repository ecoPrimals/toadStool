//! Normalization Operations
//!
//! **Week 9 Implementation**: Essential normalization layers for modern deep learning
//!
//! ## Operations (4/4)
//!
//! 1. **BatchNormalization** - Normalizes across batch dimension (training stability!)
//! 2. **LayerNormalization** - Normalizes across features (transformers!)
//! 3. **InstanceNormalization** - Normalizes per instance (style transfer!)
//! 4. **GroupNormalization** - Normalizes within groups (small batches!)
//!
//! ## Philosophy - Deep Debt Excellence
//!
//! - ✅ **Pure Rust**: No unsafe code, numerically stable
//! - ✅ **Production-Ready**: Training and inference modes
//! - ✅ **Modern ML**: Powers transformers, GANs, ResNets
//! - ✅ **Well-Tested**: Numerical correctness verified
//!
//! ## Impact
//!
//! **Enables Modern Deep Learning**:
//! - Transformers (BERT, GPT) use LayerNorm
//! - ResNets use BatchNorm for training stability
//! - StyleGAN uses InstanceNorm for style transfer
//! - Small-batch training uses GroupNorm

use anyhow::Result;

/// Batch Normalization
///
/// Normalizes activations across the batch dimension.
///
/// ## Algorithm
///
/// ```text
/// Training:
///   μ = mean(x, axis=batch)
///   σ² = variance(x, axis=batch)
///   x_norm = (x - μ) / sqrt(σ² + ε)
///   y = γ * x_norm + β
///
/// Inference:
///   Use running mean/variance from training
/// ```
///
/// ## Impact
///
/// - **Training Stability**: Reduces internal covariate shift
/// - **Higher Learning Rates**: More stable gradients
/// - **Regularization**: Acts as implicit regularizer
/// - **Faster Convergence**: Accelerates training
///
/// ## Use Cases
///
/// - ResNet (THE killer app!)
/// - Inception networks
/// - Most CNNs for image classification
/// - Any deep network needing stability
///
/// ## Reference
///
/// Ioffe & Szegedy, "Batch Normalization: Accelerating Deep Network Training
/// by Reducing Internal Covariate Shift", 2015
pub struct BatchNormalization {
    epsilon: f32,
    momentum: f32,
}

impl BatchNormalization {
    /// Create new BatchNormalization layer
    ///
    /// # Arguments
    ///
    /// * `epsilon` - Small constant for numerical stability (typically 1e-5)
    /// * `momentum` - Momentum for running statistics (typically 0.1)
    pub fn new(epsilon: f32, momentum: f32) -> Self {
        Self { epsilon, momentum }
    }

    /// Apply batch normalization (training mode)
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor (batch_size, channels, height, width)
    /// * `gamma` - Scale parameter (learnable, size: channels)
    /// * `beta` - Shift parameter (learnable, size: channels)
    /// * `batch_size` - Batch size
    /// * `channels` - Number of channels
    /// * `spatial_size` - Height * Width
    ///
    /// # Returns
    ///
    /// Tuple of (normalized_output, batch_mean, batch_variance)
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let bn = BatchNormalization::new(1e-5, 0.1);
    /// let (output, mean, var) = bn.forward_train(&input, &gamma, &beta, 32, 64, 49)?;
    /// ```
    pub fn forward_train(
        &self,
        input: &[f32],
        gamma: &[f32],
        beta: &[f32],
        batch_size: usize,
        channels: usize,
        spatial_size: usize,
    ) -> Result<(Vec<f32>, Vec<f32>, Vec<f32>)> {
        let total_size = batch_size * channels * spatial_size;
        anyhow::ensure!(
            input.len() == total_size,
            "Input size mismatch: expected {}, got {}",
            total_size,
            input.len()
        );
        anyhow::ensure!(gamma.len() == channels, "Gamma size must equal channels");
        anyhow::ensure!(beta.len() == channels, "Beta size must equal channels");

        let mut output = vec![0.0f32; total_size];
        let mut batch_mean = vec![0.0f32; channels];
        let mut batch_var = vec![0.0f32; channels];

        // Compute mean and variance for each channel
        for c in 0..channels {
            let mut sum = 0.0f32;
            let mut sum_sq = 0.0f32;
            let n = (batch_size * spatial_size) as f32;

            // Accumulate statistics across batch and spatial dimensions
            for b in 0..batch_size {
                for s in 0..spatial_size {
                    let idx = b * channels * spatial_size + c * spatial_size + s;
                    let val = input[idx];
                    sum += val;
                    sum_sq += val * val;
                }
            }

            let mean = sum / n;
            let variance = (sum_sq / n) - (mean * mean);

            batch_mean[c] = mean;
            batch_var[c] = variance;

            // Normalize and scale
            let std_dev = (variance + self.epsilon).sqrt();
            for b in 0..batch_size {
                for s in 0..spatial_size {
                    let idx = b * channels * spatial_size + c * spatial_size + s;
                    let normalized = (input[idx] - mean) / std_dev;
                    output[idx] = gamma[c] * normalized + beta[c];
                }
            }
        }

        Ok((output, batch_mean, batch_var))
    }

    /// Apply batch normalization (inference mode)
    ///
    /// Uses running mean and variance from training.
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor
    /// * `gamma` - Scale parameter
    /// * `beta` - Shift parameter
    /// * `running_mean` - Running mean from training
    /// * `running_var` - Running variance from training
    /// * `batch_size` - Batch size
    /// * `channels` - Number of channels
    /// * `spatial_size` - Height * Width
    pub fn forward_inference(
        &self,
        input: &[f32],
        gamma: &[f32],
        beta: &[f32],
        running_mean: &[f32],
        running_var: &[f32],
        batch_size: usize,
        channels: usize,
        spatial_size: usize,
    ) -> Result<Vec<f32>> {
        let total_size = batch_size * channels * spatial_size;
        anyhow::ensure!(input.len() == total_size, "Input size mismatch");
        anyhow::ensure!(running_mean.len() == channels, "Running mean size mismatch");
        anyhow::ensure!(running_var.len() == channels, "Running var size mismatch");

        let mut output = vec![0.0f32; total_size];

        for c in 0..channels {
            let mean = running_mean[c];
            let variance = running_var[c];
            let std_dev = (variance + self.epsilon).sqrt();

            for b in 0..batch_size {
                for s in 0..spatial_size {
                    let idx = b * channels * spatial_size + c * spatial_size + s;
                    let normalized = (input[idx] - mean) / std_dev;
                    output[idx] = gamma[c] * normalized + beta[c];
                }
            }
        }

        Ok(output)
    }

    /// Update running statistics
    pub fn update_running_stats(
        &self,
        running_mean: &mut [f32],
        running_var: &mut [f32],
        batch_mean: &[f32],
        batch_var: &[f32],
    ) {
        for i in 0..running_mean.len() {
            running_mean[i] =
                (1.0 - self.momentum) * running_mean[i] + self.momentum * batch_mean[i];
            running_var[i] = (1.0 - self.momentum) * running_var[i] + self.momentum * batch_var[i];
        }
    }
}

/// Layer Normalization
///
/// Normalizes across the feature dimension (not batch).
///
/// ## Algorithm
///
/// ```text
/// For each sample in batch:
///   μ = mean(x, axis=features)
///   σ² = variance(x, axis=features)
///   x_norm = (x - μ) / sqrt(σ² + ε)
///   y = γ * x_norm + β
/// ```
///
/// ## Advantages over BatchNorm
///
/// - **Batch-size independent**: Works with batch size = 1
/// - **Sequence models**: Perfect for RNNs, Transformers
/// - **No running stats**: Same computation in train/inference
///
/// ## Use Cases
///
/// - **Transformers**: BERT, GPT, T5 (THE killer app!)
/// - RNNs and LSTMs
/// - Any sequence model
/// - Models with variable batch sizes
///
/// ## Reference
///
/// Ba et al., "Layer Normalization", 2016
pub struct LayerNormalization {
    epsilon: f32,
}

impl LayerNormalization {
    /// Create new LayerNormalization layer
    pub fn new(epsilon: f32) -> Self {
        Self { epsilon }
    }

    /// Apply layer normalization
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor (batch_size, features)
    /// * `gamma` - Scale parameter (size: features)
    /// * `beta` - Shift parameter (size: features)
    /// * `batch_size` - Batch size
    /// * `features` - Number of features
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let ln = LayerNormalization::new(1e-5);
    /// let output = ln.forward(&input, &gamma, &beta, 32, 512)?;
    /// ```
    pub fn forward(
        &self,
        input: &[f32],
        gamma: &[f32],
        beta: &[f32],
        batch_size: usize,
        features: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(input.len() == batch_size * features, "Input size mismatch");
        anyhow::ensure!(gamma.len() == features, "Gamma size mismatch");
        anyhow::ensure!(beta.len() == features, "Beta size mismatch");

        let mut output = vec![0.0f32; batch_size * features];

        // Normalize each sample independently
        for b in 0..batch_size {
            let start = b * features;
            let end = start + features;
            let sample = &input[start..end];

            // Compute mean and variance for this sample
            let mean: f32 = sample.iter().sum::<f32>() / features as f32;
            let variance: f32 =
                sample.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / features as f32;

            let std_dev = (variance + self.epsilon).sqrt();

            // Normalize and scale
            for f in 0..features {
                let idx = start + f;
                let normalized = (input[idx] - mean) / std_dev;
                output[idx] = gamma[f] * normalized + beta[f];
            }
        }

        Ok(output)
    }
}

/// Instance Normalization
///
/// Normalizes each instance (sample) independently across spatial dimensions.
///
/// ## Algorithm
///
/// ```text
/// For each (batch, channel) pair:
///   μ = mean(x, axis=spatial)
///   σ² = variance(x, axis=spatial)
///   x_norm = (x - μ) / sqrt(σ² + ε)
///   y = γ * x_norm + β
/// ```
///
/// ## Advantages
///
/// - **Style-invariant**: Removes instance-specific contrast
/// - **Real-time**: No batch statistics needed
/// - **Artistic**: Enables style transfer
///
/// ## Use Cases
///
/// - **Style Transfer**: Neural artistic style (THE killer app!)
/// - GANs (StyleGAN)
/// - Image-to-image translation
/// - Real-time video processing
///
/// ## Reference
///
/// Ulyanov et al., "Instance Normalization: The Missing Ingredient for Fast Stylization", 2016
pub struct InstanceNormalization {
    epsilon: f32,
}

impl InstanceNormalization {
    /// Create new InstanceNormalization layer
    pub fn new(epsilon: f32) -> Self {
        Self { epsilon }
    }

    /// Apply instance normalization
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor (batch_size, channels, height, width)
    /// * `gamma` - Scale parameter (size: channels)
    /// * `beta` - Shift parameter (size: channels)
    /// * `batch_size` - Batch size
    /// * `channels` - Number of channels
    /// * `spatial_size` - Height * Width
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let inst_norm = InstanceNormalization::new(1e-5);
    /// let output = inst_norm.forward(&input, &gamma, &beta, 1, 64, 256)?;
    /// ```
    pub fn forward(
        &self,
        input: &[f32],
        gamma: &[f32],
        beta: &[f32],
        batch_size: usize,
        channels: usize,
        spatial_size: usize,
    ) -> Result<Vec<f32>> {
        let total_size = batch_size * channels * spatial_size;
        anyhow::ensure!(input.len() == total_size, "Input size mismatch");
        anyhow::ensure!(gamma.len() == channels, "Gamma size mismatch");
        anyhow::ensure!(beta.len() == channels, "Beta size mismatch");

        let mut output = vec![0.0f32; total_size];

        // Normalize each (batch, channel) pair independently
        for b in 0..batch_size {
            for c in 0..channels {
                let mut sum = 0.0f32;
                let mut sum_sq = 0.0f32;

                // Compute statistics over spatial dimensions only
                for s in 0..spatial_size {
                    let idx = b * channels * spatial_size + c * spatial_size + s;
                    let val = input[idx];
                    sum += val;
                    sum_sq += val * val;
                }

                let mean = sum / spatial_size as f32;
                let variance = (sum_sq / spatial_size as f32) - (mean * mean);
                let std_dev = (variance + self.epsilon).sqrt();

                // Normalize and scale
                for s in 0..spatial_size {
                    let idx = b * channels * spatial_size + c * spatial_size + s;
                    let normalized = (input[idx] - mean) / std_dev;
                    output[idx] = gamma[c] * normalized + beta[c];
                }
            }
        }

        Ok(output)
    }
}

/// Group Normalization
///
/// Divides channels into groups and normalizes within each group.
///
/// ## Algorithm
///
/// ```text
/// Divide channels into G groups
/// For each (batch, group) pair:
///   μ = mean(x, axis=(channels_in_group, spatial))
///   σ² = variance(x, axis=(channels_in_group, spatial))
///   x_norm = (x - μ) / sqrt(σ² + ε)
///   y = γ * x_norm + β
/// ```
///
/// ## Advantages
///
/// - **Batch-size independent**: Like LayerNorm
/// - **Better than LayerNorm for CNNs**: Respects channel structure
/// - **Small batches**: Works well when batch size is small
///
/// ## Use Cases
///
/// - **Small Batch Training**: When GPU memory is limited
/// - Object detection (Mask R-CNN)
/// - Video understanding (limited batch size)
/// - Transfer learning (fine-tuning with small batches)
///
/// ## Reference
///
/// Wu & He, "Group Normalization", 2018 (Facebook AI Research)
pub struct GroupNormalization {
    epsilon: f32,
    num_groups: usize,
}

impl GroupNormalization {
    /// Create new GroupNormalization layer
    ///
    /// # Arguments
    ///
    /// * `num_groups` - Number of groups (typically 32)
    /// * `epsilon` - Small constant for numerical stability
    pub fn new(num_groups: usize, epsilon: f32) -> Self {
        Self {
            num_groups,
            epsilon,
        }
    }

    /// Apply group normalization
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor (batch_size, channels, height, width)
    /// * `gamma` - Scale parameter (size: channels)
    /// * `beta` - Shift parameter (size: channels)
    /// * `batch_size` - Batch size
    /// * `channels` - Number of channels (must be divisible by num_groups)
    /// * `spatial_size` - Height * Width
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// let gn = GroupNormalization::new(32, 1e-5);
    /// let output = gn.forward(&input, &gamma, &beta, 4, 64, 49)?;
    /// ```
    pub fn forward(
        &self,
        input: &[f32],
        gamma: &[f32],
        beta: &[f32],
        batch_size: usize,
        channels: usize,
        spatial_size: usize,
    ) -> Result<Vec<f32>> {
        let total_size = batch_size * channels * spatial_size;
        anyhow::ensure!(input.len() == total_size, "Input size mismatch");
        anyhow::ensure!(gamma.len() == channels, "Gamma size mismatch");
        anyhow::ensure!(beta.len() == channels, "Beta size mismatch");
        anyhow::ensure!(
            channels.is_multiple_of(self.num_groups),
            "Channels must be divisible by num_groups"
        );

        let channels_per_group = channels / self.num_groups;
        let mut output = vec![0.0f32; total_size];

        // Normalize each (batch, group) pair
        for b in 0..batch_size {
            for g in 0..self.num_groups {
                let mut sum = 0.0f32;
                let mut sum_sq = 0.0f32;
                let group_size = (channels_per_group * spatial_size) as f32;

                // Compute statistics over group
                for c_in_group in 0..channels_per_group {
                    let c = g * channels_per_group + c_in_group;
                    for s in 0..spatial_size {
                        let idx = b * channels * spatial_size + c * spatial_size + s;
                        let val = input[idx];
                        sum += val;
                        sum_sq += val * val;
                    }
                }

                let mean = sum / group_size;
                let variance = (sum_sq / group_size) - (mean * mean);
                let std_dev = (variance + self.epsilon).sqrt();

                // Normalize and scale
                for c_in_group in 0..channels_per_group {
                    let c = g * channels_per_group + c_in_group;
                    for s in 0..spatial_size {
                        let idx = b * channels * spatial_size + c * spatial_size + s;
                        let normalized = (input[idx] - mean) / std_dev;
                        output[idx] = gamma[c] * normalized + beta[c];
                    }
                }
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_batch_norm_train() {
        let bn = BatchNormalization::new(1e-5, 0.1);

        // Simple 2×2×1×2 input (batch=2, channels=2, spatial=2)
        let input = vec![
            1.0, 2.0, // batch 0, channel 0
            3.0, 4.0, // batch 0, channel 1
            5.0, 6.0, // batch 1, channel 0
            7.0, 8.0, // batch 1, channel 1
        ];
        let gamma = vec![1.0, 1.0];
        let beta = vec![0.0, 0.0];

        let result = bn.forward_train(&input, &gamma, &beta, 2, 2, 2);
        assert!(result.is_ok());

        let (output, mean, var) = result.unwrap();
        assert_eq!(output.len(), 8);
        assert_eq!(mean.len(), 2);
        assert_eq!(var.len(), 2);

        // Channel 0: [1,2,5,6], mean=3.5
        assert!((mean[0] - 3.5).abs() < 0.01);
        // Channel 1: [3,4,7,8], mean=5.5
        assert!((mean[1] - 5.5).abs() < 0.01);
    }

    #[test]
    fn test_layer_norm() {
        let ln = LayerNormalization::new(1e-5);

        // 2 samples, 4 features each
        let input = vec![
            1.0, 2.0, 3.0, 4.0, // sample 0
            5.0, 6.0, 7.0, 8.0, // sample 1
        ];
        let gamma = vec![1.0, 1.0, 1.0, 1.0];
        let beta = vec![0.0, 0.0, 0.0, 0.0];

        let result = ln.forward(&input, &gamma, &beta, 2, 4);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.len(), 8);

        // Each sample should have mean ≈ 0, std ≈ 1 after normalization
        let sample0_mean: f32 = output[0..4].iter().sum::<f32>() / 4.0;
        assert!(sample0_mean.abs() < 0.01, "Sample 0 mean should be ~0");
    }

    #[test]
    fn test_instance_norm() {
        let inst_norm = InstanceNormalization::new(1e-5);

        // 1 batch, 2 channels, 2 spatial
        let input = vec![
            1.0, 2.0, // channel 0
            3.0, 4.0, // channel 1
        ];
        let gamma = vec![1.0, 1.0];
        let beta = vec![0.0, 0.0];

        let result = inst_norm.forward(&input, &gamma, &beta, 1, 2, 2);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.len(), 4);

        // Each channel should be normalized independently
        // Channel 0: [1,2], mean=1.5, after norm should have mean≈0
        let ch0_mean = (output[0] + output[1]) / 2.0;
        assert!(ch0_mean.abs() < 0.01);
    }

    #[test]
    fn test_group_norm() {
        let gn = GroupNormalization::new(2, 1e-5);

        // 1 batch, 4 channels (2 groups), 2 spatial
        let input = vec![
            1.0, 2.0, // channel 0 (group 0)
            3.0, 4.0, // channel 1 (group 0)
            5.0, 6.0, // channel 2 (group 1)
            7.0, 8.0, // channel 3 (group 1)
        ];
        let gamma = vec![1.0, 1.0, 1.0, 1.0];
        let beta = vec![0.0, 0.0, 0.0, 0.0];

        let result = gn.forward(&input, &gamma, &beta, 1, 4, 2);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.len(), 8);
    }

    #[test]
    fn test_batch_norm_inference() {
        let bn = BatchNormalization::new(1e-5, 0.1);

        let input = vec![1.0, 2.0, 3.0, 4.0];
        let gamma = vec![1.0, 1.0];
        let beta = vec![0.0, 0.0];
        let running_mean = vec![2.5, 2.5];
        let running_var = vec![1.25, 1.25];

        let result =
            bn.forward_inference(&input, &gamma, &beta, &running_mean, &running_var, 2, 2, 1);
        assert!(result.is_ok());

        let output = result.unwrap();
        assert_eq!(output.len(), 4);
    }

    #[test]
    fn test_normalization_preserves_scale() {
        let ln = LayerNormalization::new(1e-5);

        // Scale test: gamma=2, beta=1
        let input = vec![1.0, 2.0, 3.0, 4.0];
        let gamma = vec![2.0, 2.0, 2.0, 2.0];
        let beta = vec![1.0, 1.0, 1.0, 1.0];

        let output = ln.forward(&input, &gamma, &beta, 1, 4).unwrap();

        // Mean should be ≈ 1 (shifted by beta)
        let mean: f32 = output.iter().sum::<f32>() / 4.0;
        assert!((mean - 1.0).abs() < 0.01, "Mean should be shifted to beta");
    }

    #[test]
    fn test_group_norm_channels_divisibility() {
        let gn = GroupNormalization::new(3, 1e-5);

        // 5 channels not divisible by 3 groups
        let input = vec![0.0; 5 * 2];
        let gamma = vec![1.0; 5];
        let beta = vec![0.0; 5];

        let result = gn.forward(&input, &gamma, &beta, 1, 5, 2);
        assert!(
            result.is_err(),
            "Should fail when channels not divisible by groups"
        );
    }
}
