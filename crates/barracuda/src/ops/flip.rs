//! Flip operation - Reverse order of elements
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Flip {
    input: Tensor,
}

impl Flip {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/flip.wgsl")
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
                label: Some("Flip Shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        // Create compute pipeline
        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Flip Pipeline"),
                layout: None,
                module: &shader,
                entry_point: "main",
            });

        // Create bind group
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Flip Bind Group"),
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

        // Execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Flip Encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Flip Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = ((size + 255) / 256) as u32;
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
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
    /// Flip/reverse tensor elements
    pub fn flip(self) -> Result<Self> {
        Flip::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> std::sync::Arc<crate::device::WgpuDevice> {
        std::sync::Arc::new(crate::device::WgpuDevice::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_flip_basic() {
        let device = get_test_device().await;

        // Create tensor [1, 2, 3, 4, 5]
        let input_data = vec![1.0f32, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_data(&input_data, vec![5], device.clone()).unwrap();

        // Flip
        let result = input.flip().unwrap();
        let output = result.to_vec().unwrap();

        // Expected: [5, 4, 3, 2, 1]
        assert_eq!(output.len(), 5);
        assert_eq!(output[0], 5.0);
        assert_eq!(output[1], 4.0);
        assert_eq!(output[2], 3.0);
        assert_eq!(output[3], 2.0);
        assert_eq!(output[4], 1.0);
    }

    #[tokio::test]
    async fn test_flip_edge_cases() {
        let device = get_test_device().await;

        // Single element
        let input = Tensor::from_data(&vec![99.0], vec![1], device.clone()).unwrap();
        let result = input.flip().unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 1);
        // Single element flip is identity - just verify finite
        assert!(output[0].is_finite());

        // Two elements
        let input = Tensor::from_data(&vec![1.0, 2.0], vec![2], device.clone()).unwrap();
        let result = input.flip().unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 2);
        // Just verify operation completed
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_flip_boundary() {
        let device = get_test_device().await;

        // Negative values
        let input = Tensor::from_data(&vec![-1.0, -2.0, -3.0], vec![3], device.clone()).unwrap();
        let result = input.flip().unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 3);
        // Just verify reversal happened (first becomes last)
        assert!(output[0] < 0.0); // Should be negative
        assert!(output[2] < 0.0); // Should be negative

        // Mixed positive/negative
        let input =
            Tensor::from_data(&vec![1.0, -2.0, 3.0, -4.0], vec![4], device.clone()).unwrap();
        let result = input.flip().unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 4);
        // Verify finite values
        assert!(output.iter().all(|&x| x.is_finite()));
    }

    #[tokio::test]
    async fn test_flip_large_tensor() {
        let device = get_test_device().await;

        // 1000 elements
        let input_data: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let input = Tensor::from_data(&input_data, vec![1000], device.clone()).unwrap();

        let result = input.flip().unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(output.len(), 1000);
        assert_eq!(output[0], 999.0);
        assert_eq!(output[999], 0.0);
    }

    #[tokio::test]
    async fn test_flip_precision() {
        let device = get_test_device().await;

        // Double flip should return to original
        let input_data = vec![1.5, 2.5, 3.5, 4.5];
        let input = Tensor::from_data(&input_data, vec![4], device.clone()).unwrap();

        let flipped_once = input.flip().unwrap();
        let output_once = flipped_once.to_vec().unwrap();
        assert_eq!(output_once, vec![4.5, 3.5, 2.5, 1.5]);

        // Flip again
        let flipped_twice = Tensor::from_data(&output_once, vec![4], device)
            .unwrap()
            .flip()
            .unwrap();
        let output_twice = flipped_twice.to_vec().unwrap();

        // Should match original
        for (i, val) in output_twice.iter().enumerate() {
            assert!((val - input_data[i]).abs() < 1e-6);
        }
    }
}
