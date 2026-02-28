//! Final Operations - Completing the 100 Operation Milestone
//!
//! **Week 10 Implementation**: Essential pooling, activations, losses, and tensor ops
//!
//! ## Operations (9/9) - THE FINAL NINE!
//!
//! ### Pooling (3 operations)
//! 1. **MaxPool2D** - Max pooling (downsampling)
//! 2. **AvgPool2D** - Average pooling (smooth downsampling)
//! 3. **AdaptiveAvgPool** - Adaptive average pooling (any output size)
//!
//! ### Modern Activations (2 operations)
//! 4. **Swish/SiLU** - Self-gated activation (EfficientNet!)
//! 5. **GELU** - Gaussian Error Linear Unit (BERT, GPT!)
//!
//! ### Loss Functions (2 operations)
//! 6. **CrossEntropyLoss** - Classification loss (THE standard)
//! 7. **MSELoss** - Mean Squared Error (regression)
//!
//! ### Tensor Operations (2 operations)
//! 8. **Transpose** - Tensor transposition (critical for attention)
//! 9. **Concatenate** - Tensor concatenation (ResNet, DenseNet)
//!
//! ## Philosophy - Deep Debt Excellence
//!
//! - ✅ **Pure Rust**: No unsafe code, production-ready
//! - ✅ **Complete Toolkit**: Everything needed for modern ML
//! - ✅ **Well-Tested**: All 9 operations validated
//! - ✅ **100 OPERATIONS**: Mission accomplished!

use anyhow::Result;

/// Max Pooling 2D
///
/// Downsamples input by taking maximum value in each pooling window.
///
/// ## Algorithm
///
/// ```text
/// For each output position (i, j):
///   output`i,j` = max(input[i*stride:i*stride+kernel, j*stride:j*stride+kernel])
/// ```
///
/// ## Use Cases
///
/// - **CNNs**: AlexNet, VGG, ResNet (THE standard!)
/// - Downsampling feature maps
/// - Translation invariance
/// - Spatial hierarchy building
///
/// ## Properties
///
/// - Non-linear downsampling
/// - Preserves dominant features
/// - No learnable parameters
/// - Typical: 2×2 kernel, stride 2 (50% reduction)
pub struct MaxPool2D {
    kernel_size: usize,
    stride: usize,
}

impl MaxPool2D {
    /// Create new MaxPool2D layer
    ///
    /// # Arguments
    ///
    /// * `kernel_size` - Pooling window size (typically 2)
    /// * `stride` - Stride for pooling (typically 2)
    pub fn new(kernel_size: usize, stride: usize) -> Self {
        Self {
            kernel_size,
            stride,
        }
    }

    /// Apply max pooling
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor (batch, channels, height, width)
    /// * `batch_size` - Batch size
    /// * `channels` - Number of channels
    /// * `height` - Input height
    /// * `width` - Input width
    ///
    /// # Returns
    ///
    /// Pooled tensor with reduced spatial dimensions
    pub fn forward(
        &self,
        input: &[f32],
        batch_size: usize,
        channels: usize,
        height: usize,
        width: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch_size * channels * height * width,
            "Input size mismatch"
        );

        let out_height = (height - self.kernel_size) / self.stride + 1;
        let out_width = (width - self.kernel_size) / self.stride + 1;
        let output_size = batch_size * channels * out_height * out_width;

        let mut output = vec![f32::NEG_INFINITY; output_size];

        for b in 0..batch_size {
            for c in 0..channels {
                for oh in 0..out_height {
                    for ow in 0..out_width {
                        let mut max_val = f32::NEG_INFINITY;

                        // Pool over kernel window
                        for kh in 0..self.kernel_size {
                            for kw in 0..self.kernel_size {
                                let ih = oh * self.stride + kh;
                                let iw = ow * self.stride + kw;

                                if ih < height && iw < width {
                                    let in_idx = b * (channels * height * width)
                                        + c * (height * width)
                                        + ih * width
                                        + iw;
                                    max_val = max_val.max(input[in_idx]);
                                }
                            }
                        }

                        let out_idx = b * (channels * out_height * out_width)
                            + c * (out_height * out_width)
                            + oh * out_width
                            + ow;
                        output[out_idx] = max_val;
                    }
                }
            }
        }

        Ok(output)
    }
}

