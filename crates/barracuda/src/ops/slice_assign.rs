//! Slice Assign - In-place slice assignment with strided writes - Pure WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its slice parameters
//! - Zero hardcoding: All parameters passed at runtime
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute

use crate::device::DeviceCapabilities;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

/// f64 is the canonical source — f32 derived via downcast_f64_to_f32 when needed.
const SHADER_F64: &str = include_str!("../shaders/math/slice_assign_f64.wgsl");

static SHADER_F32: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| crate::shaders::precision::downcast_f64_to_f32(SHADER_F64));

/// Slice assignment parameters
#[derive(Debug, Clone)]
pub struct SliceRange {
    pub start: usize,
    pub end: usize,
    pub stride: usize,
}

/// Slice Assign operation - In-place slice assignment
pub struct SliceAssign {
    input: Tensor,
    slice_range: SliceRange,
    values: Tensor,
}

impl SliceAssign {
    /// Create a new slice assign operation
    pub fn new(input: Tensor, slice_range: SliceRange, values: Tensor) -> Result<Self> {
        let input_shape = input.shape();
        let input_size = input_shape.iter().product::<usize>();
        let values_size = values.shape().iter().product::<usize>();

        // Validate slice range
        if slice_range.start >= slice_range.end {
            return Err(BarracudaError::invalid_op(
                "SliceAssign",
                format!(
                    "Start {} must be less than end {}",
                    slice_range.start, slice_range.end
                ),
            ));
        }

        if slice_range.end > input_size {
            return Err(BarracudaError::invalid_op(
                "SliceAssign",
                format!("End {} exceeds input size {}", slice_range.end, input_size),
            ));
        }

        if slice_range.stride == 0 {
            return Err(BarracudaError::invalid_op(
                "SliceAssign",
                "Stride must be greater than zero",
            ));
        }

        // Calculate expected slice size
        let slice_size = (slice_range.end - slice_range.start).div_ceil(slice_range.stride);
        if values_size != slice_size {
            return Err(BarracudaError::invalid_op(
                "SliceAssign",
                format!(
                    "Values size {} doesn't match slice size {}",
                    values_size, slice_size
                ),
            ));
        }

        Ok(Self {
            input,
            slice_range,
            values,
        })
    }

    /// Get the WGSL shader source
    fn wgsl_shader() -> &'static str {
        &SHADER_F32
    }

    /// Execute the slice assign operation (modifies input in-place)
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let input_size: usize = self.input.shape().iter().product();
        let values_size = self.values.shape().iter().product::<usize>();

        // Access buffers directly (zero-copy)
        let input_buffer = self.input.buffer();
        let values_buffer = self.values.buffer();

        // Create uniform buffer for parameters
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            input_size: u32,
            start: u32,
            end: u32,
            stride: u32,
            values_size: u32,
        }

        let params = Params {
            input_size: input_size as u32,
            start: self.slice_range.start as u32,
            end: self.slice_range.end as u32,
            stride: self.slice_range.stride as u32,
            values_size: values_size as u32,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SliceAssign Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("SliceAssign Bind Group Layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
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
            label: Some("SliceAssign Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: values_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: input_buffer.as_entire_binding(),
                },
            ],
        });

        // Compile shader
        let shader_module = device.compile_shader(Self::wgsl_shader(), Some("SliceAssign Shader"));

        // Create compute pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("SliceAssign Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let compute_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("SliceAssign Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        // Execute compute shader
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("SliceAssign Encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("SliceAssign Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&compute_pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);
            // Dispatch using standard 1D shader workgroup size (256)
            let caps = DeviceCapabilities::from_device(device);
            let workgroups = caps.dispatch_1d(values_size as u32);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        let output_data = crate::utils::read_buffer(device, input_buffer, input_size)?;
        Ok(Tensor::new(
            output_data,
            self.input.shape().to_vec(),
            device.clone(),
        ))
    }
}

impl Tensor {
    /// Assign values to a slice of the tensor (in-place)
    ///
    /// # Arguments
    ///
    /// * `slice_range` - Slice range (start, end, stride)
    /// * `values` - Values to assign
    pub fn slice_assign(self, slice_range: SliceRange, values: Tensor) -> Result<Self> {
        SliceAssign::new(self, slice_range, values)?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn get_test_device() -> Option<std::sync::Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_gpu_available().await
    }

    #[tokio::test]
    async fn test_slice_assign_basic() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let input = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![5], device.clone());
        let values = Tensor::new(vec![10.0, 20.0], vec![2], device.clone());

        let slice_range = SliceRange {
            start: 1,
            end: 3,
            stride: 1,
        };

        let result = input.slice_assign(slice_range, values).unwrap();
        let output_data = result.to_vec().unwrap();

        // Expected: [1, 10, 20, 4, 5] (indices 1,2 assigned)
        assert_eq!(output_data[0], 1.0);
        assert_eq!(output_data[1], 10.0);
        assert_eq!(output_data[2], 20.0);
        assert_eq!(output_data[3], 4.0);
        assert_eq!(output_data[4], 5.0);
    }

    #[tokio::test]
    async fn test_slice_assign_strided() {
        let Some(device) = get_test_device().await else {
            return;
        };
        let input = Tensor::new(vec![1.0, 2.0, 3.0, 4.0, 5.0], vec![5], device.clone());
        // Stride 2 over [0..5] yields indices 0, 2, 4 → need 3 values
        let values = Tensor::new(vec![10.0, 20.0, 30.0], vec![3], device.clone());

        let slice_range = SliceRange {
            start: 0,
            end: 5,
            stride: 2,
        };

        let result = input.slice_assign(slice_range, values).unwrap();
        let output_data = result.to_vec().unwrap();

        // Expected: [10, 2, 20, 4, 30] (stride 2: indices 0, 2, 4)
        assert_eq!(output_data[0], 10.0);
        assert_eq!(output_data[1], 2.0);
        assert_eq!(output_data[2], 20.0);
        assert_eq!(output_data[3], 4.0);
        assert_eq!(output_data[4], 30.0);
    }
}
