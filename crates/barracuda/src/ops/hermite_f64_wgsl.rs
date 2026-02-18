//! HERMITE F64 - Physicist's Hermite polynomials - f64 precision WGSL
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//! - Complete implementation: Production-ready, no mocks
//! - Hardware-agnostic: Pure WGSL for universal compute
//!
//! Applications:
//! - Quantum harmonic oscillator wavefunctions (hotSpring)
//! - Nuclear structure calculations
//! - Gaussian quadrature weights
//! - Gaussian-Hermite basis functions

use crate::device::capabilities::WORKGROUP_SIZE_1D;
use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// f64 Hermite polynomial evaluator Hₙ(x)
///
/// Computes physicist's Hermite polynomials with full f64 precision
/// using three-term recurrence relation.
pub struct HermiteF64 {
    device: Arc<WgpuDevice>,
}

impl HermiteF64 {
    /// Create new Hermite f64 polynomial operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/hermite_f64.wgsl")
    }

    /// Compute Hermite polynomial Hₙ(x) for each element
    ///
    /// # Arguments
    /// * `x` - Input values
    /// * `n` - Polynomial order (0, 1, 2, ...)
    ///
    /// # Returns
    /// Vector of Hₙ(x) values with f64 precision
    pub fn hermite(&self, x: &[f64], n: u32) -> Result<Vec<f64>> {
        if x.is_empty() {
            return Ok(vec![]);
        }

        let size = x.len();

        // CPU fallback for small inputs
        if size < 256 {
            return Ok(self.hermite_cpu(x, n));
        }

        self.hermite_gpu(x, n)
    }

    /// Compute Hermite function ψₙ(x) (normalized wavefunction)
    ///
    /// ψₙ(x) = (2ⁿ·n!·√π)^(-1/2) · Hₙ(x) · exp(-x²/2)
    ///
    /// This is the quantum harmonic oscillator eigenfunction.
    pub fn hermite_function(&self, x: &[f64], n: u32) -> Result<Vec<f64>> {
        if x.is_empty() {
            return Ok(vec![]);
        }

        let size = x.len();

        // CPU fallback for small inputs
        if size < 256 {
            return Ok(self.hermite_function_cpu(x, n));
        }

        self.hermite_function_gpu(x, n)
    }

    fn hermite_cpu(&self, x: &[f64], n: u32) -> Vec<f64> {
        x.iter().map(|&xi| Self::hermite_scalar(n, xi)).collect()
    }

    fn hermite_function_cpu(&self, x: &[f64], n: u32) -> Vec<f64> {
        x.iter()
            .map(|&xi| Self::hermite_function_scalar(n, xi))
            .collect()
    }

    fn hermite_scalar(n: u32, x: f64) -> f64 {
        if n == 0 {
            return 1.0;
        }
        if n == 1 {
            return 2.0 * x;
        }

        let mut h_prev = 1.0;
        let mut h_curr = 2.0 * x;

        for k in 1..n {
            let h_next = 2.0 * x * h_curr - 2.0 * (k as f64) * h_prev;
            h_prev = h_curr;
            h_curr = h_next;
        }

        h_curr
    }

    fn hermite_function_scalar(n: u32, x: f64) -> f64 {
        let h_n = Self::hermite_scalar(n, x);
        let two_n = 1u64 << n.min(62); // Avoid overflow
        let n_fact = (1..=n as u64).product::<u64>() as f64;
        let norm = 1.0 / (two_n as f64 * n_fact * std::f64::consts::PI.sqrt()).sqrt();
        norm * h_n * (-x * x / 2.0).exp()
    }

    fn hermite_gpu(&self, x: &[f64], n: u32) -> Result<Vec<f64>> {
        self.execute_kernel(x, n, "main")
    }

    fn hermite_function_gpu(&self, x: &[f64], n: u32) -> Result<Vec<f64>> {
        self.execute_kernel(x, n, "hermite_function_kernel")
    }

    fn execute_kernel(&self, x: &[f64], n: u32, entry_point: &str) -> Result<Vec<f64>> {
        let size = x.len();

        let input_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Hermite f64 Input"),
                contents: bytemuck::cast_slice(x),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hermite f64 Output"),
            size: std::mem::size_of_val(x) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            size: u32,
            n: u32,
            _pad0: u32,
            _pad1: u32,
        }

        let params = Params {
            size: size as u32,
            n,
            _pad0: 0,
            _pad1: 0,
        };

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Hermite f64 Params"),
                contents: bytemuck::cast_slice(&[params]),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Hermite f64 BGL"),
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

        let bind_group = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Hermite f64 Bind Group"),
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
            .compile_shader(Self::wgsl_shader(), Some("Hermite f64"));

        let pipeline_layout =
            self.device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Hermite f64 Pipeline Layout"),
                    bind_group_layouts: &[&bgl],
                    push_constant_ranges: &[],
                });

        let pipeline = self
            .device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Hermite f64 Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point,
            cache: None,
            compilation_options: Default::default(),
            });

        let mut encoder = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Hermite f64 Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Hermite f64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = (size as u32).div_ceil(WORKGROUP_SIZE_1D);
            pass.dispatch_workgroups(workgroups, 1, 1);
        }

        // Read back results
        let staging_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Hermite f64 Staging"),
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
    fn test_hermite_f64_h0() {
        let Some(device) = get_test_device() else { return; };
        let op = HermiteF64::new(device).unwrap();

        let x = vec![0.0, 1.0, 2.0, -1.0, 0.5];
        let result = op.hermite(&x, 0).unwrap();

        // H₀(x) = 1 for all x
        for &v in &result {
            assert!((v - 1.0).abs() < 1e-10, "H₀ should be 1, got {}", v);
        }
    }

    #[test]
    fn test_hermite_f64_h1() {
        let Some(device) = get_test_device() else { return; };
        let op = HermiteF64::new(device).unwrap();

        let x = vec![0.0, 1.0, 2.0, -1.0, 0.5];
        let result = op.hermite(&x, 1).unwrap();

        // H₁(x) = 2x
        for (i, &v) in result.iter().enumerate() {
            let expected = 2.0 * x[i];
            assert!(
                (v - expected).abs() < 1e-10,
                "H₁({}) = {}, expected {}",
                x[i],
                v,
                expected
            );
        }
    }

    #[test]
    fn test_hermite_f64_h2() {
        let Some(device) = get_test_device() else { return; };
        let op = HermiteF64::new(device).unwrap();

        let x = vec![0.0, 1.0, 2.0, -1.0, 0.5];
        let result = op.hermite(&x, 2).unwrap();

        // H₂(x) = 4x² - 2
        for (i, &v) in result.iter().enumerate() {
            let xi = x[i];
            let expected = 4.0 * xi * xi - 2.0;
            assert!(
                (v - expected).abs() < 1e-10,
                "H₂({}) = {}, expected {}",
                xi,
                v,
                expected
            );
        }
    }

    #[test]
    fn test_hermite_f64_h10() {
        let Some(device) = get_test_device() else { return; };
        let op = HermiteF64::new(device).unwrap();

        // H₁₀(0) = -30240 (from tables)
        let x = vec![0.0];
        let result = op.hermite(&x, 10).unwrap();
        assert!(
            (result[0] - (-30240.0)).abs() < 1e-6,
            "H₁₀(0) = {}, expected -30240",
            result[0]
        );
    }

    #[test]
    fn test_hermite_function_normalization() {
        let Some(device) = get_test_device() else { return; };
        let op = HermiteF64::new(device).unwrap();

        // Test that ψ₀(0) = π^(-1/4) ≈ 0.7511
        let x = vec![0.0];
        let result = op.hermite_function(&x, 0).unwrap();
        let expected = std::f64::consts::PI.powf(-0.25);
        assert!(
            (result[0] - expected).abs() < 1e-10,
            "ψ₀(0) = {}, expected {}",
            result[0],
            expected
        );
    }
}
