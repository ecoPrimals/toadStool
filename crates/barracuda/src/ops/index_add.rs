//! Index Add - Add values at specific indices - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its indices and values
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Index Add operation - Add values at specific indices (scatter-add)
pub struct IndexAdd {
    input: Tensor,
    dim: usize,
    indices: Vec<u32>,
    values: Tensor,
}

impl IndexAdd {
    /// Create a new index add operation
    pub fn new(input: Tensor, dim: usize, indices: Vec<u32>, values: Tensor) -> Result<Self> {
        let input_shape = input.shape();

        // Validate dimension
        if dim >= input_shape.len() {
            return Err(BarracudaError::invalid_op(
                "IndexAdd",
                format!(
                    "Dimension {} out of bounds for rank {}",
                    dim,
                    input_shape.len()
                ),
            ));
        }

        // Calculate dimension parameters
        let dim_size = input_shape[dim];
        let outer_size: usize = input_shape[..dim].iter().product();
        let inner_size: usize = input_shape[dim + 1..].iter().product();
        let values_size = values.shape().iter().product::<usize>();
        let expected_values_size = outer_size * indices.len() * inner_size;

        if values_size != expected_values_size {
            return Err(BarracudaError::invalid_op(
                "IndexAdd",
                format!(
                    "Values size {} doesn't match expected size {}",
                    values_size, expected_values_size
                ),
            ));
        }

        // Validate indices are in bounds
        for &idx in &indices {
            if idx as usize >= dim_size {
                return Err(BarracudaError::invalid_op(
                    "IndexAdd",
                    format!(
                        "Index {} out of bounds for dimension size {}",
                        idx, dim_size
                    ),
                ));
            }
        }

        Ok(Self {
            input,
            dim,
            indices,
            values,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/math/index_add.wgsl")
    }

    /// Execute the index add operation (modifies input in-place)
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let size: usize = shape.iter().product();

        // Calculate dimension parameters
        let dim_size = shape[self.dim];
        let outer_size: usize = shape[..self.dim].iter().product();
        let inner_size: usize = shape[self.dim + 1..].iter().product();
        let scatter_size = self.indices.len();
        let values_size = outer_size * scatter_size * inner_size;

        // Access buffers directly (zero-copy)
        let input_buffer = self.input.buffer();
        let values_buffer = self.values.buffer();

        // Create indices buffer
        let indices_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("IndexAdd Indices"),
                contents: bytemuck::cast_slice(&self.indices),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            dim_size: u32,
            outer_size: u32,
            inner_size: u32,
            scatter_size: u32,
        }

        let params = Params {
            size: size as u32,
            dim_size: dim_size as u32,
            outer_size: outer_size as u32,
            inner_size: inner_size as u32,
            scatter_size: scatter_size as u32,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("IndexAdd Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("IndexAdd Bind Group Layout"),
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
            label: Some("IndexAdd Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: values_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: indices_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: input_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("IndexAdd Shader"));

        // Create compute pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("IndexAdd Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("IndexAdd Pipeline"),
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
                label: Some("IndexAdd Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("IndexAdd Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (values_size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Return the input tensor (modified in-place)
        Ok(self.input)
    }
}

impl Tensor {
    /// Add values at specific indices along a dimension (scatter-add)
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension to add along
    /// * `indices` - Indices to add at
    /// * `values` - Values to add
    pub fn index_add(self, dim: usize, indices: Vec<u32>, values: Tensor) -> Result<Self> {
        IndexAdd::new(self, dim, indices, values)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> Option<std::sync::Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_index_add_1d() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let input = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![5], device.clone());
        let values = Tensor::new(vec![10.0, 20.0], vec![2], device.clone());

        let result = input.index_add(0, vec![1, 3], values).unwrap();
        let output_data = result.to_vec().unwrap();

        // Expected: [1, 12, 3, 24, 5]
        assert_eq!(output_data[0], 1.0);
        assert_eq!(output_data[1], 12.0);
        assert_eq!(output_data[2], 3.0);
        assert_eq!(output_data[3], 24.0);
        assert_eq!(output_data[4], 5.0);
    }
}
