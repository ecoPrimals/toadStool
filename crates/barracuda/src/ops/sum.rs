//! Sum reduction - Pure WGSL
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

/// Sum reduction operation
pub struct Sum {
    input: Tensor,
    dim: Option<usize>,  // None = global sum, Some(d) = sum along dimension d
    keepdim: bool,       // Whether to keep dimension with size 1
}

impl Sum {
    /// Create a new sum operation
    pub fn new(input: Tensor, dim: Option<usize>, keepdim: bool) -> Self {
        Self { input, dim, keepdim }
    }

    /// Get the WGSL shader source for global reduction
    fn wgsl_shader_reduce() -> &'static str {
        include_str!("../shaders/sum_reduce.wgsl")
    }

    /// Get the WGSL shader source for dimension-wise reduction
    fn wgsl_shader_dim() -> &'static str {
        include_str!("../shaders/sum_dim.wgsl")
    }

    /// Execute the sum operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let input_buffer = self.input.buffer();

        match self.dim {
            None => {
                // Global sum reduction
                let size: usize = shape.iter().product();
                // Deep Debt Evolution: Capability-based dispatch
                let caps = DeviceCapabilities::from_device(&device);
                let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
                let num_workgroups = ((size as u32 + optimal_wg_size - 1) / optimal_wg_size) as u32;

                // Create output buffer for partial results
                let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Sum Reduce Output"),
                    size: (num_workgroups as usize * std::mem::size_of::<f32>()) as u64,
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

                let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Sum Reduce Params"),
                    contents: bytemuck::cast_slice(&[params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                // Compile shader
                let shader_module = device.compile_shader(Self::wgsl_shader_reduce(), Some("Sum Reduce Shader"));

                // Create bind group layout
                let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Sum Reduce Bind Group Layout"),
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
                    label: Some("Sum Reduce Bind Group"),
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
                let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sum Reduce Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

                let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Sum Reduce Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

                // Execute compute shader
                let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Sum Reduce Encoder"),
                });

                {
                    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Sum Reduce Pass"),
                        timestamp_writes: None,
                    });
                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &bind_group, &[]);
                    // Deep Debt Evolution: Capability-based dispatch
                    compute_pass.dispatch_workgroups(num_workgroups.max(1), 1, 1);
                }

                device.queue.submit(Some(encoder.finish()));

                // Read back partial results and reduce them on CPU
                // For now, we'll do a simple CPU reduction of partial results
                // In production, you might want to do a second GPU pass
                let partial_results = device.read_buffer_f32(&output_buffer, num_workgroups as usize)?;
                let global_sum: f32 = partial_results.iter().sum();

                // Return scalar tensor
                Ok(Tensor::new(
                    vec![global_sum],
                    vec![],
                    device.clone(),
                ))
            }
            Some(dim) => {
                // Dimension-wise sum reduction
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
                    label: Some("Sum Dim Output"),
                    size: (output_size * std::mem::size_of::<f32>()) as u64,
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

                let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Sum Dim Params"),
                    contents: bytemuck::cast_slice(&[params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                // Compile shader
                let shader_module = device.compile_shader(Self::wgsl_shader_dim(), Some("Sum Dim Shader"));

                // Create bind group layout
                let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Sum Dim Bind Group Layout"),
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
                    label: Some("Sum Dim Bind Group"),
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
                let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sum Dim Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

                let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Sum Dim Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

                // Execute compute shader
                let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Sum Dim Encoder"),
                });

                {
                    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Sum Dim Pass"),
                        timestamp_writes: None,
                    });
                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &bind_group, &[]);
                    // Deep Debt Evolution: Capability-based dispatch
                    let caps = DeviceCapabilities::from_device(&device);
                    let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::Reduction);
                    let workgroups = ((output_size as u32 + optimal_wg_size - 1) / optimal_wg_size) as u32;
                    compute_pass.dispatch_workgroups(workgroups.max(1), 1, 1);
                }

                device.queue.submit(Some(encoder.finish()));

                // Read back results
                let output_data = device.read_buffer_f32(&output_buffer, output_size)?;

                // Calculate output shape
                let mut output_shape = shape.to_vec();
                if self.keepdim {
                    output_shape[dim] = 1;
                } else {
                    output_shape.remove(dim);
                }

                Ok(Tensor::new(
                    output_data,
                    output_shape,
                    device.clone(),
                ))
            }
        }
    }
}

