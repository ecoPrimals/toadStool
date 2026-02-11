//! Tversky Loss - GPU-accelerated generalized Dice Loss
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (new shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for medical imaging)
//!
//! ## Algorithm
//!
//! ```text
//! TP = true positives (intersection: pred * target)
//! FP = false positives (pred_sum - intersection)
//! FN = false negatives (target_sum - intersection)
//!
//! Tversky Index = TP / (TP + alpha*FP + beta*FN)
//! Tversky Loss = 1 - Tversky Index
//! ```
//!
//! **Special Cases**:
//! - `alpha = beta = 0.5`: Equivalent to Dice Loss
//! - `alpha = beta = 1.0`: Equivalent to Tanimoto coefficient
//! - `alpha < beta`: Penalize false negatives more (recall-focused)
//! - `alpha > beta`: Penalize false positives more (precision-focused)
//!
//! **Implementation**: GPU reduction with workgroup shared memory
//!
//! **Key Properties**:
//! - Fine-grained control over FP/FN trade-off
//! - Handles class imbalance
//! - Directly optimizes precision-recall balance
//! - Generalizes Dice Loss
//!
//! **Used By**: Medical imaging, imbalanced segmentation, cost-sensitive tasks
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let predictions = Tensor::randn(vec![4, 256, 256]).await?;  // [batch, H, W]
//! let targets = Tensor::randn(vec![4, 256, 256]).await?;
//!
//! // alpha < beta: Penalize false negatives more (medical imaging)
//! let loss = predictions.tversky_loss(&targets, 0.3, 0.7, 1.0)?;
//! ```

use crate::device::DeviceCapabilities;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Tversky loss parameters for WGSL shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TverskyParams {
    alpha: f32,
    beta: f32,
    smoothing: f32,
    batch_size: u32,
    elements_per_sample: u32,
}

/// Tversky Loss operation
///
/// **Deep Debt**: Uses new WGSL shader with workgroup reduction
pub struct TverskyLoss {
    predictions: Tensor,
    targets: Tensor,
    alpha: f32,
    beta: f32,
    smoothing: f32,
}

impl TverskyLoss {
    /// Create new Tversky loss operation
    ///
    /// **Deep Debt**: Validates inputs and hyperparameters
    pub fn new(
        predictions: Tensor,
        targets: Tensor,
        alpha: f32,
        beta: f32,
        smoothing: f32,
    ) -> Result<Self> {
        // Validate shapes match
        if predictions.shape() != targets.shape() {
            return Err(BarracudaError::shape_mismatch(
                predictions.shape().to_vec(),
                targets.shape().to_vec(),
            ));
        }

        // Validate hyperparameters
        if !(0.0..=1.0).contains(&alpha) {
            return Err(BarracudaError::invalid_op(
                "TverskyLoss",
                format!("alpha must be in [0, 1], got {}", alpha),
            ));
        }
        if !(0.0..=1.0).contains(&beta) {
            return Err(BarracudaError::invalid_op(
                "TverskyLoss",
                format!("beta must be in [0, 1], got {}", beta),
            ));
        }
        if smoothing < 0.0 {
            return Err(BarracudaError::invalid_op(
                "TverskyLoss",
                format!("smoothing must be non-negative, got {}", smoothing),
            ));
        }

        Ok(Self {
            predictions,
            targets,
            alpha,
            beta,
            smoothing,
        })
    }

