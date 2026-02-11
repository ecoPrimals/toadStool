//! Cdist - Pairwise distance computation - Pure WGSL
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (GPU-optimized)
//! - ✅ Safe Rust wrapper (no unsafe code)
//! - ✅ Hardware-agnostic via WebGPU
//! - ✅ Complete implementation (production-ready)
//!
//! ## Algorithm
//!
//! Computes pairwise distances between vectors:
//! ```text
//! Input A: [M, D] - M vectors of dimension D
//! Input B: [N, D] - N vectors of dimension D
//! Output:  [M, N] - Distance matrix
//!
//! Supports: Euclidean (L2), Manhattan (L1), Cosine
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;

#[derive(Clone, Copy)]
pub enum DistanceMetric {
    Euclidean = 0,
    Manhattan = 1,
    Cosine = 2,
}

pub struct Cdist {
    input_a: Tensor,
    input_b: Tensor,
    metric: DistanceMetric,
}

impl Cdist {
    pub fn new(input_a: Tensor, input_b: Tensor, metric: DistanceMetric) -> Self {
        Self {
            input_a,
            input_b,
            metric,
        }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/misc/cdist.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input_a.device();
        let shape_a = self.input_a.shape();
        let shape_b = self.input_b.shape();

        // Expect 2D tensors [M, D] and [N, D]
        if shape_a.len() != 2 || shape_b.len() != 2 {
            return Err(crate::error::BarracudaError::InvalidShape {
                expected: vec![0, 0],
                actual: shape_a.to_vec(),
            });
        }

        let m = shape_a[0]; // Number of vectors in A
        let d_a = shape_a[1]; // Dimension of A
        let n = shape_b[0]; // Number of vectors in B
        let d_b = shape_b[1]; // Dimension of B

        if d_a != d_b {
            return Err(crate::error::BarracudaError::InvalidShape {
                expected: vec![n, d_a],
                actual: vec![n, d_b],
            });
        }

        let d = d_a;
        let output_buffer = device.create_buffer_f32(m * n)?;

        // Create params buffer
        let params_data = [m as u32, n as u32, d as u32, self.metric as u32];
        let params_buffer = device.create_uniform_buffer("Params", &params_data);

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Cdist BGL"),
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 3,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cdist BG"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input_a.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.input_b.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Cdist"));
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Cdist PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Cdist Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Cdist Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Cdist Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::MatMul);
            let workgroups_x = (m as u32).div_ceil(optimal_wg_size);
            let workgroups_y = (n as u32).div_ceil(optimal_wg_size);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            vec![m, n],
            device.clone(),
        ))
    }
}

impl Tensor {
    pub fn cdist_wgsl(self, other: Tensor, metric: DistanceMetric) -> Result<Self> {
        Cdist::new(self, other, metric).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_cdist_euclidean() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Two 2D points: (0,0) and (3,4)
        let a_data = vec![0.0, 0.0];
        let b_data = vec![3.0, 4.0];

        let a = Tensor::from_vec_on(a_data, vec![1, 2], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_data, vec![1, 2], device)
            .await
            .unwrap();

        let result = a.cdist_wgsl(b, DistanceMetric::Euclidean).unwrap();
        let output = result.to_vec().unwrap();

        // Distance = sqrt(3^2 + 4^2) = 5.0
        assert!((output[0] - 5.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_cdist_manhattan() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let a_data = vec![0.0, 0.0];
        let b_data = vec![3.0, 4.0];

        let a = Tensor::from_vec_on(a_data, vec![1, 2], device.clone())
            .await
            .unwrap();
        let b = Tensor::from_vec_on(b_data, vec![1, 2], device)
            .await
            .unwrap();

        let result = a.cdist_wgsl(b, DistanceMetric::Manhattan).unwrap();
        let output = result.to_vec().unwrap();

        // Distance = |3| + |4| = 7.0
        assert!((output[0] - 7.0).abs() < 1e-5);
    }
}
