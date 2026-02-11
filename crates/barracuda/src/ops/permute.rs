//! Permute - Pure WGSL
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

/// Permute operation (reorder dimensions)
pub struct Permute {
    input: Tensor,
    permutation: Vec<usize>,
}

impl Permute {
    /// Create a new permute operation
    pub fn new(input: Tensor, permutation: Vec<usize>) -> Result<Self> {
        let num_dims = input.shape().len();
        if permutation.len() != num_dims {
            return Err(crate::error::BarracudaError::InvalidInput {
                message: format!(
                    "Permutation length {} doesn't match tensor rank {}",
                    permutation.len(),
                    num_dims
                ),
            });
        }

        // Validate permutation is valid (contains all indices 0..num_dims-1)
        let mut seen = vec![false; num_dims];
        for &idx in &permutation {
            if idx >= num_dims {
                return Err(crate::error::BarracudaError::InvalidInput {
                    message: format!("Invalid permutation index {} for rank {}", idx, num_dims),
                });
            }
            if seen[idx] {
                return Err(crate::error::BarracudaError::InvalidInput {
                    message: format!("Duplicate index {} in permutation", idx),
                });
            }
            seen[idx] = true;
        }

        Ok(Self { input, permutation })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/tensor/permute.wgsl")
    }

    /// Execute the permute operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let input_shape = self.input.shape();
        let num_dims = input_shape.len();
        let total_size: usize = input_shape.iter().product();

        // Compute output shape
        let output_shape: Vec<usize> = self
            .permutation
            .iter()
            .map(|&idx| input_shape[idx])
            .collect();

        // Compute input strides
        let mut input_strides = vec![1; num_dims];
        for i in (0..num_dims - 1).rev() {
            input_strides[i] = input_strides[i + 1] * input_shape[i + 1];
        }

        // Compute output strides (for indexing)
        let mut output_strides = vec![1; num_dims];
        for i in (0..num_dims - 1).rev() {
            output_strides[i] = output_strides[i + 1] * output_shape[i + 1];
        }

        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        // Create buffers for shape and stride data
        let input_shape_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Permute Input Shape"),
                    contents: bytemuck::cast_slice(
                        &input_shape.iter().map(|&x| x as u32).collect::<Vec<_>>(),
                    ),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let output_shape_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Permute Output Shape"),
                    contents: bytemuck::cast_slice(
                        &output_shape.iter().map(|&x| x as u32).collect::<Vec<_>>(),
                    ),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let permutation_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Permute Permutation"),
                    contents: bytemuck::cast_slice(
                        &self
                            .permutation
                            .iter()
                            .map(|&x| x as u32)
                            .collect::<Vec<_>>(),
                    ),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        let input_strides_buffer =
            device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Permute Input Strides"),
                    contents: bytemuck::cast_slice(
                        &input_strides.iter().map(|&x| x as u32).collect::<Vec<_>>(),
                    ),
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                });

        // Create output buffer
        let output_buffer = device.create_buffer_f32(total_size)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            total_size: u32,
            num_dims: u32,
            _pad1: u32,
            _pad2: u32,
        }

        let params = Params {
            total_size: total_size as u32,
            num_dims: num_dims as u32,
            _pad1: 0,
            _pad2: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Permute Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("Permute Shader"));

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Permute Bind Group Layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 5,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 6,
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
            label: Some("Permute Bind Group"),
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
                    resource: input_shape_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: output_shape_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: permutation_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: input_strides_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Permute Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Permute Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

        // Execute compute shader
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Permute Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Permute Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (total_size as u32).div_ceil(optimal_wg_size);
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
    async fn test_permute_basic() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data: Vec<f32> = (0..24).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![2, 3, 4], device.clone()).unwrap();

        let permuted = Permute::new(input, vec![0, 2, 1])
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(permuted.shape(), &vec![2, 4, 3]);
    }

    #[tokio::test]
    async fn test_permute_identity() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data: Vec<f32> = (0..12).map(|i| i as f32).collect();
        let input = Tensor::from_data(&data, vec![2, 3, 2], device.clone()).unwrap();

        let permuted = Permute::new(input, vec![0, 1, 2])
            .unwrap()
            .execute()
            .unwrap();
        assert_eq!(permuted.shape(), &vec![2, 3, 2]);
    }

    #[tokio::test]
    async fn test_permute_invalid_length() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let input = Tensor::from_data(&[1.0, 2.0, 3.0], vec![3], device.clone()).unwrap();

        assert!(Permute::new(input, vec![0, 1, 2, 3]).is_err());
    }

    #[tokio::test]
    async fn test_permute_invalid_index() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let input = Tensor::from_data(&[1.0, 2.0, 3.0], vec![3], device.clone()).unwrap();

        assert!(Permute::new(input, vec![5]).is_err());
    }

    #[tokio::test]
    async fn test_permute_duplicate() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let input = Tensor::from_data(&[1.0, 2.0, 3.0, 4.0], vec![2, 2], device.clone()).unwrap();

        assert!(Permute::new(input, vec![0, 0]).is_err());
    }
}
