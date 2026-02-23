//! DIGAMMA F64 - Digamma function ψ(x) = Γ'(x)/Γ(x) - f64 precision
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//!
//! Note: Uses GPU for f64 log/sin/cos when available.
//!
//! Applications:
//! - Fisher information
//! - Bayesian statistics
//! - Neural network regularization

use crate::device::capabilities::WORKGROUP_SIZE_1D;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct Params {
    size: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

/// f64 Digamma function evaluator
///
/// Computes ψ(x) = d/dx ln(Γ(x)) using reflection + recurrence + asymptotic expansion.
pub struct DigammaF64 {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl DigammaF64 {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/digamma_f64.wgsl")
    }

    /// Create new Digamma f64 operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        let shader = device.compile_shader_f64(Self::wgsl_shader(), Some("DigammaF64"));

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("DigammaF64 BGL"),
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
                    label: Some("DigammaF64 PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("DigammaF64 Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: "main",
                cache: None,
                compilation_options: Default::default(),
            });

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
        })
    }

    /// Compute ψ(x) for each element
    ///
    /// # Arguments
    /// * `x` - Input values
    ///
    /// # Returns
    /// Vector of ψ(x) values with f64 precision
    pub fn digamma(&self, x: &[f64]) -> Result<Vec<f64>> {
        if x.is_empty() {
            return Ok(vec![]);
        }

        let n = x.len();
        let params = Params {
            size: n as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };

        let input_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("DigammaF64 Input"),
                contents: bytemuck::cast_slice(x),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_size = std::mem::size_of_val(x);
        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DigammaF64 Output"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("DigammaF64 Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("DigammaF64 BG"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: input_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: output_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: params_buf.as_entire_binding(),
                    },
                ],
            });

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("DigammaF64 Encoder"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("DigammaF64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups((n as u32).div_ceil(WORKGROUP_SIZE_1D), 1, 1);
        }

        let staging_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("DigammaF64 Staging"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size as u64);

        self.device.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| BarracudaError::Gpu(format!("Digamma readback: {}", e)))?
            .map_err(|e| BarracudaError::Gpu(format!("Digamma map: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        Ok(result)
    }

    #[cfg(test)]
    fn digamma_cpu(&self, x: &[f64]) -> Vec<f64> {
        x.iter().map(|&xi| Self::digamma_scalar(xi)).collect()
    }

    #[cfg(test)]
    fn digamma_scalar(x: f64) -> f64 {
        use std::f64::consts::PI;

        // Non-positive integer: pole
        if x <= 0.0 && x == x.floor() {
            return f64::NAN;
        }

        let mut y = x;
        let mut result = 0.0;

        // Reflection formula for x < 0
        if y < 0.0 {
            let cot_pi_y = (PI * y).cos() / (PI * y).sin();
            result -= PI * cot_pi_y;
            y = 1.0 - y;
        }

        // Recurrence to shift to larger argument
        while y < 6.0 {
            result -= 1.0 / y;
            y += 1.0;
        }

        // Asymptotic expansion for y >= 6
        result + Self::digamma_asymptotic(y)
    }

    #[cfg(test)]
    fn digamma_asymptotic(x: f64) -> f64 {
        let inv_x = 1.0 / x;
        let inv_x2 = inv_x * inv_x;

        // Bernoulli number coefficients
        const B2: f64 = 1.0 / 12.0;
        const B4: f64 = -1.0 / 120.0;
        const B6: f64 = 1.0 / 252.0;
        const B8: f64 = -1.0 / 240.0;
        const B10: f64 = 1.0 / 132.0;
        const B12: f64 = -691.0 / 32760.0;

        let mut sum = x.ln() - 0.5 * inv_x;
        let mut term = inv_x2;

        sum -= B2 * term;
        term *= inv_x2;
        sum -= B4 * term;
        term *= inv_x2;
        sum -= B6 * term;
        term *= inv_x2;
        sum -= B8 * term;
        term *= inv_x2;
        sum -= B10 * term;
        term *= inv_x2;
        sum -= B12 * term;

        sum
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_digamma_at_1() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let digamma = DigammaF64::new(device).unwrap();

        // ψ(1) = -γ (Euler-Mascheroni constant)
        let euler_mascheroni = 0.5772156649015329;
        let result = digamma.digamma(&[1.0]).unwrap();

        assert!(
            (result[0] + euler_mascheroni).abs() < 1e-6,
            "ψ(1) = {}, expected -γ = {}",
            result[0],
            -euler_mascheroni
        );
    }

    #[tokio::test]
    async fn test_digamma_recurrence() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let digamma = DigammaF64::new(device).unwrap();

        // ψ(x+1) = ψ(x) + 1/x
        for x in [1.0, 2.0, 3.0, 4.5, 7.3] {
            let result = digamma.digamma(&[x, x + 1.0]).unwrap();
            let psi_x = result[0];
            let psi_x1 = result[1];

            assert!(
                (psi_x1 - psi_x - 1.0 / x).abs() < 1e-6,
                "ψ({}) + 1/{} = {} should equal ψ({}) = {}",
                x,
                x,
                psi_x + 1.0 / x,
                x + 1.0,
                psi_x1
            );
        }
    }

    #[tokio::test]
    async fn test_digamma_known_values() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let digamma = DigammaF64::new(device).unwrap();

        // ψ(2) = 1 - γ
        let euler_mascheroni = 0.5772156649015329;
        let result = digamma.digamma(&[2.0]).unwrap();
        let expected = 1.0 - euler_mascheroni;

        assert!(
            (result[0] - expected).abs() < 1e-6,
            "ψ(2) = {}, expected {}",
            result[0],
            expected
        );

        // ψ(1/2) = -γ - 2*ln(2)
        let result = digamma.digamma(&[0.5]).unwrap();
        let expected = -euler_mascheroni - 2.0 * 2.0_f64.ln();

        assert!(
            (result[0] - expected).abs() < 1e-6,
            "ψ(0.5) = {}, expected {}",
            result[0],
            expected
        );
    }

    #[tokio::test]
    async fn test_digamma_large_x() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let digamma = DigammaF64::new(device).unwrap();

        // For large x, ψ(x) ≈ ln(x) - 1/(2x)
        let x = 100.0;
        let result = digamma.digamma(&[x]).unwrap();
        let approx = x.ln() - 0.5 / x;

        // The actual value is more accurate than the simple approximation
        // Asymptotic expansion includes higher order terms that improve accuracy
        assert!(
            (result[0] - approx).abs() < 1e-4,
            "ψ({}) = {}, asymptotic approx = {}",
            x,
            result[0],
            approx
        );
    }
}
