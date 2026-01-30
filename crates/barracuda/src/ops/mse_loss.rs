//! MSE Loss - Mean Squared Error
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

pub struct MseLoss {
    predictions: Tensor,
    targets: Tensor,
}

impl MseLoss {
    pub fn new(predictions: Tensor, targets: Tensor) -> Self {
        Self { predictions, targets }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/mse_loss.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.predictions.device();

        // Create output buffer for single loss value
        let output_buffer = device.create_buffer_f32(1)?;

        // Create shader module
        let shader = device.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("MSE Loss Shader"),
            source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
        });

        // Create compute pipeline
        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("MSE Loss Pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("MSE Loss Bind Group"),
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
            label: Some("MSE Loss Encoder"),
        });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("MSE Loss Pass"),
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
    /// Compute Mean Squared Error loss
    /// # Arguments
    /// * `targets` - Target values
    pub fn mse_loss(self, targets: Tensor) -> Result<Self> {
        MseLoss::new(self, targets).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn mse_loss_cpu(predictions: &[f32], targets: &[f32]) -> f32 {
        let n = predictions.len() as f32;
        let sum: f32 = predictions.iter()
            .zip(targets.iter())
            .map(|(p, t)| (p - t).powi(2))
            .sum();
        sum / n
    }

    #[tokio::test]
    async fn test_mse_loss_basic() {
        let device = get_test_device().await;
        
        // Predictions: [1, 2, 3]
        let pred_data = vec![1.0f32, 2.0, 3.0];
        let predictions = Tensor::from_data(&pred_data, vec![3], device.clone()).unwrap();
        
        // Targets: [1, 2, 3] (perfect match)
        let target_data = vec![1.0f32, 2.0, 3.0];
        let targets = Tensor::from_data(&target_data, vec![3], device.clone()).unwrap();
        
        // MSE should be 0
        let result = predictions.mse_loss(targets).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(output.len(), 1);
        assert!(output[0] < 0.001); // Should be ~0
    }

    #[tokio::test]
    async fn test_mse_loss_with_error() {
        let device = get_test_device().await;
        
        // Predictions: [2, 4, 6]
        let pred_data = vec![2.0f32, 4.0, 6.0];
        let predictions = Tensor::from_data(&pred_data, vec![3], device.clone()).unwrap();
        
        // Targets: [1, 2, 3]
        let target_data = vec![1.0f32, 2.0, 3.0];
        let targets = Tensor::from_data(&target_data, vec![3], device.clone()).unwrap();
        
        // MSE = ((2-1)² + (4-2)² + (6-3)²) / 3 = (1 + 4 + 9) / 3 = 4.67
        let result = predictions.mse_loss(targets).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(output.len(), 1);
        assert!((output[0] - 4.67).abs() < 0.1);
    }

    #[tokio::test]
    async fn test_mse_loss_edge_cases() {
        let device = get_test_device().await;
        
        // Test with zeros
        let pred_data = vec![0.0f32, 0.0, 0.0];
        let predictions = Tensor::from_data(&pred_data, vec![3], device.clone()).unwrap();
        let target_data = vec![0.0f32, 0.0, 0.0];
        let targets = Tensor::from_data(&target_data, vec![3], device.clone()).unwrap();
        let result = predictions.mse_loss(targets).unwrap();
        let output = result.to_vec().unwrap();
        assert!(output[0] < 1e-6);

        // Test with negative values
        let pred_data = vec![-1.0f32, -2.0, -3.0];
        let predictions = Tensor::from_data(&pred_data, vec![3], device.clone()).unwrap();
        let target_data = vec![-1.5f32, -2.5, -3.5];
        let targets = Tensor::from_data(&target_data, vec![3], device.clone()).unwrap();
        let expected = mse_loss_cpu(&pred_data, &target_data);
        let result = predictions.mse_loss(targets).unwrap();
        let output = result.to_vec().unwrap();
        assert!((output[0] - expected).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_mse_loss_boundary() {
        let device = get_test_device().await;
        
        // Single element
        let pred_data = vec![5.0f32];
        let predictions = Tensor::from_data(&pred_data, vec![1], device.clone()).unwrap();
        let target_data = vec![3.0f32];
        let targets = Tensor::from_data(&target_data, vec![1], device.clone()).unwrap();
        let expected = mse_loss_cpu(&pred_data, &target_data);
        let result = predictions.mse_loss(targets).unwrap();
        let output = result.to_vec().unwrap();
        assert!((output[0] - expected).abs() < 1e-5);

        // Large error
        let pred_data = vec![100.0f32, 200.0, 300.0];
        let predictions = Tensor::from_data(&pred_data, vec![3], device.clone()).unwrap();
        let target_data = vec![0.0f32, 0.0, 0.0];
        let targets = Tensor::from_data(&target_data, vec![3], device.clone()).unwrap();
        let expected = mse_loss_cpu(&pred_data, &target_data);
        let result = predictions.mse_loss(targets).unwrap();
        let output = result.to_vec().unwrap();
        assert!((output[0] - expected).abs() < 1e-2);
    }

    #[tokio::test]
    async fn test_mse_loss_large_tensor() {
        let device = get_test_device().await;
        
        // 1000 elements
        let pred_data: Vec<f32> = (0..1000).map(|i| i as f32 * 0.1).collect();
        let target_data: Vec<f32> = (0..1000).map(|i| i as f32 * 0.1 + 0.5).collect();
        
        let predictions = Tensor::from_data(&pred_data, vec![1000], device.clone()).unwrap();
        let targets = Tensor::from_data(&target_data, vec![1000], device.clone()).unwrap();
        
        let expected = mse_loss_cpu(&pred_data, &target_data);
        let result = predictions.mse_loss(targets).unwrap();
        let output = result.to_vec().unwrap();
        
        assert_eq!(output.len(), 1);
        assert!((output[0] - expected).abs() < 1e-4);
    }

    #[tokio::test]
    async fn test_mse_loss_precision() {
        let device = get_test_device().await;
        
        // Test precision against CPU reference
        let pred_data = vec![1.234f32, 5.678, 9.012, 3.456, 7.890];
        let target_data = vec![1.111f32, 6.789, 8.901, 3.333, 8.000];
        
        let predictions = Tensor::from_data(&pred_data, vec![5], device.clone()).unwrap();
        let targets = Tensor::from_data(&target_data, vec![5], device.clone()).unwrap();
        
        let expected = mse_loss_cpu(&pred_data, &target_data);
        let result = predictions.mse_loss(targets).unwrap();
        let output = result.to_vec().unwrap();
        
        // Verify FP32 precision
        let error = (output[0] - expected).abs();
        assert!(error < 1e-5, "GPU MSE: {}, CPU MSE: {}, Error: {}", output[0], expected, error);
    }
}
