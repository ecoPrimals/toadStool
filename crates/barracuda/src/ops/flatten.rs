//! Flatten - Pure WGSL
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

/// Flatten operation
pub struct Flatten {
    input: Tensor,
    start_dim: usize,
    end_dim: usize,
}

impl Flatten {
    /// Create a new flatten operation
    pub fn new(input: Tensor, start_dim: usize, end_dim: usize) -> Result<Self> {
        let shape = input.shape();
        if start_dim >= shape.len() || end_dim >= shape.len() || start_dim > end_dim {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!(
                    "Invalid flatten dimensions: start_dim={}, end_dim={}, shape={:?}",
                    start_dim, end_dim, shape
                ),
            });
        }
        Ok(Self {
            input,
            start_dim,
            end_dim,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/tensor/flatten.wgsl")
    }

    /// Execute the flatten operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size: usize = self.input.shape().iter().product();

        // Compute output shape
        let input_shape = self.input.shape();
        let mut output_shape = input_shape[..self.start_dim].to_vec();
        let flattened_size: usize = input_shape[self.start_dim..=self.end_dim].iter().product();
        output_shape.push(flattened_size);
        if self.end_dim + 1 < input_shape.len() {
            output_shape.extend_from_slice(&input_shape[self.end_dim + 1..]);
        }

        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            _pad1: u32,
            _pad2: u32,
            _pad3: u32,
        }

        let params = Params {
            size: size as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Flatten Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("Flatten Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Flatten Bind Group Layout"),
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
            label: Some("Flatten Bind Group"),
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
                    label: Some("Flatten Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Flatten Pipeline"),
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
                label: Some("Flatten Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Flatten Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Return tensor without reading back (zero-copy)
        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape,
            device.clone(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn get_test_device() -> Option<Arc<WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_flatten_basic() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data: Vec<f32> = (0..24).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![2, 3, 4], device.clone()).unwrap();

        let flattened = Flatten::new(input, 1, 2).unwrap().execute().unwrap();
        assert_eq!(flattened.shape(), &vec![2, 12]);
    }

    #[tokio::test]
    async fn test_flatten_all() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![2, 3, 2], device.clone()).unwrap();

        let flattened = Flatten::new(input, 0, 2).unwrap().execute().unwrap();
        assert_eq!(flattened.shape(), &vec![12]);
    }

    #[tokio::test]
    async fn test_flatten_partial() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data: Vec<f32> = (0..60).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![2, 3, 5, 2], device.clone()).unwrap();

        let flattened = Flatten::new(input, 1, 2).unwrap().execute().unwrap();
        assert_eq!(flattened.shape(), &vec![2, 15, 2]);
    }

    #[tokio::test]
    async fn test_flatten_single_dim() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![4, 5], device.clone()).unwrap();

        let flattened = Flatten::new(input, 0, 0).unwrap().execute().unwrap();
        assert_eq!(flattened.shape(), &vec![4, 5]);
    }

    #[tokio::test]
    async fn test_flatten_invalid() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![2, 3, 2], device.clone()).unwrap();

        assert!(Flatten::new(input.clone(), 3, 2).is_err());
        assert!(Flatten::new(input.clone(), 1, 0).is_err());
    }
}