/// Average Pooling 2D
///
/// Downsamples input by computing average value in each pooling window.
///
/// ## Algorithm
///
/// ```text
/// For each output position (i, j):
///   output`i,j` = mean(input[i*stride:i*stride+kernel, j*stride:j*stride+kernel])
/// ```
///
/// ## Use Cases
///
/// - Global average pooling (GAP) for classification
/// - Smooth downsampling (less aggressive than max)
/// - Final layer before FC (ResNet, EfficientNet)
pub struct AvgPool2D {
    kernel_size: usize,
    stride: usize,
}

impl AvgPool2D {
    pub fn new(kernel_size: usize, stride: usize) -> Self {
        Self {
            kernel_size,
            stride,
        }
    }

    pub fn forward(
        &self,
        input: &[f32],
        batch_size: usize,
        channels: usize,
        height: usize,
        width: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch_size * channels * height * width,
            "Input size mismatch"
        );

        let out_height = (height - self.kernel_size) / self.stride + 1;
        let out_width = (width - self.kernel_size) / self.stride + 1;
        let output_size = batch_size * channels * out_height * out_width;

        let mut output = vec![0.0f32; output_size];

        for b in 0..batch_size {
            for c in 0..channels {
                for oh in 0..out_height {
                    for ow in 0..out_width {
                        let mut sum = 0.0f32;
                        let mut count = 0usize;

                        // Average over kernel window
                        for kh in 0..self.kernel_size {
                            for kw in 0..self.kernel_size {
                                let ih = oh * self.stride + kh;
                                let iw = ow * self.stride + kw;

                                if ih < height && iw < width {
                                    let in_idx = b * (channels * height * width)
                                        + c * (height * width)
                                        + ih * width
                                        + iw;
                                    sum += input[in_idx];
                                    count += 1;
                                }
                            }
                        }

                        let out_idx = b * (channels * out_height * out_width)
                            + c * (out_height * out_width)
                            + oh * out_width
                            + ow;
                        output[out_idx] = sum / count as f32;
                    }
                }
            }
        }

        Ok(output)
    }
}

/// Adaptive Average Pooling
///
/// Pools to a specific output size regardless of input size.
///
/// ## Algorithm
///
/// Dynamically computes kernel size and stride to produce desired output.
///
/// ## Use Cases
///
/// - **Classification heads**: ResNet, EfficientNet use (1,1) output
/// - Variable input sizes
/// - Transfer learning (different image sizes)
pub struct AdaptiveAvgPool {
    output_height: usize,
    output_width: usize,
}

impl AdaptiveAvgPool {
    pub fn new(output_height: usize, output_width: usize) -> Self {
        Self {
            output_height,
            output_width,
        }
    }

    pub fn forward(
        &self,
        input: &[f32],
        batch_size: usize,
        channels: usize,
        height: usize,
        width: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch_size * channels * height * width,
            "Input size mismatch"
        );

        let output_size = batch_size * channels * self.output_height * self.output_width;
        let mut output = vec![0.0f32; output_size];

        for b in 0..batch_size {
            for c in 0..channels {
                for oh in 0..self.output_height {
                    for ow in 0..self.output_width {
                        // Compute input window for this output position
                        let h_start = (oh * height) / self.output_height;
                        let h_end = ((oh + 1) * height) / self.output_height;
                        let w_start = (ow * width) / self.output_width;
                        let w_end = ((ow + 1) * width) / self.output_width;

                        let mut sum = 0.0f32;
                        let mut count = 0usize;

                        for ih in h_start..h_end {
                            for iw in w_start..w_end {
                                let in_idx = b * (channels * height * width)
                                    + c * (height * width)
                                    + ih * width
                                    + iw;
                                sum += input[in_idx];
                                count += 1;
                            }
                        }

                        let out_idx = b * (channels * self.output_height * self.output_width)
                            + c * (self.output_height * self.output_width)
                            + oh * self.output_width
                            + ow;
                        output[out_idx] = sum / count as f32;
                    }
                }
            }
        }

        Ok(output)
    }
}

/// Swish / SiLU Activation
///
/// Self-gated activation: f(x) = x * sigmoid(x)
///
/// ## Properties
///
/// - Smooth, non-monotonic
/// - Self-gated (no separate gate parameters)
/// - Better than ReLU in many cases
///
/// ## Use Cases
///
/// - **EfficientNet**: Uses Swish throughout (state-of-the-art!)
/// - Mobile models (better accuracy/size trade-off)
/// - Modern CNNs
///
/// ## Reference
///
/// Ramachandran et al., "Searching for Activation Functions", 2017 (Google Brain)
pub struct Swish;

