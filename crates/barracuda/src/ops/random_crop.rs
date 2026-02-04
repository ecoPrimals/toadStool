//! Random crop augmentation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Randomly crops images to specified size

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RandomCropParams {
    batch_size: u32,
    channels: u32,
    in_height: u32,
    in_width: u32,
    out_height: u32,
    out_width: u32,
    _padding: [u32; 2],
}

pub struct RandomCrop {
    input: Tensor,
    crop_positions: Tensor,
    out_height: usize,
    out_width: usize,
}

impl RandomCrop {
    /// Create RandomCrop operation
    pub fn new(input: Tensor, crop_positions: Tensor, out_height: usize, out_width: usize) -> Result<Self> {
        // Validate crop_positions shape: [batch_size, 2] (top, left)
        let crop_shape = crop_positions.shape();
        if crop_shape.len() != 2 || crop_shape[1] != 2 {
            return Err(BarracudaError::invalid_op(
                "RandomCrop",
                format!("crop_positions must be 2D [batch_size, 2], got shape {:?}", crop_shape),
            ));
        }

        Ok(Self {
            input,
            crop_positions,
            out_height,
            out_width,
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/random_crop.wgsl")
    }

    /// Execute RandomCrop on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let input_shape = self.input.shape();
        
        if input_shape.len() != 4 {
            return Err(BarracudaError::invalid_op(
                "RandomCrop",
                format!("input must be 4D [batch, channels, height, width], got shape {:?}", input_shape),
            ));
        }

        let batch_size = input_shape[0];
        let channels = input_shape[1];
        let in_height = input_shape[2];
        let in_width = input_shape[3];

        if self.crop_positions.shape()[0] != batch_size {
            return Err(BarracudaError::invalid_op(
                "RandomCrop",
                format!("crop_positions batch size {} must match input batch size {}", self.crop_positions.shape()[0], batch_size),
            ));
        }

        // Create output buffer: [batch, channels, out_height, out_width]
        let output_size = batch_size * channels * self.out_height * self.out_width;
        let output_buffer = device.create_buffer_f32(output_size)?;

        let params = RandomCropParams {
            batch_size: batch_size as u32,
            channels: channels as u32,
            in_height: in_height as u32,
            in_width: in_width as u32,
            out_height: self.out_height as u32,
            out_width: self.out_width as u32,
            _padding: [0; 2],
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("RandomCrop Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("RandomCrop Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RandomCrop Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.crop_positions.buffer().as_entire_binding(),
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

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("RandomCrop"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("RandomCrop Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("RandomCrop Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("RandomCrop Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RandomCrop Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (8x8x1 threads per workgroup)
            let workgroups_x = (self.out_width as u32 + 7) / 8;
            let workgroups_y = (self.out_height as u32 + 7) / 8;
            let workgroups_z = (batch_size * channels) as u32;
            pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, channels, self.out_height, self.out_width],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_random_crop_basic() {
        let device = get_test_device().await;

        let batch_size = 2;
        let channels = 3;
        let in_height = 32;
        let in_width = 32;
        let out_height = 16;
        let out_width = 16;

        let input = Tensor::from_vec_on(
            vec![1.0; batch_size * channels * in_height * in_width],
            vec![batch_size, channels, in_height, in_width],
            device.clone(),
        )
        .await
        .unwrap();

        let crop_positions = Tensor::from_vec_on(
            vec![5u32, 5, 10, 10], // [batch, 2] - (top, left)
            vec![batch_size, 2],
            device.clone(),
        )
        .await
        .unwrap();

        let result = RandomCrop::new(input, crop_positions, out_height, out_width)
            .unwrap()
            .execute()
            .unwrap();

        assert_eq!(result.shape(), &[batch_size, channels, out_height, out_width]);
    }
}
