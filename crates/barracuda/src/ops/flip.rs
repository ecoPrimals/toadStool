//! Flip operation - Reverse order of elements
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Flip {
    input: Tensor,
}

impl Flip {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/flip.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Flip Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Flip Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flip Bind Group"),
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

        // Execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Flip Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Flip Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
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
    /// Flip/reverse tensor elements
    pub fn flip(self) -> Result<Self> {
        Flip::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_flip_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());
        
        // Create tensor [1, 2, 3, 4, 5]
        let input_data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_data(&input_data, vec![5], device.clone()).unwrap();
        
        // Flip
        let result = input.flip().unwrap();
        let output = result.to_vec().unwrap();
        
        // Expected: [5, 4, 3, 2, 1]
        assert_eq!(output.len(), 5);
        assert_eq!(output[0], 5.0);
        assert_eq!(output[1], 4.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 2.0);
        assert_eq!(output[4], 1.0);
    }
}