impl Swish {
    /// Apply Swish activation: f(x) = x * sigmoid(x)
    pub fn forward(input: &[f32]) -> Vec<f32> {
        input
            .iter()
            .map(|&x| {
                let sigmoid = 1.0 / (1.0 + (-x).exp());
                x * sigmoid
            })
            .collect()
    }
}

/// GELU Activation
///
/// Gaussian Error Linear Unit: f(x) ≈ 0.5 * x * (1 + tanh(√(2/π) * (x + 0.044715 * x³)))
///
/// ## Properties
///
/// - Smooth, probabilistic activation
/// - Based on Gaussian CDF
/// - Better than ReLU for transformers
///
/// ## Use Cases
///
/// - **BERT, GPT, Transformers**: THE standard activation!
/// - Most modern NLP models
/// - Vision transformers (ViT)
///
/// ## Reference
///
/// Hendrycks & Gimpel, "Gaussian Error Linear Units (GELUs)", 2016
pub struct GELU;

impl GELU {
    /// Apply GELU activation (fast approximation)
    pub fn forward(input: &[f32]) -> Vec<f32> {
        input
            .iter()
            .map(|&x| {
                let sqrt_2_over_pi = (2.0f32 / std::f32::consts::PI).sqrt();
                let inner = sqrt_2_over_pi * (x + 0.044715 * x.powi(3));
                let tanh_val = inner.tanh();
                0.5 * x * (1.0 + tanh_val)
            })
            .collect()
    }
}

/// Cross Entropy Loss
///
/// Standard loss for multi-class classification.
///
/// ## Formula
///
/// ```text
/// Loss = -mean(sum(y_true * log(y_pred)))
/// ```
///
/// For single-label: Loss = -log(y_pred`true_class`)
///
/// ## Use Cases
///
/// - **Classification**: THE standard loss!
/// - Image classification (ImageNet)
/// - NLP classification (BERT fine-tuning)
/// - Any multi-class problem
pub struct CrossEntropyLoss;

impl CrossEntropyLoss {
    /// Compute cross entropy loss
    ///
    /// # Arguments
    ///
    /// * `predictions` - Model predictions (logits) [batch_size, num_classes]
    /// * `targets` - True class indices `batch_size`
    /// * `batch_size` - Batch size
    /// * `num_classes` - Number of classes
    ///
    /// # Returns
    ///
    /// Average loss over batch
    pub fn compute(
        predictions: &[f32],
        targets: &[usize],
        batch_size: usize,
        num_classes: usize,
    ) -> Result<f32> {
        anyhow::ensure!(
            predictions.len() == batch_size * num_classes,
            "Predictions size mismatch"
        );
        anyhow::ensure!(targets.len() == batch_size, "Targets size mismatch");

        let mut total_loss = 0.0f32;

        for b in 0..batch_size {
            // Compute softmax for this sample
            let start = b * num_classes;
            let end = start + num_classes;
            let logits = &predictions[start..end];

            // Softmax: exp normalization for numerical stability
            let max_logit = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let exp_sum: f32 = logits.iter().map(|&x| (x - max_logit).exp()).sum();

            let true_class = targets[b];
            anyhow::ensure!(
                true_class < num_classes,
                "Target class {true_class} out of range"
            );

            // Log probability of true class
            let log_prob = (logits[true_class] - max_logit) - exp_sum.ln();
            total_loss -= log_prob;
        }

        Ok(total_loss / batch_size as f32)
    }
}

/// Mean Squared Error Loss
///
/// Standard loss for regression problems.
///
/// ## Formula
///
/// ```text
/// Loss = mean((y_pred - y_true)²)
/// ```
///
/// ## Use Cases
///
/// - **Regression**: Continuous value prediction
/// - Autoencoders (reconstruction loss)
/// - Super-resolution
/// - Any continuous output
pub struct MSELoss;

impl MSELoss {
    /// Compute mean squared error
    ///
    /// # Arguments
    ///
    /// * `predictions` - Model predictions
    /// * `targets` - True values
    ///
    /// # Returns
    ///
    /// Average MSE
    pub fn compute(predictions: &[f32], targets: &[f32]) -> Result<f32> {
        anyhow::ensure!(
            predictions.len() == targets.len(),
            "Predictions and targets must have same size"
        );

        let mse: f32 = predictions
            .iter()
            .zip(targets.iter())
            .map(|(pred, target)| (pred - target).powi(2))
            .sum::<f32>()
            / predictions.len() as f32;

        Ok(mse)
    }
}

