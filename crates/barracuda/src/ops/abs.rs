//! Absolute value operation - Pure WGSL

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Abs {
    input: Tensor,
}

impl Abs {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/abs.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Abs BGL"),
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
            label: Some("Abs BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Abs"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Abs PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Abs Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Abs Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Abs Pass"),
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
    pub fn abs(self) -> Result<Self> {
        Abs::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn abs_cpu(x: f32) -> f32 {
        x.abs()
    }

    #[tokio::test]
    async fn test_abs_basic() {
        let device = get_test_device().await;

        let input_data = vec![-5.0, -2.0, 0.0, 3.0, 7.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device).await.unwrap();
        let result = input.abs().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| abs_cpu(x)).collect();

        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_abs_edge_cases() {
        let device = get_test_device().await;

        // All negative
        let input_data = vec![-10.0, -5.0, -1.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone()).await.unwrap();
        let result = input.abs().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| abs_cpu(x)).collect();
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6);
        }

        // All positive (should be unchanged)
        let input_data = vec![1.0, 5.0, 10.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.abs().unwrap().to_vec().unwrap();
        for (r, orig) in result.iter().zip(input_data.iter()) {
            assert!((r - orig).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_abs_boundary() {
        let device = get_test_device().await;

        // Zero (boundary case)
        let input_data = vec![0.0, -0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![2], device.clone()).await.unwrap();
        let result = input.abs().unwrap().to_vec().unwrap();
        for r in result.iter() {
            assert!(r.abs() < 1e-6);
        }

        // Very small values
        let input_data = vec![-1e-10, 1e-10, -1e-6, 1e-6];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device).await.unwrap();
        let result = input.abs().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| abs_cpu(x)).collect();
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-10);
        }
    }

    #[tokio::test]
    async fn test_abs_large_tensor() {
        let device = get_test_device().await;

        // 1000 elements
        let input_data: Vec<f32> = (0..1000).map(|i| (i as f32 - 500.0) * 0.1).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![1000], device).await.unwrap();
        
        let result = input.abs().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| abs_cpu(x)).collect();
        
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_abs_precision() {
        let device = get_test_device().await;

        // Test FP32 precision
        let input_data = vec![-123.456, -78.901, -2.345, 0.0, 1.234, 56.789, 123.456];
        let input = Tensor::from_vec_on(input_data.clone(), vec![7], device).await.unwrap();
        let result = input.abs().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| abs_cpu(x)).collect();
        
        // Verify FP32 precision (abs is exact operation)
        let max_error = result.iter().zip(expected.iter())
            .map(|(r, e)| (r - e).abs())
            .fold(0.0f32, f32::max);
        
        assert!(max_error < 1e-6, "Max error: {} exceeds threshold", max_error);
    }
}
