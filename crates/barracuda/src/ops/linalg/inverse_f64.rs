//! Matrix Inverse (f64) — Gauss-Jordan GPU elimination
//!
//! Computes A⁻¹ via [A | I] → [I | A⁻¹] with partial pivoting.
//! Full f64 precision for ill-conditioned matrices (κ(A) >> 1).
//!
//! Optimized for small–medium matrices (N ≤ 32) in a single workgroup.

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

const SHADER: &str = include_str!("../../shaders/linalg/inverse_f64.wgsl");

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Params {
    n: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

/// GPU-accelerated matrix inverse with f64 precision.
pub struct InverseF64 {
    device: Arc<WgpuDevice>,
}

impl InverseF64 {
    pub fn new(device: Arc<WgpuDevice>) -> Self {
        Self { device }
    }

    /// Compute the inverse of an n×n matrix.
    ///
    /// Returns the inverse as a flat `Vec<f64>` (n×n, row-major).
    /// Returns zeros if the matrix is singular.
    pub fn compute(&self, matrix_data: &[f64], n: usize) -> Result<Vec<f64>> {
        if matrix_data.len() != n * n {
            return Err(BarracudaError::InvalidShape {
                expected: vec![n, n],
                actual: vec![matrix_data.len()],
            });
        }

        let params = Params {
            n: n as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };

        let aug_size = n * 2 * n;
        let input_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("InvF64 input"),
                contents: bytemuck::cast_slice(matrix_data),
                usage: wgpu::BufferUsages::STORAGE,
            });
        let work_buf = self.device.create_buffer_f64(aug_size)?;
        let output_buf = self.device.create_buffer_f64(n * n)?;
        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("InvF64 params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("InvF64 BGL"),
                entries: &[
                    bgl_entry(0, true),
                    bgl_entry(1, false),
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

        let bg = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("InvF64 BG"),
                layout: &bgl,
                entries: &[
                    bg_entry(0, &input_buf),
                    bg_entry(1, &work_buf),
                    bg_entry(2, &output_buf),
                    bg_entry(3, &params_buf),
                ],
            });

        let shader = self.device.compile_shader_f64(SHADER, Some("InverseF64"));
        let pl = self
            .device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("InvF64 PL"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("InvF64 Pipeline"),
                    layout: Some(&pl),
                    module: &shader,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("InvF64 Encoder"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("InvF64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        self.device.submit_and_poll(Some(encoder.finish()));
        self.device.read_buffer_f64(&output_buf, n * n)
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
    async fn test_inverse_f64_identity() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let inv = InverseF64::new(device);
        let a = vec![1.0, 0.0, 0.0, 1.0];
        let result = inv.compute(&a, 2).unwrap();
        assert!((result[0] - 1.0).abs() < 1e-12);
        assert!(result[1].abs() < 1e-12);
        assert!(result[2].abs() < 1e-12);
        assert!((result[3] - 1.0).abs() < 1e-12);
    }

    #[tokio::test]
    async fn test_inverse_f64_2x2() {
        let Some(device) = get_test_device_if_gpu_available().await else {
            return;
        };
        let inv = InverseF64::new(device);
        // [[2, 1], [1, 2]] → inverse [[2/3, -1/3], [-1/3, 2/3]]
        let a = vec![2.0, 1.0, 1.0, 2.0];
        let result = inv.compute(&a, 2).unwrap();
        assert!((result[0] - 2.0 / 3.0).abs() < 1e-12);
        assert!((result[1] + 1.0 / 3.0).abs() < 1e-12);
        assert!((result[2] + 1.0 / 3.0).abs() < 1e-12);
        assert!((result[3] - 2.0 / 3.0).abs() < 1e-12);
    }
}