/// Transpose
///
/// Transposes dimensions of a tensor.
///
/// ## Use Cases
///
/// - **Attention mechanisms**: Query/Key transpose for attention scores
/// - Matrix operations
/// - Channel reordering
/// - CRITICAL for transformer operations!
pub struct Transpose;

impl Transpose {
    /// Transpose last two dimensions (most common case)
    ///
    /// For shape (batch, seq_len, hidden): no change
    /// For shape (batch, heads, seq, hidden): transposes seq and hidden
    ///
    /// # Arguments
    ///
    /// * `input` - Input tensor
    /// * `dims` - Dimension sizes (e.g., [batch, rows, cols])
    ///
    /// # Returns
    ///
    /// Transposed tensor (swaps last two dimensions)
    pub fn transpose_2d(input: &[f32], rows: usize, cols: usize) -> Result<Vec<f32>> {
        anyhow::ensure!(input.len() == rows * cols, "Input size mismatch");

        let mut output = vec![0.0f32; rows * cols];

        for i in 0..rows {
            for j in 0..cols {
                output[j * rows + i] = input[i * cols + j];
            }
        }

        Ok(output)
    }

    /// Transpose with batch dimension
    pub fn transpose_batched(
        input: &[f32],
        batch_size: usize,
        rows: usize,
        cols: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(
            input.len() == batch_size * rows * cols,
            "Input size mismatch"
        );

        let mut output = vec![0.0f32; batch_size * rows * cols];

        for b in 0..batch_size {
            let batch_start = b * rows * cols;
            let out_start = b * rows * cols;

            for i in 0..rows {
                for j in 0..cols {
                    output[out_start + j * rows + i] = input[batch_start + i * cols + j];
                }
            }
        }

        Ok(output)
    }
}

/// Concatenate
///
/// Concatenates tensors along a dimension.
///
/// ## Use Cases
///
/// - **ResNet**: Skip connections (identity + residual)
/// - **DenseNet**: Dense connections (all previous layers)
/// - **U-Net**: Encoder-decoder connections
/// - Multi-scale feature fusion
pub struct Concatenate;

impl Concatenate {
    /// Concatenate along channel dimension
    ///
    /// # Arguments
    ///
    /// * `tensors` - Vector of tensors to concatenate
    /// * `batch_size` - Batch size (must be same for all)
    /// * `channels` - Vector of channel counts for each tensor
    /// * `spatial_size` - Spatial size (must be same for all)
    ///
    /// # Returns
    ///
    /// Concatenated tensor with channels = sum(channels)
    pub fn concat_channels(
        tensors: &[&[f32]],
        batch_size: usize,
        channels: &[usize],
        spatial_size: usize,
    ) -> Result<Vec<f32>> {
        anyhow::ensure!(!tensors.is_empty(), "No tensors to concatenate");
        anyhow::ensure!(
            tensors.len() == channels.len(),
            "Tensors and channels length mismatch"
        );

        let total_channels: usize = channels.iter().sum();
        let output_size = batch_size * total_channels * spatial_size;
        let mut output = vec![0.0f32; output_size];

        for b in 0..batch_size {
            let mut out_channel = 0;

            for (tensor_idx, &tensor) in tensors.iter().enumerate() {
                let tensor_channels = channels[tensor_idx];
                anyhow::ensure!(
                    tensor.len() == batch_size * tensor_channels * spatial_size,
                    "Tensor {tensor_idx} size mismatch"
                );

                // Copy this tensor's channels
                for c in 0..tensor_channels {
                    for s in 0..spatial_size {
                        let in_idx = b * (tensor_channels * spatial_size) + c * spatial_size + s;
                        let out_idx = b * (total_channels * spatial_size)
                            + (out_channel + c) * spatial_size
                            + s;
                        output[out_idx] = tensor[in_idx];
                    }
                }

                out_channel += tensor_channels;
            }
        }

        Ok(output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_maxpool2d() {
        let pool = MaxPool2D::new(2, 2);

        // 1×1×4×4 input
        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];

        let output = pool.forward(&input, 1, 1, 4, 4).unwrap();

        // Expected 2×2 output: max of each 2×2 region
        assert_eq!(output.len(), 4);
        assert_eq!(output[0], 6.0); // max(1,2,5,6)
        assert_eq!(output[1], 8.0); // max(3,4,7,8)
        assert_eq!(output[2], 14.0); // max(9,10,13,14)
        assert_eq!(output[3], 16.0); // max(11,12,15,16)
    }

    #[test]
    fn test_avgpool2d() {
        let pool = AvgPool2D::new(2, 2);

        let input = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0, 16.0,
        ];

        let output = pool.forward(&input, 1, 1, 4, 4).unwrap();

        assert_eq!(output.len(), 4);
        assert!((output[0] - 3.5).abs() < 0.01); // mean(1,2,5,6)
        assert!((output[1] - 5.5).abs() < 0.01); // mean(3,4,7,8)
    }

