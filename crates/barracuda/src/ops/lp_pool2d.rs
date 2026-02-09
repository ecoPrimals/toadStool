//! Lp Pool 2D - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Lp Pool 2D operation
pub struct LpPool2D {
    input: Tensor,
    kernel_size: usize,
    stride: usize,
    padding: usize,
    p: f32,
}

impl LpPool2D {
    /// Create a new Lp Pool 2D operation
    pub fn new(
        input: Tensor,
        kernel_size: usize,
        stride: usize,
        padding: usize,
        p: f32,
    ) -> Result<Self> {
        let shape = input.shape();
        if shape.len() != 4 {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!(
                    "LpPool2D expects 4D tensor [B, C, H, W], got shape {:?}",
                    shape
                ),
            });
        }

        if p <= 0.0 {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: "p must be positive".to_string(),
            });
        }

        Ok(Self {
            input,
            kernel_size,
            stride,
            padding,
            p,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/lp_pool2d.wgsl")
    }

    /// Execute the Lp Pool 2D operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let batch_size = shape[0];
        let channels = shape[1];
        let in_height = shape[2];
        let in_width = shape[3];

        // Compute output dimensions
        let out_height = ((in_height + 2 * self.padding - self.kernel_size) / self.stride) + 1;
        let out_width = ((in_width + 2 * self.padding - self.kernel_size) / self.stride) + 1;
        let output_size = batch_size * channels * out_height * out_width;

        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            batch_size: u32,
            channels: u32,
            in_height: u32,
            in_width: u32,
            out_height: u32,
            out_width: u32,
            kernel_size: u32,
            stride: u32,
            padding: u32,
            p: f32,
            _pad1: u32,
            _pad2: u32,
        }

        let params = Params {
            batch_size: batch_size as u32,
            channels: channels as u32,
            in_height: in_height as u32,
            in_width: in_width as u32,
            out_height: out_height as u32,
            out_width: out_width as u32,
            kernel_size: self.kernel_size as u32,
            stride: self.stride as u32,
            padding: self.padding as u32,
            p: self.p,
            _pad1: 0,
            _pad2: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LpPool2D Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("LpPool2D Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("LpPool2D Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
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
                    ],
                });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LpPool2D Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("LpPool2D Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("LpPool2D Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

        // Execute compute shader
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LpPool2D Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LpPool2D Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Convolution);
            let workgroups_x = (out_width as u32).div_ceil(optimal_wg_size);
            let workgroups_y = (out_height as u32).div_ceil(optimal_wg_size);
            let workgroups_z = (batch_size * channels) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        device.queue.submit(Some(encoder.finish()));

        // Return tensor without reading back (zero-copy)
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, channels, out_height, out_width],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_lp_pool2d_basic() {
        let device = get_test_device().await;
        let data: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![1, 1, 8, 8], device.clone()).unwrap();

        let pooled = LpPool2D::new(input, 2, 2, 0, 2.0)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(pooled.shape(), &vec![1, 1, 4, 4]);
    }

    #[tokio::test]
    async fn test_lp_pool2d_l1() {
        let device = get_test_device().await;
        let data: Vec<f32> = (0..32).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![1, 1, 4, 8], device.clone()).unwrap();

        let pooled = LpPool2D::new(input, 2, 1, 0, 1.0)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(pooled.shape().len(), 4);
    }

    #[tokio::test]
    async fn test_lp_pool2d_invalid_shape() {
        let device = get_test_device().await;
        let input = Tensor::from_data(&[1.0, 2.0, 3.0], vec![3], device.clone()).unwrap();

        assert!(LpPool2D::new(input, 2, 2, 0, 2.0).is_err());
    }

    #[tokio::test]
    async fn test_lp_pool2d_invalid_p() {
        let device = get_test_device().await;
        let input = Tensor::from_data(&[1.0; 16], vec![1, 1, 4, 4], device.clone()).unwrap();

        assert!(LpPool2D::new(input.clone(), 2, 2, 0, 0.0).is_err());
        assert!(LpPool2D::new(input, 2, 2, 0, -1.0).is_err());
    }

    #[tokio::test]
    async fn test_lp_pool2d_with_padding() {
        let device = get_test_device().await;
        let data: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![1, 1, 8, 8], device.clone()).unwrap();

        let pooled = LpPool2D::new(input, 3, 1, 1, 2.0)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(pooled.shape().len(), 4);
    }
}
