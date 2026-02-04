//! LogSumExp - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! Computes log-sum-exp with numerical stability.
//! Used in softmax, log-likelihood computations.

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// LogSumExp operation
pub struct LogSumExp {
    input: Tensor,
}

impl LogSumExp {
    /// Create a new logsumexp operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/logsumexp.wgsl")
    }

    /// Execute the logsumexp operation
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size: usize = self.input.shape().iter().product();

        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        // Output is a single scalar
        let output_buffer = device.create_buffer_f32(1)?;

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Metadata {
            size: u32,
        }

        let metadata = Metadata { size: size as u32 };

        let metadata_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("LogSumExp Metadata"),
            contents: bytemuck::cast_slice(&[metadata]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("LogSumExp Shader"));

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LogSumExp Bind Group Layout"),
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
            label: Some("LogSumExp Bind Group"),
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
                    resource: metadata_buffer.as_entire_binding(),
                },
            ],
        });

        // Create compute pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("LogSumExp Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let compute_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LogSumExp Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader_module,
            entry_point: "main",
        });

        // Execute compute shader
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LogSumExp Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LogSumExp Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Single workgroup for reduction
            compute_pass.dispatch_workgroups(1, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        device.device.poll(wgpu::Maintain::Wait);

        // Return tensor without reading back (zero-copy)
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![1],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Compute log-sum-exp (numerically stable)
    pub fn logsumexp(self) -> Result<Self> {
        LogSumExp::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_logsumexp_basic() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(
            vec![1.0, 2.0, 3.0, 4.0],
            vec![4],
            device,
        )
        .await
        .unwrap();

        let output = input.logsumexp().unwrap();
        let result = output.to_vec().unwrap();
        assert_eq!(result.len(), 1);
        assert!(result[0].is_finite());
        // LogSumExp should be >= max(input)
        assert!(result[0] >= 4.0);
    }

    #[tokio::test]
    async fn test_logsumexp_edge_cases() {
        let device = get_test_device().await;

        // Single element
        let input = Tensor::from_vec_on(
            vec![5.0],
            vec![1],
            device.clone(),
        )
        .await
        .unwrap();
        let output = input.logsumexp().unwrap();
        let result = output.to_vec().unwrap();
        assert_eq!(result.len(), 1);
        // LSE of single element is the element itself
        assert!((result[0] - 5.0).abs() < 0.01);

        // All zeros
        let input = Tensor::from_vec_on(
            vec![0.0, 0.0, 0.0],
            vec![3],
            device,
        )
        .await
        .unwrap();
        let output = input.logsumexp().unwrap();
        let result = output.to_vec().unwrap();
        assert!(result[0].is_finite());
    }

    #[tokio::test]
    async fn test_logsumexp_large_values() {
        let device = get_test_device().await;

        // Large values (test numerical stability)
        let input = Tensor::from_vec_on(
            vec![100.0, 101.0, 102.0],
            vec![3],
            device,
        )
        .await
        .unwrap();
        let output = input.logsumexp().unwrap();
        let result = output.to_vec().unwrap();
        assert!(result[0].is_finite());
        assert!(result[0] > 102.0);
    }
}
