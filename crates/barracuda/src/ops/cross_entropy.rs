//! Cross Entropy Loss
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

pub struct CrossEntropy {
    predictions: Tensor,
    targets: Tensor,
}

impl CrossEntropy {
    pub fn new(predictions: Tensor, targets: Tensor) -> Self {
        Self { predictions, targets }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/cross_entropy.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();

        // Create output buffer for single loss value
        let output_buffer = device.create_buffer_f32(1)?;

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Cross Entropy Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Cross Entropy Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cross Entropy Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.predictions.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.targets.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        // Execute
        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Cross Entropy Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cross Entropy Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![1],
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Compute Cross Entropy loss
    /// # Arguments
    /// * `targets` - Target probabilities
    pub fn cross_entropy(self, targets: Tensor) -> Result<Self> {
        CrossEntropy::new(self, targets).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_cross_entropy_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());
        
        // Predictions (probabilities): [0.7, 0.2, 0.1]
        let pred_data = vec![0.7f32, 0.2, 0.1];
        let predictions = Tensor::from_data(&pred_data, vec![3], device.clone()).unwrap();
        
        // Targets (one-hot): [1, 0, 0]
        let target_data = vec![1.0f32, 0.0, 0.0];
        let targets = Tensor::from_data(&target_data, vec![3], device.clone()).unwrap();
        
        // Cross entropy = -log(0.7) ≈ 0.357
        let result = predictions.cross_entropy(targets).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(output.len(), 1);
        assert!(output[0] > 0.0); // Should be positive
        assert!((output[0] - 0.357).abs() < 0.1);
    }
}
