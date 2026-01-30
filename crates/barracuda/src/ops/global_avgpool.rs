//! GlobalAvgPool - Global Average Pooling
//! Pure WGSL implementation
//!
//! Reduces spatial dimensions (H × W) to 1×1 by averaging
//! Formula: output[b, c] = mean(input[b, c, :, :])
//!
//! Used in: Modern CNNs (ResNet, EfficientNet) as replacement for FC layers
//! Benefits: Reduces parameters dramatically, increases spatial invariance

use crate::tensor::Tensor;
use crate::error::Result;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GlobalAvgPoolParams {
    batch_size: u32,
    channels: u32,
    height: u32,
    width: u32,
}

pub struct GlobalAvgPool {
    input: Tensor,
}

impl GlobalAvgPool {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/global_avgpool.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();

        // Input shape: [batch, channels, height, width]
        let batch_size = shape[0];
        let channels = shape[1];
        let height = shape[2];
        let width = shape[3];

        let output_size = batch_size * channels;

        // Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create params
        let params = GlobalAvgPoolParams {
            batch_size: batch_size as u32,
            channels: channels as u32,
            height: height as u32,
            width: width as u32,
        };
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GlobalAvgPool Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GlobalAvgPool Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GlobalAvgPool Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GlobalAvgPool Bind Group"),
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
            label: Some("GlobalAvgPool Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("GlobalAvgPool Pass"),
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
            vec![batch_size, channels, 1, 1],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply global average pooling (reduce spatial dimensions to 1×1)
    /// Used in modern CNN architectures as replacement for fully connected layers
    /// # Returns
    /// Tensor with shape [batch, channels, 1, 1]
    pub fn global_avgpool(self) -> Result<Self> {
        GlobalAvgPool::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_global_avgpool_basic() {
        let device = Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Create input [1, 2, 2, 2] - 1 batch, 2 channels, 2×2 spatial
        let input_data = vec![
            1.0f32, 2.0, 3.0, 4.0,  // Channel 0: [[1,2],[3,4]]
            5.0, 6.0, 7.0, 8.0,      // Channel 1: [[5,6],[7,8]]
        ];
        let input = Tensor::from_data(&input_data, vec![1, 2, 2, 2], device.clone()).unwrap();

        // Apply GlobalAvgPool
        let result = input.global_avgpool().unwrap();
        let output = result.to_vec().unwrap();

        // Output shape should be [1, 2, 1, 1]
        assert_eq!(result.shape(), &[1, 2, 1, 1]);
        assert_eq!(output.len(), 2);

        // Channel 0 average: (1+2+3+4)/4 = 2.5
        // Channel 1 average: (5+6+7+8)/4 = 6.5
        assert!((output[0] - 2.5).abs() < 0.01);
        assert!((output[1] - 6.5).abs() < 0.01);
    }
}
