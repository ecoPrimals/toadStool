//! Unfold - Pure WGSL
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

/// Unfold operation (im2col)
pub struct Unfold {
    input: Tensor,
    kernel_size: (usize, usize),
    stride: usize,
    padding: usize,
    dilation: usize,
}

impl Unfold {
    /// Create a new unfold operation
    pub fn new(
        input: Tensor,
        kernel_size: (usize, usize),
        stride: usize,
        padding: usize,
        dilation: usize,
    ) -> Result<Self> {
        let shape = input.shape();
        if shape.len() != 4 {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!(
                    "Unfold expects 4D tensor [B, C, H, W], got shape {:?}",
                    shape
                ),
            });
        }

        Ok(Self {
            input,
            kernel_size,
            stride,
            padding,
            dilation,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/tensor/unfold.wgsl")
    }

    /// Execute the unfold operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let batch_size = shape[0];
        let channels = shape[1];
        let in_height = shape[2];
        let in_width = shape[3];

        // Compute output dimensions
        let out_height =
            ((in_height + 2 * self.padding - self.dilation * (self.kernel_size.0 - 1) - 1)
                / self.stride)
                + 1;
        let out_width =
            ((in_width + 2 * self.padding - self.dilation * (self.kernel_size.1 - 1) - 1)
                / self.stride)
                + 1;
        let num_blocks = out_height * out_width;
        let output_size =
            batch_size * channels * self.kernel_size.0 * self.kernel_size.1 * num_blocks;

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
            kernel_height: u32,
            kernel_width: u32,
            stride: u32,
            padding: u32,
            dilation: u32,
            out_height: u32,
            out_width: u32,
            _pad1: u32,
        }

        let params = Params {
            batch_size: batch_size as u32,
            channels: channels as u32,
            in_height: in_height as u32,
            in_width: in_width as u32,
            kernel_height: self.kernel_size.0 as u32,
            kernel_width: self.kernel_size.1 as u32,
            stride: self.stride as u32,
            padding: self.padding as u32,
            dilation: self.dilation as u32,
            out_height: out_height as u32,
            out_width: out_width as u32,
            _pad1: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Unfold Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("Unfold Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Unfold Bind Group Layout"),
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
            label: Some("Unfold Bind Group"),
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
                    label: Some("Unfold Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Unfold Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Execute compute shader
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Unfold Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Unfold Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Convolution);
            let workgroups_x = (out_width as u32).div_ceil(optimal_wg_size);
            let workgroups_y = (out_height as u32).div_ceil(optimal_wg_size);
            let workgroups_z = batch_size as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        device.queue.submit(Some(encoder.finish()));

        // Return tensor without reading back (zero-copy)
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![
                batch_size,
                channels * self.kernel_size.0 * self.kernel_size.1,
                num_blocks,
            ],
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_unfold_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let data: Vec<f32> = (0..48).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![1, 1, 6, 8], device.clone()).unwrap();

        let unfolded = Unfold::new(input, (3, 3), 1, 0, 1)
            .unwrap()
            .execute()
            .unwrap();
        // Output shape: [B, C*K*K, L] where L = num_blocks
        assert_eq!(unfolded.shape().len(), 3);
    }

    #[tokio::test]
    async fn test_unfold_with_padding() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let data: Vec<f32> = (0..32).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![1, 1, 4, 8], device.clone()).unwrap();

        let unfolded = Unfold::new(input, (3, 3), 1, 1, 1)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(unfolded.shape().len(), 3);
    }

    #[tokio::test]
    async fn test_unfold_invalid_shape() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_data(&[1.0, 2.0, 3.0], vec![3], device.clone()).unwrap();

        assert!(Unfold::new(input, (3, 3), 1, 0, 1).is_err());
    }

    #[tokio::test]
    async fn test_unfold_dilation() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let data: Vec<f32> = (0..64).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![1, 1, 8, 8], device.clone()).unwrap();

        let unfolded = Unfold::new(input, (3, 3), 1, 0, 2)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(unfolded.shape().len(), 3);
    }

    #[tokio::test]
    async fn test_unfold_stride() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let data: Vec<f32> = (0..128).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![1, 1, 8, 16], device.clone()).unwrap();

        let unfolded = Unfold::new(input, (3, 3), 2, 0, 1)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(unfolded.shape().len(), 3);
    }
}
