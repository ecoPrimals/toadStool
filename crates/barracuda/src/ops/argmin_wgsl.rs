//! Argmin - Find indices of minimum values - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its dimension
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// Argmin operation - Find indices of minimum values along a dimension
pub struct Argmin {
    input: Tensor,
    dim: usize,
}

impl Argmin {
    /// Create a new argmin operation
    pub fn new(input: Tensor, dim: usize) -> Self {
        Self { input, dim }
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/argmin.wgsl")
    }

    /// Execute the argmin operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();

        // Calculate dimension parameters
        let dim_size = shape[self.dim];
        let outer_size: usize = shape[..self.dim].iter().product();
        let inner_size: usize = shape[self.dim + 1..].iter().product();
        
        let output_size = outer_size * inner_size;

        // Create buffers
        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Argmin Output"),
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

        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Argmin Params"),
            contents: bytemuck::cast_slice(&[params]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Argmin Bind Group Layout"),
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
            label: Some("Argmin Bind Group"),
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
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("Shader"));

        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Argmin Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Argmin Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });

        // Execute compute shader
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Argmin Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Argmin Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (output_size as u32 + 255) / 256;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Read back results
        let output_data = crate::utils::read_buffer_u32(device, &output_buffer, output_size)?;
        
        // Convert u32 to f32 for tensor
        let output_f32: Vec<f32> = output_data.iter().map(|&x| x as f32).collect();

        // Calculate output shape (remove the reduced dimension)
        let mut output_shape = shape.to_vec();
        output_shape.remove(self.dim);

        Ok(Tensor::new(
            output_f32,
            output_shape,
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Find indices of minimum values along a dimension
    ///
    /// # Arguments
    ///
    /// * `dim` - Dimension to find min along
    pub fn argmin_wgsl(self, dim: usize) -> Result<Self> {
        Argmin::new(self, dim).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> std::sync::Arc<crate::device::WgpuDevice> {
        use crate::device::test_pool::get_test_device;
        get_test_device().await
    }

    #[tokio::test]
    async fn test_argmin_1d() {
        let device = get_test_device().await;

        let data = vec![5.0, 1.0, 3.0, 2.0];
        let input = Tensor::new(data, vec![4], device.clone());

        let output = input.argmin_wgsl(0).unwrap();

        assert_eq!(output.shape(), &[] as &[usize]); // Scalar output
        let result = output.to_vec().unwrap();
        assert_eq!(result[0] as u32, 1); // Index of min value (1.0)
    }

    #[tokio::test]
    async fn test_argmin_2d_dim0() {
        let device = get_test_device().await;

        let data = vec![4.0, 6.0, 3.0, 2.0, 5.0, 1.0];
        let input = Tensor::new(data, vec![3, 2], device.clone());

        let output = input.argmin_wgsl(0).unwrap();

        assert_eq!(output.shape(), &[2]);
        let result = output.to_vec().unwrap();
        assert_eq!(result[0] as u32, 1); // Min in column 0 is at index 1 (value 3.0)
        assert_eq!(result[1] as u32, 2); // Min in column 1 is at index 2 (value 1.0)
    }
}