    /// WGSL shader source
    fn shader() -> &'static str {
        include_str!("../shaders/loss/tversky_loss.wgsl")
    }

    /// Execute Tversky loss (GPU reduction)
    ///
    /// **Deep Debt**: Efficient workgroup reduction for large segmentations
    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();

        // Determine batch and elements
        let total_size = self.predictions.len();
        let batch_size = if !self.predictions.shape().is_empty() {
            self.predictions.shape()[0]
        } else {
            1
        };
        let elements_per_sample = total_size / batch_size;

        // Create parameters
        let params = TverskyParams {
            alpha: self.alpha,
            beta: self.beta,
            smoothing: self.smoothing,
            batch_size: batch_size as u32,
            elements_per_sample: elements_per_sample as u32,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Tversky Loss Params"),
            size: std::mem::size_of::<TverskyParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Output buffer (one loss value per batch)
        let output_buffer = device.create_buffer_f32(batch_size)?;

        // Compile shader
        let shader = device.compile_shader(Self::shader(), Some("Tversky Loss"));

        // Create bind group layout
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Tversky Loss BGL"),
                entries: &[
                    // Predictions
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Targets
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Output
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    // Params
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tversky Loss BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.predictions.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.targets.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Tversky Loss Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Tversky Loss Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Tversky Loss Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Tversky Loss Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch using standard 1D shader workgroup size (256)
            let caps = DeviceCapabilities::from_device(device);
            let workgroups = caps.dispatch_1d(batch_size as u32);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        let output_data = crate::utils::read_buffer(device, &output_buffer, batch_size)?;
        Ok(Tensor::new(output_data, vec![batch_size], device.clone()))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Tversky loss for segmentation (generalized Dice)
    ///
    /// **Deep Debt**: Essential for imbalanced segmentation, control FP/FN trade-off
    ///
    /// # Arguments
    /// - `targets`: Ground truth [same shape as predictions]
    /// - `alpha`: Weight for false positives (0.0-1.0, typically 0.3-0.7)
    /// - `beta`: Weight for false negatives (0.0-1.0, typically 0.3-0.7)
    /// - `smoothing`: Smoothing factor (typically 1.0)
    ///
    /// # Returns
    /// - Loss tensor [batch_size] (one value per batch)
    ///
    /// # Example
    /// ```rust,ignore
    /// // Penalize false negatives more (medical imaging)
    /// let loss = preds.tversky_loss(&targets, 0.3, 0.7, 1.0)?;
    ///
    /// // Equivalent to Dice Loss
    /// let loss = preds.tversky_loss(&targets, 0.5, 0.5, 1.0)?;
    /// ```
    ///
    /// # Note
    /// - `alpha < beta`: Recall-focused (minimize false negatives)
    /// - `alpha > beta`: Precision-focused (minimize false positives)
    /// - `alpha = beta = 0.5`: Equivalent to Dice Loss
    pub fn tversky_loss(
        self,
        targets: &Self,
        alpha: f32,
        beta: f32,
        smoothing: f32,
    ) -> Result<Self> {
        TverskyLoss::new(self, targets.clone(), alpha, beta, smoothing)?.execute()
    }
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_tversky_loss_gpu_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let batch = 2;
        let h = 16;
        let w = 16;

        // Perfect predictions
        let preds =
            Tensor::from_vec_on(vec![1.0; batch * h * w], vec![batch, h, w], device.clone())
                .await
                .unwrap();

        let targets = Tensor::from_vec_on(vec![1.0; batch * h * w], vec![batch, h, w], device)
            .await
            .unwrap();

        let loss = preds.tversky_loss(&targets, 0.5, 0.5, 1.0).unwrap();

        assert_eq!(loss.shape(), &[batch]);
        let data = loss.to_vec().unwrap();
        // Perfect prediction should have low loss
        assert!(data.iter().all(|&x| x < 0.1));
    }

    #[tokio::test]
    async fn test_tversky_loss_gpu_equivalent_to_dice() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let batch = 1;
        let size = 100;

        let preds = Tensor::from_vec_on(vec![0.8; batch * size], vec![batch, size], device.clone())
            .await
            .unwrap();

        let targets = Tensor::from_vec_on(vec![1.0; batch * size], vec![batch, size], device)
            .await
            .unwrap();

        // Tversky with alpha=beta=0.5 should be equivalent to Dice
        let tversky_loss = preds.clone().tversky_loss(&targets, 0.5, 0.5, 1.0).unwrap();
        let dice_loss = preds.dice_loss(&targets, 1.0).unwrap();

        let tversky_data = tversky_loss.to_vec().unwrap();
        let dice_data = dice_loss.to_vec().unwrap();

        // Should be approximately equal (different GPU computations may vary in float precision)
        let diff = (tversky_data[0] - dice_data[0]).abs();
        assert!(
            diff < 0.01,
            "Tversky (alpha=beta=0.5) should be close to Dice, got diff={}",
            diff
        );
    }

    #[tokio::test]
    async fn test_tversky_loss_gpu_recall_focused() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let batch = 1;
        let size = 100;

        let preds = Tensor::from_vec_on(vec![0.5; batch * size], vec![batch, size], device.clone())
            .await
            .unwrap();

        let targets = Tensor::from_vec_on(vec![1.0; batch * size], vec![batch, size], device)
            .await
            .unwrap();

        // alpha < beta: Penalize false negatives more (recall-focused)
        let recall_loss = preds.clone().tversky_loss(&targets, 0.3, 0.7, 1.0).unwrap();

        // alpha > beta: Penalize false positives more (precision-focused)
        let precision_loss = preds.tversky_loss(&targets, 0.7, 0.3, 1.0).unwrap();

        let recall_data = recall_loss.to_vec().unwrap();
        let precision_data = precision_loss.to_vec().unwrap();

        // Recall-focused should have higher loss (more penalty on FN)
        assert!(recall_data[0] > precision_data[0]);
    }

    #[tokio::test]
    async fn test_tversky_loss_gpu_mismatch() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let batch = 1;
        let size = 100;

        // Complete mismatch
        let preds = Tensor::from_vec_on(vec![1.0; batch * size], vec![batch, size], device.clone())
            .await
            .unwrap();

        let targets = Tensor::from_vec_on(vec![0.0; batch * size], vec![batch, size], device)
            .await
            .unwrap();

        let loss = preds.tversky_loss(&targets, 0.5, 0.5, 1.0).unwrap();
        let data = loss.to_vec().unwrap();

        // High loss for mismatch
        assert!(data[0] > 0.5);
    }

    #[tokio::test]
    async fn test_tversky_loss_gpu_medical_scale() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Medical imaging scale: 128x128 slices
        let batch = 4;
        let h = 128;
        let w = 128;

        let preds =
            Tensor::from_vec_on(vec![0.8; batch * h * w], vec![batch, h, w], device.clone())
                .await
                .unwrap();

        let targets = Tensor::from_vec_on(vec![1.0; batch * h * w], vec![batch, h, w], device)
            .await
            .unwrap();

        // Medical imaging: Penalize false negatives more
        let loss = preds.tversky_loss(&targets, 0.3, 0.7, 1.0).unwrap();

        assert_eq!(loss.shape(), &[batch]);
        let data = loss.to_vec().unwrap();
        assert!(data
            .iter()
            .all(|&x| x.is_finite() && (0.0..=1.0).contains(&x)));
    }

    #[tokio::test]
    async fn test_tversky_loss_gpu_validation() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let preds = Tensor::from_vec_on(vec![1.0; 100], vec![100], device.clone())
            .await
            .unwrap();

        let targets = Tensor::from_vec_on(vec![1.0; 100], vec![100], device)
            .await
            .unwrap();

        // Invalid alpha (> 1.0) should error
        assert!(preds.clone().tversky_loss(&targets, 1.5, 0.5, 1.0).is_err());

        // Invalid beta (< 0.0) should error
        assert!(preds
            .clone()
            .tversky_loss(&targets, 0.5, -0.1, 1.0)
            .is_err());

        // Invalid smoothing (< 0.0) should error
        assert!(preds.tversky_loss(&targets, 0.5, 0.5, -1.0).is_err());
    }
}
