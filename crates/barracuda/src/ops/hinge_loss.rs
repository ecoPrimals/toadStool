//! Hinge Loss - GPU-accelerated SVM-style classification loss
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (new shader!)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready for SVMs)
//!
//! ## Algorithm
//!
//! ```text
//! HingeLoss = max(0, margin - y * pred)
//! where y ∈ {-1, +1} is true label, pred is prediction
//! ```
//!
//! **Parameters**:
//! - `margin`: Typically 1.0 for standard SVM hinge loss
//!
//! **Key Properties**:
//! - Zero loss when prediction is correct and confident (y*pred > margin)
//! - Linear penalty for incorrect predictions
//! - Encourages max-margin separation
//! - Standard for support vector machines
//!
//! **Used By**: SVMs, max-margin classifiers, ranking
//!
//! ## Usage
//!
//! ```rust,ignore
//! use barracuda::tensor::Tensor;
//!
//! let predictions = Tensor::randn(vec![1000]).await?;  // Raw scores
//! let targets = Tensor::randn(vec![1000]).await?;      // Labels {-1, +1}
//!
//! let loss = predictions.hinge_loss(&targets, 1.0)?;  // margin=1.0
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct HingeLossParams {
    size: u32,
    margin: f32,
    _padding: [u32; 2],
}

pub struct HingeLoss {
    predictions: Tensor,
    targets: Tensor,
    margin: f32,
}

impl HingeLoss {
    pub fn new(predictions: Tensor, targets: Tensor, margin: f32) -> Result<Self> {
        // Validate shapes match
        if predictions.shape() != targets.shape() {
            return Err(BarracudaError::shape_mismatch(
                predictions.shape().to_vec(),
                targets.shape().to_vec(),
            ));
        }

        // Validate margin is positive
        if margin <= 0.0 {
            return Err(BarracudaError::invalid_op(
                "HingeLoss",
                format!("margin must be positive, got {}", margin),
            ));
        }

        Ok(Self {
            predictions,
            targets,
            margin,
        })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/loss/hinge_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();
        let size = self.predictions.shape().iter().product::<usize>();

        let params = HingeLossParams {
            size: size as u32,
            margin: self.margin,
            _padding: [0; 2],
        };

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("hinge_loss_output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("hinge_loss_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("hinge_loss_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("hinge_loss_bind_group_layout"),
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
                    label: Some("hinge_loss_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("hinge_loss_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("hinge_loss_bind_group"),
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
                label: Some("hinge_loss_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("hinge_loss_pass"),
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
    /// Hinge loss for SVM-style max-margin classification
    ///
    /// **Deep Debt**: Essential for support vector machines
    ///
    /// # Arguments
    /// - `targets`: Ground truth labels [same shape, values in {-1, +1}]
    /// - `margin`: Margin threshold (typically 1.0)
    ///
    /// # Returns
    /// - Loss tensor [same shape as input]
    ///
    /// # Example
    /// ```rust,ignore
    /// // Standard SVM hinge loss
    /// let loss = predictions.hinge_loss(&targets, 1.0)?;
    ///
    /// // Multi-class SVM (one-vs-all)
    /// let loss = scores.hinge_loss(&labels, 1.0)?;
    /// ```
    ///
    /// # Note
    /// - Targets should be {-1, +1} (not {0, 1})
    /// - Predictions are raw scores (not probabilities)
    /// - Loss = 0 when y*pred > margin (correct and confident)
    /// - Standard margin = 1.0 for SVMs
    pub fn hinge_loss(self, targets: &Self, margin: f32) -> Result<Self> {
        HingeLoss::new(self, targets.clone(), margin)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_hinge_loss_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Good predictions (correct sign, high magnitude)
        let predictions = Tensor::from_vec_on(vec![2.0, -2.0, 1.5], vec![3], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0, -1.0, 1.0], vec![3], device.clone())
            .await
            .unwrap();

        let loss = predictions.hinge_loss(&targets, 1.0).unwrap();
        let data = loss.to_vec().unwrap();

        // All should have zero or near-zero loss
        assert!(data.iter().all(|&x| x < 0.1));
    }

    #[tokio::test]
    async fn test_hinge_loss_wrong_predictions() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Wrong predictions (opposite sign)
        let predictions = Tensor::from_vec_on(vec![-1.0, 1.0], vec![2], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0, -1.0], vec![2], device.clone())
            .await
            .unwrap();

        let loss = predictions.hinge_loss(&targets, 1.0).unwrap();
        let data = loss.to_vec().unwrap();

        // Should have significant loss
        assert!(data.iter().all(|&x| x > 1.0));
    }

    #[tokio::test]
    async fn test_hinge_loss_exact_margin() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Prediction exactly at margin
        let predictions = Tensor::from_vec_on(vec![0.5], vec![1], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0], vec![1], device.clone())
            .await
            .unwrap();

        let loss = predictions.hinge_loss(&targets, 1.0).unwrap();
        let data = loss.to_vec().unwrap();

        // Loss = max(0, 1 - 1*0.5) = 0.5
        assert!(
            (data[0] - 0.5).abs() < 1e-5,
            "Expected 0.5, got {}",
            data[0]
        );
    }

    #[tokio::test]
    async fn test_hinge_loss_validation() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Shape mismatch
        let predictions = Tensor::from_vec_on(vec![1.0; 10], vec![10], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0; 5], vec![5], device.clone())
            .await
            .unwrap();

        assert!(predictions.hinge_loss(&targets, 1.0).is_err());

        // Negative margin
        let predictions = Tensor::from_vec_on(vec![1.0; 5], vec![5], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0; 5], vec![5], device.clone())
            .await
            .unwrap();

        assert!(predictions.hinge_loss(&targets, -1.0).is_err());
    }

    #[tokio::test]
    async fn test_hinge_loss_large_batch() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let predictions: Vec<f32> = (0..1000)
            .map(|i| if i % 2 == 0 { 2.0 } else { -2.0 })
            .collect();
        let targets: Vec<f32> = (0..1000)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();

        let pred_tensor = Tensor::from_vec_on(predictions, vec![1000], device.clone())
            .await
            .unwrap();
        let target_tensor = Tensor::from_vec_on(targets, vec![1000], device.clone())
            .await
            .unwrap();

        let loss = pred_tensor.hinge_loss(&target_tensor, 1.0).unwrap();
        let data = loss.to_vec().unwrap();

        assert_eq!(data.len(), 1000);
        assert!(data.iter().all(|&x| x.is_finite() && x >= 0.0));
        // Good predictions should have near-zero loss
        assert!(data.iter().all(|&x| x < 0.1));
    }
}
