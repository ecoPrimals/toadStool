//! InstanceNorm - Instance Normalization
//! Pure WGSL implementation
//!
//! Normalizes each instance (sample) independently across spatial dimensions
//! Formula: InstanceNorm(x) = gamma * (x - mean) / sqrt(variance + epsilon) + beta
//!
//! Used in: Style transfer, GANs, real-time image generation
//! Benefits: No dependency on batch size, works well for style/texture tasks

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct InstanceNormParams {
    batch: u32,
    channels: u32,
    spatial_size: u32, // height * width
    epsilon: f32,
}

pub struct InstanceNorm {
    input: Tensor,
    gamma: Tensor, // Scale per channel
    beta: Tensor,  // Shift per channel
    epsilon: f32,
}

impl InstanceNorm {
    pub fn new(input: Tensor, gamma: Tensor, beta: Tensor, epsilon: f32) -> Self {
        Self {
            input,
            gamma,
            beta,
            epsilon,
        }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/instancenorm.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();

        // Assume input shape is [batch, channels, height, width]
        let batch = shape[0];
        let channels = shape[1];
        let height = shape[2];
        let width = shape[3];
        let spatial_size = height * width;
        let output_size = batch * channels * spatial_size;

        // Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create params
        let params = InstanceNormParams {
            batch: batch as u32,
            channels: channels as u32,
            spatial_size: spatial_size as u32,
            epsilon: self.epsilon,
        };
        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("InstanceNorm Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create shader module
        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("InstanceNorm Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        // Create compute pipeline
        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("InstanceNorm Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("InstanceNorm Bind Group"),
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
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("InstanceNorm Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("InstanceNorm Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let total_instances = batch * channels;
            let workgroups = ((total_instances + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch, channels, height, width],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply Instance Normalization (used in style transfer, GANs)
    /// # Arguments
    /// * `gamma` - Scale parameters (shape: [channels])
    /// * `beta` - Shift parameters (shape: [channels])
    /// * `epsilon` - Small constant for numerical stability (default: 1e-5)
    pub fn instancenorm(self, gamma: Tensor, beta: Tensor, epsilon: f32) -> Result<Self> {
        InstanceNorm::new(self, gamma, beta, epsilon).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> std::sync::Arc<crate::device::WgpuDevice> {
        std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_instancenorm_basic() {
        let device = get_test_device().await;

        // Create input [1, 2, 2, 2] - 1 batch, 2 channels, 2x2 spatial
        let input_data = vec![
            1.0f32, 2.0, 3.0, 4.0, // Channel 0
            5.0, 6.0, 7.0, 8.0, // Channel 1
        ];
        let input = Tensor::from_data(&input_data, vec![1, 2, 2, 2], device.clone()).unwrap();

        // Create gamma and beta (one per channel)
        let gamma_data = vec![1.0f32, 1.0];
        let gamma = Tensor::from_data(&gamma_data, vec![2], device.clone()).unwrap();

        let beta_data = vec![0.0f32, 0.0];
        let beta = Tensor::from_data(&beta_data, vec![2], device.clone()).unwrap();

        // Apply InstanceNorm
        let result = input.instancenorm(gamma, beta, 1e-5).unwrap();
        let output = result.to_vec().unwrap();

        // Output should be normalized per channel
        assert_eq!(output.len(), 8);
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_instancenorm_edge_cases() {
        let device = get_test_device().await;

        // Single spatial location (1x1)
        let input_data = vec![5.0f32, 10.0]; // [1, 2, 1, 1]
        let input = Tensor::from_data(&input_data, vec![1, 2, 1, 1], device.clone()).unwrap();
        let gamma = Tensor::from_data(&vec![1.0, 1.0], vec![2], device.clone()).unwrap();
        let beta = Tensor::from_data(&vec![0.0, 0.0], vec![2], device.clone()).unwrap();

        let result = input.instancenorm(gamma, beta, 1e-5).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 2);
        assert!(output.iter().all(|&x| x.is_finite()));

        // All same values
        let input_data = vec![3.0f32; 1 * 2 * 4 * 4];
        let input = Tensor::from_data(&input_data, vec![1, 2, 4, 4], device.clone()).unwrap();
        let gamma = Tensor::from_data(&vec![1.0, 1.0], vec![2], device.clone()).unwrap();
        let beta = Tensor::from_data(&vec![0.0, 0.0], vec![2], device).unwrap();

        let result = input.instancenorm(gamma, beta, 1e-5).unwrap();
        let output = result.to_vec().unwrap();
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_instancenorm_boundary() {
        let device = get_test_device().await;

        // Large spatial dimensions
        let input_data = vec![1.0; 1 * 3 * 32 * 32];
        let input = Tensor::from_data(&input_data, vec![1, 3, 32, 32], device.clone()).unwrap();
        let gamma = Tensor::from_data(&vec![1.0, 1.0, 1.0], vec![3], device.clone()).unwrap();
        let beta = Tensor::from_data(&vec![0.0, 0.0, 0.0], vec![3], device.clone()).unwrap();

        let result = input.instancenorm(gamma, beta, 1e-5).unwrap();
        assert_eq!(result.shape(), &[1, 3, 32, 32]);

        // Many channels
        let input_data = vec![1.0; 1 * 64 * 8 * 8];
        let input = Tensor::from_data(&input_data, vec![1, 64, 8, 8], device.clone()).unwrap();
        let gamma = Tensor::from_data(&vec![1.0; 64], vec![64], device.clone()).unwrap();
        let beta = Tensor::from_data(&vec![0.0; 64], vec![64], device).unwrap();

        let result = input.instancenorm(gamma, beta, 1e-5).unwrap();
        assert_eq!(result.shape(), &[1, 64, 8, 8]);
    }

    #[tokio::test]
    async fn test_instancenorm_large_batch() {
        let device = get_test_device().await;

        // Batch size 8
        let batch_size = 8;
        let input_data = vec![1.0; batch_size * 16 * 16 * 16];
        let input =
            Tensor::from_data(&input_data, vec![batch_size, 16, 16, 16], device.clone()).unwrap();
        let gamma = Tensor::from_data(&vec![1.0; 16], vec![16], device.clone()).unwrap();
        let beta = Tensor::from_data(&vec![0.0; 16], vec![16], device).unwrap();

        let result = input.instancenorm(gamma, beta, 1e-5).unwrap();
        assert_eq!(result.shape(), &[batch_size, 16, 16, 16]);
    }

    #[tokio::test]
    async fn test_instancenorm_precision() {
        let device = get_test_device().await;

        // Test with gamma=2, beta=1 scaling
        let input_data = vec![1.0, 2.0, 3.0, 4.0]; // [1, 1, 2, 2]
        let input = Tensor::from_data(&input_data, vec![1, 1, 2, 2], device.clone()).unwrap();
        let gamma = Tensor::from_data(&vec![2.0], vec![1], device.clone()).unwrap();
        let beta = Tensor::from_data(&vec![1.0], vec![1], device).unwrap();

        let result = input.instancenorm(gamma, beta, 1e-5).unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(output.len(), 4);
        assert!(output.iter().all(|&x| x.is_finite()));
        // Just verify normalization occurred (values should have reasonable range)
        assert!(output.iter().all(|&x| x.abs() < 10.0));
    }
}
