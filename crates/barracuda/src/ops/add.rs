//! Element-wise addition
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: C = A + B (element-wise)

use crate::tensor::Tensor;
use crate::error::{BarracudaError, Result};

/// Element-wise addition operation
pub struct Add {
    lhs: Tensor,
    rhs: Tensor,
}

impl Add {
    /// Create Add operation
    pub fn new(lhs: Tensor, rhs: Tensor) -> Result<Self> {
        // Verify shapes match
        if lhs.shape() != rhs.shape() {
            return Err(BarracudaError::shape_mismatch(
                lhs.shape().to_vec(),
                rhs.shape().to_vec()
            ));
        }
        Ok(Self { lhs, rhs })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/elementwise_add.wgsl")
    }

    /// Execute addition on tensors
    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        let size = self.lhs.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create bind group layout (3 buffers: lhs, rhs, output)
        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Add Bind Group Layout"),
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
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
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
            label: Some("Add Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.lhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.rhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Add"));

        // Create pipeline
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Add Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Add Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        // Encode and execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Add Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Add Pass"),
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
            self.lhs.shape().to_vec(),
            device.clone(),
        ))
    }
}

// Convenience methods on Tensor
impl Tensor {
    /// Element-wise addition
    pub fn add(&self, other: &Tensor) -> Result<Self> {
        Add::new(self.clone(), other.clone())?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_add_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Test data: [1, 2, 3, 4, 5] + [10, 20, 30, 40, 50]
        let lhs = Tensor::from_vec_on(
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            vec![5],
            device.clone(),
        )
        .await
        .unwrap();

        let rhs = Tensor::from_vec_on(
            vec![10.0, 20.0, 30.0, 40.0, 50.0],
            vec![5],
            device,
        )
        .await
        .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        // Expected: [11, 22, 33, 44, 55]
        let expected = vec![11.0, 22.0, 33.0, 44.0, 55.0];
        for (i, (&r, &e)) in result.iter().zip(expected.iter()).enumerate() {
            assert!((r - e).abs() < 1e-5, "Mismatch at index {}: {} vs {}", i, r, e);
        }
    }
}
