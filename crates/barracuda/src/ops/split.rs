//! Split - Tensor splitting operation
//! Pure WGSL implementation
//!
//! Splits a tensor into multiple parts along a dimension (inverse of Concat)
//! Formula: [output1, output2] = split(input, split_point)
//!
//! Used in: Multi-branch networks, Inception modules, ResNeXt
//! Benefits: Enables parallel processing paths, modular architecture design

use crate::tensor::Tensor;
use crate::error::Result;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct SplitParams {
    total_size: u32,
    split_point: u32,
    _pad: u32,
    _pad2: u32,
}

pub struct Split {
    input: Tensor,
    split_point: usize,
}

impl Split {
    pub fn new(input: Tensor, split_point: usize) -> Self {
        Self { input, split_point }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/split.wgsl")
    }

    pub fn execute(self) -> Result<(Tensor, Tensor)> {
        let device = self.input.device();
        let shape = self.input.shape();
        
        // For simplicity, split along the last dimension
        let total_size: usize = shape.iter().product();
        let size1 = self.split_point;
        let size2 = total_size - self.split_point;

        // Create output buffers
        let output1_buffer = device.create_buffer_f32(size1)?;
        let output2_buffer = device.create_buffer_f32(size2)?;

        // Create params
        let params = SplitParams {
            total_size: total_size as u32,
            split_point: self.split_point as u32,
            _pad: 0,
            _pad2: 0,
        };
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Split Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Split Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Split Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Split Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: output1_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output2_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Split Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Split Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = ((total_size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        // Determine output shapes (split along last dimension for simplicity)
        let mut shape1 = shape.to_vec();
        let mut shape2 = shape.to_vec();
        let last_dim = shape.len() - 1;
        let last_size = shape[last_dim];
        
        shape1[last_dim] = self.split_point;
        shape2[last_dim] = last_size - self.split_point;

        Ok((
            Tensor::from_buffer(output1_buffer, shape1, device.clone()),
            Tensor::from_buffer(output2_buffer, shape2, device.clone()),
        ))
    }
}

impl Tensor {
    /// Split tensor into two parts at the specified point
    /// # Arguments
    /// * `split_point` - Position to split (along last dimension)
    /// # Returns
    /// Tuple of two tensors (before split_point, after split_point)
    /// # Example
    /// ```ignore
    /// // Split [batch, 512] into [batch, 256] and [batch, 256]
    /// let (left, right) = tensor.split(256)?;
    /// ```
    pub fn split(self, split_point: usize) -> Result<(Self, Self)> {
        Split::new(self, split_point).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_split_basic() {
        let device = Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Create input [2, 4] - 2 samples, 4 features each
        let input_data = vec![
            1.0f32, 2.0, 3.0, 4.0,  // Sample 0
            5.0, 6.0, 7.0, 8.0,      // Sample 1
        ];
        let input = Tensor::from_data(&input_data, vec![2, 4], device.clone()).unwrap();

        // Split at position 2 (middle)
        let (left, right) = input.split(4).unwrap();  // Split total of 8 elements at 4

        let left_data = left.to_vec().unwrap();
        let right_data = right.to_vec().unwrap();

        // Left should be first 4 elements
        assert_eq!(left_data.len(), 4);
        assert_eq!(left_data[0], 1.0);

        // Right should be last 4 elements
        assert_eq!(right_data.len(), 4);
        assert_eq!(right_data[0], 5.0);
    }
}
