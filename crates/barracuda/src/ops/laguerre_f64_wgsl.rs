//! LAGUERRE F64 - Generalized Laguerre polynomials - f64 precision WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! Applications:
//! - Hydrogen/helium radial wavefunctions (hotSpring)
//! - Nuclear structure calculations
//! - 2D/3D harmonic oscillator basis
//! - Molecular dynamics radial basis

use crate::device::capabilities::WORKGROUP_SIZE_1D;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// f64 Generalized Laguerre polynomial evaluator L_n^(α)(x)
///
/// Computes generalized Laguerre polynomials with full f64 precision
/// using three-term recurrence relation.
pub struct LaguerreF64 {
    device: Arc<WgpuDevice>,
}

impl LaguerreF64 {
    /// Create new Laguerre f64 polynomial operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/laguerre_f64.wgsl")
    }

    /// Compute generalized Laguerre polynomial L_n^(α)(x) for each element
    ///
    /// # Arguments
    /// * `x` - Input values
    /// * `n` - Polynomial degree (0, 1, 2, ...)
    /// * `alpha` - Generalization parameter (0.0 for simple Laguerre)
    ///
    /// # Returns
    /// Vector of L_n^(α)(x) values with f64 precision
    pub fn laguerre(&self, x: &[f64], n: u32, alpha: f64) -> Result<Vec<f64>> {
        if x.is_empty() {
            return Ok(vec![]);
        }

        let size = x.len();

        // CPU fallback for small inputs
        if size < 256 {
            return Ok(self.laguerre_cpu(x, n, alpha));
        }

        self.laguerre_gpu(x, n, alpha)
    }

    /// Compute simple Laguerre polynomial Lₙ(x) (α = 0)
    pub fn laguerre_simple(&self, x: &[f64], n: u32) -> Result<Vec<f64>> {
        self.laguerre(x, n, 0.0)
    }

    fn laguerre_cpu(&self, x: &[f64], n: u32, alpha: f64) -> Vec<f64> {
        x.iter()
            .map(|&xi| Self::laguerre_scalar(n, alpha, xi))
            .collect()
    }

    fn laguerre_scalar(n: u32, alpha: f64, x: f64) -> f64 {
        if n == 0 {
            return 1.0;
        }
        if n == 1 {
            return 1.0 + alpha - x;
        }

        let mut l_prev = 1.0;
        let mut l_curr = 1.0 + alpha - x;

        for k in 1..n {
            let kf = k as f64;
            // Three-term recurrence: n·Lₙ = (2n-1+α-x)·L_{n-1} - (n-1+α)·L_{n-2}
            let l_next =
                ((2.0 * kf + 1.0 + alpha - x) * l_curr - (kf + alpha) * l_prev) / (kf + 1.0);
            l_prev = l_curr;
            l_curr = l_next;
        }

        l_curr
    }

    fn laguerre_gpu(&self, x: &[f64], n: u32, alpha: f64) -> Result<Vec<f64>> {
        let size = x.len();

        let input_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Laguerre f64 Input"),
                contents: bytemuck::cast_slice(x),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Laguerre f64 Output"),
            size: std::mem::size_of_val(x) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Params struct must match WGSL: size, n, _pad0, _pad1, alpha (f64)
        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            n: u32,
            _pad0: u32,
            _pad1: u32,
            alpha: f64,
        }

        let params = Params {
            size: size as u32,
            n,
            _pad0: 0,
            _pad1: 0,
            alpha,
        };

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Laguerre f64 Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Laguerre f64 BGL"),
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

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Laguerre f64 Bind Group"),
                layout: &bgl,
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

        let shader = self
            .device
            .compile_shader_f64(Self::wgsl_shader(), Some("Laguerre f64"));

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Laguerre f64 Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Laguerre f64 Pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Laguerre f64 Encoder"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Laguerre f64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32).div_ceil(WORKGROUP_SIZE_1D);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Read back results
        let staging_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Laguerre f64 Staging"),
            size: std::mem::size_of_val(x) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(
            &output_buf,
            0,
            &staging_buf,
            0,
            std::mem::size_of_val(x) as u64,
        );

        self.device.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        self.device.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| BarracudaError::Device(format!("Buffer map failed: {}", e)))?
            .map_err(|e| BarracudaError::Device(format!("Buffer map error: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_device() -> Option<Arc<crate::device::WgpuDevice>> {
        crate::device::test_pool::get_test_device_if_f64_gpu_available_sync()
    }

    #[test]
    fn test_laguerre_f64_l0() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = LaguerreF64::new(device).unwrap();

        let x = vec![0.0, 1.0, 2.0, -1.0, 0.5];
        let result = op.laguerre(&x, 0, 0.0).unwrap();

        // L₀(x) = 1 for all x
        for &v in &result {
            assert!((v - 1.0).abs() < 1e-10, "L₀ should be 1, got {}", v);
        }
    }

    #[test]
    fn test_laguerre_f64_l1() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = LaguerreF64::new(device).unwrap();

        let x = vec![0.0, 1.0, 2.0, 0.5];
        let result = op.laguerre(&x, 1, 0.0).unwrap();

        // L₁(x) = 1 - x
        for (i, &v) in result.iter().enumerate() {
            let expected = 1.0 - x[i];
            assert!(
                (v - expected).abs() < 1e-10,
                "L₁({}) = {}, expected {}",
                x[i],
                v,
                expected
            );
        }
    }

    #[test]
    fn test_laguerre_f64_l2() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = LaguerreF64::new(device).unwrap();

        let x = vec![0.0, 1.0, 2.0, 0.5];
        let result = op.laguerre(&x, 2, 0.0).unwrap();

        // L₂(x) = (x² - 4x + 2) / 2 = 0.5x² - 2x + 1
        for (i, &v) in result.iter().enumerate() {
            let xi = x[i];
            let expected = 0.5 * xi * xi - 2.0 * xi + 1.0;
            assert!(
                (v - expected).abs() < 1e-10,
                "L₂({}) = {}, expected {}",
                xi,
                v,
                expected
            );
        }
    }

    #[test]
    fn test_laguerre_f64_generalized() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = LaguerreF64::new(device).unwrap();

        // L₁^(1)(x) = 2 - x (α = 1)
        let x = vec![0.0, 1.0, 2.0];
        let result = op.laguerre(&x, 1, 1.0).unwrap();

        for (i, &v) in result.iter().enumerate() {
            let expected = 2.0 - x[i];
            assert!(
                (v - expected).abs() < 1e-10,
                "L₁^(1)({}) = {}, expected {}",
                x[i],
                v,
                expected
            );
        }
    }

    #[test]
    fn test_laguerre_f64_at_zero() {
        let Some(device) = get_test_device() else {
            return;
        };
        let op = LaguerreF64::new(device).unwrap();

        // L_n^(α)(0) = C(n+α, n) = (n+α)! / (n! α!)
        // For α=0: L_n(0) = 1 for all n
        let x = vec![0.0];

        for n in 0..=5 {
            let result = op.laguerre(&x, n, 0.0).unwrap();
            assert!(
                (result[0] - 1.0).abs() < 1e-10,
                "L_{}(0) = {}, expected 1.0",
                n,
                result[0]
            );
        }
    }
}
