//! Tensor Split - Pure WGSL
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

/// Tensor split operation
pub struct TensorSplit {
    input: Tensor,
    split_indices: Vec<usize>,
    dim: usize,
}

impl TensorSplit {
    /// Create a new tensor split operation
    pub fn new(input: Tensor, split_indices: Vec<usize>, dim: usize) -> Result<Self> {
        let shape = input.shape();
        if dim >= shape.len() {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!("dim {} exceeds tensor rank {}", dim, shape.len()),
            });
        }

        let dim_size = shape[dim];
        for &idx in &split_indices {
            if idx > dim_size {
                return Err(crate::error::BarracudaError::InvalidInput {
                    message: format!("Split index {} exceeds dimension size {}", idx, dim_size),
                });
            }
        }

        Ok(Self {
            input,
            split_indices,
            dim,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/tensor_split.wgsl")
    }

    /// Execute the tensor split operation
    /// Returns a vector of tensors (one per split)
    pub fn execute(self) -> Result<Vec<Tensor>> {
        let device = self.input.device();
        let shape = self.input.shape();
        let dim_size = shape[self.dim];

        // Compute sizes and start indices for each split
        let mut split_info = Vec::new();
        let mut prev_idx = 0;
        for &idx in &self.split_indices {
            split_info.push((prev_idx, idx - prev_idx));
            prev_idx = idx;
        }
        split_info.push((prev_idx, dim_size - prev_idx));

        // Compute inner and outer sizes
        let outer_size: usize = shape[..self.dim].iter().product();
        let inner_size: usize = shape[self.dim + 1..].iter().product();

        let total_size: usize = shape.iter().product();

        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        // Compile shader once
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("TensorSplit Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("TensorSplit Bind Group Layout"),
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

        // Create output tensors for each split
        let mut output_tensors = Vec::new();
        for (i, &(split_start, split_size)) in split_info.iter().enumerate() {
            let mut output_shape = shape.to_vec();
            output_shape[self.dim] = split_size;
            let output_size: usize = output_shape.iter().product();

            let output_buffer = device.create_buffer_f32(output_size)?;

            // Create uniform buffer for parameters (per split)
            #[repr(C)]
            #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
            struct Params {
                total_size: u32,
                num_splits: u32,
                split_dim: u32,
                dim_size: u32,
                inner_size: u32,
                outer_size: u32,
                split_start: u32,
                split_size: u32,
                output_size: u32,
            }

            let params = Params {
                total_size: total_size as u32,
                num_splits: split_info.len() as u32,
                split_dim: self.dim as u32,
                dim_size: dim_size as u32,
                inner_size: inner_size as u32,
                outer_size: outer_size as u32,
                split_start: split_start as u32,
                split_size: split_size as u32,
                output_size: output_size as u32,
            };

            let params_buffer =
                device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&format!("TensorSplit Params {}", i)),
                        contents: bytemuck::cast_slice(&[params]),
                        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    });

            // Create bind group for this split
            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some(&format!("TensorSplit Bind Group {}", i)),
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
                        label: Some("TensorSplit Pipeline Layout"),
                        bind_group_layouts: &[&bind_group_layout],
                        push_constant_ranges: &[],
                    });

            let compute_pipeline =
                device
                    .device
                    .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                        label: Some("TensorSplit Pipeline"),
                        layout: Some(&pipeline_layout),
                        module: &shader_module,
                        entry_point: "main",
                    });

            // Execute compute shader
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some(&format!("TensorSplit Encoder {}", i)),
                    });

            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some(&format!("TensorSplit Pass {}", i)),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&compute_pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                // Deep Debt Evolution: Capability-based dispatch
                let caps = DeviceCapabilities::from_device(device);
                let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
                let workgroups = (output_size as u32).div_ceil(optimal_wg_size);
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }

            device.queue.submit(Some(encoder.finish()));

            output_tensors.push(Tensor::from_buffer(
                output_buffer,
                output_shape,
                device.clone(),
            ));
        }

        Ok(output_tensors)
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
    async fn test_tensor_split_basic() {
        let device = get_test_device().await;
        let data: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![3, 4], device.clone()).unwrap();

        let splits = TensorSplit::new(input, vec![1, 2], 0)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(splits.len(), 3);
        assert_eq!(splits[0].shape(), &vec![1, 4]);
        assert_eq!(splits[1].shape(), &vec![1, 4]);
        assert_eq!(splits[2].shape(), &vec![1, 4]);
    }

    #[tokio::test]
    async fn test_tensor_split_single() {
        let device = get_test_device().await;
        let data: Vec<f32> = (0..20).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![4, 5], device.clone()).unwrap();

        let splits = TensorSplit::new(input, vec![2], 0)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(splits.len(), 2);
        assert_eq!(splits[0].shape(), &vec![2, 5]);
        assert_eq!(splits[1].shape(), &vec![2, 5]);
    }

    #[tokio::test]
    async fn test_tensor_split_invalid_dim() {
        let device = get_test_device().await;
        let input = Tensor::from_data(&[1.0, 2.0, 3.0], vec![3], device.clone()).unwrap();

        assert!(TensorSplit::new(input, vec![1], 10).is_err());
    }

    #[tokio::test]
    async fn test_tensor_split_invalid_index() {
        let device = get_test_device().await;
        let input = Tensor::from_data(&[1.0, 2.0, 3.0], vec![3], device.clone()).unwrap();

        assert!(TensorSplit::new(input, vec![5], 0).is_err());
    }

    #[tokio::test]
    async fn test_tensor_split_empty() {
        let device = get_test_device().await;
        let data: Vec<f32> = (0..10).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![5, 2], device.clone()).unwrap();

        let splits = TensorSplit::new(input, vec![], 0)
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(splits.len(), 1);
        assert_eq!(splits[0].shape(), &vec![5, 2]);
    }
}
