//! BCE Loss - GPU-accelerated Binary Cross Entropy loss
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (uses existing shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for binary classification)
//!
//! ## Algorithm
//!
//! ```text
//! BCE(p, t) = -[t * log(p) + (1 - t) * log(1 - p)]
//! where p = predicted probability, t = target (0 or 1)
//! ```
//!
//! **Key Properties**:
//! - Standard loss for binary classification
//! - Works with sigmoid outputs (probabilities)
//! - Convex and smooth (easy optimization)
//! - Foundation for cross-entropy variants
//!
//! **Used By**: Logistic regression, binary classification, GANs
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let predictions = Tensor::randn(vec![1000]).await?;  // Probabilities [0, 1]
//! let targets = Tensor::randn(vec![1000]).await?;      // Binary labels {0, 1}
//!
//! let loss = predictions.bce_loss(&targets)?;
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct BCELossParams {
    epsilon: f32,        // Numerical stability
    reduction_mode: u32, // 0=mean, 1=sum, 2=none
    size: u32,
    _padding: u32,
}

pub struct BCELoss {
    predictions: Tensor,
    targets: Tensor,
}

impl BCELoss {
    pub fn new(predictions: Tensor, targets: Tensor) -> Result<Self> {
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
        })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/loss/bce_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();
        let size = self.predictions.shape().iter().product::<usize>();

        let params = BCELossParams {
            epsilon: 1e-7,
            reduction_mode: 0, // mean (not used in per-element version)
            size: size as u32,
            _padding: 0,
        };

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bce_loss_output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("bce_loss_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("bce_loss_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bce_loss_bind_group_layout"),
                    entries: &[
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("bce_loss_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("bce_loss_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bce_loss_bind_group"),
            layout: &bind_group_layout,
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

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("bce_loss_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("bce_loss_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
            let workgroups = (size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            self.predictions.shape().to_vec(),
            device.clone(),
        ))
    }
}

// ═══════════════════════════════════════════════════════════════
// TENSOR API INTEGRATION
// ═══════════════════════════════════════════════════════════════

impl Tensor {
    /// Binary Cross Entropy loss for binary classification
    ///
    /// **Deep Debt**: Foundation for binary classification and GANs
    ///
    /// # Arguments
    /// - `targets`: Ground truth binary labels [same shape, values in {0, 1}]
    ///
    /// # Returns
    /// - Loss tensor [same shape as input]
    ///
    /// # Example
    /// ```rust,ignore
    /// // Standard binary classification
    /// let loss = predictions.bce_loss(&targets)?;
    ///
    /// // With sigmoid activation
    /// let probs = logits.sigmoid()?;
    /// let loss = probs.bce_loss(&targets)?;
    /// ```
    ///
    /// # Note
    /// - Predictions should be probabilities in [0, 1] (use sigmoid first!)
    /// - Targets should be binary {0, 1}
    /// - Numerically stable with epsilon=1e-7
    /// - Standard for logistic regression and binary GANs
    pub fn bce_loss(self, targets: &Self) -> Result<Self> {
        BCELoss::new(self, targets.clone())?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_bce_loss_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Perfect predictions (p=1 for t=1, p=0 for t=0)
        let predictions = Tensor::from_vec_on(vec![0.9, 0.1, 0.9, 0.1], vec![4], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0, 0.0, 1.0, 0.0], vec![4], device.clone())
            .await
            .unwrap();

        let loss = predictions.bce_loss(&targets).unwrap();
        let data = loss.to_vec().unwrap();

        // Loss should be small for good predictions
        assert!(data.iter().all(|&x| x >= 0.0 && x.is_finite()));
        assert!(data.iter().all(|&x| x < 1.0)); // All losses should be < 1.0
    }

    #[tokio::test]
    async fn test_bce_loss_validation() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Shape mismatch should fail
        let predictions = Tensor::from_vec_on(vec![0.5; 10], vec![10], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0; 5], vec![5], device.clone())
            .await
            .unwrap();

        assert!(predictions.bce_loss(&targets).is_err());
    }

    #[tokio::test]
    async fn test_bce_loss_perfect_prediction() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Nearly perfect predictions
        let predictions = Tensor::from_vec_on(vec![0.99, 0.01], vec![2], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0, 0.0], vec![2], device.clone())
            .await
            .unwrap();

        let loss = predictions.bce_loss(&targets).unwrap();
        let data = loss.to_vec().unwrap();

        // Very small loss for near-perfect predictions
        assert!(data.iter().all(|&x| x < 0.1));
    }

    #[tokio::test]
    async fn test_bce_loss_worst_prediction() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Worst predictions (completely wrong)
        let predictions = Tensor::from_vec_on(vec![0.01, 0.99], vec![2], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0, 0.0], vec![2], device.clone())
            .await
            .unwrap();

        let loss = predictions.bce_loss(&targets).unwrap();
        let data = loss.to_vec().unwrap();

        // Large loss for bad predictions
        assert!(data.iter().all(|&x| x > 1.0));
    }

    #[tokio::test]
    async fn test_bce_loss_large_batch() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Large batch
        let predictions: Vec<f32> = (0..1000)
            .map(|i| if i % 2 == 0 { 0.8 } else { 0.2 })
            .collect();
        let targets: Vec<f32> = (0..1000)
            .map(|i| if i % 2 == 0 { 1.0 } else { 0.0 })
            .collect();

        let pred_tensor = Tensor::from_vec_on(predictions, vec![1000], device.clone())
            .await
            .unwrap();
        let target_tensor = Tensor::from_vec_on(targets, vec![1000], device.clone())
            .await
            .unwrap();

        let loss = pred_tensor.bce_loss(&target_tensor).unwrap();
        let data = loss.to_vec().unwrap();

        assert_eq!(data.len(), 1000);
        assert!(data.iter().all(|&x| x.is_finite() && x >= 0.0));
    }
}
