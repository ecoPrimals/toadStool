//! Sum reduction - Pure WGSL

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Sum {
    input: Tensor,
}

impl Sum {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/sum_simple.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let output_buffer = device.create_buffer_f32(1)?; // Single output value

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Sum BGL"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Sum BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Sum"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sum PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Sum Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sum Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sum Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, vec![1], device.clone()))
    }
}

impl Tensor {
    pub fn sum(self) -> Result<Self> {
        Sum::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn sum_cpu(input: &[f32]) -> f32 {
        input.iter().sum()
    }

    #[tokio::test]
    async fn test_sum_basic() {
        let device = get_test_device().await;
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        let expected = sum_cpu(&input_data);

        assert!(
            (result[0] - expected).abs() < 1e-4,
            "Expected {}, got {}",
            expected,
            result[0]
        );
    }

    #[tokio::test]
    async fn test_sum_edge_cases() {
        let device = get_test_device().await;

        // All zeros
        let input_data = vec![0.0, 0.0, 0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone())
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);

        // Mixed positive/negative
        let input_data = vec![5.0, -3.0, 2.0, -4.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        let expected = sum_cpu(&input_data);
        assert!((result[0] - expected).abs() < 1e-4);
    }

    #[tokio::test]
    async fn test_sum_boundary() {
        let device = get_test_device().await;
        let input_data = vec![1e6, 1e-6, -1e6, 1e-6];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        let expected = sum_cpu(&input_data);

        // Use relative error for large sums
        let rel_error = if expected.abs() > 1e-5 {
            (result[0] - expected).abs() / expected.abs()
        } else {
            (result[0] - expected).abs()
        };
        assert!(
            rel_error < 1e-3,
            "Expected {}, got {} (rel error {})",
            expected,
            result[0],
            rel_error
        );
    }

    #[tokio::test]
    async fn test_sum_large_tensor() {
        let device = get_test_device().await;
        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device)
            .await
            .unwrap();
        let result = input.sum().unwrap().to_vec().unwrap();
        let expected = sum_cpu(&input_data);

        let rel_error = (result[0] - expected).abs() / expected.abs();
        assert!(rel_error < 1e-3, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_sum_precision() {
        let device = get_test_device().await;
        let input_data = vec![1.5, 2.5, 3.5, 4.5, 5.5];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let gpu_result = input.sum().unwrap().to_vec().unwrap();
        let cpu_result = sum_cpu(&input_data);

        let error = (gpu_result[0] - cpu_result).abs();
        assert!(error < 1e-4, "Error {} exceeds threshold", error);
    }
}
