//! Reciprocal operation - 1/x
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Reciprocal {
    input: Tensor,
}

impl Reciprocal {
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/reciprocal.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Reciprocal BGL"),
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
            label: Some("Reciprocal BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Reciprocal"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Reciprocal PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Reciprocal Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Reciprocal Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Reciprocal Pass"),
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
    pub fn reciprocal(self) -> Result<Self> {
        Reciprocal::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    fn reciprocal_cpu(x: f32) -> f32 {
        1.0 / x
    }

    #[tokio::test]
    async fn test_reciprocal_basic() {
        let device = get_test_device().await;
        let input_data = vec![1.0, 2.0, 4.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![3], device)
            .await
            .unwrap();
        let result = input.reciprocal().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| reciprocal_cpu(x)).collect();

        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6, "Expected {}, got {}", e, r);
        }
    }

    #[tokio::test]
    async fn test_reciprocal_edge_cases() {
        let device = get_test_device().await;
        let input_data = vec![1.0, -1.0, 0.5, -0.5];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.reciprocal().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| reciprocal_cpu(x)).collect();

        for (r, e) in result.iter().zip(expected.iter()) {
            assert!((r - e).abs() < 1e-6, "Expected {}, got {}", e, r);
        }
    }

    #[tokio::test]
    async fn test_reciprocal_boundary() {
        let device = get_test_device().await;
        // Avoid division by zero - use small but safe values
        let input_data = vec![1e-5, 1e5, -1e-5, -1e5];
        let input = Tensor::from_vec_on(input_data.clone(), vec![4], device)
            .await
            .unwrap();
        let result = input.reciprocal().unwrap().to_vec().unwrap();
        let expected: Vec<f32> = input_data.iter().map(|&x| reciprocal_cpu(x)).collect();

        for (r, e) in result.iter().zip(expected.iter()) {
            let rel_error = if e.abs() > 1e-5 {
                (r - e).abs() / e.abs()
            } else {
                (r - e).abs()
            };
            assert!(
                rel_error < 1e-4,
                "Expected {}, got {} (rel error {})",
                e,
                r,
                rel_error
            );
        }
    }

    #[tokio::test]
    async fn test_reciprocal_large_tensor() {
        let device = get_test_device().await;
        let size = 1000;
        let input_data: Vec<f32> = (1..=size).map(|i| i as f32).collect();
        let input = Tensor::from_vec_on(input_data.clone(), vec![size], device)
            .await
            .unwrap();
        let result = input.reciprocal().unwrap().to_vec().unwrap();

        assert_eq!(result.len(), size);
        for i in 0..10 {
            let expected = reciprocal_cpu(input_data[i]);
            assert!((result[i] - expected).abs() < 1e-6);
        }
    }

    #[tokio::test]
    async fn test_reciprocal_precision() {
        let device = get_test_device().await;
        let input_data = vec![0.5, 1.0, 2.0, 4.0, 8.0, 16.0];
        let input = Tensor::from_vec_on(input_data.clone(), vec![6], device)
            .await
            .unwrap();
        let gpu_result = input.reciprocal().unwrap().to_vec().unwrap();
        let cpu_result: Vec<f32> = input_data.iter().map(|&x| reciprocal_cpu(x)).collect();

        let mut max_error = 0.0f32;
        for (r, e) in gpu_result.iter().zip(cpu_result.iter()) {
            let error = (r - e).abs();
            max_error = max_error.max(error);
        }
        assert!(
            max_error < 1e-6,
            "Max error {} exceeds threshold",
            max_error
        );
    }
}
