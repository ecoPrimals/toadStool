//! Softplus activation - smooth approximation of ReLU
//! Pure WGSL implementation

use crate::tensor::Tensor;
use crate::error::Result;

pub struct Softplus {
    input: Tensor,
}

impl Softplus {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/softplus.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout = device.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Softplus BGL"),
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
            label: Some("Softplus BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Softplus"));
        let pipeline_layout = device.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Softplus PL"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Softplus Pipeline"),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point: "main",
        });

        let mut encoder = device.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Softplus Encoder"),
        });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Softplus Pass"),
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
    pub fn softplus(self) -> Result<Self> {
        Softplus::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn softplus_cpu(x: f32) -> f32 {
        // Softplus(x) = ln(1 + e^x)
        // For numerical stability: if x > 20, use x (since e^x dominates)
        if x > 20.0 {
            x
        } else if x < -20.0 {
            0.0
        } else {
            (1.0 + x.exp()).ln()
        }
    }

    #[tokio::test]
    async fn test_softplus_basic() {
        let device = get_test_device().await;

        let input_data = vec![-2.0, 0.0, 2.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.softplus().unwrap().to_vec().unwrap();
        
        let expected: Vec<f32> = input_data.iter().map(|&x| softplus_cpu(x)).collect();
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-4);
        }
    }

    #[tokio::test]
    async fn test_softplus_edge_cases() {
        let device = get_test_device().await;

        // Zero should give ln(2) ≈ 0.693
        let input_data = vec![0.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![1], device.clone()).await.unwrap();
        let result = input.softplus().unwrap().to_vec().unwrap();
        assert!((result[0] - 0.693147).abs() < 1e-3);

        // Negative values
        let input_data = vec![-5.0, -2.0, -0.5];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.softplus().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| softplus_cpu(x)).collect();
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-4);
        }
    }

    #[tokio::test]
    async fn test_softplus_boundary() {
        let device = get_test_device().await;

        // Large positive (should approximate x)
        let input_data = vec![10.0, 20.0, 50.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device.clone()).await.unwrap();
        let result = input.softplus().unwrap().to_vec().unwrap();
        for (r, orig) in result.iter().zip(input_data.iter()) {
            assert!((r - orig).abs() < 0.1); // Should be close to x
        }

        // Large negative (should approximate 0)
        let input_data = vec![-10.0, -20.0, -50.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device).await.unwrap();
        let result = input.softplus().unwrap().to_vec().unwrap();
        for r in result.iter() {
            assert!(*r < 1e-3); // Should be very close to 0
        }
    }

    #[tokio::test]
    async fn test_softplus_large_tensor() {
        let device = get_test_device().await;

        // 1000 elements
        let input_data: Vec<f32> = (0..1000).map(|i| (i as f32 - 500.0) * 0.01).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![1000], device).await.unwrap();
        
        let result = input.softplus().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| softplus_cpu(x)).collect();
        
        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-3);
        }
    }

    #[tokio::test]
    async fn test_softplus_precision() {
        let device = get_test_device().await;

        // Test FP32 precision with typical values
        let input_data = vec![-2.345, -1.234, 0.0, 1.234, 2.345];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device).await.unwrap();
        let result = input.softplus().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| softplus_cpu(x)).collect();
        
        // Verify FP32 precision
        let max_error = result.iter().zip(expected.iter())
            .map(|(r, e)| (r - e).abs())
            .fold(0.0f32, f32::max);
        
        assert!(max_error < 1e-4, "Max error: {} exceeds threshold", max_error);
    }
}
