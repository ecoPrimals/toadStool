//! L1 Loss - Mean Absolute Error
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

pub struct L1Loss {
    predictions: Tensor,
    targets: Tensor,
}

impl L1Loss {
    pub fn new(predictions: Tensor, targets: Tensor) -> Self {
        Self {
            predictions,
            targets,
        }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/l1_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();

        // Create output buffer for single loss value
        let output_buffer = device.create_buffer_f32(1)?;

        // Create shader module
        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("L1 Loss Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        // Create compute pipeline
        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("L1 Loss Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("L1 Loss Bind Group"),
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
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("L1 Loss Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("L1 Loss Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, vec![1], device.clone()))
    }
}

impl Tensor {
    /// Compute L1 (Mean Absolute Error) loss
    /// # Arguments
    /// * `targets` - Target values
    pub fn l1_loss(self, targets: Tensor) -> Result<Self> {
        L1Loss::new(self, targets).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_l1_loss_basic() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Predictions: [1, 2, 3]
        let pred_data = vec![1.0f32, 2.0, 3.0];
        let predictions = Tensor::from_data(&pred_data, vec![3], device.clone()).unwrap();

        // Targets: [1, 2, 3] (perfect match)
        let target_data = vec![1.0f32, 2.0, 3.0];
        let targets = Tensor::from_data(&target_data, vec![3], device.clone()).unwrap();

        // L1 should be 0
        let result = predictions.l1_loss(targets).unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(output.len(), 1);
        assert!(output[0] < 0.001); // Should be ~0
    }

    #[tokio::test]
    async fn test_l1_loss_with_error() {
        let device = std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Predictions: [2, 4, 6]
        let pred_data = vec![2.0f32, 4.0, 6.0];
        let predictions = Tensor::from_data(&pred_data, vec![3], device.clone()).unwrap();

        // Targets: [1, 2, 3]
        let target_data = vec![1.0f32, 2.0, 3.0];
        let targets = Tensor::from_data(&target_data, vec![3], device.clone()).unwrap();

        // L1 = (|2-1| + |4-2| + |6-3|) / 3 = (1 + 2 + 3) / 3 = 2.0
        let result = predictions.l1_loss(targets).unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(output.len(), 1);
        assert!((output[0] - 2.0).abs() < 0.1);
    }
}
