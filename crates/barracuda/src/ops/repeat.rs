//! Repeat operation - Repeat tensor along axis
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RepeatParams {
    repeats: u32,
    input_size: u32,
    _padding: [u32; 2],
}

pub struct Repeat {
    input: Tensor,
    repeats: usize,
}

impl Repeat {
    pub fn new(input: Tensor, repeats: usize) -> Self {
        Self { input, repeats }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/repeat.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let input_size = self.input.len();
        let output_size = input_size * self.repeats;

        // Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create params
        let params = RepeatParams {
            repeats: self.repeats as u32,
            input_size: input_size as u32,
            _padding: [0; 2],
        };
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Repeat Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Repeat Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Repeat Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Repeat Bind Group"),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Repeat Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Repeat Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = ((output_size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        // Calculate output shape (repeat on last dimension)
        let mut output_shape = self.input.shape().to_vec();
        if let Some(last) = output_shape.last_mut() {
            *last *= self.repeats;
        }

        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape,
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Repeat tensor elements
    /// # Arguments
    /// * `repeats` - Number of times to repeat
    pub fn repeat(self, repeats: usize) -> Result<Self> {
        Repeat::new(self, repeats).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_repeat_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());
        
        // Create tensor [1, 2, 3]
        let input_data = vec![1.0f32, 2.0, 3.0];
        let input = Tensor::from_data(&input_data, vec![3], device.clone()).unwrap();
        
        // Repeat 3 times
        let result = input.repeat(3).unwrap();
        let output = result.to_vec().unwrap();
        
        // Expected: [1, 2, 3, 1, 2, 3, 1, 2, 3]
        assert_eq!(output.len(), 9);
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 2.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 1.0);
        assert_eq!(output[4], 2.0);
        assert_eq!(output[5], 3.0);
    }
}
