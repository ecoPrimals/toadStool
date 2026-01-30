//! Exponential operation - Pure WGSL

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Exp {
    input: Tensor,
}

impl Exp {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/exp.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Exp BGL"),
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
            label: Some("Exp BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Exp"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Exp PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Exp Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Exp Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Exp Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
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
    pub fn exp(self) -> Result<Self> {
        Exp::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn exp_cpu(x: f32) -> f32 {
        x.exp()
    }

    #[tokio::test]
    async fn test_exp_basic() {
        let device = get_test_device().await;

        let input_data = vec![0.0, 1.0, 2.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.exp().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| exp_cpu(x)).collect();

        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_exp_edge_cases() {
        let device = get_test_device().await;

        // Zero should give 1
        let input_data = vec![0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![1], device.clone()).await.unwrap();
        let result = input.exp().unwrap().to_vec().unwrap();
        assert!((result[0] - 1.0).abs() < 1e-6);

        // Negative values
        let input_data = vec![-5.0, -2.0, -0.5];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.exp().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| exp_cpu(x)).collect();
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_exp_boundary() {
        let device = get_test_device().await;

        // Very small positive (should be close to 1)
        let input_data = vec![1e-6, 1e-10];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2], device.clone()).await.unwrap();
        let result = input.exp().unwrap().to_vec().unwrap();
        for r in result.iter() {
            assert!((r - 1.0).abs() < 0.001);
        }

        // Moderate negative (exp approaches 0)
        let input_data = vec![-10.0, -20.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2], device).await.unwrap();
        let result = input.exp().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| exp_cpu(x)).collect();
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-8);
        }
    }

    #[tokio::test]
    async fn test_exp_large_tensor() {
        let device = get_test_device().await;

        // 1000 elements with small values to avoid overflow
        let input_data: Vec<f32> = (0..1000).map(|i| (i as f32 - 500.0) * 0.01).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![1000], device).await.unwrap();
        
        let result = input.exp().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| exp_cpu(x)).collect();
        
        for (r, e) in result.iter().zip(expected.iter()) {
            let rel_error = ((r - e) / e.max(1e-10)).abs();
            assert!(rel_error < 1e-4, "Relative error too large: {}", rel_error);
        }
    }

    #[tokio::test]
    async fn test_exp_precision() {
        let device = get_test_device().await;

        // Test FP32 precision
        let input_data = vec![-2.345, -1.234, 0.0, 1.234, 2.345];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device).await.unwrap();
        let result = input.exp().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| exp_cpu(x)).collect();
        
        // Verify FP32 precision with relative error
        for (r, e) in result.iter().zip(expected.iter()) {
            let rel_error = ((r - e) / e.max(1e-10)).abs();
            assert!(rel_error < 1e-5, "GPU: {}, CPU: {}, Rel error: {}", r, e, rel_error);
        }
    }
}
