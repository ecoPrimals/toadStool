//! Norm Reduction (f64) — GPU-Accelerated via WGSL
//!
//! Computes vector norms over f64 arrays:
//! - L1 norm: sum(|x|)
//! - L2 norm: sqrt(sum(x^2))
//! - Linf norm: max(|x|)
//! - Frobenius norm: sqrt(sum(|a_ij|^2)) for matrices
//! - Generic p-norm: (sum(|x|^p))^(1/p)
//!
//! **Use cases**:
//! - Convergence checking (||residual||)
//! - Error metrics
//! - Regularization terms
//! - Scientific computing
//!
//! **Deep Debt Principles**:
//! - Pure WGSL implementation (hardware-agnostic)
//! - Full f64 precision via SPIR-V/Vulkan
//! - Safe Rust wrapper (no unsafe code)

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Parameters for norm shader
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct NormParams {
    size: u32,
    norm_type: u32, // 1=L1, 2=L2, 0=Linf
    p_lo: u32,      // f64 p as two u32s (for p-norm)
    p_hi: u32,
}

/// GPU-accelerated f64 norm operations
pub struct NormReduceF64;

impl NormReduceF64 {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/reduce/norm_reduce_f64.wgsl")
    }

    /// Compute L1 norm: sum(|x|)
    pub fn l1(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        Self::reduce_op(device, data, "norm_l1_f64", None)
    }

    /// Compute L2 norm: sqrt(sum(x^2))
    pub fn l2(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        let sum_sq = Self::reduce_op(device, data, "norm_l2_f64", None)?;
        Ok(sum_sq.sqrt())
    }

    /// Compute L2 norm squared: sum(x^2) (without sqrt)
    pub fn l2_squared(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        Self::reduce_op(device, data, "norm_l2_f64", None)
    }

    /// Compute Linf norm: max(|x|)
    pub fn linf(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        Self::reduce_op(device, data, "norm_linf_f64", None)
    }

    /// Compute Frobenius norm (same as L2, but semantically for matrices)
    pub fn frobenius(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        let sum_sq = Self::reduce_op(device, data, "norm_frobenius_f64", None)?;
        Ok(sum_sq.sqrt())
    }

    /// Compute generic p-norm: (sum(|x|^p))^(1/p)
    ///
    /// Note: Falls back to CPU for arbitrary p since many GPUs (especially AMD)
    /// don't support f64 log/exp operations required for pow.
    pub fn p_norm(_device: Arc<WgpuDevice>, data: &[f64], p: f64) -> Result<f64> {
        if data.is_empty() {
            return Ok(0.0);
        }
        if p == 1.0 {
            // CPU fallback for L1
            return Ok(data.iter().map(|x| x.abs()).sum());
        }
        if p == 2.0 {
            // CPU fallback for L2
            let sum_sq: f64 = data.iter().map(|x| x * x).sum();
            return Ok(sum_sq.sqrt());
        }
        if p.is_infinite() && p > 0.0 {
            // CPU fallback for Linf
            return Ok(data.iter().map(|x| x.abs()).fold(f64::NEG_INFINITY, f64::max));
        }

        // CPU fallback for generic p-norm (GPU f64 pow not widely supported)
        let sum_p: f64 = data.iter().map(|x| x.abs().powf(p)).sum();
        Ok(sum_p.powf(1.0 / p))
    }

    fn reduce_op(
        device: Arc<WgpuDevice>,
        data: &[f64],
        entry_point: &str,
        p: Option<f64>,
    ) -> Result<f64> {
        if data.is_empty() {
            return Ok(0.0);
        }
        if data.len() == 1 {
            return Ok(data[0].abs());
        }

        let shader = device.compile_shader(Self::wgsl_shader(), Some("Norm Reduce f64"));

        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("NormReduce BGL"),
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

        let pl = device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("NormReduce PL"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some(entry_point),
                layout: Some(&pl),
                module: &shader,
                entry_point,
            cache: None,
            compilation_options: Default::default(),
            });

        let n = data.len();
        let wg_size = 256;
        let n_workgroups = n.div_ceil(wg_size);

        // Create input buffer
        let input_bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let input_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("NormReduce input"),
                contents: &input_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let partial_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("NormReduce partials"),
            size: (n_workgroups * 8) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Pack p as two u32s (low and high bits of f64)
        let p_val = p.unwrap_or(2.0);
        let p_bits = p_val.to_bits();
        let params = NormParams {
            size: n as u32,
            norm_type: 0,
            p_lo: p_bits as u32,
            p_hi: (p_bits >> 32) as u32,
        };
        let params_buffer = device.create_uniform_buffer("NormReduce params", &params);

        let bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("NormReduce BG pass 1"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: input_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: partial_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("NormReduce pass 1"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("NormReduce pass 1"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(n_workgroups as u32, 1, 1);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        if n_workgroups <= 1 {
            return Self::read_f64_scalar(&device, &partial_buffer);
        }

        // Second pass: use sum reduction for L1/L2/p-norm, max for Linf
        let second_entry = if entry_point == "norm_linf_f64" {
            "norm_linf_f64"
        } else {
            "norm_l1_f64" // L1 is just sum for second pass
        };

        let second_pipeline =
            device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("NormReduce pass 2"),
                    layout: Some(&pl),
                    module: &shader,
                    entry_point: second_entry,
                    cache: None,
                    compilation_options: Default::default(),
                });

        let final_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("NormReduce final"),
            size: 8 * n_workgroups.div_ceil(wg_size) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params2 = NormParams {
            size: n_workgroups as u32,
            norm_type: 0,
            p_lo: 0,
            p_hi: 0,
        };
        let params2_buffer = device.create_uniform_buffer("NormReduce params 2", &params2);

        let n_workgroups2 = n_workgroups.div_ceil(wg_size);
        let bg2 = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("NormReduce BG pass 2"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: partial_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: final_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: params2_buffer.as_entire_binding(),
                },
            ],
        });

        {
            let mut encoder =
                device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("NormReduce pass 2"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("NormReduce pass 2"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&second_pipeline);
                pass.set_bind_group(0, &bg2, &[]);
                pass.dispatch_workgroups(n_workgroups2 as u32, 1, 1);
            }
            device.queue.submit(Some(encoder.finish()));
        }

        if n_workgroups2 > 1 {
            let partials = Self::read_f64_buffer(&device, &final_buffer, n_workgroups2)?;
            if entry_point == "norm_linf_f64" {
                return Ok(partials.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b)));
            } else {
                return Ok(partials.iter().sum());
            }
        }

        Self::read_f64_scalar(&device, &final_buffer)
    }

    fn read_f64_scalar(device: &Arc<WgpuDevice>, buffer: &wgpu::Buffer) -> Result<f64> {
        let values = Self::read_f64_buffer(device, buffer, 1)?;
        Ok(values[0])
    }

    fn read_f64_buffer(
        device: &Arc<WgpuDevice>,
        buffer: &wgpu::Buffer,
        count: usize,
    ) -> Result<Vec<f64>> {
        let staging = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("NormReduce staging"),
            size: (count * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("NormReduce readback"),
            });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (count * 8) as u64);
        device.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (sender, receiver) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        device.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("GPU buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = slice.get_mapped_range();
        let result: Vec<f64> = data
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().expect("chunks_exact(8) invariant")))
            .collect();
        drop(data);
        staging.unmap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_l1_norm() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let data: Vec<f64> = vec![1.0, -2.0, 3.0, -4.0, 5.0];
        let norm = NormReduceF64::l1(device, &data).unwrap();
        assert!(
            (norm - 15.0).abs() < 1e-6,
            "L1 norm should be 15, got {}",
            norm
        );
    }

    #[tokio::test]
    async fn test_l2_norm() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let data: Vec<f64> = vec![3.0, 4.0];
        let norm = NormReduceF64::l2(device, &data).unwrap();
        assert!(
            (norm - 5.0).abs() < 1e-6,
            "L2 norm of [3,4] should be 5, got {}",
            norm
        );
    }

    #[tokio::test]
    async fn test_linf_norm() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let data: Vec<f64> = vec![1.0, -7.0, 3.0, -4.0, 5.0];
        let norm = NormReduceF64::linf(device, &data).unwrap();
        assert!(
            (norm - 7.0).abs() < 1e-6,
            "Linf norm should be 7, got {}",
            norm
        );
    }

    #[tokio::test]
    async fn test_frobenius_norm() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        // 2x2 identity matrix has Frobenius norm sqrt(2)
        let data: Vec<f64> = vec![1.0, 0.0, 0.0, 1.0];
        let norm = NormReduceF64::frobenius(device, &data).unwrap();
        let expected = 2.0_f64.sqrt();
        assert!(
            (norm - expected).abs() < 1e-6,
            "Frobenius norm should be sqrt(2), got {}",
            norm
        );
    }

    #[tokio::test]
    async fn test_p_norm() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let data: Vec<f64> = vec![1.0, 2.0, 3.0, 4.0];
        // p=3 norm: (1^3 + 2^3 + 3^3 + 4^3)^(1/3) = (1+8+27+64)^(1/3) = 100^(1/3)
        let norm = NormReduceF64::p_norm(device, &data, 3.0).unwrap();
        let expected = 100.0_f64.powf(1.0 / 3.0);
        assert!(
            (norm - expected).abs() < 1e-4,
            "p=3 norm should be {}, got {}",
            expected,
            norm
        );
    }
}
