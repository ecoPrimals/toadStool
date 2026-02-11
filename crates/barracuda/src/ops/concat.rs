//! Concatenate operation - Pure WGSL

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;

pub struct Concat {
    lhs: Tensor,
    rhs: Tensor,
}

impl Concat {
    pub fn new(lhs: Tensor, rhs: Tensor) -> Self {
        Self { lhs, rhs }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/tensor/concat.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        let size1 = self.lhs.len();
        let size2 = self.rhs.len();
        let output_size = size1 + size2;

        let output_buffer = device.create_buffer_f32(output_size)?;

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Concat BGL"),
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
            label: Some("Concat BG"),
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

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Concat"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Concat PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Concat Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Concat Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Concat Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (output_size as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![output_size],
            device.clone(),
        ))
    }
}

impl Tensor {
    pub fn concat(self, other: &Self) -> Result<Self> {
        Concat::new(self, other.clone()).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_concat_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        let t1 = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device.clone())
            .await
            .unwrap();
        let t2 = Tensor::from_vec_on(vec![4.0, 5.0], vec![2], device)
            .await
            .unwrap();

        let result = t1.concat(&t2).unwrap().to_vec().unwrap();

        assert_eq!(result.len(), 5);
        assert!((result[0] - 1.0).abs() < 1e-5);
        assert!((result[1] - 2.0).abs() < 1e-5);
        assert!((result[2] - 3.0).abs() < 1e-5);
        assert!((result[3] - 4.0).abs() < 1e-5);
        assert!((result[4] - 5.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_concat_edge_cases() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Single element tensors
        let t1 = Tensor::from_vec_on(vec![1.0], vec![1], device.clone())
            .await
            .unwrap();
        let t2 = Tensor::from_vec_on(vec![2.0], vec![1], device.clone())
            .await
            .unwrap();

        let result = t1.concat(&t2).unwrap().to_vec().unwrap();
        assert_eq!(result, vec![1.0, 2.0]);

        // Same size tensors
        let t3 = Tensor::from_vec_on(vec![3.0, 4.0], vec![2], device.clone())
            .await
            .unwrap();
        let t4 = Tensor::from_vec_on(vec![5.0, 6.0], vec![2], device)
            .await
            .unwrap();

        let result = t3.concat(&t4).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 4);
    }

    #[tokio::test]
    async fn test_concat_boundary() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Different sized tensors
        let t1 = Tensor::from_vec_on(vec![1.0; 10], vec![10], device.clone())
            .await
            .unwrap();
        let t2 = Tensor::from_vec_on(vec![2.0; 5], vec![5], device)
            .await
            .unwrap();

        let result = t1.concat(&t2).unwrap().to_vec().unwrap();
        assert_eq!(result.len(), 15);

        // First 10 should be 1.0
        assert!(result[0..10].iter().all(|&x| (x - 1.0).abs() < 1e-5));
        // Next 5 should be 2.0
        assert!(result[10..15].iter().all(|&x| (x - 2.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_concat_large_tensors() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Large tensors
        let size1 = 1000;
        let size2 = 500;

        let t1 = Tensor::from_vec_on(vec![1.0; size1], vec![size1], device.clone())
            .await
            .unwrap();
        let t2 = Tensor::from_vec_on(vec![2.0; size2], vec![size2], device)
            .await
            .unwrap();

        let result = t1.concat(&t2).unwrap().to_vec().unwrap();

        assert_eq!(result.len(), size1 + size2);
        assert!(result[0..size1].iter().all(|&x| (x - 1.0).abs() < 1e-5));
        assert!(result[size1..].iter().all(|&x| (x - 2.0).abs() < 1e-5));
    }

    #[tokio::test]
    async fn test_concat_precision() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Test with specific values
        let t1 = Tensor::from_vec_on(vec![1.5, 2.5, 3.5], vec![3], device.clone())
            .await
            .unwrap();
        let t2 = Tensor::from_vec_on(vec![4.5, 5.5], vec![2], device)
            .await
            .unwrap();

        let result = t1.concat(&t2).unwrap().to_vec().unwrap();

        assert_eq!(result.len(), 5);
        assert!((result[0] - 1.5).abs() < 1e-5);
        assert!((result[1] - 2.5).abs() < 1e-5);
        assert!((result[2] - 3.5).abs() < 1e-5);
        assert!((result[3] - 4.5).abs() < 1e-5);
        assert!((result[4] - 5.5).abs() < 1e-5);
    }
}
