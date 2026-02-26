//! Argmax - Find indices of maximum values - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its dimension
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Argsort shader (tracks original indices).
pub const WGSL_ARGSORT: &str = include_str!("../shaders/misc/argsort.wgsl");

/// Generic sort shader.
pub const WGSL_SORT: &str = include_str!("../shaders/misc/sort.wgsl");

/// Argmax operation - Find indices of maximum values
pub struct Argmax {
    input: Tensor,
    dim: Option<usize>, // None = global argmax, Some(d) = argmax along dimension d
    keepdim: bool,      // Whether to keep dimension with size 1
}

impl Argmax {
    /// Create a new argmax operation
    pub fn new(input: Tensor, dim: Option<usize>, keepdim: bool) -> Self {
        Self {
            input,
            dim,
            keepdim,
        }
    }

    /// Get the WGSL shader source for global reduction
    fn wgsl_shader_reduce() -> &'static str {
        static SHADER_REDUCE: std::sync::LazyLock<String> =
            std::sync::LazyLock::new(|| crate::shaders::precision::downcast_f64_to_f32_with_transcendentals(include_str!("../shaders/math/argmax_reduce_f64.wgsl")));
        &SHADER_REDUCE
    }

    /// Get the WGSL shader source for dimension-wise reduction
    fn wgsl_shader_dim() -> &'static str {
        static SHADER_DIM: std::sync::LazyLock<String> =
            std::sync::LazyLock::new(|| crate::shaders::precision::downcast_f64_to_f32_with_transcendentals(include_str!("../shaders/math/argmax_f64.wgsl")));
        &SHADER_DIM
    }

    /// Execute the argmax operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let input_buffer = self.input.buffer();

        match self.dim {
            None => {
                // Global argmax reduction
                let size: usize = shape.iter().product();
                // Deep Debt Evolution: Capability-based dispatch
                let caps = DeviceCapabilities::from_device(device);
                let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
                let num_workgroups = (size as u32).div_ceil(optimal_wg_size);

                // Create output buffer for partial results (indices)
                let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Argmax Reduce Output"),
                    size: (num_workgroups as usize * std::mem::size_of::<u32>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });

                // Create uniform buffer for parameters
                #[repr(C)]
                #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
                struct Params {
                    size: u32,
                }

                let params = Params { size: size as u32 };

                let params_buffer =
                    device
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Argmax Reduce Params"),
                            contents: bytemuck::cast_slice(&[params]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });

                // Compile shader
                let shader_module =
                    device.compile_shader(Self::wgsl_shader_reduce(), Some("Argmax Reduce Shader"));

                // Create bind group layout
                let bind_group_layout =
                    device
                        .device
                        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                            label: Some("Argmax Reduce Bind Group Layout"),
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
                                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                                        has_dynamic_offset: false,
                                        min_binding_size: None,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 2,
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
                    label: Some("Argmax Reduce Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: input_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: output_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: params_buffer.as_entire_binding(),
                        },
                    ],
                });

                // Create compute pipeline
                let pipeline_layout =
                    device
                        .device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some("Argmax Reduce Pipeline Layout"),
                            bind_group_layouts: &[&bind_group_layout],
                            push_constant_ranges: &[],
                        });

                let compute_pipeline =
                    device
                        .device
                        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("Argmax Reduce Pipeline"),
                            layout: Some(&pipeline_layout),
                            module: &shader_module,
                            entry_point: "main",
                            cache: None,
                            compilation_options: Default::default(),
                        });

                // Execute compute shader
                let mut encoder =
                    device
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Argmax Reduce Encoder"),
                        });

                {
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Argmax Reduce Pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &bind_group, &[]);
                    // Deep Debt Evolution: Capability-based dispatch (already computed above)
                    compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
                }

                device.queue.submit(Some(encoder.finish()));

                // Read back partial results and find the global argmax on CPU
                // We need to compare values at the partial indices to find the true global argmax
                let partial_indices =
                    crate::utils::read_buffer_u32(device, &output_buffer, num_workgroups as usize)?;

                // Read the input values at these indices to find the maximum
                let input_data = device.read_buffer_f32(input_buffer, size)?;
                let mut global_max_idx = 0u32;
                let mut global_max_val = f32::NEG_INFINITY;

                for &idx in &partial_indices {
                    if (idx as usize) < size {
                        let val = input_data[idx as usize];
                        if val > global_max_val {
                            global_max_val = val;
                            global_max_idx = idx;
                        }
                    }
                }

                // Return scalar tensor (single index)
                Ok(Tensor::new(
                    vec![global_max_idx as f32],
                    vec![],
                    device.clone(),
                ))
            }
            Some(dim) => {
                // Dimension-wise argmax reduction
                if dim >= shape.len() {
                    return Err(crate::error::BarracudaError::InvalidInput {
                        message: format!("Dimension {} out of range for shape {:?}", dim, shape),
                    });
                }

                let dim_size = shape[dim];
                let outer_size: usize = shape[..dim].iter().product();
                let inner_size: usize = shape[dim + 1..].iter().product();
                let output_size = outer_size * inner_size;

                // Create output buffer
                let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Argmax Dim Output"),
                    size: (output_size * std::mem::size_of::<u32>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });

                // Create uniform buffer for parameters
                #[repr(C)]
                #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
                struct Params {
                    dim_size: u32,
                    outer_size: u32,
                    inner_size: u32,
                }

                let params = Params {
                    dim_size: dim_size as u32,
                    outer_size: outer_size as u32,
                    inner_size: inner_size as u32,
                };

                let params_buffer =
                    device
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Argmax Dim Params"),
                            contents: bytemuck::cast_slice(&[params]),
                            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                        });

                // Compile shader
                let shader_module =
                    device.compile_shader(Self::wgsl_shader_dim(), Some("Argmax Dim Shader"));

                // Create bind group layout
                let bind_group_layout =
                    device
                        .device
                        .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                            label: Some("Argmax Dim Bind Group Layout"),
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
                                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                                        has_dynamic_offset: false,
                                        min_binding_size: None,
                                    },
                                    count: None,
                                },
                                wgpu::BindGroupLayoutEntry {
                                    binding: 2,
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
                    label: Some("Argmax Dim Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: input_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: output_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: params_buffer.as_entire_binding(),
                        },
                    ],
                });

                // Create compute pipeline
                let pipeline_layout =
                    device
                        .device
                        .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                            label: Some("Argmax Dim Pipeline Layout"),
                            bind_group_layouts: &[&bind_group_layout],
                            push_constant_ranges: &[],
                        });

                let compute_pipeline =
                    device
                        .device
                        .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                            label: Some("Argmax Dim Pipeline"),
                            layout: Some(&pipeline_layout),
                            module: &shader_module,
                            entry_point: "main",
                            cache: None,
                            compilation_options: Default::default(),
                        });

                // Execute compute shader
                let mut encoder =
                    device
                        .device
                        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                            label: Some("Argmax Dim Encoder"),
                        });

                {
                    let mut compute_pass =
                        encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                            label: Some("Argmax Dim Pass"),
                            timestamp_writes: None,
                        });
                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &bind_group, &[]);
                    // Deep Debt Evolution: Capability-based dispatch
                    let caps = DeviceCapabilities::from_device(device);
                    let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
                    let workgroups = (output_size as u32).div_ceil(optimal_wg_size);
                    compute_pass.dispatch_workgroups(workgroups, 1, 1);
                }

                device.queue.submit(Some(encoder.finish()));

                // Read back results
                let output_data =
                    crate::utils::read_buffer_u32(device, &output_buffer, output_size)?;

                // Convert u32 to f32 for tensor
                let output_f32: Vec<f32> = output_data.iter().map(|&x| x as f32).collect();

                // Calculate output shape
                let mut output_shape = shape.to_vec();
                if self.keepdim {
                    output_shape[dim] = 1;
                } else {
                    output_shape.remove(dim);
                }

                Ok(Tensor::new(output_f32, output_shape, device.clone()))
            }
        }
    }
}

