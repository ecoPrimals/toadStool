//! Cumsum operation - Cumulative sum
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Cumsum {
    input: Tensor,
}

impl Cumsum {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/cumsum.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create shader module
        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Cumsum Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        // Create compute pipeline
        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Cumsum Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cumsum Bind Group"),
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

        // Execute (single workgroup for sequential cumsum)
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Cumsum Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cumsum Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            compute_pass.dispatch_workgroups(1, 1, 1);
        }
        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Cumulative sum of elements
    pub fn cumsum(self) -> Result<Self> {
        Cumsum::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_cumsum_basic() {
        let device = Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Create tensor [1, 2, 3, 4]
        let input_data = vec![1.0f32, 2.0, 3.0, 4.0];
        let input = Tensor::from_data(&input_data, vec![4], device.clone()).unwrap();

        // Cumsum
        let result = input.cumsum().unwrap();
        let output = result.to_vec().unwrap();

        // Expected: [1, 3, 6, 10]
        assert_eq!(output.len(), 4);
        assert_eq!(output[0], 1.0);
        assert_eq!(output[1], 3.0);
        assert_eq!(output[2], 6.0);
        assert_eq!(output[3], 10.0);
    }

    #[tokio::test]
    async fn test_cumsum_edge_cases() {
        let device = Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Single element
        let input_data = vec![5.0f32];
        let input = Tensor::from_data(&input_data, vec![1], device.clone()).unwrap();
        let result = input.cumsum().unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output, vec![5.0]);

        // All zeros
        let input_data = vec![0.0f32, 0.0, 0.0];
        let input = Tensor::from_data(&input_data, vec![3], device.clone()).unwrap();
        let result = input.cumsum().unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output, vec![0.0, 0.0, 0.0]);
    }

    #[tokio::test]
    async fn test_cumsum_boundary() {
        let device = Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Negative numbers
        let input_data = vec![-1.0f32, -2.0, -3.0];
        let input = Tensor::from_data(&input_data, vec![3], device.clone()).unwrap();
        let result = input.cumsum().unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output, vec![-1.0, -3.0, -6.0]);

        // Mixed positive/negative
        let input_data = vec![1.0f32, -2.0, 3.0, -4.0];
        let input = Tensor::from_data(&input_data, vec![4], device.clone()).unwrap();
        let result = input.cumsum().unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output, vec![1.0, -1.0, 2.0, -2.0]);
    }

    #[tokio::test]
    async fn test_cumsum_large_batch() {
        let device = Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Larger tensor
        let size = 100;
        let input_data: Vec<f32> = (1..=size).map(|i| i as f32).collect();
        let input = Tensor::from_data(&input_data, vec![size], device.clone()).unwrap();

        let result = input.cumsum().unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(output.len(), size);
        // Last element should be sum of 1..100 = 5050
        assert_eq!(output[size - 1], 5050.0);
    }

    #[tokio::test]
    async fn test_cumsum_precision() {
        let device = Arc::new(crate::device::WgpuDevice::new().await.unwrap());

        // Test with fractional values
        let input_data = vec![0.1f32, 0.2, 0.3, 0.4];
        let input = Tensor::from_data(&input_data, vec![4], device.clone()).unwrap();

        let result = input.cumsum().unwrap();
        let output = result.to_vec().unwrap();

        // Expected: [0.1, 0.3, 0.6, 1.0]
        assert!((output[0] - 0.1).abs() < 1e-5);
        assert!((output[1] - 0.3).abs() < 1e-5);
        assert!((output[2] - 0.6).abs() < 1e-5);
        assert!((output[3] - 1.0).abs() < 1e-5);
    }
}
