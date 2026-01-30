//! Broadcast operation - Expand tensor dimensions
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Broadcast {
    input: Tensor,
    target_shape: Vec<usize>,
}

impl Broadcast {
    pub fn new(input: Tensor, target_shape: Vec<usize>) -> Self {
        Self { input, target_shape }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/broadcast.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let output_size: usize = self.target_shape.iter().product();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Broadcast Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Broadcast Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Broadcast Bind Group"),
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
            label: Some("Broadcast Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Broadcast Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = ((output_size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            self.target_shape,
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Broadcast tensor to target shape
    /// # Arguments
    /// * `target_shape` - Target shape to broadcast to
    pub fn broadcast(self, target_shape: Vec<usize>) -> Result<Self> {
        Broadcast::new(self, target_shape).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_broadcast_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());
        
        // Create scalar [5.0]
        let input_data = vec![5.0f32];
        let input = Tensor::from_data(&input_data, vec![1], device.clone()).unwrap();
        
        // Broadcast to shape [10]
        let result = input.broadcast(vec![10]).unwrap();
        let output = result.to_vec().unwrap();
        
        // All should be 5.0
        assert_eq!(output.len(), 10);
        for val in output.iter() {
            assert_eq!(*val, 5.0);
        }
    }
}
