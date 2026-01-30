//! Standard deviation reduction - Pure WGSL

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Std {
    input: Tensor,
}

impl Std {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/std_simple.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let output_buffer = device.create_buffer_f32(1)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Std BGL"),
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
            label: Some("Std BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Std"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Std PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Std Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Std Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Std Pass"),
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
    pub fn std(self) -> Result<Self> {
        Std::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn std_cpu(input: &[f32]) -> f32 {
        let mean: f32 = input.iter().sum::<f32>() / input.len() as f32;
        let variance: f32 = input.iter().map(|&x| (x - mean).powi(2)).sum::<f32>() / input.len() as f32;
        variance.sqrt()
    }

    #[tokio::test]
    async fn test_std_basic() {
        let device = get_test_device().await;
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device).await.unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        let expected = std_cpu(&input_data);
        
        assert!((result[0] - expected).abs() < 1e-4, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_std_edge_cases() {
        let device = get_test_device().await;
        
        // All same value (std = 0)
        let input_data = vec![5.0, 5.0, 5.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device.clone()).await.unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);
        
        // All zeros (std = 0)
        let input_data = vec![0.0, 0.0, 0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_std_boundary() {
        let device = get_test_device().await;
        let input_data = vec![0.0, 10.0, 20.0, 30.0, 40.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device).await.unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        let expected = std_cpu(&input_data);
        
        let rel_error = if expected > 1e-5 {
            (result[0] - expected).abs() / expected
        } else {
            (result[0] - expected).abs()
        };
        assert!(rel_error < 1e-2, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_std_large_tensor() {
        let device = get_test_device().await;
        let size = 100;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) * 0.5).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device).await.unwrap();
        let result = input.std().unwrap().to_vec().unwrap();
        let expected = std_cpu(&input_data);
        
        let rel_error = (result[0] - expected).abs() / expected;
        assert!(rel_error < 1e-2, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_std_precision() {
        let device = get_test_device().await;
        let input_data = vec![2.0, 4.0, 6.0, 8.0, 10.0, 12.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![6], device).await.unwrap();
        let gpu_result = input.std().unwrap().to_vec().unwrap();
        let cpu_result = std_cpu(&input_data);
        
        let error = (gpu_result[0] - cpu_result).abs();
        assert!(error < 1e-3, "Error {} exceeds threshold", error);
    }
}