impl Tensor {
    /// Find index of maximum value (global reduction)
    pub fn argmax(&self) -> Result<Self> {
        Argmax::new(self.clone(), None, false).execute()
    }

    /// Find indices of maximum values along a dimension (GPU/WGSL).
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension to find max along
    /// * `keepdim` - Whether to keep the reduced dimension with size 1
    pub fn argmax_dim_keepdim(&self, dim: usize, keepdim: bool) -> Result<Self> {
        Argmax::new(self.clone(), Some(dim), keepdim).execute()
    }

    /// Find indices of maximum values along a dimension (legacy method for backward compatibility)
    pub fn argmax_wgsl(self, dim: usize) -> Result<Self> {
        Argmax::new(self, Some(dim), false).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> Option<std::sync::Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_argmax_1d() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data = vec![1.0, 5.0, 3.0, 2.0];
        let input = Tensor::new(data, vec![4], device.clone());

        let output = input.argmax_wgsl(0).unwrap();

        assert_eq!(output.shape(), &[] as &[usize]); // Scalar output
        let result = output.to_vec().unwrap();
        assert_eq!(result[0] as u32, 1); // Index of max value (5.0)
    }

    #[tokio::test]
    async fn test_argmax_2d_dim0() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data = vec![1.0, 5.0, 3.0, 2.0, 4.0, 6.0];
        let input = Tensor::new(data, vec![3, 2], device.clone());

        let output = input.argmax_wgsl(0).unwrap();

        assert_eq!(output.shape(), &[2]);
        let result = output.to_vec().unwrap();
        assert_eq!(result[0] as u32, 2); // Max in column 0 is at index 2 (value 4.0)
        assert_eq!(result[1] as u32, 2); // Max in column 1 is at index 2 (value 6.0)
    }

    #[tokio::test]
    async fn test_argmax_global() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data = vec![1.0, 5.0, 3.0, 2.0];
        let input = Tensor::new(data, vec![4], device.clone());

        let output = input.argmax().unwrap();

        assert_eq!(output.shape(), &[] as &[usize]); // Scalar output
        let result = output.to_vec().unwrap();
        assert_eq!(result[0] as u32, 1); // Index of max value (5.0)
    }

    #[tokio::test]
    async fn test_argmax_dim_keepdim() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let data = vec![1.0, 5.0, 3.0, 2.0, 4.0, 6.0];
        let input = Tensor::new(data, vec![3, 2], device.clone());

        let output = input.argmax_dim_keepdim(0, true).unwrap();

        assert_eq!(output.shape(), &[1, 2]); // Dimension kept
        let result = output.to_vec().unwrap();
        assert_eq!(result[0] as u32, 2); // Max in column 0 is at index 2
        assert_eq!(result[1] as u32, 2); // Max in column 1 is at index 2
    }
}
