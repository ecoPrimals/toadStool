//! ELU (Exponential Linear Unit) activation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: ELU(x) = x if x > 0, α(e^x - 1) if x ≤ 0

use crate::tensor::Tensor;
use crate::error::Result;

/// ELU activation operation
pub struct ELU {
    input: Tensor,
}

impl ELU {
    /// Create ELU operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/elu_simple.wgsl")
    }

    /// Execute ELU on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ELU Bind Group Layout"),
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

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ELU Bind Group"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("ELU"));

        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ELU Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("ELU Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ELU Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("ELU Pass"),
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
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply ELU activation
    pub fn elu(self) -> Result<Self> {
        ELU::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_elu_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        let input = Tensor::from_vec_on(
            vec![-2.0, -1.0, 0.0, 1.0, 2.0],
            vec![5],
            device,
        )
        .await
        .unwrap();

        let output = input.elu().unwrap();
        let result = output.to_vec().unwrap();

        // ELU properties:
        // ELU(x) = x for x > 0
        assert!((result[3] - 1.0).abs() < 1e-5);
        assert!((result[4] - 2.0).abs() < 1e-5);
        // ELU(x) = α(e^x - 1) for x ≤ 0, α typically 1.0
        // ELU(0) = 0
        assert!(result[2].abs() < 1e-5);
    }
}
