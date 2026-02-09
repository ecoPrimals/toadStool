//! Reduce Operation - Aggregation across tensor elements
//!
//! **Deep Debt Evolution**: Modernized from trait-based to direct `impl Tensor`
//!
//! ## Deep Debt Principles
//!
//! - ✅ Modern idiomatic Rust (direct `impl Tensor`, not trait extension)
//! - ✅ Universal compute (WGSL shader for all substrates)
//! - ✅ Safe Rust (no unsafe blocks)
//! - ✅ Agnostic design (operation enum, not hardcoded)
//!
//! ## Evolution History
//!
//! **Before** (Phase 3): `ReduceExt` trait extension  
//! **After** (Phase 6): Direct `impl Tensor` method
//!
//! ## Usage
//!
//! ```no_run
//! use barracuda::tensor::Tensor;
//! use barracuda::ops::reduce::ReduceOperation;
//!
//! let input = Tensor::from_data(&vec![1.0, 2.0, 3.0, 4.0], vec![4], device)?;
//! let sum_tensor = input.reduce(ReduceOperation::Sum)?;
//! ```

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct ReduceParams {
    size: u32,
    operation: u32,
    _pad0: u32,
    _pad1: u32,
}

pub struct Reduce {
    input: Tensor,
    operation: ReduceOperation,
}

#[derive(Debug, Clone, Copy)]
pub enum ReduceOperation {
    Sum,
    Max,
    Min,
    Mean,
}

impl ReduceOperation {
    fn to_u32(&self) -> u32 {
        match self {
            ReduceOperation::Sum => 0,
            ReduceOperation::Max => 1,
            ReduceOperation::Min => 2,
            ReduceOperation::Mean => 3,
        }
    }
}

impl Reduce {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/reduce.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.shape().iter().product::<usize>();

        let params = ReduceParams {
            size: size as u32,
            operation: self.operation.to_u32(),
            _pad0: 0,
            _pad1: 0,
        };

        // Deep Debt Evolution: Capability-based dispatch
        // Note: Shader hardcodes @workgroup_size(256), so we must use 256 here
        let workgroup_size = 256u32;  // Must match shader's @workgroup_size(256)
        let num_workgroups = (size as u32).div_ceil(workgroup_size);

        // Initialize output buffer to zeros
        let output_data = vec![0.0f32; num_workgroups as usize];
        let output_buffer = device.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("reduce_output"),
            contents: bytemuck::cast_slice(&output_data),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("reduce_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("reduce_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("reduce_bind_group_layout"),
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("reduce_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("reduce_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("reduce_bind_group"),
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
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("reduce_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("reduce_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            compute_pass.dispatch_workgroups(num_workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));
        
        // Ensure GPU finishes before returning
        device.device.poll(wgpu::Maintain::Wait);

        // Return partial results (caller can reduce further if needed)
        Ok(Tensor::from_buffer(
            output_buffer,
            vec![num_workgroups as usize],
            device.clone(),
        ))
    }
}

// ============================================================================
// Modern API: Direct impl Tensor (Phase 6 Evolution)
// ============================================================================

impl Tensor {
    /// Reduce tensor elements using aggregation operation
    ///
    /// Returns partial reduction results (caller can reduce further if needed)
    ///
    /// **Deep Debt**: Modern direct method, no trait extension needed
    ///
    /// ## Arguments
    ///
    /// * `operation` - Reduce operation (Sum, Max, Min, Mean)
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use barracuda::ops::reduce::ReduceOperation;
    /// # let input = todo!();
    /// // Sum all elements
    /// let partial_sums = input.reduce(ReduceOperation::Sum)?;
    /// let total: f32 = partial_sums.to_vec()?.iter().sum();
    ///
    /// // Find maximum
    /// let partial_maxes = input.reduce(ReduceOperation::Max)?;
    /// let max = partial_maxes.to_vec()?.iter().fold(f32::NEG_INFINITY, |a, &b| a.max(b));
    /// ```
    pub fn reduce(self, operation: ReduceOperation) -> Result<Self> {
        let op = Reduce {
            input: self,
            operation,
        };
        op.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_reduce_sum() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());

        let input = Tensor::from_data(&vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone()).unwrap();

        let result = input.reduce(ReduceOperation::Sum).unwrap();
        println!("Result shape: {:?}, len: {}", result.shape(), result.len());
        let partial_sums = result.to_vec().unwrap();

        // Sum all partial results
        let total: f32 = partial_sums.iter().sum();
        println!("Partial sums: {:?}, Total: {}", partial_sums, total);
        assert!((total - 10.0).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_reduce_max() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());

        let input = Tensor::from_data(&vec![1.0, 5.0, 3.0, 2.0], vec![4], device.clone()).unwrap();

        let result = input.reduce(ReduceOperation::Max).unwrap();
        let partial_maxes = result.to_vec().unwrap();

        // Max of partial results
        let max_val = partial_maxes
            .iter()
            .fold(f32::NEG_INFINITY, |a, &b| a.max(b));
        assert!((max_val - 5.0).abs() < 1e-5);
    }
}
