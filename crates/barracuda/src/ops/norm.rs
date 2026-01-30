//! L2 Norm reduction - Pure WGSL

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Norm {
    input: Tensor,
}

impl Norm {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/norm_simple.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let output_buffer = device.create_buffer_f32(1)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Norm BGL"),
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
            label: Some("Norm BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Norm"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Norm PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Norm Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Norm Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Norm Pass"),
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
    pub fn norm(self) -> Result<Self> {
        Norm::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn norm_cpu(input: &[f32]) -> f32 {
        let sum_sq: f32 = input.iter().map(|&x| x * x).sum();
        sum_sq.sqrt()
    }

    #[tokio::test]
    async fn test_norm_basic() {
        let device = get_test_device().await;
        let input_data = vec![3.0, 4.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2], device).await.unwrap();
        let result = input.norm().unwrap().to_vec().unwrap();
        let expected = norm_cpu(&input_data);
        
        assert!((result[0] - expected).abs() < 1e-5, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_norm_edge_cases() {
        let device = get_test_device().await;
        
        // All zeros (norm = 0)
        let input_data = vec![0.0, 0.0, 0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone()).await.unwrap();
        let result = input.norm().unwrap().to_vec().unwrap();
        assert!(result[0].abs() < 1e-6);
        
        // Single element
        let input_data = vec![5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![1], device).await.unwrap();
        let result = input.norm().unwrap().to_vec().unwrap();
        let expected = norm_cpu(&input_data);
        assert!((result[0] - expected).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_norm_boundary() {
        let device = get_test_device().await;
        let input_data = vec![1e5, 1e-5, -1e5, 1e-5];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device).await.unwrap();
        let result = input.norm().unwrap().to_vec().unwrap();
        let expected = norm_cpu(&input_data);
        
        let rel_error = (result[0] - expected).abs() / expected;
        assert!(rel_error < 1e-3, "Expected {}, got {} (rel error {})", expected, result[0], rel_error);
    }

    #[tokio::test]
    async fn test_norm_large_tensor() {
        let device = get_test_device().await;
        let size = 100;
        let input_data: Vec<f32> = (1..=size).map(|i| i as f32).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device).await.unwrap();
        let result = input.norm().unwrap().to_vec().unwrap();
        let expected = norm_cpu(&input_data);
        
        let rel_error = (result[0] - expected).abs() / expected;
        assert!(rel_error < 1e-3, "Expected {}, got {}", expected, result[0]);
    }

    #[tokio::test]
    async fn test_norm_precision() {
        let device = get_test_device().await;
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device).await.unwrap();
        let gpu_result = input.norm().unwrap().to_vec().unwrap();
        let cpu_result = norm_cpu(&input_data);
        
        let error = (gpu_result[0] - cpu_result).abs();
        assert!(error < 1e-4, "Error {} exceeds threshold", error);
    }
}
