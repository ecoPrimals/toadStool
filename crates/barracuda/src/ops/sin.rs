//! Sin operation - Sine
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Sin {
    input: Tensor,
}

impl Sin {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/sin.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Sin BGL"),
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
            label: Some("Sin BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Sin"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Sin PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Sin Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Sin Encoder"),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Sin Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((size as u32 + 255) / 256, 1, 1);
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
    pub fn sin(self) -> Result<Self> {
        Sin::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;
    use std::f32::consts::PI;

    fn sin_cpu(x: f32) -> f32 {
        x.sin()
    }

    #[tokio::test]
    async fn test_sin_basic() {
        let device = get_test_device().await;
        let input_data = vec![0.0, PI / 6.0, PI / 4.0, PI / 2.0, PI];
        let input = Tensor::from_vec_on(input_data.clone(), vec![5], device)
            .await
            .unwrap();
        let result = input.sin().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| sin_cpu(x)).collect();

        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5, "Expected {}, got {}", e, r);
        }
    }

    #[tokio::test]
    async fn test_sin_edge_cases() {
        let device = get_test_device().await;
        // Test special angles and negative values
        let input_data = vec![0.0, -PI / 2.0, 2.0 * PI, -PI];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.sin().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| sin_cpu(x)).collect();

        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-5, "Expected {}, got {}", e, r);
        }
    }

    #[tokio::test]
    async fn test_sin_boundary() {
        let device = get_test_device().await;
        // Large angles (periodicity)
        let input_data = vec![10.0 * PI, -10.0 * PI, 100.0, -100.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.sin().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| sin_cpu(x)).collect();

        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-4, "Expected {}, got {}", e, r);
        }
    }

    #[tokio::test]
    async fn test_sin_large_tensor() {
        let device = get_test_device().await;
        let size = 1000;
        let input_data: Vec<f32> = (0..size).map(|i| (i as f32) * 0.01).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device)
            .await
            .unwrap();
        let result = input.sin().unwrap().to_vec().unwrap();

        assert_eq!(result.len(), size);
        for i in 0..10 {
            let expected = sin_cpu(input_data[i]);
            assert!((result[i] - expected).abs() < 1e-5);
        }
    }

    #[tokio::test]
    async fn test_sin_precision() {
        let device = get_test_device().await;
        let input_data = vec![0.0, PI / 6.0, PI / 4.0, PI / 3.0, PI / 2.0, PI];
        let input = Tensor::from_vec_on(input_data.clone(), vec![6], device)
            .await
            .unwrap();
        let gpu_result = input.sin().unwrap().to_vec().unwrap();
        let cpu_result: Vec<f32> = input_data.iter().map(|&x| sin_cpu(x)).collect();

        let mut max_error = 0.0f32;
        for (r, e) in gpu_result.iter().zip(cpu_result.iter()) {
            let error = (r - e).abs();
            max_error = max_error.max(error);
        }
        assert!(
            max_error < 1e-5,
            "Max error {} exceeds threshold",
            max_error
        );
    }
}
