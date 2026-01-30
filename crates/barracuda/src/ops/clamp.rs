//! Clamp operation (clamp to [0, 6]) - Pure WGSL

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Clamp {
    input: Tensor,
}

impl Clamp {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/clamp_simple.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Clamp BGL"),
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
            label: Some("Clamp BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Clamp"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Clamp PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Clamp Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Clamp Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Clamp Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(output_buffer, self.input.shape().to_vec(), device.clone()))
    }
}

impl Tensor {
    pub fn clamp(self) -> Result<Self> {
        Clamp::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn clamp_cpu(x: f32) -> f32 {
        x.clamp(0.0, 6.0) // Clamp to [0, 6] to match shader
    }

    #[tokio::test]
    async fn test_clamp_basic() {
        let device = get_test_device().await;
        let input_data = vec![-5.0, 2.0, 10.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.clamp().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| clamp_cpu(x)).collect();
        
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6, "Expected {}, got {}", e, r);
        }
    }

    #[tokio::test]
    async fn test_clamp_edge_cases() {
        let device = get_test_device().await;
        
        // Exact boundaries
        let input_data = vec![0.0, 6.0, -0.0001, 6.0001];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device.clone()).await.unwrap();
        let result = input.clamp().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| clamp_cpu(x)).collect();
        
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
        
        // All within range
        let input_data = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device).await.unwrap();
        let result = input.clamp().unwrap().to_vec().unwrap();
        for (r, &orig) in result.iter().zip(input_data.iter()) {
            assert!((r - orig).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_clamp_boundary() {
        let device = get_test_device().await;
        let input_data = vec![-1e10, 1e10, -100.0, 100.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device).await.unwrap();
        let result = input.clamp().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| clamp_cpu(x)).collect();
        
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_clamp_large_tensor() {
        let device = get_test_device().await;
        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) * 0.01 - 5.0).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device).await.unwrap();
        let result = input.clamp().unwrap().to_vec().unwrap();
        
        assert_eq!(result.len(), size);
        for i in 0..10 {
            let expected = clamp_cpu(input_data[i]);
            assert!((result[i] - expected).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_clamp_precision() {
        let device = get_test_device().await;
        let input_data = vec![-10.0, -5.0, -1.0, 0.0, 1.0, 3.0, 6.0, 7.0, 10.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![9], device).await.unwrap();
        let gpu_result = input.clamp().unwrap().to_vec().unwrap();
        let cpu_result: Vec<f32> = input_data.iter().map(|&x| clamp_cpu(x)).collect();
        
        let mut max_error = 0.0f32;
        for (r, e) in gpu_result.iter().zip(cpu_result.iter()) {
            let error = (r - e).abs();
            max_error = max_error.max(error);
        }
        assert!(max_error < 1e-6, "Max error {} exceeds threshold", max_error);
    }
}
