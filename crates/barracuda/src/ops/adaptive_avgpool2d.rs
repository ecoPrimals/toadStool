//! Adaptive Average Pooling 2D - Output-size-driven pooling
//!
//! **Deep Debt Evolution**: Modernized from trait-based to direct `impl Tensor`
//!
//! ## Deep Debt Principles
//!
//! - ✅ Modern idiomatic Rust (direct `impl Tensor`, not trait extension)
//! - ✅ Universal compute (WGSL shader for all substrates)
//! - ✅ Safe Rust (no unsafe blocks)
//! - ✅ Flexible (specify output size, not kernel size)
//!
//! ## Evolution History
//!
//! **Before** (Phase 3): `AdaptiveAvgPool2DExt` trait extension  
//! **After** (Phase 6): Direct `impl Tensor` method
//!
//! ## Usage
//!
//! ```no_run
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! # use barracuda::tensor::Tensor;
//! # use barracuda::device::test_pool;
//! # let device = futures::executor::block_on(test_pool::get_test_device_if_gpu_available()).unwrap();
//! # let input = Tensor::from_data(&[1.0f32; 196], vec![1, 1, 14, 14], device).unwrap();
//! // Input: [batch, channels, 14, 14]
//! // Output: [batch, channels, 7, 7] (adaptive to target size)
//! let _pooled = input.adaptive_avgpool2d((7, 7))?;
//! # Ok(())
//! # }
//! ```

use crate::device::DeviceCapabilities;
use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct AdaptiveAvgPool2DParams {
    batch: u32,
    channels: u32,
    in_height: u32,
    in_width: u32,
    out_height: u32,
    out_width: u32,
}

pub struct AdaptiveAvgPool2D {
    input: Tensor,
    output_size: (usize, usize),
}

impl AdaptiveAvgPool2D {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/pooling/adaptive_avgpool2d.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();

        if shape.len() != 4 {
            return Err(crate::error::BarracudaError::invalid_op("Shape Error", 
                format!("AdaptiveAvgPool2D expects 4D input [batch, channels, height, width], got shape {:?}", shape)
            ));
        }

        let batch = shape[0];
        let channels = shape[1];
        let in_height = shape[2];
        let in_width = shape[3];
        let (out_height, out_width) = self.output_size;

        let params = AdaptiveAvgPool2DParams {
            batch: batch as u32,
            channels: channels as u32,
            in_height: in_height as u32,
            in_width: in_width as u32,
            out_height: out_height as u32,
            out_width: out_width as u32,
        };

        let output_shape = &[batch, channels, out_height, out_width];
        let output_size = output_shape.iter().product::<usize>() * std::mem::size_of::<f32>();

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("adaptive_avgpool2d_output"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("adaptive_avgpool2d_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("adaptive_avgpool2d_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("adaptive_avgpool2d_bind_group_layout"),
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
                    label: Some("adaptive_avgpool2d_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("adaptive_avgpool2d_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("adaptive_avgpool2d_bind_group"),
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
                label: Some("adaptive_avgpool2d_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("adaptive_avgpool2d_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch using standard 2D shader workgroup size (16, 16)
            let caps = DeviceCapabilities::from_device(device);
            let (workgroups_x, workgroups_y) =
                caps.dispatch_2d(out_width as u32, out_height as u32);
            let workgroups_z = (batch * channels) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, workgroups_z);
        }

        device.queue.submit(Some(encoder.finish()));

        let output_elem_count = output_shape.iter().product::<usize>();
        let output_data = crate::utils::read_buffer(device, &output_buffer, output_elem_count)?;
        Ok(Tensor::new(
            output_data,
            output_shape.to_vec(),
            device.clone(),
        ))
    }
}

// ============================================================================
// Modern API: Direct impl Tensor (Phase 6 Evolution)
// ============================================================================

impl Tensor {
    /// Apply adaptive average pooling with target output size
    ///
    /// Automatically calculates kernel/stride to achieve desired output dimensions
    ///
    /// **Deep Debt**: Modern direct method, no trait extension needed
    ///
    /// ## Arguments
    ///
    /// * `output_size` - Target (height, width) for output
    ///
    /// ## Input/Output Shapes
    ///
    /// - Input: `[batch, channels, height_in, width_in]`
    /// - Output: `[batch, channels, height_out, width_out]`
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// # use barracuda::tensor::Tensor;
    /// # use barracuda::device::test_pool;
    /// # let device = futures::executor::block_on(test_pool::get_test_device_if_gpu_available()).unwrap();
    /// # let input = Tensor::from_data(&[1.0f32; 49], vec![1, 1, 7, 7], device).unwrap();
    /// // Adaptive pool to 7x7 (regardless of input size)
    /// let _pooled = input.adaptive_avgpool2d((7, 7))?;
    /// # Ok(())
    /// # }
    /// ```
    pub fn adaptive_avgpool2d(self, output_size: (usize, usize)) -> Result<Self> {
        let op = AdaptiveAvgPool2D {
            input: self,
            output_size,
        };
        op.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_adaptive_avgpool2d() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        // Test case: 4x4 input -> 2x2 output
        let input = Tensor::from_data(
            &vec![
                // Batch 0, Channel 0
                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0, 13.0, 14.0, 15.0,
                16.0,
            ],
            vec![1, 1, 4, 4],
            device.clone(),
        )
        .unwrap();

        let result = input.adaptive_avgpool2d((2, 2)).unwrap();
        let output = result.to_vec().unwrap();

        // Each 2x2 region should be averaged
        assert_eq!(result.shape(), &[1, 1, 2, 2]);
        assert_eq!(output.len(), 4);

        // Top-left: avg(1,2,5,6) = 3.5
        assert!((output[0] - 3.5).abs() < 1e-5);
        // Top-right: avg(3,4,7,8) = 5.5
        assert!((output[1] - 5.5).abs() < 1e-5);
        // Bottom-left: avg(9,10,13,14) = 11.5
        assert!((output[2] - 11.5).abs() < 1e-5);
        // Bottom-right: avg(11,12,15,16) = 13.5
        assert!((output[3] - 13.5).abs() < 1e-5);
    }

    #[tokio::test]
    async fn test_adaptive_avgpool2d_1x1_output() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        // Test global average pooling (adaptive pool to 1x1)
        let input =
            Tensor::from_data(&vec![1.0, 2.0, 3.0, 4.0], vec![1, 1, 2, 2], device.clone()).unwrap();

        let result = input.adaptive_avgpool2d((1, 1)).unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(result.shape(), &[1, 1, 1, 1]);
        assert_eq!(output.len(), 1);
        assert!((output[0] - 2.5).abs() < 1e-5); // avg(1,2,3,4) = 2.5
    }
}
