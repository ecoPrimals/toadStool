//! GPU helpers for sparse linear algebra
//!
//! This module provides common utilities for GPU-accelerated sparse solvers,
//! extracted from cg_gpu.rs to enable code reuse across solvers.
//!
//! **DEEP DEBT EVOLUTION**: Smart refactoring - domain separation, not just splitting.
//!
//! ## Design Principles
//!
//! - **Single Responsibility**: Buffer creation, BGL creation, and I/O are separate concerns
//! - **Reusability**: These helpers work for CG, BiCGSTAB, and future solvers
//! - **Type Safety**: Strongly typed buffer creation prevents precision mismatches
//! - **Performance**: Zero-cost abstractions over wgpu

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Buffer creation helpers for sparse GPU solvers
pub struct SparseBuffers;

impl SparseBuffers {
    /// Create an f64 storage buffer from data
    pub fn f64_from_slice(device: &Arc<WgpuDevice>, label: &str, data: &[f64]) -> wgpu::Buffer {
        Self::f64_from_slice_raw(&device.device, label, data)
    }

    /// Create a zero-initialized f64 storage buffer
    pub fn f64_zeros(device: &Arc<WgpuDevice>, label: &str, count: usize) -> wgpu::Buffer {
        Self::f64_zeros_raw(&device.device, label, count)
    }

    /// Create an f64 storage buffer from slice (raw device)
    pub fn f64_from_slice_raw(device: &wgpu::Device, label: &str, data: &[f64]) -> wgpu::Buffer {
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &bytes,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    /// Create a zero-initialized f64 storage buffer (raw device)
    pub fn f64_zeros_raw(device: &wgpu::Device, label: &str, count: usize) -> wgpu::Buffer {
        let zeros = vec![0u8; count * 8];
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: &zeros,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
        })
    }

    /// Create a zero-initialized i32 storage buffer (raw device)
    pub fn i32_zeros_raw(device: &wgpu::Device, label: &str, count: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (count * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    /// Read f64 data from GPU buffer (raw device/queue)
    pub fn read_f64_raw(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f64>> {
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("f64 staging"),
            size: (count * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("f64 readback"),
        });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (count * 8) as u64);
        queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = slice.get_mapped_range();
        // SAFETY: chunks_exact(8) guarantees exactly 8-byte chunks
        let result: Vec<f64> = data
            .chunks_exact(8)
            .map(|chunk| {
                f64::from_le_bytes(
                    chunk
                        .try_into()
                        .expect("chunks_exact(8) yields 8-byte chunks"),
                )
            })
            .collect();
        drop(data);
        staging.unmap();

        Ok(result)
    }

    /// Read i32 data from GPU buffer (raw device/queue)
    pub fn read_i32_raw(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<i32>> {
        let staging = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("i32 staging"),
            size: (count * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("i32 readback"),
        });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (count * 4) as u64);
        queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = slice.get_mapped_range();
        // SAFETY: chunks_exact(4) guarantees exactly 4-byte chunks
        let result: Vec<i32> = data
            .chunks_exact(4)
            .map(|chunk| {
                i32::from_le_bytes(
                    chunk
                        .try_into()
                        .expect("chunks_exact(4) yields 4-byte chunks"),
                )
            })
            .collect();
        drop(data);
        staging.unmap();

        Ok(result)
    }

    /// Create a u32 storage buffer from usize data (for CSR indices)
    pub fn u32_from_usize(device: &Arc<WgpuDevice>, label: &str, data: &[usize]) -> wgpu::Buffer {
        let u32_data: Vec<u32> = data.iter().map(|&x| x as u32).collect();
        device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(&u32_data),
                usage: wgpu::BufferUsages::STORAGE,
            })
    }

    /// Create a uniform buffer from u32 params
    pub fn uniform_u32(device: &Arc<WgpuDevice>, label: &str, params: &[u32]) -> wgpu::Buffer {
        device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: bytemuck::cast_slice(params),
                usage: wgpu::BufferUsages::UNIFORM,
            })
    }

    /// Read f64 data from GPU buffer
    pub fn read_f64(
        device: &Arc<WgpuDevice>,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f64>> {
        Self::read_f64_raw(&device.device, &device.queue, buffer, count)
    }

    /// Write f64 data to GPU buffer
    pub fn write_f64(device: &Arc<WgpuDevice>, buffer: &wgpu::Buffer, data: &[f64]) {
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        device.queue.write_buffer(buffer, 0, &bytes);
    }

    /// Copy buffer to buffer (f64)
    pub fn copy_f64(
        device: &Arc<WgpuDevice>,
        src: &wgpu::Buffer,
        dst: &wgpu::Buffer,
        count: usize,
    ) {
        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Buffer copy"),
            });
        encoder.copy_buffer_to_buffer(src, 0, dst, 0, (count * 8) as u64);
        device.queue.submit(Some(encoder.finish()));
    }
}

