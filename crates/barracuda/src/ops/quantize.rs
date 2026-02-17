//! Quantize - Convert FP32 to INT8/INT4 quantization
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! Quantizes floating point values to low-precision integers.
//! Used for model compression and efficient inference.

use crate::device::DeviceCapabilities;
use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct QuantizeParams {
    size: u32,
    scale: f32,
    zero_point: f32,
    num_bits: u32,
    _padding: u32,
}

/// Quantize operation
pub struct Quantize {
    input: Tensor,
    scale: f32,
    zero_point: f32,
    num_bits: u32,
}

impl Quantize {
    /// Create quantize operation
    ///
    /// # Arguments
    /// * `input` - Input tensor (FP32)
    /// * `scale` - Quantization scale (inverse of quantization scale)
    /// * `zero_point` - Quantization zero point
    /// * `num_bits` - Number of bits (4 for INT4, 8 for INT8)
    pub fn new(input: Tensor, scale: f32, zero_point: f32, num_bits: u32) -> Result<Self> {
        if scale <= 0.0 {
            return Err(BarracudaError::invalid_op(
                "quantize",
                format!("scale must be positive, got {}", scale),
            ));
        }
        if num_bits != 4 && num_bits != 8 {
            return Err(BarracudaError::invalid_op(
                "quantize",
                format!("num_bits must be 4 or 8, got {}", num_bits),
            ));
        }

        Ok(Self {
            input,
            scale,
            zero_point,
            num_bits,
        })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/misc/quantize.wgsl")
    }

    /// Execute quantize on tensor
    /// Returns a tensor with i32 values (quantized integers)
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let size = self.input.len();

        // Create output buffer (i32 for quantized values)
        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quantize Output"),
            size: (size * std::mem::size_of::<i32>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Access input buffer directly (zero-copy)
        let input_buffer = self.input.buffer();

        // Create params
        let params = QuantizeParams {
            size: size as u32,
            scale: self.scale,
            zero_point: self.zero_point,
            num_bits: self.num_bits,
            _padding: 0,
        };

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Quantize Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Quantize Bind Group Layout"),
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

        // Create bind group
        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Quantize Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
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

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Quantize"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Quantize Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Quantize Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            cache: None,
            compilation_options: Default::default(),
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Quantize Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Quantize Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch using standard 1D shader workgroup size (256)
            let caps = DeviceCapabilities::from_device(device);
            let workgroups = caps.dispatch_1d(size as u32);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Create staging buffer for reading i32 data
        let staging_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quantize Staging Buffer"),
            size: (size * std::mem::size_of::<i32>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Copy output to staging buffer (must be same encoder - compute must run before copy)
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (size * std::mem::size_of::<i32>()) as u64,
        );
        device.queue.submit(Some(encoder.finish()));

        // Read i32 data from staging buffer
        let buffer_slice = staging_buffer.slice(..);
        let (sender, receiver) = futures::channel::oneshot::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device.device.poll(wgpu::Maintain::Wait);

        futures::executor::block_on(receiver)
            .map_err(|e| BarracudaError::gpu(format!("Failed to map buffer: {:?}", e)))?
            .map_err(|e| BarracudaError::gpu(format!("Buffer mapping error: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let i32_data: &[i32] = bytemuck::cast_slice(&data);
        let f32_data: Vec<f32> = i32_data.iter().map(|&x| x as f32).collect();
        drop(data);
        staging_buffer.unmap();

        // Create tensor from f32 data (values represent quantized integers)
        Tensor::from_data(&f32_data, self.input.shape().to_vec(), device.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_quantize_basic() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_vec_on(vec![-1.0, 0.0, 1.0], vec![3], device.clone())
            .await
            .unwrap();

        let output = Quantize::new(input, 0.01, 0.0, 8)
            .unwrap()
            .execute()
            .unwrap();
        let result = output.to_vec().unwrap();

        assert_eq!(result.len(), 3);
        // Values should be quantized (as f32 representation of i32)
    }

    #[tokio::test]
    async fn test_quantize_edge_cases() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        // Test clamping at boundaries
        let input = Tensor::from_vec_on(vec![-1000.0, 1000.0, 0.0], vec![3], device.clone())
            .await
            .unwrap();

        let output = Quantize::new(input, 1.0, 0.0, 8)
            .unwrap()
            .execute()
            .unwrap();
        let result = output.to_vec().unwrap();

        // Should clamp to INT8 range [-128, 127]
        assert_eq!(result[0] as i32, -128);
        assert_eq!(result[1] as i32, 127);
        assert_eq!(result[2] as i32, 0);
    }

    #[tokio::test]
    async fn test_quantize_int4() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let input = Tensor::from_vec_on(vec![-10.0, 0.0, 10.0], vec![3], device.clone())
            .await
            .unwrap();

        let output = Quantize::new(input, 1.0, 0.0, 4)
            .unwrap()
            .execute()
            .unwrap();
        let result = output.to_vec().unwrap();

        // Should clamp to INT4 range [-8, 7]
        assert_eq!(result[0] as i32, -8);
        assert_eq!(result[1] as i32, 0);
        assert_eq!(result[2] as i32, 7);
    }
}
