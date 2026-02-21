//! Linear System Solve (f64) — Full double-precision GPU Gaussian elimination
//!
//! **Deep Debt Principles**:
//! - Pure WGSL implementation (GPU-optimized)
//! - Safe Rust wrapper (no unsafe code)
//! - Hardware-agnostic via WebGPU
//! - Full f64 precision for ill-conditioned systems
//!
//! Solves A·x = b using Gaussian elimination with partial pivoting.
//! For systems where f32 precision is insufficient (condition number > 10⁶).

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
#[allow(unused_imports)]
use wgpu::util::DeviceExt;

const SHADER: &str = include_str!("../../shaders/linalg/linsolve_f64.wgsl");

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Params {
    n: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

/// GPU-accelerated linear system solve with f64 precision.
pub struct LinSolveF64 {
    device: Arc<WgpuDevice>,
}

impl LinSolveF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Self {
        Self { device }
    }

    /// Solve A·x = b where A is n×n and b is length n.
    ///
    /// Returns solution vector x (length n) as `Vec<f64>`.
    /// Returns zeros if the matrix is singular.
    pub fn solve(
        &self,
        matrix_data: &[f64],
        rhs_data: &[f64],
        n: usize,
    ) -> Result<Vec<f64>> {
        if matrix_data.len() != n * n {
            return Err(BarracudaError::InvalidShape {
                expected: vec![n, n],
                actual: vec![matrix_data.len()],
            });
        }
        if rhs_data.len() != n {
            return Err(BarracudaError::InvalidShape {
                expected: vec![n],
                actual: vec![rhs_data.len()],
            });
        }

        let output_size = n * n + n;
        let params = Params {
            n: n as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };

        let matrix_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LinSolveF64 A"),
                contents: bytemuck::cast_slice(matrix_data),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let rhs_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LinSolveF64 b"),
                contents: bytemuck::cast_slice(rhs_data),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let output_buf = self.device.create_buffer_f64(output_size)?;
        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("LinSolveF64 params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("LinSolveF64 BGL"),
                entries: &[
                    bgl_entry(0, true),
                    bgl_entry(1, true),
                    bgl_entry(2, false),
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

        let bg = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("LinSolveF64 BG"),
            layout: &bgl,
            entries: &[
                bg_entry(0, &matrix_buf),
                bg_entry(1, &rhs_buf),
                bg_entry(2, &output_buf),
                bg_entry(3, &params_buf),
            ],
        });

        let shader = self.device.compile_shader_f64(SHADER, Some("LinSolveF64"));
        let pl = self
            .device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("LinSolveF64 PL"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("LinSolveF64 Pipeline"),
                layout: Some(&pl),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        let mut encoder = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("LinSolveF64 Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("LinSolveF64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        self.device.queue.submit(Some(encoder.finish()));

        let full = self.device.read_buffer_f64(&output_buf, output_size)?;
        Ok(full[n * n..].to_vec())
    }
}

fn bgl_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bg_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_gpu_available;

    #[tokio::test]
    async fn test_linsolve_f64_identity() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let solver = LinSolveF64::new(device);
        let a = vec![1.0, 0.0, 0.0, 1.0];
        let b = vec![3.0, 7.0];
        let x = solver.solve(&a, &b, 2).unwrap();
        assert!((x[0] - 3.0).abs() < 1e-12);
        assert!((x[1] - 7.0).abs() < 1e-12);
    }

    #[tokio::test]
    async fn test_linsolve_f64_2x2() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let solver = LinSolveF64::new(device);
        let a = vec![2.0, 1.0, 1.0, 2.0];
        let b = vec![5.0, 4.0];
        let x = solver.solve(&a, &b, 2).unwrap();
        assert!((x[0] - 2.0).abs() < 1e-12, "x[0]={}", x[0]);
        assert!((x[1] - 1.0).abs() < 1e-12, "x[1]={}", x[1]);
    }
}
