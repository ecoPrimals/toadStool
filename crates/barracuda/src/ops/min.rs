//! Min reduction - Pure WGSL

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Min {
    input: Tensor,
}

impl Min {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/min_simple.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let output_buffer = device.create_buffer_f32(1)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Min BGL"),
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
            label: Some("Min BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Min"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Min PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Min Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Min Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Min Pass"),
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
    pub fn min(self) -> Result<Self> {
        Min::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn min_cpu(input: &[f32]) -> f32 {
        input.iter().copied().fold(f32::INFINITY, f32::min)
    }

    #[tokio::test]
    async fn test_min_basic() {
        let device = get_test_device().await;
        let input_data = vec![5.0, 1.0, 9.0, 2.0, 3.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device).await.unwrap();
        let result = input.min().unwrap().to_vec().unwrap();
        let expected = min_cpu(&input_data);
        
        assert!((result[0] - expected).abs() < 1e-5, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_min_edge_cases() {
        let device = get_test_device().await;
        
        // All same value
        let input_data = vec![3.0, 3.0, 3.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone()).await.unwrap();
        let result = input.min().unwrap().to_vec().unwrap();
        assert!((result[0] - 3.0).abs() < 1e-6);
        
        // Negative values
        let input_data = vec![-5.0, -1.0, -9.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.min().unwrap().to_vec().unwrap();
        let expected = min_cpu(&input_data);
        assert!((result[0] - expected).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_min_boundary() {
        let device = get_test_device().await;
        let input_data = vec![1e10, 1e-10, 0.0, -1e10];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device).await.unwrap();
        let result = input.min().unwrap().to_vec().unwrap();
        let expected = min_cpu(&input_data);
        
        assert!((result[0] - expected).abs() < 1e-5, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_min_large_tensor() {
        let device = get_test_device().await;
        let size = 1000;
        let mut input_data: Vec<f32> = (0..size).map(|i| (i as f32) * 0.5).collect();
        input_data[500] = -100.0; // Insert min value
        
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device).await.unwrap();
        let result = input.min().unwrap().to_vec().unwrap();
        let expected = min_cpu(&input_data);
        
        assert!((result[0] - expected).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_min_precision() {
        let device = get_test_device().await;
        let input_data = vec![100.0, 50.0, 25.0, 12.5, 6.25, 3.125, 1.5625];
        let input = Tensor::from_vec_on(input_data.clone(), vec![7], device).await.unwrap();
        let gpu_result = input.min().unwrap().to_vec().unwrap();
        let cpu_result = min_cpu(&input_data);
        
        let error = (gpu_result[0] - cpu_result).abs();
        assert!(error < 1e-6, "Error {} exceeds threshold", error);
    }
}