/// Bind group layout builders for common sparse operations
pub struct SparseBindGroupLayouts;

impl SparseBindGroupLayouts {
    /// Helper to create a storage buffer entry (read-only)
    fn storage_ro(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    /// Helper to create a storage buffer entry (read-write)
    fn storage_rw(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    /// Helper to create a uniform buffer entry
    fn uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    /// SpMV: values, col_idx, row_ptr, x, y, params
    pub fn spmv(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SpMV BGL"),
                entries: &[
                    Self::storage_ro(0), // values
                    Self::storage_ro(1), // col_indices
                    Self::storage_ro(2), // row_ptr
                    Self::storage_ro(3), // x
                    Self::storage_rw(4), // y
                    Self::uniform(5),    // params
                ],
            })
    }

    /// Dot product: a, b, partial_sums, params
    pub fn dot(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Dot BGL"),
                entries: &[
                    Self::storage_ro(0), // a
                    Self::storage_ro(1), // b
                    Self::storage_rw(2), // partial_sums
                    Self::uniform(3),    // params
                ],
            })
    }

    /// Final reduction: partial_sums, result, params
    pub fn reduce(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Reduce BGL"),
                entries: &[
                    Self::storage_ro(0), // partial_sums
                    Self::storage_rw(1), // result
                    Self::uniform(2),    // params
                ],
            })
    }

    /// AXPY: x, y, params (alpha is in params struct)
    pub fn axpy(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("AXPY BGL"),
                entries: &[
                    Self::storage_ro(0), // x
                    Self::storage_rw(1), // y (y = y + alpha*x)
                    Self::uniform(2),    // params (includes alpha)
                ],
            })
    }

    /// CG update xr: x, r, p, Ap, alpha, params (all read_write for consistency)
    pub fn cg_update_xr(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("CG update_xr BGL"),
                entries: &[
                    Self::storage_rw(0), // x
                    Self::storage_rw(1), // r
                    Self::storage_rw(2), // p
                    Self::storage_rw(3), // Ap
                    Self::storage_rw(4), // alpha
                    Self::uniform(5),    // params
                ],
            })
    }

    /// CG update p: r, p, beta, params (all read_write for consistency)
    pub fn cg_update_p(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("CG update_p BGL"),
                entries: &[
                    Self::storage_rw(0), // r (or z for preconditioned)
                    Self::storage_rw(1), // p
                    Self::storage_rw(2), // beta
                    Self::uniform(3),    // params
                ],
            })
    }

    /// Compute alpha: rz, pAp, alpha (all read_write for consistency)
    pub fn compute_alpha(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute alpha BGL"),
                entries: &[
                    Self::storage_rw(0), // rz
                    Self::storage_rw(1), // pAp
                    Self::storage_rw(2), // alpha
                ],
            })
    }

    /// Compute beta: rz_new, rz, beta (all read_write for consistency)
    pub fn compute_beta(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Compute beta BGL"),
                entries: &[
                    Self::storage_rw(0), // rz_new
                    Self::storage_rw(1), // rz
                    Self::storage_rw(2), // beta
                ],
            })
    }

    /// Preconditioner: r, diag, z, params
    pub fn precond(device: &Arc<WgpuDevice>) -> wgpu::BindGroupLayout {
        device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Precond BGL"),
                entries: &[
                    Self::storage_ro(0), // r
                    Self::storage_ro(1), // diag (M⁻¹)
                    Self::storage_rw(2), // z
                    Self::uniform(3),    // params
                ],
            })
    }
}

/// CG-specific pipelines (SpMV, dot, reduce, update_xr, update_p, alpha, beta)
pub struct CgPipelineSet {
    pub spmv: wgpu::ComputePipeline,
    pub dot: wgpu::ComputePipeline,
    pub reduce: wgpu::ComputePipeline,
    pub update_xr: wgpu::ComputePipeline,
    pub update_p: wgpu::ComputePipeline,
    pub compute_alpha: wgpu::ComputePipeline,
    pub compute_beta: wgpu::ComputePipeline,
}

