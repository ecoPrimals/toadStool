//! ReLU (Rectified Linear Unit) activation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: ReLU(x) = max(0, x)

use crate::tensor::Tensor;
use crate::error::Result;

/// ReLU activation operation
pub struct ReLU {
    input: Tensor,
}

impl ReLU {
    /// Create ReLU operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/relu.wgsl")
    }

    /// Execute ReLU on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ReLU Bind Group Layout"),
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
            ],
        });

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ReLU Bind Group"),
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
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("ReLU"));

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ReLU Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ReLU Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // Encode and execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ReLU Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ReLU Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (256 threads per workgroup)
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

// Convenience method on Tensor
impl Tensor {
    /// Apply ReLU activation
    pub fn relu(self) -> Result<Self> {
        ReLU::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_relu_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Test data: [-2, -1, 0, 1, 2]
        let input = Tensor::from_vec_on(
            vec![-2.0, -1.0, 0.0, 1.0, 2.0],
            vec![5],
            device,
        )
        .await
        .unwrap();

        let output = input.relu().unwrap();
        let result = output.to_vec().unwrap();

        // Expected: [0, 0, 0, 1, 2]
        assert!((result[0] - 0.0).abs() < 1e-5);
        assert!((result[1] - 0.0).abs() < 1e-5);
        assert!((result[2] - 0.0).abs() < 1e-5);
        assert!((result[3] - 1.0).abs() < 1e-5);
        assert!((result[4] - 2.0).abs() < 1e-5);
    }
}
