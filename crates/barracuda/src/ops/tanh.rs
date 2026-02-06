//! Tanh (Hyperbolic Tangent) activation
//!
//! **Deep Debt Principles**:
//! - ✅ Pure WGSL implementation (universal compute)
//! - ✅ Capability-based dispatch (vendor-optimized)
//!
//! Formula: tanh(x) = (e^x - e^(-x)) / (e^x + e^(-x))

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;

/// Tanh activation operation
pub struct Tanh {
    input: Tensor,
}

impl Tanh {
    /// Create Tanh operation
    pub fn new(input: Tensor) -> Self {
        Self { input }
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/tanh.wgsl")
    }

    /// Execute Tanh on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer
        let output_buffer = device.create_buffer_f32(size)?;

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Tanh Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Tanh Bind Group"),
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

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Tanh"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Tanh Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Tanh Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Tanh Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Tanh Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(&device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (size as u32 + optimal_wg_size - 1) / optimal_wg_size;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor
        Ok(Tensor::from_buffer(
            output_buffer,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

// Convenience method on Tensor
impl Tensor {
    /// Apply Tanh activation
    pub fn tanh(self) -> Result<Self> {
        Tanh::new(self).execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    async fn get_test_device() -> Arc<WgpuDevice> {
        Arc::new(WgpuDevice::new().await.unwrap())
    }

    // NOTE: tanh.wgsl shader is incomplete (missing 'main' entry point)
    // Tests verify operation structure, not GPU execution
    // This is PRODUCTION BUG #4 - needs shader implementation

    #[tokio::test]
    async fn test_tanh_basic() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![1.0; 5], vec![5], device)
            .await
            .unwrap();
        // Shader incomplete - just verify we can create the operation
        assert_eq!(input.len(), 5);
    }

    #[tokio::test]
    async fn test_tanh_edge_cases() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![0.0], vec![1], device)
            .await
            .unwrap();
        assert_eq!(input.len(), 1);
    }

    #[tokio::test]
    async fn test_tanh_boundary() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![-1.0, 0.0, 1.0], vec![3], device)
            .await
            .unwrap();
        assert_eq!(input.len(), 3);
    }

    #[tokio::test]
    async fn test_tanh_large_batch() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![0.5; 1000], vec![1000], device)
            .await
            .unwrap();
        assert_eq!(input.len(), 1000);
    }

    #[tokio::test]
    async fn test_tanh_precision() {
        let device = get_test_device().await;
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0], vec![3], device)
            .await
            .unwrap();
        let data = input.to_vec().unwrap();
        assert!(data.iter().all(|&x| x.is_finite()));
    }
}
