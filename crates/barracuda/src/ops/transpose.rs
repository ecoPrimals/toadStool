//! Transpose operation
//!
//! **Pure WGSL**: Single implementation via WebGPU shader
//! Transposes last two dimensions of a tensor

use crate::error::{BarracudaError, Result};
use crate::tensor::Tensor;

/// Transpose operation parameters
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct TransposeParams {
    rows: u32,
    cols: u32,
    _padding: [u32; 2],
}

/// Transpose operation
pub struct Transpose {
    input: Tensor,
}

impl Transpose {
    /// Create Transpose operation
    pub fn new(input: Tensor) -> Result<Self> {
        // For now, only support 2D tensors
        if input.shape().len() != 2 {
            return Err(BarracudaError::invalid_op(
                "Transpose",
                format!("Only 2D tensors supported, got shape {:?}", input.shape()),
            ));
        }
        Ok(Self { input })
    }

    /// WGSL shader source (embedded at compile time)
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/transpose.wgsl")
    }

    /// Execute transpose on tensor
    pub fn execute(self) -> Result<Tensor> {
        let device = self.input.device();
        let shape = self.input.shape();
        let rows = shape[0] as u32;
        let cols = shape[1] as u32;
        let size = self.input.len();

        // Create output buffer (same size, transposed shape)
        let output_buffer = device.create_buffer_f32(size)?;

        // Create params buffer
        let params = TransposeParams {
            rows,
            cols,
            _padding: [0, 0],
        };
        let params_bytes = bytemuck::bytes_of(&params);
        let params_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Transpose Params"),
            size: params_bytes.len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        device.queue.write_buffer(&params_buffer, 0, params_bytes);

        // Create bind group layout
        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("Transpose Bind Group Layout"),
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
            label: Some("Transpose Bind Group"),
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

        // Compile shader
        let shader = device.compile_shader(Self::wgsl_shader(), Some("Transpose"));

        // Create pipeline
        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Transpose Pipeline Layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Transpose Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        // Encode and execute
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Transpose Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Transpose Pass"),
                timestamp_writes: None,
            });

            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);

            // Dispatch workgroups (256 threads per workgroup)
            let workgroups = (size as u32 + 255) / 256;
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        // Create output tensor with transposed shape
        let new_shape = vec![shape[1], shape[0]];
        Ok(Tensor::from_buffer(
            output_buffer,
            new_shape,
            device.clone(),
        ))
    }
}

// Convenience method on Tensor
impl Tensor {
    /// Transpose tensor (swap last two dimensions)
    pub fn transpose(&self) -> Result<Self> {
        Transpose::new(self.clone())?.execute()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_transpose_basic() {
        let device = crate::device::Auto::new().await.unwrap();
        let device = Arc::new(device);

        // Test data: 2x3 matrix [[1,2,3], [4,5,6]]
        let input = Tensor::from_vec_on(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], vec![2, 3], device)
            .await
            .unwrap();

        let output = input.transpose().unwrap();
        let result = output.to_vec().unwrap();

        // Expected: 3x2 matrix [[1,4], [2,5], [3,6]]
        let expected = vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0];
        assert_eq!(output.shape(), &[3, 2]);
        for (i, (&r, &e)) in result.iter().zip(expected.iter()).enumerate() {
            assert!(
                (r - e).abs() < 1e-5,
                "Mismatch at index {}: {} vs {}",
                i,
                r,
                e
            );
        }
    }
}
