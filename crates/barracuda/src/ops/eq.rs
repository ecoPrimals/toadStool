//! Eq operation - Element-wise equality
//! Pure WGSL implementation

use crate::error::Result;
use crate::tensor::Tensor;

pub struct Eq {
    lhs: Tensor,
    rhs: Tensor,
}

impl Eq {
    pub fn new(lhs: Tensor, rhs: Tensor) -> Self {
        Self { lhs, rhs }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/eq.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        let size = self.lhs.len();
        let output_buffer = device.create_buffer_f32(size)?;

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Eq BGL"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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
            label: Some("Eq BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.lhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.rhs.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Eq"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Eq PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Eq Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Eq Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Eq Pass"),
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
            self.lhs.shape().to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    pub fn eq(self, other: &Self) -> Result<Self> {
        Eq::new(self, other.clone()).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    async fn get_test_device() -> Arc<crate::device::WgpuDevice> {
        Arc::new(crate::device::Auto::new().await.unwrap())
    }

    #[tokio::test]
    async fn test_eq_basic() {
        let device = get_test_device().await;

        let a = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(vec![1.0, 2.1, 3.0], vec![3], device)
            .await
            .unwrap();

        let result = a.eq(&b).unwrap().to_vec().unwrap();
        assert!((result[0] - 1.0).abs() < 1e-5); // equal
        assert!((result[1] - 0.0).abs() < 1e-5); // not equal
        assert!((result[2] - 1.0).abs() < 1e-5); // equal
    }

    #[tokio::test]
    async fn test_eq_edge_cases() {
        let device = get_test_device().await;

        // All equal
        let a = Tensor::from_vec_on(vec![5.0, 5.0, 5.0], vec![3], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(vec![5.0, 5.0, 5.0], vec![3], device.clone())
            .await
            .unwrap();
        let result = a.eq(&b).unwrap().to_vec().unwrap();
        assert!(result.iter().all(|&x| (x - 1.0).abs() < 1e-5));

        // None equal
        let a = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(vec![4.0, 5.0, 6.0], vec![3], device)
            .await
            .unwrap();
        let result = a.eq(&b).unwrap().to_vec().unwrap();
        assert!(result.iter().all(|&x| x.abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_eq_boundary() {
        let device = get_test_device().await;

        // Negative values
        let a = Tensor::from_vec_on(vec![-1.0, -2.0, -3.0], vec![3], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(vec![-1.0, -2.0, -3.0], vec![3], device.clone())
            .await
            .unwrap();
        let result = a.eq(&b).unwrap().to_vec().unwrap();
        assert!(result.iter().all(|&x| (x - 1.0).abs() < 1e-5));

        // Zero comparison
        let a = Tensor::from_vec_on(vec![0.0, 0.0, 1.0], vec![3], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(vec![0.0, 0.0, 1.0], vec![3], device)
            .await
            .unwrap();
        let result = a.eq(&b).unwrap().to_vec().unwrap();
        assert!(result.iter().all(|&x| (x - 1.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_eq_large_tensor() {
        let device = get_test_device().await;

        // 1000 elements
        let a_data: Vec<f32> = (0..1000).map(|i| i as f32).collect();
        let b_data: Vec<f32> = (0..1000).map(|i| i as f32).collect();

        let a = Tensor::from_vec_on(a_data, vec![1000], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_data, vec![1000], device)
            .await
            .unwrap();

        let result = a.eq(&b).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 1000);
        assert!(result.iter().all(|&x| (x - 1.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_eq_precision() {
        let device = get_test_device().await;

        // Test exact equality with known values
        let a = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();

        let result = a.eq(&b).unwrap().to_vec().unwrap();
        // All should be equal
        assert!(result.iter().all(|&x| (x - 1.0).abs() < 1e-5));

        // Test with clearly different values
        let a = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(vec![1.0, 5.0, 3.0], vec![3], device)
            .await
            .unwrap();

        let result = a.eq(&b).unwrap().to_vec().unwrap();
        // First and third equal, second not equal
        assert!((result[0] - 1.0).abs() < 1e-5);
        assert!(result[1].abs() < 1e-5);
        assert!((result[2] - 1.0).abs() < 1e-5);
    }
}
