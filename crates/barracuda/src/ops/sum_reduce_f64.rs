//! Sum Reduction (f64) — GPU-Accelerated via WGSL
//!
//! Computes sum, max, or min over all elements of an f64 buffer.
//! Two-pass reduction: first pass produces partial sums per workgroup,
//! second pass reduces partial sums to a single scalar.
//!
//! **Use cases**:
//! - Energy functional integration (trapezoid rule: sum of integrand * dr)
//! - RMS error computation: sqrt(sum(errors^2) / N)
//! - Convergence checking: max(|delta_E|)
//! - Any global f64 reduction
//!
//! **Deep Debt Principles**:
//! - Pure WGSL implementation (hardware-agnostic)
//! - Full f64 precision via SPIR-V/Vulkan
//! - Safe Rust wrapper (no unsafe code)

use crate::device::WgpuDevice;
use crate::error::Result;
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Parameters for reduce shader
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct ReduceParams {
    size: u32,
    _pad1: u32,
    _pad2: u32,
    _pad3: u32,
}

/// GPU-accelerated f64 reduction operations
pub struct SumReduceF64;

impl SumReduceF64 {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/reduce/sum_reduce_f64.wgsl")
    }

    /// Compute the sum of all elements in a f64 buffer on GPU
    ///
    /// # Arguments
    /// * `device` - WgpuDevice
    /// * `data` - Input f64 slice
    ///
    /// # Returns
    /// The sum as a single f64
    pub fn sum(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        Self::reduce_op(device, data, "sum_reduce_f64")
    }

    /// Compute the max of all elements in a f64 buffer on GPU
    pub fn max(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        Self::reduce_op(device, data, "max_reduce_f64")
    }

    /// Compute the min of all elements in a f64 buffer on GPU
    pub fn min(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        Self::reduce_op(device, data, "min_reduce_f64")
    }

    /// Compute the mean of all elements
    pub fn mean(device: Arc<WgpuDevice>, data: &[f64]) -> Result<f64> {
        let sum = Self::sum(device, data)?;
        Ok(sum / data.len() as f64)
    }

    fn reduce_op(device: Arc<WgpuDevice>, data: &[f64], entry_point: &str) -> Result<f64> {
        if data.is_empty() {
            return Ok(0.0);
        }
        if data.len() == 1 {
            return Ok(data[0]);
        }

        let shader = device.compile_shader_f64(Self::wgsl_shader(), Some("Sum Reduce f64"));

        let bgl = device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Reduce BGL"),
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
                label: Some("Reduce PL"),
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

        // Two-pass reduction
        let n = data.len();
        let wg_size = 256;
        let n_workgroups = n.div_ceil(wg_size);

        // Pass 1: data -> partial sums
        let input_bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        let input_buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Reduce input"),
                contents: &input_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let partial_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reduce partials"),
            size: (n_workgroups * 8) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = ReduceParams {
            size: n as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        let params_buffer = device.create_uniform_buffer("Reduce params", &params);

        let bg = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce BG pass 1"),
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
                        label: Some("Reduce pass 1"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reduce pass 1"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(n_workgroups as u32, 1, 1);
            }
            device.submit_and_poll(Some(encoder.finish()));
        }

        if n_workgroups <= 1 {
            // Single workgroup — result is ready
            return Self::read_f64_scalar(&device, &partial_buffer);
        }

        // Pass 2: partial sums -> final result
        let final_buffer = device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Reduce final"),
            size: 8 * n_workgroups.div_ceil(wg_size) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params2 = ReduceParams {
            size: n_workgroups as u32,
            _pad1: 0,
            _pad2: 0,
            _pad3: 0,
        };
        let params2_buffer = device.create_uniform_buffer("Reduce params 2", &params2);

        let n_workgroups2 = n_workgroups.div_ceil(wg_size);
        let bg2 = device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Reduce BG pass 2"),
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
                        label: Some("Reduce pass 2"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("Reduce pass 2"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bg2, &[]);
                pass.dispatch_workgroups(n_workgroups2 as u32, 1, 1);
            }
            device.submit_and_poll(Some(encoder.finish()));
        }

        // For very large inputs, may need a third pass — but for nuclear EOS
        // (max ~2042 elements), two passes always suffice (ceil(2042/256) = 8 < 256)
        if n_workgroups2 > 1 {
            // Third pass (extremely rare): read back partials and sum on CPU
            let partials = device.read_f64_buffer(&final_buffer, n_workgroups2)?;
            return Ok(partials.iter().sum());
        }

        Self::read_f64_scalar(&device, &final_buffer)
    }

    fn read_f64_scalar(device: &WgpuDevice, buffer: &wgpu::Buffer) -> Result<f64> {
        let values = device.read_f64_buffer(buffer, 1)?;
        Ok(values[0])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sum_small() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let data: Vec<f64> = (1..=100).map(|i| i as f64).collect();
        let sum = SumReduceF64::sum(device, &data).unwrap();
        assert!(
            (sum - 5050.0).abs() < 1e-6,
            "Sum of 1..100 should be 5050, got {}",
            sum
        );
    }

    #[tokio::test]
    async fn test_sum_large() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        // 2048 elements (multiple workgroups)
        let data: Vec<f64> = (1..=2048).map(|i| i as f64).collect();
        let expected = 2048.0 * 2049.0 / 2.0;
        let sum = SumReduceF64::sum(device, &data).unwrap();
        assert!(
            (sum - expected).abs() < 1e-3,
            "Sum of 1..2048 should be {}, got {}",
            expected,
            sum
        );
    }

    #[tokio::test]
    async fn test_max() {
        let Some(device) = crate::device::test_pool::get_test_device_if_f64_gpu_available().await
        else {
            return;
        };

        let data = vec![3.0_f64, 1.0, 4.0, 1.0, 5.0, 9.0, 2.0, 6.0];
        let max = SumReduceF64::max(device, &data).unwrap();
        assert!((max - 9.0).abs() < 1e-10, "Max should be 9, got {}", max);
    }
}
