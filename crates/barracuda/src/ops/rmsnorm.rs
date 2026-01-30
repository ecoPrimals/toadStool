//! RMSNorm - Root Mean Square Normalization
//! Pure WGSL implementation
//!
//! Simpler alternative to LayerNorm used in modern LLMs (LLaMA, GPT-NeoX, T5)
//! Formula: RMSNorm(x) = x / sqrt(mean(x²) + epsilon) * gamma
//!
//! Key difference from LayerNorm: No mean subtraction, only RMS scaling
//! Benefits: Faster computation, similar performance

use crate::tensor::Tensor;
use crate::error::Result;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct RMSNormParams {
    batch_size: u32,
    feature_size: u32,
    epsilon: f32,
    _padding: u32,
}

pub struct RMSNorm {
    input: Tensor,
    gamma: Tensor,  // Scale parameters
    epsilon: f32,
}

impl RMSNorm {
    pub fn new(input: Tensor, gamma: Tensor, epsilon: f32) -> Self {
        Self { input, gamma, epsilon }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/rmsnorm.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        
        // Assume input shape is [batch_size, feature_size]
        let batch_size = shape[0];
        let feature_size = shape[1];
        let output_size = batch_size * feature_size;

        // Create output buffer
        let output_buffer = device.create_buffer_f32(output_size)?;

        // Create params
        let params = RMSNormParams {
            batch_size: batch_size as u32,
            feature_size: feature_size as u32,
            epsilon: self.epsilon,
            _padding: 0,
        };
        let params_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("RMSNorm Params"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("RMSNorm Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("RMSNorm Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("RMSNorm Bind Group"),
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
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("RMSNorm Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("RMSNorm Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = ((batch_size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![batch_size, feature_size],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Apply RMS Normalization (used in LLaMA, GPT-NeoX, T5)
    /// # Arguments
    /// * `gamma` - Scale parameters (shape: [feature_size])
    /// * `epsilon` - Small constant for numerical stability (default: 1e-6)
    pub fn rmsnorm(self, gamma: Tensor, epsilon: f32) -> Result<Self> {
        RMSNorm::new(self, gamma, epsilon).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rmsnorm_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());
        
        // Create input [2, 4] - 2 samples, 4 features each
        let input_data = vec![
            1.0f32, 2.0, 3.0, 4.0,  // Sample 1
            2.0, 4.0, 6.0, 8.0,      // Sample 2
        ];
        let input = Tensor::from_data(&input_data, vec![2, 4], device.clone()).unwrap();
        
        // Create gamma (scale) parameters - one per feature
        let gamma_data = vec![1.0f32, 1.0, 1.0, 1.0];
        let gamma = Tensor::from_data(&gamma_data, vec![4], device.clone()).unwrap();
        
        // Apply RMSNorm
        let result = input.rmsnorm(gamma, 1e-6).unwrap();
        let output = result.to_vec().unwrap();
        
        // Output should be normalized
        assert_eq!(output.len(), 8);
        // Values should be scaled by RMS
        assert!(output[0].abs() > 0.0);
    }
}
