//! Transpose operation - N-Dimensional transpose with arbitrary permutations
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Supports both 2D transpose (swap last two dims) and N-D with permutation

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Transpose operation parameters (2D)
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TransposeParams2D {
    rows: u32,
    cols: u32,
    _padding: [u32; 2],
}

/// Transpose operation
pub struct Transpose {
    input: Tensor,
    permutation: Option<Vec<usize>>,
}

impl Transpose {
    /// Create Transpose operation
    /// 
    /// For 2D tensors: swaps rows and columns (default behavior)
    /// For N-D tensors: requires permutation vector specifying dimension order
    pub fn new(input: Tensor) -> Result<Self> {
        Ok(Self {
            input,
            permutation: None,
        })
    }

    /// Create Transpose operation with explicit permutation
    /// 
    /// # Arguments
    /// * `input` - Input tensor
    /// * `permutation` - Dimension permutation (e.g., [0, 2, 1] swaps dims 1 and 2)
    pub fn with_permutation(input: Tensor, permutation: Vec<usize>) -> Result<Self> {
        let num_dims = input.shape().len();
        if permutation.len() != num_dims {
            return Err(BarracudaError::invalid_op(
                "Transpose",
                format!("Permutation length {} doesn't match tensor rank {}", 
                    permutation.len(), num_dims),
            ));
        }

        // Validate permutation
        let mut seen = vec![false; num_dims];
        for &idx in &permutation {
            if idx >= num_dims {
                return Err(BarracudaError::invalid_op(
                    "Transpose",
                    format!("Invalid permutation index {} for rank {}", idx, num_dims),
                ));
            }
            if seen[idx] {
                return Err(BarracudaError::invalid_op(
                    "Transpose",
                    format!("Duplicate index {} in permutation", idx),
                ));
            }
            seen[idx] = true;
        }

        Ok(Self {
            input,
            permutation: Some(permutation),
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/transpose.wgsl")
    }

    /// Execute transpose on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let num_dims = shape.len();
        let size = self.input.len();

        // Determine if 2D or N-D
        let is_2d = num_dims == 2 && self.permutation.is_none();

        if is_2d {
            // Optimized 2D transpose
            self.execute_2d(&device, shape, size)
        } else {
            // N-D transpose with permutation
            let permutation = self.permutation.clone().unwrap_or_else(|| {
                // Default: swap last two dimensions
                let mut perm: Vec<usize> = (0..num_dims).collect();
                if num_dims >= 2 {
                    perm.swap(num_dims - 2, num_dims - 1);
                }
                perm
            });
            self.execute_nd(&device, shape, size, permutation)
        }
    }

    fn execute_2d(&self, device: &std::sync::Arc<crate::device::WgpuDevice>, shape: &[usize], size: usize) -> Result<Tensor> {
        let rows = shape[0] as u32;
        let cols = shape[1] as u32;

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create params buffer
        let params_2d = TransposeParams2D {
            rows,
            cols,
            _padding: [0, 0],
        };
        let params_bytes = bytemuck::bytes_of(&params_2d);
        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transpose Params 2D"),
            size: params_bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, params_bytes);

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transpose Bind Group Layout 2D"),
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
                    binding: 3,
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
            label: Some("Transpose Bind Group 2D"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Transpose 2D"));

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Transpose Pipeline Layout 2D"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Transpose Pipeline 2D"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main_2d",
        });

        // Encode and execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Transpose Encoder 2D"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Transpose Pass 2D"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups for 2D tiled transpose
            let workgroups_x = (cols as u32 + 15) / 16;
            let workgroups_y = (rows as u32 + 15) / 16;
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor with transposed shape
        let new_shape = vec![shape[1], shape[0]];
        Ok(Tensor::from_buffer(
            output_buffer,
            new_shape,
            device.clone(),
        ))
    }

    fn execute_nd(&self, device: &std::sync::Arc<crate::device::WgpuDevice>, shape: &[usize], size: usize, permutation: Vec<usize>) -> Result<Tensor> {
        let num_dims = shape.len();

        // Compute output shape
        let output_shape: Vec<usize> = permutation.iter()
            .map(|&idx| shape[idx])
            .collect();

        // Compute input strides
        let mut input_strides = vec![1; num_dims];
        for i in (0..num_dims - 1).rev() {
            input_strides[i] = input_strides[i + 1] * shape[i + 1];
        }

        // Compute output strides
        let mut output_strides = vec![1; num_dims];
        for i in (0..num_dims - 1).rev() {
            output_strides[i] = output_strides[i + 1] * output_shape[i + 1];
        }

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create buffers for shape and stride data
        let input_shape_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Input Shape"),
            contents: bytemuck::cast_slice(&shape.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let output_shape_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Output Shape"),
            contents: bytemuck::cast_slice(&output_shape.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let permutation_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Permutation"),
            contents: bytemuck::cast_slice(&permutation.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let input_strides_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Input Strides"),
            contents: bytemuck::cast_slice(&input_strides.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        let output_strides_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Output Strides"),
            contents: bytemuck::cast_slice(&output_strides.iter().map(|&x| x as u32).collect::<Vec<_>>()),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        });

        // Create params
        #[repr(C)]
        #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            total_size: u32,
            num_dims: u32,
            is_2d: u32,
            _padding: u32,
        }

        let params = Params {
            total_size: size as u32,
            num_dims: num_dims as u32,
            is_2d: 0,
            _padding: 0,
        };

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Transpose Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Transpose Bind Group Layout ND"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 7,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 8,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Transpose Bind Group ND"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: input_shape_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: output_shape_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: permutation_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 7,
                    resource: input_strides_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 8,
                    resource: output_strides_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Transpose ND"));

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Transpose Pipeline Layout ND"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Transpose Pipeline ND"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main_nd",
        });

        // Encode and execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Transpose Encoder ND"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Transpose Pass ND"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (256 threads per workgroup)
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor with transposed shape
        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape,
            std::sync::Arc::clone(device),
        ))
    }
}

// Convenience method on Tensor
impl Tensor {
    /// Transpose tensor (swap last two dimensions for 2D, or use permutation for N-D)
    pub fn transpose(&self) -> Result<Self> {
        Transpose::new(self.clone())?.execute()
    }

    /// Transpose tensor with explicit permutation
    pub fn transpose_with_permutation(&self, permutation: Vec<usize>) -> Result<Self> {
        Transpose::with_permutation(self.clone(), permutation)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_transpose_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Test data: 2x3 matrix [[1,2,3], [4,5,6]]
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3], device)
            .await
            .unwrap();

        let output = input.transpose().unwrap();
        let result = output.to_vec().unwrap();

        // Expected: 3x2 matrix [[1,4], [2,5], [3,6]]
        let expected = vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0];
        assert_eq!(output.shape(), &[3, 2]);
        for (i, (&r, &e)) in result.iter().zip(expected.iter()).enumerate() {
            assert!(
                (r - e).abs() < 1e-5,
                "Mismatch at index {}: {} vs {}",
                i,
                r,
                e
            );
        }
    }

    #[tokio::test]
    async fn test_transpose_nd() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Test 3D transpose: [B, C, H] -> [B, H, C]
        let input = Tensor::from_vec_on(
            (0..24).map(|i| i as f32).collect(),
            vec![2, 3, 4],
            device.clone(),
        )
        .await
        .unwrap();

        let output = input.transpose_with_permutation(vec![0, 2, 1]).unwrap();
        assert_eq!(output.shape(), &[2, 4, 3]);
    }
}
