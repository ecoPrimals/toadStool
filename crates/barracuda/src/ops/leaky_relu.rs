//! LeakyReLU activation - Pure WGSL

use crate::tensor::Tensor;
use crate::error::Result;

pub struct LeakyReLU {
    input: Tensor,
}

impl LeakyReLU {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/leaky_relu_simple.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("LeakyReLU BGL"),
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
            label: Some("LeakyReLU BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("LeakyReLU"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("LeakyReLU PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("LeakyReLU Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("LeakyReLU Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LeakyReLU Pass"),
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
    pub fn leaky_relu(self) -> Result<Self> {
        LeakyReLU::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn leaky_relu_cpu(x: f32, alpha: f32) -> f32 {
        if x > 0.0 { x } else { alpha * x }
    }

    #[tokio::test]
    async fn test_leaky_relu_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![-10.0, 0.0, 10.0], vec![3], device).await.unwrap();
        let result = input.leaky_relu().unwrap().to_vec().unwrap();

        // LeakyReLU(x) = x if x > 0, else 0.01x
        assert!((result[0] - (-0.1)).abs() < 1e-5);
        assert!(result[1].abs() < 1e-5);
        assert!((result[2] - 10.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_leaky_relu_edge_cases() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![-1e-6, 1e-6, -1.0, 1.0], vec![4], device).await.unwrap();
        let result = input.leaky_relu().unwrap().to_vec().unwrap();

        assert!((result[0] - (-1e-8)).abs() < 1e-12);
        assert!((result[1] - 1e-6).abs() < 1e-12);
        assert!((result[2] - (-0.01)).abs() < 1e-6);
        assert_eq!(result[3], 1.0);
    }

    #[tokio::test]
    async fn test_leaky_relu_boundary() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY], vec![5], device).await.unwrap();
        let result = input.leaky_relu().unwrap().to_vec().unwrap();

        assert!(result[0].is_infinite() && result[0].is_sign_negative());
        assert_eq!(result[2], 0.0);
        assert!(result[4].is_infinite() && result[4].is_sign_positive());
    }

    #[tokio::test]
    async fn test_leaky_relu_large_tensor() {
        let device = get_test_device().await;

        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) - 500.0).collect();
        let input = Tensor::from_vec_on(input_data, vec![size], device).await.unwrap();
        let result = input.leaky_relu().unwrap().to_vec().unwrap();

        // Check negative values are scaled by 0.01
        for i in 0..500 {
            let expected = (i as f32 - 500.0) * 0.01;
            assert!((result[i] - expected).abs() < 1e-3);
        }
    }

    #[tokio::test]
    async fn test_leaky_relu_precision() {
        let device = get_test_device().await;

        let input_data = vec![-5.0, -2.0, -1.0, 0.0, 1.0, 2.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![7], device).await.unwrap();
        let result = input.leaky_relu().unwrap().to_vec().unwrap();

        let cpu_result: Vec<f32> = input_data.iter().map(|&x| leaky_relu_cpu(x, 0.01)).collect();

        for (i, (&gpu, &cpu)) in result.iter().zip(cpu_result.iter()).enumerate() {
            assert!((gpu - cpu).abs() < 1e-6, "Error at {}: GPU={}, CPU={}", i, gpu, cpu);
        }
    }
}
