//! Tiled Matrix Multiplication - High-performance matmul with shared memory
//!
//! **Deep Debt Evolution**: Modernized from trait-based to direct `impl Tensor`
//!
//! ## Deep Debt Principles
//!
//! - ✅ Modern idiomatic Rust (direct `impl Tensor`, not trait extension)
//! - ✅ Universal compute (WGSL shader for all substrates)
//! - ✅ Safe Rust (no unsafe blocks)
//! - ✅ High performance (tile-based blocking for cache efficiency)
//!
//! ## Evolution History
//!
//! **Before** (Phase 3): `MatmulTiledExt` trait extension  
//! **After** (Phase 6): Direct `impl Tensor` method
//!
//! ## Usage
//!
//! ```no_run
//! use barracuda::tensor::Tensor;
//!
//! let a = Tensor::from_data(&data_a, vec![128, 256], device)?;
//! let b = Tensor::from_data(&data_b, vec![256, 512], device)?;
//! let c = a.matmul_tiled(&b)?;  // Result: [128, 512]
//! ```

use crate::error::Result;
use crate::tensor::Tensor;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct MatmulTiledParams {
    m: u32,
    k: u32,
    n: u32,
}

pub struct MatmulTiled {
    a: Tensor,
    b: Tensor,
}

impl MatmulTiled {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/matmul_tiled.wgsl")
    }

    pub fn execute(self) -> Result<Tensor> {
        let device = self.a.device();
        let a_shape = self.a.shape();
        let b_shape = self.b.shape();

        if a_shape.len() != 2 || b_shape.len() != 2 {
            return Err(crate::error::BarracudaError::invalid_op(
                "MatmulTiled",
                format!(
                    "Expected 2D tensors, got shapes {:?} and {:?}",
                    a_shape, b_shape
                ),
            ));
        }

        let m = a_shape[0];
        let k = a_shape[1];
        let n = b_shape[1];

        if b_shape[0] != k {
            return Err(crate::error::BarracudaError::invalid_op(
                "MatmulTiled",
                format!("Inner dimensions must match: {} != {}", k, b_shape[0]),
            ));
        }

        let params = MatmulTiledParams {
            m: m as u32,
            k: k as u32,
            n: n as u32,
        };

        let output_shape = vec![m, n];
        let output_size = output_shape.iter().product::<usize>() * std::mem::size_of::<f32>();

        let output_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("matmul_tiled_output"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("matmul_tiled_params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let shader = device
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("matmul_tiled_shader"),
                source: wgpu::ShaderSource::Wgsl(Self::wgsl_shader().into()),
            });

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("matmul_tiled_bind_group_layout"),
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("matmul_tiled_pipeline_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("matmul_tiled_pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
            });

        let bind_group = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("matmul_tiled_bind_group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.a.buffer().as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: self.b.buffer().as_entire_binding(),
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

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("matmul_tiled_encoder"),
            });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("matmul_tiled_pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&pipeline);
            compute_pass.set_bind_group(0, &bind_group, &[]);

            // Tile size is 16x16 per workgroup
            let workgroups_x = ((n + 15) / 16) as u32;
            let workgroups_y = ((m + 15) / 16) as u32;
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        device.queue.submit(Some(encoder.finish()));

        Ok(Tensor::from_buffer(
            output_buffer,
            output_shape,
            device.clone(),
        ))
    }
}

// ============================================================================
// Modern API: Direct impl Tensor (Phase 6 Evolution)
// ============================================================================

impl Tensor {
    /// High-performance tiled matrix multiplication
    ///
    /// Uses tile-based blocking for improved cache locality and performance
    ///
    /// **Deep Debt**: Modern direct method, no trait extension needed
    ///
    /// ## Arguments
    ///
    /// * `b` - Second matrix (columns must match self's rows)
    ///
    /// ## Matrix Dimensions
    ///
    /// - `self`: [M, K]
    /// - `b`: [K, N]
    /// - Result: [M, N]
    ///
    /// ## Example
    ///
    /// ```no_run
    /// # let a = todo!();
    /// # let b = todo!();
    /// // C = A × B (optimized with tiling)
    /// let c = a.matmul_tiled(&b)?;
    /// ```
    pub fn matmul_tiled(self, b: &Self) -> Result<Self> {
        let op = MatmulTiled {
            a: self,
            b: b.clone(),
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
    async fn test_matmul_tiled() {
        let device = Arc::new(WgpuDevice::new().await.unwrap());

        // 2x3 * 3x2 = 2x2
        let a = Tensor::from_data(
            &vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            vec![2, 3],
            device.clone(),
        )
        .unwrap();

        let b = Tensor::from_data(
            &vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0],
            vec![3, 2],
            device.clone(),
        )
        .unwrap();

        let result = a.matmul_tiled(&b).unwrap();
        let output = result.to_vec().unwrap();

        assert_eq!(result.shape(), &[2, 2]);
        assert_eq!(output.len(), 4);
    }
}