    #[test]
    fn test_adaptive_avgpool() {
        let pool = AdaptiveAvgPool::new(1, 1);

        // Global average pooling (any size to 1×1)
        let input = vec![1.0, 2.0, 3.0, 4.0];

        let output = pool.forward(&input, 1, 1, 2, 2).unwrap();

        assert_eq!(output.len(), 1);
        assert!((output[0] - 2.5).abs() < 0.01); // mean of all
    }

    #[test]
    fn test_swish_activation() {
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let output = Swish::forward(&input);

        assert_eq!(output.len(), 5);

        // Swish(0) = 0
        assert!(output[2].abs() < 0.01);

        // Swish(x) = x for large positive x
        // Swish(2) should be close to 2
        assert!((output[4] - 1.76).abs() < 0.1);

        // Negative values should be negative but closer to 0
        assert!(output[0] < 0.0);
        assert!(output[0] > -2.0);
    }

    #[test]
    fn test_gelu_activation() {
        let input = vec![-2.0, -1.0, 0.0, 1.0, 2.0];
        let output = GELU::forward(&input);

        assert_eq!(output.len(), 5);

        // GELU(0) ≈ 0
        assert!(output[2].abs() < 0.01);

        // GELU is roughly monotonically increasing
        assert!(output[4] > output[0]);
    }

    #[test]
    fn test_cross_entropy_loss() {
        // 2 samples, 3 classes
        let predictions = vec![
            1.0, 2.0, 3.0, // sample 0 (class 2 has highest logit)
            3.0, 2.0, 1.0, // sample 1 (class 0 has highest logit)
        ];
        let targets = vec![2, 0]; // Correct predictions

        let loss = CrossEntropyLoss::compute(&predictions, &targets, 2, 3).unwrap();

        // Loss should be low (correct predictions)
        assert!(loss < 1.0, "Loss too high for correct predictions");
        assert!(loss > 0.0, "Loss should be positive");
    }

    #[test]
    fn test_mse_loss() {
        let predictions = vec![1.0, 2.0, 3.0, 4.0];
        let targets = vec![1.0, 2.0, 3.0, 4.0];

        let loss = MSELoss::compute(&predictions, &targets).unwrap();

        // Perfect prediction = zero loss
        assert!(loss.abs() < 0.001);
    }

    #[test]
    fn test_mse_loss_with_error() {
        let predictions = vec![1.0, 2.0, 3.0, 4.0];
        let targets = vec![2.0, 3.0, 4.0, 5.0]; // Off by 1

        let loss = MSELoss::compute(&predictions, &targets).unwrap();

        // MSE = mean((1)²) = 1.0
        assert!((loss - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_transpose_2d() {
        let input = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];

        let output = Transpose::transpose_2d(&input, 2, 3).unwrap();

        // Should transpose to 3×2
        assert_eq!(output.len(), 6);
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 4.0);
        assert_eq!(output[2], 2.0);
        assert_eq!(output[3], 5.0);
    }

    #[test]
    fn test_concatenate_channels() {
        // Two 1×2×2 tensors
        let tensor1 = vec![1.0, 2.0, 3.0, 4.0]; // 1 batch, 2 channels, 2 spatial
        let tensor2 = vec![5.0, 6.0, 7.0, 8.0]; // 1 batch, 2 channels, 2 spatial

        let tensors = vec![tensor1.as_slice(), tensor2.as_slice()];
        let channels = vec![2, 2];

        let output = Concatenate::concat_channels(&tensors, 1, &channels, 2).unwrap();

        // Should have 4 channels total
        assert_eq!(output.len(), 8);

        // First tensor's channels, then second tensor's channels
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[4], 5.0);
        assert_eq!(output[5], 6.0);
    }
}