impl Tensor {
    /// Compute sum (global reduction)
    pub fn sum(&self) -> Result<Self> {
        Sum::new(self.clone(), None, false).execute()
    }

    /// Compute sum along a dimension
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension to sum along
    /// * `keepdim` - Whether to keep the reduced dimension with size 1
    pub fn sum_dim(&self, dim: usize, keepdim: bool) -> Result<Self> {
        Sum::new(self.clone(), Some(dim), keepdim).execute()
    }

    /// Compute sum (legacy method for backward compatibility)
    pub fn sum_wgsl(self, dim: Option<usize>) -> Result<Self> {
        match dim {
            None => Sum::new(self, None, false).execute(),
            Some(d) => Sum::new(self, Some(d), false).execute(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn sum_cpu(input: &[f32]) -> f32 {
        input.iter().sum()
    }

    #[tokio::test]
    async fn test_sum_basic() {
        let device = get_test_device().await;
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        let expected = sum_cpu(&input_data);

        assert!(
            (result[0] - expected).abs() < 1e-4,
            "Expected {}, got {}",
            expected,
            result[0]
        );
    }

    #[tokio::test]
    async fn test_sum_edge_cases() {
        let device = get_test_device().await;

        // All zeros
        let input_data = vec![0.0, 0.0, 0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone())
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);

        // Mixed positive/negative
        let input_data = vec![5.0, -3.0, 2.0, -4.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        let expected = sum_cpu(&input_data);
        assert!((result[0] - expected).abs() < 1e-4);
    }

    #[tokio::test]
    async fn test_sum_boundary() {
        let device = get_test_device().await;
        let input_data = vec![1e6, 1e-6, -1e6, 1e-6];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        let expected = sum_cpu(&input_data);

        // Use relative error for large sums
        let rel_error = if expected.abs() > 1e-5 {
            (result[0] - expected).abs() / expected.abs()
        } else {
            (result[0] - expected).abs()
        };
        assert!(
            rel_error < 1e-3,
            "Expected {}, got {} (rel error {})",
            expected,
            result[0],
            rel_error
        );
    }

    #[tokio::test]
    async fn test_sum_large_tensor() {
        let device = get_test_device().await;
        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device)
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        let expected = sum_cpu(&input_data);

        let rel_error = (result[0] - expected).abs() / expected.abs();
        assert!(rel_error < 1e-3, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_sum_precision() {
        let device = get_test_device().await;
        let input_data = vec![1.5, 2.5, 3.5, 4.5, 5.5];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let gpu_result = input.sum().unwrap().to_vec().unwrap();
        let cpu_result = sum_cpu(&input_data);

        let error = (gpu_result[0] - cpu_result).abs();
        assert!(error < 1e-4, "Error {} exceeds threshold", error);
    }

    #[tokio::test]
    async fn test_sum_dim() {
        let device = get_test_device().await;
        // Test 2D tensor: [[1, 2, 3], [4, 5, 6]]
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2, 3], device.clone())
            .await
            .unwrap();
        
        // Sum along dim 0 (columns): [5, 7, 9]
        let result = input.sum_dim(0, false).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 3);
        assert!((result[0] - 5.0).abs() < 1e-4);
        assert!((result[1] - 7.0).abs() < 1e-4);
        assert!((result[2] - 9.0).abs() < 1e-4);
        
        // Sum along dim 1 (rows): [6, 15]
        let result = input.sum_dim(1, false).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 2);
        assert!((result[0] - 6.0).abs() < 1e-4);
        assert!((result[1] - 15.0).abs() < 1e-4);
        
        // Sum along dim 0 with keepdim: [[5, 7, 9]]
        let result = input.sum_dim(0, true).unwrap();
        assert_eq!(result.shape(), &[1, 3]);
    }
}
