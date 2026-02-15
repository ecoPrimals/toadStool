//! Dense Matrix Multiply (f64) — GPU-Accelerated via WGSL
//!
//! Batched GEMM: C = alpha * A * B + beta * C
//! Supports batched, matrix-vector, and element-wise operations.
//!
//! **Use cases**:
//! - HFB Hamiltonian assembly (radial integrals as matrix products)
//! - Density computation (matrix-vector products)
//! - Energy functional evaluation
//! - Any dense f64 linear algebra on GPU
//!
//! **Deep Debt Principles**:
//! - Pure WGSL implementation (hardware-agnostic)
//! - Full f64 precision via SPIR-V/Vulkan
//! - Safe Rust wrapper (no unsafe code)
//! - Runtime-configured dimensions and batch size

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Parameters for GEMM shader (must match WGSL struct layout)
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct GemmParams {
    m: u32,
    k: u32,
    n: u32,
    batch_size: u32,
    alpha_lo: u32,  // f64 split into two u32s for Pod
    alpha_hi: u32,
    beta_lo: u32,
    beta_hi: u32,
}

impl GemmParams {
    fn new(m: u32, k: u32, n: u32, batch_size: u32, alpha: f64, beta: f64) -> Self {
        let alpha_bits = alpha.to_bits();
        let beta_bits = beta.to_bits();
        GemmParams {
            m,
            k,
            n,
            batch_size,
            alpha_lo: alpha_bits as u32,
            alpha_hi: (alpha_bits >> 32) as u32,
            beta_lo: beta_bits as u32,
            beta_hi: (beta_bits >> 32) as u32,
        }
    }
}

/// GPU-accelerated dense matrix multiply (f64)
pub struct GemmF64;

impl GemmF64 {
    fn wgsl_shader() -> &'static str {
        include_str!("../../shaders/linalg/gemm_f64.wgsl")
    }

    /// Execute batched matrix multiply: C = A * B
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `a` - Packed A matrices [batch_size × M × K] row-major f64
    /// * `b` - Packed B matrices [batch_size × K × N] row-major f64
    /// * `m` - Rows of A / C
    /// * `k` - Cols of A / Rows of B
    /// * `n` - Cols of B / C
    /// * `batch_size` - Number of independent multiplications
    ///
    /// # Returns
    /// C matrices [batch_size × M × N] row-major f64
    pub fn execute(
        device: Arc<WgpuDevice>,
        a: &[f64],
        b: &[f64],
        m: usize,
        k: usize,
        n: usize,
        batch_size: usize,
    ) -> Result<Vec<f64>> {
        Self::execute_gemm(device, a, b, m, k, n, batch_size, 1.0, 0.0)
    }

    /// Execute batched GEMM with alpha/beta: C = alpha * A * B + beta * C
    pub fn execute_gemm(
        device: Arc<WgpuDevice>,
        a: &[f64],
        b: &[f64],
        m: usize,
        k: usize,
        n: usize,
        batch_size: usize,
        alpha: f64,
        beta: f64,
    ) -> Result<Vec<f64>> {
        let expected_a = batch_size * m * k;
        let expected_b = batch_size * k * n;
        if a.len() != expected_a {
            return Err(BarracudaError::InvalidInput {
                message: format!("A: expected {} elements, got {}", expected_a, a.len()),
            });
        }
        if b.len() != expected_b {
            return Err(BarracudaError::InvalidInput {
                message: format!("B: expected {} elements, got {}", expected_b, b.len()),
            });
        }

        let c_size = batch_size * m * n;

        // Create buffers
        let a_bytes: Vec<u8> = a.iter().flat_map(|v| v.to_le_bytes()).collect();
        let a_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GEMM A f64"),
                contents: &a_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let b_bytes: Vec<u8> = b.iter().flat_map(|v| v.to_le_bytes()).collect();
        let b_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("GEMM B f64"),
                contents: &b_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let c_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GEMM C f64"),
            size: (c_size * 8) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = GemmParams::new(
            m as u32,
            k as u32,
            n as u32,
            batch_size as u32,
            alpha,
            beta,
        );
        let params_buffer = device.create_uniform_buffer("GEMM Params", &params);

        // Compile shader and create pipeline
        let shader = device.compile_shader(Self::wgsl_shader(), Some("GEMM f64"));

        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("GEMM BGL"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
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
                ],
            });

        let pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("GEMM PL"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("GEMM f64"),
                layout: Some(&pl),
                module: &shader,
                entry_point: "gemm_f64",
            });

        let bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("GEMM BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: a_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: b_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: c_buffer.as_entire_binding(),
                },
            ],
        });

        // Dispatch: (ceil(N/16), ceil(M/16), batch_size)
        let wg_x = ((n as u32) + 15) / 16;
        let wg_y = ((m as u32) + 15) / 16;
        let wg_z = batch_size as u32;

        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("GEMM Encoder"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("GEMM Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(wg_x, wg_y, wg_z);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        // Read back results
        Self::read_f64_buffer(&device, &c_buffer, c_size)
    }

    /// Helper: Read f64 buffer from GPU
    fn read_f64_buffer(
        device: &Arc<WgpuDevice>,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f64>> {
        let staging = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("GEMM f64 staging"),
            size: (count * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder =
            device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("GEMM readback"),
                });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (count * 8) as u64);
        device.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            sender.send(result).unwrap();
        });
        device.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .unwrap()
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = slice.get_mapped_range();
        let result: Vec<f64> = data
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
            .collect();
        drop(data);
        staging.unmap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx_eq(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() < tol
    }

    #[tokio::test]
    async fn test_gemm_2x2() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        // A = [[1, 2], [3, 4]], B = [[5, 6], [7, 8]]
        // C = [[19, 22], [43, 50]]
        let a = vec![1.0_f64, 2.0, 3.0, 4.0];
        let b = vec![5.0_f64, 6.0, 7.0, 8.0];

        let c = GemmF64::execute(device, &a, &b, 2, 2, 2, 1).unwrap();

        assert_eq!(c.len(), 4);
        assert!(approx_eq(c[0], 19.0, 1e-10));
        assert!(approx_eq(c[1], 22.0, 1e-10));
        assert!(approx_eq(c[2], 43.0, 1e-10));
        assert!(approx_eq(c[3], 50.0, 1e-10));
    }

    #[tokio::test]
    async fn test_gemm_batched() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        // Two identity multiplications (3x3)
        let a = vec![
            1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, // I_1
            2.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 2.0, // 2*I_2
        ];
        let b = vec![
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, // B_1
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, // B_2
        ];

        let c = GemmF64::execute(device, &a, &b, 3, 3, 3, 2).unwrap();

        assert_eq!(c.len(), 18);
        // First batch: I * B = B
        assert!(approx_eq(c[0], 1.0, 1e-10));
        assert!(approx_eq(c[4], 5.0, 1e-10));
        // Second batch: 2I * B = 2B
        assert!(approx_eq(c[9], 2.0, 1e-10));
        assert!(approx_eq(c[13], 10.0, 1e-10));
    }
}
