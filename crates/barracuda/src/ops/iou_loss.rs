//! IoULoss - Intersection over Union loss
//!
//! **Canonical BarraCuda Pattern**: Struct with new/execute
//!
//! Direct optimization of IoU metric.
//! Used in segmentation and object detection.

use crate::device::DeviceCapabilities;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// IoU Loss operation
pub struct IoULoss {
    predictions: Tensor,
    targets: Tensor,
    smooth: f32,
}

impl IoULoss {
    /// Create a new IoU loss operation
    pub fn new(predictions: Tensor, targets: Tensor, smooth: f32) -> Result<Self> {
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
            smooth,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/loss/iou_loss.wgsl")
    }

    /// Execute the IoU loss operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();
        let size = self.predictions.len();

        // Number of workgroups for pass 1 (workgroup_size 256)
        let num_workgroups = (size as u32).div_ceil(crate::device::capabilities::WORKGROUP_SIZE_1D);

        // Create reduction buffers - one slot per workgroup for partial sums
        let intersection_buffer = device.create_buffer_f32(num_workgroups as usize)?;
        let union_buffer = device.create_buffer_f32(num_workgroups as usize)?;
        let output_buffer = device.create_buffer_f32(1)?;

        // Zero-initialize partial sum buffers
        device.write_buffer_f32(&intersection_buffer, &vec![0.0; num_workgroups as usize])?;
        device.write_buffer_f32(&union_buffer, &vec![0.0; num_workgroups as usize])?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            smooth_val: f32,
            num_partials: u32,
            _pad1: u32,
        }

        let params = Params {
            size: size as u32,
            smooth_val: self.smooth,
            num_partials: num_workgroups,
            _pad1: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("IoU Loss Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("IoU Loss Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("IoU Loss Bind Group Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
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
            label: Some("IoU Loss Bind Group"),
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
                    resource: intersection_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: union_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipelines
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("IoU Loss Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline_pass1 =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("IoU Loss Pipeline Pass1"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let compute_pipeline_pass2 =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("IoU Loss Pipeline Pass2"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "compute_loss",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Execute compute shader (two passes)
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("IoU Loss Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("IoU Loss Pass"),
                timestamp_writes: None,
            });

            // Pass 1: Compute intersection and union
            compute_pass.set_pipeline(&compute_pipeline_pass1);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Dispatch using standard 1D shader workgroup size (256)
            let caps = DeviceCapabilities::from_device(device);
            let workgroups = caps.dispatch_1d(size as u32);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);

            // Pass 2: Sum partial results and compute final loss (1 workgroup)
            compute_pass.set_pipeline(&compute_pipeline_pass2);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        let output_data = crate::utils::read_buffer(device, &output_buffer, 1)?;
        Ok(Tensor::new(output_data, vec![1], device.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_iou_loss() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let predictions = Tensor::from_vec_on(vec![0.8; 500], vec![500], device.clone())
            .await
            .unwrap();
        let targets = Tensor::from_vec_on(vec![1.0; 500], vec![500], device.clone())
            .await
            .unwrap();
        let loss = IoULoss::new(predictions, targets, 1e-6)
            .unwrap()
            .execute()
            .unwrap();
        let result = loss.to_vec().unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0] > 0.0 && result[0] < 1.0);
    }
}
