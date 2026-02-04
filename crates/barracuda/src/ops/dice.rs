//! Dice Loss - GPU-accelerated segmentation loss
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (uses existing shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for medical imaging)
//!
//! ## Algorithm
//!
//! ```text
//! Dice coefficient: DC = 2|X ∩ Y| / (|X| + |Y|)
//! Dice loss: L = 1 - DC
//! ```
//!
//! **Implementation**: GPU reduction with workgroup shared memory
//!
//! **Key Properties**:
//! - Handles class imbalance naturally
//! - Directly optimizes IoU-like metric
//! - Common in medical image segmentation
//!
//! **Used By**: U-Net, V-Net, medical imaging models
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let predictions = Tensor::randn(vec![4, 256, 256]).await?;  // [batch, H, W]
//! let targets = Tensor::randn(vec![4, 256, 256]).await?;
//!
//! let loss = predictions.dice_loss(&targets, 1.0)?;  // smoothing = 1.0
//! ```

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Dice loss parameters for WGSL shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct DiceParams {
    smoothing: f32,
    reduction_mode: u32,
    batch_size: u32,
    elements_per_sample: u32,
}

/// Dice Loss operation
///
/// **Deep Debt**: Uses existing WGSL shader with workgroup reduction
pub struct DiceLoss {
    predictions: Tensor,
    targets: Tensor,
    smoothing: f32,
}

impl DiceLoss {
    /// Create new Dice loss operation
    pub fn new(predictions: Tensor, targets: Tensor, smoothing: f32) -> Result<Self> {
        // Validate shapes match
        if predictions.shape() != targets.shape() {
            return Err(BarracudaError::shape_mismatch(
                predictions.shape().to_vec(),
                targets.shape().to_vec(),
            ));
        }

        Ok(Self {
            predictions,
            targets,
            smoothing,
        })
    }

    /// WGSL shader source
    fn shader() -> &'static str {
        include_str!("../shaders/dice_loss.wgsl")
    }

    /// Execute Dice loss (GPU reduction)
    ///
    /// **Deep Debt**: Efficient workgroup reduction for large segmentations
    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();

        // Determine batch and elements
        let total_size = self.predictions.len();
        let batch_size = if self.predictions.shape().len() > 0 {
            self.predictions.shape()[0]
        } else {
            1
        };
        let elements_per_sample = total_size / batch_size;

        // Create parameters
        let params = DiceParams {
            smoothing: self.smoothing,
            reduction_mode: 0, // Mean reduction
            batch_size: batch_size as u32,
            elements_per_sample: elements_per_sample as u32,
        };

        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dice Loss Params"),
            size: std::mem::size_of::<DiceParams>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device
            .queue
            .write_buffer(&params_buffer, 0, bytemuck::bytes_of(&params));

        // Output buffer (one loss value per batch)
        let output_buffer = device.create_buffer_f32(batch_size)?;

        // Compile shader
        let shader = device.compile_shader(Self::shader(), Some("Dice Loss"));

        // Create bind group layout
        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Dice Loss BGL"),
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
            label: Some("Dice Loss BG"),
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
                    label: Some("Dice Loss Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Dice Loss Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Dice Loss Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Dice Loss Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // One workgroup per batch sample
            pass.dispatch_workgroups(batch_size as u32, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Return output tensor [batch_size]
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size],
            device.clone(),
        ))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Dice loss for segmentation
    ///
    /// **Deep Debt**: Essential for medical imaging, handles class imbalance
    ///
    /// # Arguments
    /// - `targets`: Ground truth [same shape as predictions]
    /// - `smoothing`: Smoothing factor (typically 1.0)
    ///
    /// # Returns
    /// - Loss tensor [batch_size] (one value per batch)
    ///
    /// # Example
    /// ```rust,ignore
    /// let preds = Tensor::randn(vec![4, 256, 256]).await?;
    /// let targets = Tensor::randn(vec![4, 256, 256]).await?;
    /// let loss = preds.dice_loss(&targets, 1.0)?;  // U-Net segmentation
    /// ```
    pub fn dice_loss(self, targets: &Self, smoothing: f32) -> Result<Self> {
        DiceLoss::new(self, targets.clone(), smoothing)?.execute()
    }
}

// ═══════════════════════════════════════════════════════════════
// TESTS
// ═══════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_dice_loss_gpu_basic() {
        let device = get_test_device().await;

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

        let loss = preds.dice_loss(&targets, 1.0).unwrap();

        assert_eq!(loss.shape(), &[batch]);
        let data = loss.to_vec().unwrap();
        // Perfect prediction should have low loss
        assert!(data.iter().all(|&x| x < 0.1));
    }

    #[tokio::test]
    async fn test_dice_loss_gpu_mismatch() {
        let device = get_test_device().await;

        let batch = 1;
        let size = 100;

        // Complete mismatch
        let preds = Tensor::from_vec_on(vec![1.0; batch * size], vec![batch, size], device.clone())
            .await
            .unwrap();

        let targets = Tensor::from_vec_on(vec![0.0; batch * size], vec![batch, size], device)
            .await
            .unwrap();

        let loss = preds.dice_loss(&targets, 1.0).unwrap();
        let data = loss.to_vec().unwrap();

        // High loss for mismatch
        assert!(data[0] > 0.5);
    }

    #[tokio::test]
    async fn test_dice_loss_gpu_medical_scale() {
        let device = get_test_device().await;

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

        let loss = preds.dice_loss(&targets, 1.0).unwrap();

        assert_eq!(loss.shape(), &[batch]);
        let data = loss.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite() && x >= 0.0 && x <= 1.0));
    }
}
