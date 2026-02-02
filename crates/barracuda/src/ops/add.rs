//! Element-wise addition
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Formula: C = A + B (element-wise)

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Element-wise addition operation
pub struct Add {
    lhs: Tensor,
    rhs: Tensor,
}

impl Add {
    /// Create Add operation
    pub fn new(lhs: Tensor, rhs: Tensor) -> Result<Self> {
        // Verify shapes match
        if lhs.shape() != rhs.shape() {
            return Err(BarracudaError::shape_mismatch(
                lhs.shape().to_vec(),
                rhs.shape().to_vec(),
            ));
        }
        Ok(Self { lhs, rhs })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/elementwise_add.wgsl")
    }

    /// Execute addition on tensors
    pub fn execute(self) -> Result<Tensor> {
        let device = self.lhs.device();
        let size = self.lhs.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create bind group layout (3 buffers: lhs, rhs, output)
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Add Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Add Bind Group"),
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

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Add"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Add Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Add Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Add Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Add Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (256 threads per workgroup)
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            self.lhs.shape().to_vec(),
            device.clone(),
        ))
    }
}

// Convenience methods on Tensor
impl Tensor {
    /// Element-wise addition
    pub fn add(&self, other: &Tensor) -> Result<Self> {
        Add::new(self.clone(), other.clone())?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device;

    #[tokio::test]
    async fn test_add_basic() {
        let device = get_test_device().await;

        let lhs = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![5], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(vec![10.0, 20.0, 30.0, 40.0, 50.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        let expected = vec![11.0, 22.0, 33.0, 44.0, 55.0];
        for (i, (&r, &e)) in result.iter().zip(expected.iter()).enumerate() {
            assert!((r - e).abs() < 1e-6, "Mismatch at {}: {} vs {}", i, r, e);
        }
    }

    #[tokio::test]
    async fn test_add_edge_cases() {
        let device = get_test_device().await;

        // Very small values, zero, negatives
        let lhs = Tensor::from_vec_on(vec![-1e-6, 0.0, 1e-6, -1.0, 1.0], vec![5], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(vec![1e-6, 0.0, -1e-6, 1.0, -1.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        assert!((result[0] - 0.0).abs() < 1e-12); // -1e-6 + 1e-6 = 0
        assert_eq!(result[1], 0.0); // 0 + 0 = 0
        assert!((result[2] - 0.0).abs() < 1e-12); // 1e-6 + (-1e-6) = 0
        assert_eq!(result[3], 0.0); // -1 + 1 = 0
        assert_eq!(result[4], 0.0); // 1 + (-1) = 0
    }

    #[tokio::test]
    async fn test_add_boundary() {
        let device = get_test_device().await;

        // Infinities and large values
        let lhs = Tensor::from_vec_on(
            vec![f32::NEG_INFINITY, -1e10, 0.0, 1e10, f32::INFINITY],
            vec![5],
            device.clone(),
        )
        .await
        .unwrap();

        let rhs = Tensor::from_vec_on(vec![100.0, 1e10, 0.0, -1e10, 100.0], vec![5], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        assert!(result[0].is_infinite() && result[0].is_sign_negative()); // -inf + 100 = -inf
        assert_eq!(result[1], 0.0); // -1e10 + 1e10 = 0 (approximately)
        assert_eq!(result[2], 0.0); // 0 + 0 = 0
        assert_eq!(result[3], 0.0); // 1e10 + (-1e10) = 0 (approximately)
        assert!(result[4].is_infinite() && result[4].is_sign_positive()); // inf + 100 = inf
    }

    #[tokio::test]
    async fn test_add_large_tensor() {
        let device = get_test_device().await;

        let size = 1000;
        let lhs_data: Vec<f32> = (0..size).map(|i| i as f32).collect();
        let rhs_data: Vec<f32> = (0..size).map(|i| (size - i) as f32).collect();

        let lhs = Tensor::from_vec_on(lhs_data.clone(), vec![size], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(rhs_data.clone(), vec![size], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let result = output.to_vec().unwrap();

        // All should equal size
        for (i, &val) in result.iter().enumerate() {
            assert!(
                (val - size as f32).abs() < 1e-4,
                "Mismatch at {}: {}",
                i,
                val
            );
        }
    }

    #[tokio::test]
    async fn test_add_precision() {
        let device = get_test_device().await;

        let lhs_data = vec![-5.0, -2.5, -1.0, 0.0, 1.0, 2.5, 5.0];
        let rhs_data = vec![2.0, 1.5, 0.5, 0.0, -0.5, -1.5, -2.0];

        let lhs = Tensor::from_vec_on(lhs_data.clone(), vec![7], device.clone())
            .await
            .unwrap();
        let rhs = Tensor::from_vec_on(rhs_data.clone(), vec![7], device)
            .await
            .unwrap();

        let output = lhs.add(&rhs).unwrap();
        let gpu_result = output.to_vec().unwrap();

        // CPU reference
        let cpu_result: Vec<f32> = lhs_data
            .iter()
            .zip(rhs_data.iter())
            .map(|(&a, &b)| a + b)
            .collect();

        // Should be exact for addition
        for (i, (&gpu, &cpu)) in gpu_result.iter().zip(cpu_result.iter()).enumerate() {
            assert!(
                (gpu - cpu).abs() < 1e-6,
                "Error at {}: GPU={}, CPU={}",
                i,
                gpu,
                cpu
            );
        }
    }
}
