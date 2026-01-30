//! Softmax activation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: softmax(x_i) = exp(x_i) / Σ exp(x_j)

use crate::tensor::Tensor;
use crate::error::{BarracudaError, Result};

/// Softmax activation operation
pub struct Softmax {
    input: Tensor,
}

impl Softmax {
    /// Create Softmax operation
    pub fn new(input: Tensor) -> Result<Self> {
        // Softmax expects 1D or last dimension for now
        if input.shape().is_empty() {
            return Err(BarracudaError::invalid_op(
                "Softmax",
                "Empty tensor not supported"
            ));
        }
        Ok(Self { input })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/softmax_simple.wgsl")
    }

    /// Execute Softmax on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create bind group layout
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Softmax Bind Group Layout"),
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
            label: Some("Softmax Bind Group"),
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
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Softmax"));

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Softmax Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Softmax Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // Encode and execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Softmax Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softmax Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Softmax uses single workgroup for reduction
            pass.dispatch_workgroups(1, 1, 1);
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
    /// Apply Softmax activation
    pub fn softmax(self) -> Result<Self> {
        Softmax::new(self)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_softmax_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Test data: [1, 2, 3]
        let input = Tensor::from_vec_on(
            vec![1.0, 2.0, 3.0],
            vec![3],
            device,
        )
        .await
        .unwrap();

        let output = input.softmax().unwrap();
        let result = output.to_vec().unwrap();

        // Softmax properties:
        // 1. All values in range (0, 1)
        // 2. Sum equals 1
        assert!(result.iter().all(|&x| x > 0.0 && x < 1.0));
        let sum: f32 = result.iter().sum();
        assert!((sum - 1.0).abs() < 1e-5, "Sum should be 1, got {}", sum);
        
        // Larger values should have larger probabilities
        assert!(result[2] > result[1]);
        assert!(result[1] > result[0]);
    }
}
