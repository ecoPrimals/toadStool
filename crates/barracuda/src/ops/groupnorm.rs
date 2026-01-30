//! GroupNorm - Group Normalization
//! Pure WGSL implementation
//!
//! Modern alternative to BatchNorm that works well with small batch sizes
//! Formula: output = gamma * (input - group_mean) / sqrt(group_var + epsilon) + beta
//!
//! Used in: Transformers, ResNets, style transfer, generative models
//! Benefits: Batch-size independent, better for small batches than BatchNorm

use crate::tensor::Tensor;
use crate::error::Result;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GroupNormParams {
    batch_size: u32,
    channels: u32,
    spatial_size: u32,  // H * W
    num_groups: u32,
    channels_per_group: u32,
    epsilon: f32,
}

pub struct GroupNorm {
    input: Tensor,
    gamma: Tensor,  // Scale per channel
    beta: Tensor,   // Shift per channel
    num_groups: usize,
    epsilon: f32,
}

impl GroupNorm {
    pub fn new(input: Tensor, gamma: Tensor, beta: Tensor, num_groups: usize, epsilon: f32) -> Self {
        Self { input, gamma, beta, num_groups, epsilon }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/groupnorm.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        
        // Assume input shape is [batch, channels, height, width]
        let batch_size = shape[0];
        let channels = shape[1];
        let height = shape[2];
        let width = shape[3];
        let spatial_size = height * width;
        let output_size = batch_size * channels * spatial_size;
        let channels_per_group = channels / self.num_groups;

        // Create output and stats buffers
        let output_buffer = device.create_buffer_f32(output_size)?;
        let stats_size = batch_size * self.num_groups * 2;  // mean and variance per group
        let stats_buffer = device.create_buffer_f32(stats_size)?;

        // Create params
        let params = GroupNormParams {
            batch_size: batch_size as u32,
            channels: channels as u32,
            spatial_size: spatial_size as u32,
            num_groups: self.num_groups as u32,
            channels_per_group: channels_per_group as u32,
            epsilon: self.epsilon,
        };
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("GroupNorm Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("GroupNorm Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipelines for both passes
        let stats_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GroupNorm Stats Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "compute_stats",
        });

        let norm_pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("GroupNorm Normalize Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "normalize",
        });

        // Pass 1: Compute group statistics
        {
            let bind_group_layout = stats_pipeline.get_bind_group_layout(0);
            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GroupNorm Stats Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.input.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.gamma.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.beta.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: stats_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GroupNorm Stats Encoder"),
            });
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("GroupNorm Stats Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&stats_pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                let total_groups = batch_size * self.num_groups;
                compute_pass.dispatch_workgroups(1, 1, total_groups as u32);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // Pass 2: Normalize using statistics
        {
            let bind_group_layout = norm_pipeline.get_bind_group_layout(0);
            let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("GroupNorm Normalize Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.input.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: self.gamma.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: self.beta.buffer().as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: output_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: stats_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: params_buffer.as_entire_binding(),
                    },
                ],
            });

            let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("GroupNorm Normalize Encoder"),
            });
            {
                let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("GroupNorm Normalize Pass"),
                    timestamp_writes: None,
                });
                compute_pass.set_pipeline(&norm_pipeline);
                compute_pass.set_bind_group(0, &bind_group, &[]);
                let workgroups = ((output_size + 255) / 256) as u32;
                compute_pass.dispatch_workgroups(workgroups, 1, 1);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, channels, height, width],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply Group Normalization (modern alternative to BatchNorm)
    /// # Arguments
    /// * `gamma` - Scale parameters (shape: [channels])
    /// * `beta` - Shift parameters (shape: [channels])
    /// * `num_groups` - Number of groups to divide channels into
    /// * `epsilon` - Small constant for numerical stability (default: 1e-5)
    pub fn groupnorm(self, gamma: Tensor, beta: Tensor, num_groups: usize, epsilon: f32) -> Result<Self> {
        GroupNorm::new(self, gamma, beta, num_groups, epsilon).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_groupnorm_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());
        
        // Create input [1, 4, 2, 2] - 1 batch, 4 channels, 2x2 spatial
        let input_data = vec![
            1.0f32, 2.0, 3.0, 4.0,    // Channel 0
            5.0, 6.0, 7.0, 8.0,        // Channel 1
            9.0, 10.0, 11.0, 12.0,     // Channel 2
            13.0, 14.0, 15.0, 16.0,    // Channel 3
        ];
        let input = Tensor::from_data(&input_data, vec![1, 4, 2, 2], device.clone()).unwrap();
        
        // Create gamma and beta (one per channel)
        let gamma_data = vec![1.0f32, 1.0, 1.0, 1.0];
        let gamma = Tensor::from_data(&gamma_data, vec![4], device.clone()).unwrap();
        
        let beta_data = vec![0.0f32, 0.0, 0.0, 0.0];
        let beta = Tensor::from_data(&beta_data, vec![4], device.clone()).unwrap();
        
        // Apply GroupNorm with 2 groups (2 channels per group)
        let result = input.groupnorm(gamma, beta, 2, 1e-5).unwrap();
        let output = result.to_vec().unwrap();
        
        // Output should be normalized per group
        assert_eq!(output.len(), 16);
        assert!(output[0].abs() > 0.0);
    }
}
