//! Focal Loss with class weighting
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Extension of focal loss with per-class weights (alpha)

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FocalLossAlphaParams {
    batch_size: u32,
    num_classes: u32,
    gamma: f32,
    _padding: u32,
}

pub struct FocalLossAlpha {
    predictions: Tensor,
    targets: Tensor,
    alpha: Tensor,
    gamma: f32,
}

impl FocalLossAlpha {
    /// Create FocalLossAlpha operation
    pub fn new(predictions: Tensor, targets: Tensor, alpha: Tensor, gamma: f32) -> Result<Self> {
        if gamma < 0.0 {
            return Err(BarracudaError::invalid_op(
                "FocalLossAlpha",
                format!("gamma must be non-negative, got {}", gamma),
            ));
        }

        Ok(Self {
            predictions,
            targets,
            alpha,
            gamma,
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/loss/focal_loss_alpha.wgsl")
    }

    /// Execute FocalLossAlpha on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();
        let pred_shape = self.predictions.shape();
        let target_shape = self.targets.shape();

        if pred_shape.len() != 2 {
            return Err(BarracudaError::invalid_op(
                "FocalLossAlpha",
                format!(
                    "predictions must be 2D [batch, num_classes], got shape {:?}",
                    pred_shape
                ),
            ));
        }

        if target_shape.len() != 1 {
            return Err(BarracudaError::invalid_op(
                "FocalLossAlpha",
                format!("targets must be 1D [batch], got shape {:?}", target_shape),
            ));
        }

        let batch_size = pred_shape[0];
        let num_classes = pred_shape[1];

        if target_shape[0] != batch_size {
            return Err(BarracudaError::invalid_op(
                "FocalLossAlpha",
                format!(
                    "targets batch size {} must match predictions batch size {}",
                    target_shape[0], batch_size
                ),
            ));
        }

        if self.alpha.shape() != [num_classes] {
            return Err(BarracudaError::invalid_op(
                "FocalLossAlpha",
                format!(
                    "alpha must be 1D [num_classes], got shape {:?}",
                    self.alpha.shape()
                ),
            ));
        }

        // Create output buffer: [batch] - per-sample loss
        let output_buffer = device.create_buffer_f32(batch_size)?;

        let params = FocalLossAlphaParams {
            batch_size: batch_size as u32,
            num_classes: num_classes as u32,
            gamma: self.gamma,
            _padding: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("FocalLossAlpha Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("FocalLossAlpha Bind Group Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
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
            label: Some("FocalLossAlpha Bind Group"),
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
                    resource: self.alpha.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("FocalLossAlpha"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("FocalLossAlpha Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("FocalLossAlpha Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("FocalLossAlpha Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("FocalLossAlpha Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
            let workgroups = (batch_size as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_focal_loss_alpha_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let batch_size = 4;
        let num_classes = 3;

        let predictions = Tensor::from_vec_on(
            vec![0.33; batch_size * num_classes],
            vec![batch_size, num_classes],
            device.clone(),
        )
        .await
        .unwrap();

        let targets =
            Tensor::from_vec_on(vec![0.0, 1.0, 2.0, 0.0], vec![batch_size], device.clone())
                .await
                .unwrap();

        let alpha = Tensor::from_vec_on(vec![0.25, 0.25, 0.5], vec![num_classes], device.clone())
            .await
            .unwrap();

        let result = FocalLossAlpha::new(predictions, targets, alpha, 2.0)
            .unwrap()
            .execute()
            .unwrap();

        assert_eq!(result.shape(), &[batch_size]);
    }
}
