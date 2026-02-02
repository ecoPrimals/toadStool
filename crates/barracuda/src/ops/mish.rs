//! Mish activation - Pure WGSL
//! Formula: Mish(x) = x * tanh(softplus(x)) = x * tanh(ln(1 + e^x))

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Mish {
    input: Tensor,
}

impl Mish {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/mish.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Mish BGL"),
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
            label: Some("Mish BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Mish"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Mish PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Mish Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Mish Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Mish Pass"),
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
    pub fn mish(self) -> Result<Self> {
        Mish::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_mish_basic() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(vec![-2.0, 0.0, 2.0], vec![3], device)
            .await
            .unwrap();
        let result = input.mish().unwrap().to_vec().unwrap();

        // Mish(0) ≈ 0, Mish(x) → x for large x
        assert!(result[1].abs() < 0.01);
        assert!(result[2] > 1.9);
    }

    #[tokio::test]
    async fn test_mish_edge_cases() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(
            vec![-10.0, -5.0, -1e-6, 0.0, 1e-6, 5.0, 10.0],
            vec![7],
            device,
        )
        .await
        .unwrap();
        let result = input.mish().unwrap().to_vec().unwrap();

        assert!(result[0].abs() < 1e-3); // Mish(-10) ≈ 0
        assert!(result[3].abs() < 0.01); // Mish(0) ≈ 0
        assert!((result[5] - 5.0).abs() < 0.05); // Mish(5) ≈ 5
        assert!((result[6] - 10.0).abs() < 0.05); // Mish(10) ≈ 10
    }

    #[tokio::test]
    async fn test_mish_boundary() {
        let device = get_test_device().await;

        let input = Tensor::from_vec_on(
            vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY],
            vec![5],
            device,
        )
        .await
        .unwrap();
        let result = input.mish().unwrap().to_vec().unwrap();

        assert!(result[0].abs() < 1e-10 || result[0].is_nan()); // Mish(-inf) → 0
        assert!(result[2].abs() < 0.01); // Mish(0) ≈ 0
        assert!(result[3] > 1e9 || result[3].is_infinite()); // Mish(large) ≈ x
    }

    #[tokio::test]
    async fn test_mish_large_tensor() {
        let device = get_test_device().await;

        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) / 100.0 - 5.0).collect();
        let input = Tensor::from_vec_on(input_data, vec![size], device)
            .await
            .unwrap();
        let result = input.mish().unwrap().to_vec().unwrap();

        // For positive values, should approach identity
        for (i, &val) in result.iter().enumerate() {
            let x = (i as f32) / 100.0 - 5.0;
            if x > 2.0 {
                assert!((val - x).abs() < 0.1);
            }
        }
    }

    #[tokio::test]
    async fn test_mish_precision() {
        let device = get_test_device().await;

        fn mish_cpu(x: f32) -> f32 {
            x * (1.0 + x.exp()).ln().tanh()
        }

        let input_data = vec![-5.0, -2.0, -1.0, 0.0, 1.0, 2.0, 5.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![7], device)
            .await
            .unwrap();
        let result = input.mish().unwrap().to_vec().unwrap();
        let cpu_result: Vec<f32> = input_data.iter().map(|&x| mish_cpu(x)).collect();

        for (i, (&gpu, &cpu)) in result.iter().zip(cpu_result.iter()).enumerate() {
            assert!(
                (gpu - cpu).abs() < 1e-4,
                "Error at {}: GPU={}, CPU={}",
                i,
                gpu,
                cpu
            );
        }
    }
}
