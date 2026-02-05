//! Standard deviation reduction - Pure WGSL
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

/// Standard deviation reduction operation
pub struct Std {
    input: Tensor,
    dim: Option<usize>,  // None = global std, Some(d) = std along dimension d
    keepdim: bool,       // Whether to keep dimension with size 1
}

impl Std {
    /// Create a new std operation
    pub fn new(input: Tensor, dim: Option<usize>, keepdim: bool) -> Self {
        Self { input, dim, keepdim }
    }

    /// Get the WGSL shader source for global reduction
    fn wgsl_shader_reduce() -> &'static str {
        include_str!("../shaders/std_reduce.wgsl")
    }

    /// Get the WGSL shader source for dimension-wise reduction
    fn wgsl_shader_dim() -> &'static str {
        include_str!("../shaders/std_dim.wgsl")
    }

    /// Execute the std operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let input_buffer = self.input.buffer();

        match self.dim {
            None => {
                // Global std reduction
                // Two-pass algorithm: first compute mean, then variance, then sqrt
                let size: usize = shape.iter().product();
                let num_workgroups = ((size + 255) / 256) as u32;

                // Pass 1: Compute mean using tree reduction
                let mean_output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Std Mean Output"),
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
                    label: Some("Std Mean Params"),
                    contents: bytemuck::cast_slice(&[params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                let shader_module = device.compile_shader(Self::wgsl_shader_reduce(), Some("Std Reduce Shader"));

                let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Std Reduce Bind Group Layout"),
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
                    label: Some("Std Mean Bind Group"),
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
                    label: Some("Std Reduce Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

                let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Std Reduce Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

                let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Std Reduce Encoder"),
                });

                {
                    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Std Mean Pass"),
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
                    label: Some("Std Diff Squared"),
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
                    label: Some("Std Output"),
                    size: (num_workgroups as usize * std::mem::size_of::<f32>()) as u64,
                    usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                });

                let variance_bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Std Variance Bind Group"),
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
                    label: Some("Std Encoder 2"),
                });

                {
                    let mut compute_pass = encoder2.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Std Variance Pass"),
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
                let global_std = global_variance.sqrt();

                // Return scalar tensor
                Ok(Tensor::new(
                    vec![global_std],
                    vec![],
                    device.clone(),
                ))
            }
            Some(dim) => {
                // Dimension-wise std reduction
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
                    label: Some("Std Dim Output"),
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
                    label: Some("Std Dim Params"),
                    contents: bytemuck::cast_slice(&[params]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

                // Compile shader
                let shader_module = device.compile_shader(Self::wgsl_shader_dim(), Some("Std Dim Shader"));

                // Create bind group layout
                let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Std Dim Bind Group Layout"),
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
                    label: Some("Std Dim Bind Group"),
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
                    label: Some("Std Dim Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

                let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Std Dim Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                });

                // Execute compute shader
                let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Std Dim Encoder"),
                });

                {
                    let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                        label: Some("Std Dim Pass"),
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
    /// Compute standard deviation (global reduction)
    pub fn std(&self) -> Result<Self> {
        Std::new(self.clone(), None, false).execute()
    }

    /// Compute standard deviation along a dimension
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension to compute std along
    /// * `keepdim` - Whether to keep the reduced dimension with size 1
    pub fn std_dim(&self, dim: usize, keepdim: bool) -> Result<Self> {
        Std::new(self.clone(), Some(dim), keepdim).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn std_cpu(input: &[f32]) -> f32 {
        let mean: f32 = input.iter().sum::<f32>() / input.len() as f32;
        let variance: f32 =
            input.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / input.len() as f32;
        variance.sqrt()
    }

    #[tokio::test]
    async fn test_std_basic() {
        let device = get_test_device().await;
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        let expected = std_cpu(&input_data);

        assert!(
            (result[0] - expected).abs() < 1e-4,
            "Expected {}, got {}",
            expected,
            result[0]
        );
    }

    #[tokio::test]
    async fn test_std_edge_cases() {
        let device = get_test_device().await;

        // All same value (std = 0)
        let input_data = vec![5.0, 5.0, 5.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device.clone())
            .await
            .unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);

        // All zeros (std = 0)
        let input_data = vec![0.0, 0.0, 0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device)
            .await
            .unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_std_boundary() {
        let device = get_test_device().await;
        let input_data = vec![0.0, 10.0, 20.0, 30.0, 40.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        let expected = std_cpu(&input_data);

        let rel_error = if expected > 1e-5 {
            (result[0] - expected).abs() / expected
        } else {
            (result[0] - expected).abs()
        };
        assert!(rel_error < 1e-2, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_std_large_tensor() {
        let device = get_test_device().await;
        let size = 100;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) * 0.5).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device)
            .await
            .unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        let expected = std_cpu(&input_data);

        let rel_error = (result[0] - expected).abs() / expected;
        assert!(rel_error < 1e-2, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_std_precision() {
        let device = get_test_device().await;
        let input_data = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![6], device)
            .await
            .unwrap();
        let gpu_result = input.std().unwrap().to_vec().unwrap();
        let cpu_result = std_cpu(&input_data);

        let error = (gpu_result[0] - cpu_result).abs();
        assert!(error < 1e-3, "Error {} exceeds threshold", error);
    }

    #[tokio::test]
    async fn test_std_dim() {
        let device = get_test_device().await;
        // Test 2D tensor: [[1, 2, 3], [4, 5, 6]]
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2, 3], device.clone())
            .await
            .unwrap();
        
        // Std along dim 0 (columns): std of [1,4], [2,5], [3,6]
        let result = input.std_dim(0, false).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 3);
        // Std of [1, 4] = sqrt(2.25) = 1.5
        assert!((result[0] - 1.5).abs() < 1e-4);
        // Std of [2, 5] = sqrt(2.25) = 1.5
        assert!((result[1] - 1.5).abs() < 1e-4);
        // Std of [3, 6] = sqrt(2.25) = 1.5
        assert!((result[2] - 1.5).abs() < 1e-4);
        
        // Std along dim 1 (rows): std of [1,2,3], [4,5,6]
        let result = input.std_dim(1, false).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 2);
        // Std of [1, 2, 3] = sqrt(0.666...) ≈ 0.8165
        assert!((result[0] - 0.8164966).abs() < 1e-4);
        // Std of [4, 5, 6] = sqrt(0.666...) ≈ 0.8165
        assert!((result[1] - 0.8164966).abs() < 1e-4);
        
        // Std along dim 0 with keepdim: [[1.5, 1.5, 1.5]]
        let result = input.std_dim(0, true).unwrap();
        assert_eq!(result.shape(), &[1, 3]);
    }
}
