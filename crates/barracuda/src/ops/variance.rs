//! Variance reduction - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Variance reduction operation
pub struct Variance {
    input: Tensor,
    dim: Option<usize>,  // None = global variance, Some(d) = variance along dimension d
    keepdim: bool,       // Whether to keep dimension with size 1
}

impl Variance {
    /// Create a new variance operation
    pub fn new(input: Tensor, dim: Option<usize>, keepdim: bool) -> Self {
        Self { input, dim, keepdim }
    }

    /// Get the WGSL shader source for global reduction
    fn wgsl_shader_reduce() -> &'static str {
        include_str!("../shaders/variance_reduce.wgsl")
    }

    /// Get the WGSL shader source for dimension-wise reduction
    fn wgsl_shader_dim() -> &'static str {
        include_str!("../shaders/variance_dim.wgsl")
    }

    /// Execute the variance operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let input_buffer = self.input.buffer();

        match self.dim {
            None => {
                // Global variance reduction
                // Two-pass algorithm: first compute mean, then variance
                let size: usize = shape.iter().product();
                let num_workgroups = ((size + 255) / 256) as u32;

                // Pass 1: Compute mean using tree reduction
                let mean_output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Variance Mean Output"),
                    size: (num_workgroups as usize * std::mem::size_of::<f32>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });

                #[repr(C)]
                #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
                struct Params {
                    size: u32,
                }

                let params = Params { size: size as u32 };

                let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Variance Mean Params"),
                    contents: bytemuck::cast_slice(&[params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                let shader_module = device.compile_shader(Self::wgsl_shader_reduce(), Some("Variance Reduce Shader"));

                let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Variance Reduce Bind Group Layout"),
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

                let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Variance Mean Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: input_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: mean_output_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: params_buffer.as_entire_binding(),
                        },
                    ],
                });

                let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Variance Reduce Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

                let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Variance Reduce Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

                let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Variance Reduce Encoder"),
                });

                {
                    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Variance Mean Pass"),
                        timestamp_writes: None,
                    });
                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &bind_group, &[]);
                    compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
                }

                device.queue.submit(Some(encoder.finish()));

                // Read back partial sums and compute mean
                let partial_sums = device.read_buffer_f32(&mean_output_buffer, num_workgroups as usize)?;
                let global_sum: f32 = partial_sums.iter().sum();
                let global_mean = global_sum / size as f32;

                // Pass 2: Compute variance using tree reduction with mean
                // Create a buffer with (x - mean)^2 values
                let diff_squared_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Variance Diff Squared"),
                    size: (size * std::mem::size_of::<f32>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

                // Compute (x - mean)^2 on CPU for now
                // In a more optimized version, this could be done on GPU
                let input_data = device.read_buffer_f32(input_buffer, size)?;
                let diff_squared: Vec<f32> = input_data.iter().map(|&x| {
                    let diff = x - global_mean;
                    diff * diff
                }).collect();

                device.queue.write_buffer(&diff_squared_buffer, 0, bytemuck::cast_slice(&diff_squared));

                // Now reduce the diff_squared buffer
                let variance_output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Variance Output"),
                    size: (num_workgroups as usize * std::mem::size_of::<f32>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });

                let variance_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Variance Bind Group"),
                    layout: &bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: diff_squared_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: variance_output_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: params_buffer.as_entire_binding(),
                        },
                    ],
                });

                let mut encoder2 = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Variance Encoder 2"),
                });

                {
                    let mut compute_pass = encoder2.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Variance Pass"),
                        timestamp_writes: None,
                    });
                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &variance_bind_group, &[]);
                    compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
                }

                device.queue.submit(Some(encoder2.finish()));

                // Read back partial variance results
                let partial_variances = device.read_buffer_f32(&variance_output_buffer, num_workgroups as usize)?;
                let global_variance_sum: f32 = partial_variances.iter().sum();
                let global_variance = global_variance_sum / size as f32;

                // Return scalar tensor
                Ok(Tensor::new(
                    vec![global_variance],
                    vec![],
                    device.clone(),
                ))
            }
            Some(dim) => {
                // Dimension-wise variance reduction
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
                    label: Some("Variance Dim Output"),
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
                    label: Some("Variance Dim Params"),
                    contents: bytemuck::cast_slice(&[params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                // Compile shader
                let shader_module = device.compile_shader(Self::wgsl_shader_dim(), Some("Variance Dim Shader"));

                // Create bind group layout
                let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Variance Dim Bind Group Layout"),
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
                    label: Some("Variance Dim Bind Group"),
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
                    label: Some("Variance Dim Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

                let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Variance Dim Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

                // Execute compute shader
                let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Variance Dim Encoder"),
                });

                {
                    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Variance Dim Pass"),
                        timestamp_writes: None,
                    });
                    compute_pass.set_pipeline(&compute_pipeline);
                    compute_pass.set_bind_group(0, &bind_group, &[]);
                    let workgroups = ((output_size as u32 + 255) / 256) as u32;
                    compute_pass.dispatch_workgroups(workgroups, 1, 1);
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
    /// Compute variance (global reduction)
    pub fn variance(&self) -> Result<Self> {
        Variance::new(self.clone(), None, false).execute()
    }

    /// Compute variance along a dimension
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension to compute variance along
    /// * `keepdim` - Whether to keep the reduced dimension with size 1
    pub fn variance_dim(&self, dim: usize, keepdim: bool) -> Result<Self> {
        Variance::new(self.clone(), Some(dim), keepdim).execute()
    }

    /// Compute variance (legacy method for backward compatibility)
    pub fn var(self) -> Result<Self> {
        Variance::new(self, None, false).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn variance_cpu(input: &[f32]) -> f32 {
        let mean: f32 = input.iter().sum::<f32>() / input.len() as f32;
        let variance: f32 =
            input.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / input.len() as f32;
        variance
    }

    #[tokio::test]
    async fn test_variance_basic() {
        let device = get_test_device().await;
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let result = input.variance().unwrap().to_vec().unwrap();
        let expected = variance_cpu(&input_data);

        assert!(
            (result[0] - expected).abs() < 1e-4,
            "Expected {}, got {}",
            expected,
            result[0]
        );
    }

    #[tokio::test]
    async fn test_variance_edge_cases() {
        let device = get_test_device().await;

        // All zeros (variance = 0)
        let input_data = vec![0.0, 0.0, 0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone())
            .await
            .unwrap();
        let result = input.variance().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);

        // All same value (variance = 0)
        let input_data = vec![5.0, 5.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device)
            .await
            .unwrap();
        let result = input.variance().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_variance_boundary() {
        let device = get_test_device().await;
        let input_data = vec![0.0, 10.0, 20.0, 30.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.variance().unwrap().to_vec().unwrap();
        let expected = variance_cpu(&input_data);

        let rel_error = if expected > 1e-5 {
            (result[0] - expected).abs() / expected
        } else {
            (result[0] - expected).abs()
        };
        assert!(rel_error < 1e-3, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_variance_large_tensor() {
        let device = get_test_device().await;
        let size = 100;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) * 0.5).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device)
            .await
            .unwrap();
        let result = input.variance().unwrap().to_vec().unwrap();
        let expected = variance_cpu(&input_data);

        let rel_error = (result[0] - expected).abs() / expected;
        assert!(rel_error < 1e-2, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_variance_precision() {
        let device = get_test_device().await;
        let input_data = vec![2.0, 4.0, 6.0, 8.0, 10.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let gpu_result = input.variance().unwrap().to_vec().unwrap();
        let cpu_result = variance_cpu(&input_data);

        let error = (gpu_result[0] - cpu_result).abs();
        assert!(error < 1e-3, "Error {} exceeds threshold", error);
    }

    #[tokio::test]
    async fn test_variance_dim() {
        let device = get_test_device().await;
        // Test 2D tensor: [[1, 2, 3], [4, 5, 6]]
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2, 3], device.clone())
            .await
            .unwrap();
        
        // Variance along dim 0 (columns): variance of [1,4], [2,5], [3,6]
        let result = input.variance_dim(0, false).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 3);
        // Variance of [1, 4] = ((1-2.5)^2 + (4-2.5)^2) / 2 = (2.25 + 2.25) / 2 = 2.25
        assert!((result[0] - 2.25).abs() < 1e-4);
        // Variance of [2, 5] = ((2-3.5)^2 + (5-3.5)^2) / 2 = (2.25 + 2.25) / 2 = 2.25
        assert!((result[1] - 2.25).abs() < 1e-4);
        // Variance of [3, 6] = ((3-4.5)^2 + (6-4.5)^2) / 2 = (2.25 + 2.25) / 2 = 2.25
        assert!((result[2] - 2.25).abs() < 1e-4);
        
        // Variance along dim 1 (rows): variance of [1,2,3], [4,5,6]
        let result = input.variance_dim(1, false).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 2);
        // Variance of [1, 2, 3] = ((1-2)^2 + (2-2)^2 + (3-2)^2) / 3 = (1 + 0 + 1) / 3 = 0.666...
        assert!((result[0] - 0.6666667).abs() < 1e-4);
        // Variance of [4, 5, 6] = ((4-5)^2 + (5-5)^2 + (6-5)^2) / 3 = (1 + 0 + 1) / 3 = 0.666...
        assert!((result[1] - 0.6666667).abs() < 1e-4);
        
        // Variance along dim 0 with keepdim: [[2.25, 2.25, 2.25]]
        let result = input.variance_dim(0, true).unwrap();
        assert_eq!(result.shape(), &[1, 3]);
    }
}
