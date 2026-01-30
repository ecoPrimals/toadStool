//! GELU (Gaussian Error Linear Unit) activation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: GELU(x) = x * Φ(x) where Φ is the cumulative distribution function of the standard normal distribution
//! Approximation: GELU(x) ≈ 0.5 * x * (1 + tanh(√(2/π) * (x + 0.044715 * x³)))

use crate::tensor::Tensor;
use crate::error::Result;

/// GELU activation operation
pub struct GELU {
    input: Tensor,
}

impl GELU {
    /// Create GELU operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/gelu.wgsl")
    }

    /// Execute GELU on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("GELU Bind Group Layout"),
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
            label: Some("GELU Bind Group"),
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
        let shader = device.compile_shader(Self::wgsl_shader(), Some("GELU"));

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("GELU Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GELU Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // Encode and execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("GELU Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GELU Pass"),
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
    /// Apply GELU activation
    pub fn gelu(self) -> Result<Self> {
        GELU::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_gelu_basic() {
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

        let output = input.gelu().unwrap();
        let result = output.to_vec().unwrap();

        // GELU properties:
        // GELU(0) ≈ 0
        // GELU(x) is smooth and monotonic
        // For positive x, GELU(x) ≈ x
        // For large negative x, GELU(x) ≈ 0
        assert!(result[2].abs() < 0.01); // GELU(0) ≈ 0
        assert!(result[3] > 0.8); // GELU(1) ≈ 0.84
        assert!(result[4] > 1.9); // GELU(2) ≈ 1.95
        assert!(result[0] > -0.06 && result[0] < 0.0); // GELU(-2) ≈ -0.045
        assert!(result[1] > -0.2 && result[1] < 0.0); // GELU(-1) ≈ -0.16
    }
}
