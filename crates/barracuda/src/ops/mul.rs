//! Element-wise multiplication
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: C = A * B (element-wise, Hadamard product)

use crate::tensor::Tensor;
use crate::error::{BarracudaError, Result};

/// Element-wise multiplication operation
pub struct Mul {
    lhs: Tensor,
    rhs: Tensor,
}

impl Mul {
    /// Create Mul operation
    pub fn new(lhs: Tensor, rhs: Tensor) -> Result<Self> {
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
        include_str!("../shaders/elementwise_mul.wgsl")
    }

    /// Execute multiplication on tensors
    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        let size = self.lhs.len();

        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Mul Bind Group Layout"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Mul Bind Group"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Mul"));

        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Mul Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Mul Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Mul Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Mul Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            self.lhs.shape().to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Element-wise multiplication
    pub fn mul(&self, other: &Tensor) -> Result<Self> {
        Mul::new(self.clone(), other.clone())?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_mul_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        let lhs = Tensor::from_vec_on(
            vec![1.0, 2.0, 3.0, 4.0, 5.0],
            vec![5],
            device.clone(),
        )
        .await
        .unwrap();

        let rhs = Tensor::from_vec_on(
            vec![2.0, 3.0, 4.0, 5.0, 6.0],
            vec![5],
            device,
        )
        .await
        .unwrap();

        let output = lhs.mul(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        let expected = vec![2.0, 6.0, 12.0, 20.0, 30.0];
        for (i, (&r, &e)) in result.iter().zip(expected.iter()).enumerate() {
            assert!((r - e).abs() < 1e-5, "Mismatch at {}: {} vs {}", i, r, e);
        }
    }
}