impl CgPipelineSet {
    /// Create all CG pipelines from spmv/dot shader and cg_kernels shader
    pub fn new(
        device: &Arc<WgpuDevice>,
        spmv_shader: &wgpu::ShaderModule,
        dot_reduce_shader: &wgpu::ShaderModule,
        cg_kernels_shader: &wgpu::ShaderModule,
        spmv_bgl: &wgpu::BindGroupLayout,
        dot_bgl: &wgpu::BindGroupLayout,
        reduce_bgl: &wgpu::BindGroupLayout,
        update_xr_bgl: &wgpu::BindGroupLayout,
        update_p_bgl: &wgpu::BindGroupLayout,
        compute_alpha_bgl: &wgpu::BindGroupLayout,
        compute_beta_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let spmv = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("SpMV f64"),
                layout: Some(&device.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("SpMV PL"),
                        bind_group_layouts: &[spmv_bgl],
                        push_constant_ranges: &[],
                    },
                )),
                module: spmv_shader,
                entry_point: "spmv_f64",
                cache: None,
                compilation_options: Default::default(),
            });
        let dot = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Dot f64"),
                layout: Some(&device.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("Dot PL"),
                        bind_group_layouts: &[dot_bgl],
                        push_constant_ranges: &[],
                    },
                )),
                module: dot_reduce_shader,
                entry_point: "dot_f64",
                cache: None,
                compilation_options: Default::default(),
            });
        let reduce = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Final reduce f64"),
                layout: Some(&device.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("Reduce PL"),
                        bind_group_layouts: &[reduce_bgl],
                        push_constant_ranges: &[],
                    },
                )),
                module: dot_reduce_shader,
                entry_point: "final_reduce_f64",
                cache: None,
                compilation_options: Default::default(),
            });
        let update_xr = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("CG update xr"),
                layout: Some(&device.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("Update xr PL"),
                        bind_group_layouts: &[update_xr_bgl],
                        push_constant_ranges: &[],
                    },
                )),
                module: cg_kernels_shader,
                entry_point: "cg_update_xr",
                cache: None,
                compilation_options: Default::default(),
            });
        let update_p = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("CG update p"),
                layout: Some(&device.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("Update p PL"),
                        bind_group_layouts: &[update_p_bgl],
                        push_constant_ranges: &[],
                    },
                )),
                module: cg_kernels_shader,
                entry_point: "cg_update_p",
                cache: None,
                compilation_options: Default::default(),
            });
        let compute_alpha =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute alpha"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Compute alpha PL"),
                            bind_group_layouts: &[compute_alpha_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: cg_kernels_shader,
                    entry_point: "compute_alpha",
                    cache: None,
                    compilation_options: Default::default(),
                });
        let compute_beta =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Compute beta"),
                    layout: Some(&device.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: Some("Compute beta PL"),
                            bind_group_layouts: &[compute_beta_bgl],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: cg_kernels_shader,
                    entry_point: "compute_beta",
                    cache: None,
                    compilation_options: Default::default(),
                });
        Self {
            spmv,
            dot,
            reduce,
            update_xr,
            update_p,
            compute_alpha,
            compute_beta,
        }
    }
}

/// Helper to dispatch a compute pass (reduces verbosity in CG iteration)
#[inline(always)]
pub fn cg_dispatch_pass(
    pass: &mut wgpu::ComputePass<'_>,
    pipeline: &wgpu::ComputePipeline,
    bind_group: &wgpu::BindGroup,
    x: u32,
    y: u32,
    z: u32,
) {
    pass.set_pipeline(pipeline);
    pass.set_bind_group(0, bind_group, &[]);
    pass.dispatch_workgroups(x, y, z);
}

/// Pipeline builder for sparse operations
pub struct SparsePipelines {
    pub spmv: wgpu::ComputePipeline,
    pub dot: wgpu::ComputePipeline,
    pub reduce: wgpu::ComputePipeline,
}

impl SparsePipelines {
    /// Create common sparse pipelines from shader module
    pub fn new(
        device: &Arc<WgpuDevice>,
        shader: &wgpu::ShaderModule,
        spmv_bgl: &wgpu::BindGroupLayout,
        dot_bgl: &wgpu::BindGroupLayout,
        reduce_bgl: &wgpu::BindGroupLayout,
    ) -> Self {
        let spmv = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("SpMV f64"),
                layout: Some(&device.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("SpMV PL"),
                        bind_group_layouts: &[spmv_bgl],
                        push_constant_ranges: &[],
                    },
                )),
                module: shader,
                entry_point: "spmv_f64",
                cache: None,
                compilation_options: Default::default(),
            });

        let dot = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Dot f64"),
                layout: Some(&device.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("Dot PL"),
                        bind_group_layouts: &[dot_bgl],
                        push_constant_ranges: &[],
                    },
                )),
                module: shader,
                entry_point: "dot_f64",
                cache: None,
                compilation_options: Default::default(),
            });

        let reduce = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Final reduce f64"),
                layout: Some(&device.device.create_pipeline_layout(
                    &wgpu::PipelineLayoutDescriptor {
                        label: Some("Reduce PL"),
                        bind_group_layouts: &[reduce_bgl],
                        push_constant_ranges: &[],
                    },
                )),
                module: shader,
                entry_point: "final_reduce_f64",
                cache: None,
                compilation_options: Default::default(),
            });

        Self { spmv, dot, reduce }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_sparse_buffers_creation() {
        // Would need a device for actual testing
        // This test verifies the module compiles correctly
    }
}
