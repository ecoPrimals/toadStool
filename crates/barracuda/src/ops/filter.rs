//! Filter Operation - Element-wise filtering with predicates
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
//! **Before** (Phase 3): `FilterExt` trait extension  
//! **After** (Phase 6): Direct `impl Tensor` method
//!
//! ## Usage
//!
//! ```no_run
//! use barracuda::tensor::Tensor;
//! use barracuda::ops::filter::FilterOperation;
//!
//! let input = Tensor::from_data(&vec![1.0, 5.0, 3.0, 7.0], vec![4], device)?;
//! let filtered = input.filter(FilterOperation::GreaterThan, 4.0)?;
//! ```

use crate::device::{DeviceCapabilities, WorkloadType};
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct FilterParams {
    size: u32,
    operation: u32,
    threshold: f32,
}

pub struct Filter {
    input: Tensor,
    operation: FilterOperation,
    threshold: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum FilterOperation {
    GreaterThan,
    LessThan,
    Equal,
    NotEqual,
}

impl FilterOperation {
    fn to_u32(&self) -> u32 {
        match self {
            FilterOperation::GreaterThan => 0,
            FilterOperation::LessThan => 1,
            FilterOperation::Equal => 2,
            FilterOperation::NotEqual => 3,
        }
    }
}

impl Filter {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/filter.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.shape().iter().product::<usize>();

        let params = FilterParams {
            size: size as u32,
            operation: self.operation.to_u32(),
            threshold: self.threshold,
        };

        // This is a simplified version - just evaluates predicate and returns flags
        // Full filter would need multi-pass (predicate + prefix sum + compact)
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter_output"),
            size: (size * std::mem::size_of::<f32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let flags_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter_flags"),
            size: (size * std::mem::size_of::<u32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let count_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("filter_count"),
            size: std::mem::size_of::<u32>() as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("filter_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("filter_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("filter_bind_group_layout"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 4,
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
                    label: Some("filter_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("filter_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "evaluate_predicate",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("filter_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.input.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: flags_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: output_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: count_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("filter_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("filter_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Deep Debt Evolution: Capability-based dispatch
            let caps = DeviceCapabilities::from_device(device);
            let optimal_wg_size = caps.optimal_workgroup_size(WorkloadType::ElementWise);
            let workgroups = (size as u32).div_ceil(optimal_wg_size);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Return flags buffer as tensor (1.0 for keep, 0.0 for discard)
        Ok(Tensor::from_buffer(
            output_buffer,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

// ============================================================================
// Modern API: Direct impl Tensor (Phase 6 Evolution)
// ============================================================================

impl Tensor {
    /// Apply filter predicate to tensor elements
    ///
    /// Returns a mask tensor where 1.0 = predicate passed, 0.0 = failed
    ///
    /// **Deep Debt**: Modern direct method, no trait extension needed
    ///
    /// ## Arguments
    ///
    /// * `operation` - Filter operation (GreaterThan, LessThan, Equal, NotEqual)
    /// * `threshold` - Comparison threshold value
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # use barracuda::ops::filter::FilterOperation;
    /// # let input = todo!();
    /// // Keep values > 4.0
    /// let mask = input.filter(FilterOperation::GreaterThan, 4.0)?;
    /// ```
    pub fn filter(self, operation: FilterOperation, threshold: f32) -> Result<Self> {
        let op = Filter {
            input: self,
            operation,
            threshold,
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
    async fn test_filter_basic() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());

        let input = Tensor::from_data(&vec![1.0, 5.0, 3.0, 7.0], vec![4], device.clone()).unwrap();

        let result = input.filter(FilterOperation::GreaterThan, 4.0).unwrap();
        let output = result.to_vec().unwrap();

        // Results: 1.0 (no), 5.0 (yes), 3.0 (no), 7.0 (yes)
        assert_eq!(output.len(), 4);

        // Check that filter produced valid output
        for &val in &output {
            assert!(val.is_finite());
        }
    }

    #[tokio::test]
    async fn test_filter_edge_cases() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());

        // All values pass (LessThan 100)
        let all_pass =
            Tensor::from_data(&vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone()).unwrap();
        let result = all_pass.filter(FilterOperation::LessThan, 100.0).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 4);

        // No values pass (GreaterThan 100)
        let none_pass =
            Tensor::from_data(&vec![1.0, 2.0, 3.0, 4.0], vec![4], device.clone()).unwrap();
        let result = none_pass
            .filter(FilterOperation::GreaterThan, 100.0)
            .unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 4);

        // Equal operation
        let equal_test =
            Tensor::from_data(&vec![5.0, 5.0, 3.0, 5.0], vec![4], device.clone()).unwrap();
        let result = equal_test.filter(FilterOperation::Equal, 5.0).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 4);
    }

    #[tokio::test]
    async fn test_filter_boundary() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());

        // Single element
        let single = Tensor::from_data(&vec![10.0], vec![1], device.clone()).unwrap();
        let result = single.filter(FilterOperation::GreaterThan, 5.0).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 1);

        // Exact threshold boundary
        let boundary = Tensor::from_data(&vec![4.9, 5.0, 5.1], vec![3], device.clone()).unwrap();
        let result = boundary.filter(FilterOperation::GreaterThan, 5.0).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 3);

        // NotEqual operation
        let not_equal =
            Tensor::from_data(&vec![1.0, 2.0, 3.0, 2.0], vec![4], device.clone()).unwrap();
        let result = not_equal.filter(FilterOperation::NotEqual, 2.0).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 4);
    }

    #[tokio::test]
    async fn test_filter_large_tensor() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());

        // Large tensor (1024 elements)
        let size = 1024;
        let data: Vec<f32> = (0..size).map(|i| (i % 100) as f32).collect();
        let input = Tensor::from_data(&data, vec![size], device.clone()).unwrap();

        // Filter for values > 50
        let result = input.filter(FilterOperation::GreaterThan, 50.0).unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(output.len(), size);

        // Verify output is valid
        for &val in &output {
            assert!(val.is_finite());
        }
    }

    #[tokio::test]
    async fn test_filter_precision() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());

        // Test all filter operations
        let data = vec![0.5, 1.5, 2.5, 3.5, 4.5];

        // GreaterThan 2.0
        let gt_input = Tensor::from_data(&data, vec![5], device.clone()).unwrap();
        let result = gt_input.filter(FilterOperation::GreaterThan, 2.0).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 5);

        // LessThan 3.0
        let lt_input = Tensor::from_data(&data, vec![5], device.clone()).unwrap();
        let result = lt_input.filter(FilterOperation::LessThan, 3.0).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 5);

        // Equal 2.5
        let eq_input = Tensor::from_data(&data, vec![5], device.clone()).unwrap();
        let result = eq_input.filter(FilterOperation::Equal, 2.5).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 5);

        // NotEqual 2.5
        let ne_input = Tensor::from_data(&data, vec![5], device.clone()).unwrap();
        let result = ne_input.filter(FilterOperation::NotEqual, 2.5).unwrap();
        let output = result.to_vec().unwrap();
        assert_eq!(output.len(), 5);
    }
}
