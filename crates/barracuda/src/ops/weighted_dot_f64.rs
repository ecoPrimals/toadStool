//! Weighted Dot Product (f64) — GPU-Accelerated via WGSL
//!
//! Computes weighted inner products: result = Σ_k w[k] · a[k] · b[k]
//!
//! **Use cases**:
//! - Galerkin methods: <φ_i|W|φ_j>
//! - FEM assembly: element matrices
//! - Nuclear physics: potential matrix elements (validated by hotSpring)
//! - Energy integrals: ∫ρ(r)V(r)dr via quadrature
//!
//! **Deep Debt Principles**:
//! - Pure WGSL implementation (hardware-agnostic)
//! - Full f64 precision for scientific computing
//! - Safe Rust wrapper (no unsafe code)

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Parameters for weighted dot product
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct DotParams {
    n: u32,
    _pad0: u32,
    _pad1: u32,
    _pad2: u32,
}

/// GPU-accelerated f64 weighted dot product
pub struct WeightedDotF64 {
    device: Arc<WgpuDevice>,
}

impl WeightedDotF64 {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/reduce/weighted_dot_f64.wgsl")
    }

    /// Create a new WeightedDotF64 orchestrator
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    /// Compute weighted dot product: Σ w[i] * a[i] * b[i]
    ///
    /// # Arguments
    /// * `weights` - Weight vector
    /// * `a` - First vector
    /// * `b` - Second vector
    ///
    /// # Returns
    /// The weighted dot product as a single f64
    pub fn weighted_dot(&self, weights: &[f64], a: &[f64], b: &[f64]) -> Result<f64> {
        let n = weights.len();
        if a.len() != n || b.len() != n {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Vector lengths must match: weights={}, a={}, b={}",
                    n,
                    a.len(),
                    b.len()
                ),
            });
        }

        if n == 0 {
            return Ok(0.0);
        }

        // CPU fallback for small inputs
        if n < 1024 {
            return Ok(self.weighted_dot_cpu(weights, a, b));
        }

        self.weighted_dot_gpu(weights, a, b)
    }

    /// CPU reference implementation
    fn weighted_dot_cpu(&self, weights: &[f64], a: &[f64], b: &[f64]) -> f64 {
        weights
            .iter()
            .zip(a.iter())
            .zip(b.iter())
            .map(|((w, a), b)| w * a * b)
            .sum()
    }

    /// Unweighted dot product: Σ a[i] * b[i]
    pub fn dot(&self, a: &[f64], b: &[f64]) -> Result<f64> {
        let n = a.len();
        if b.len() != n {
            return Err(BarracudaError::InvalidInput {
                message: format!("Vector lengths must match: a={}, b={}", n, b.len()),
            });
        }

        if n == 0 {
            return Ok(0.0);
        }

        // CPU fallback for small inputs
        if n < 1024 {
            return Ok(a.iter().zip(b.iter()).map(|(a, b)| a * b).sum());
        }

        // Use weights = 1.0
        let ones = vec![1.0f64; n];
        self.weighted_dot_gpu(&ones, a, b)
    }

    /// Squared L2 norm: Σ a[i]²
    pub fn norm_squared(&self, a: &[f64]) -> Result<f64> {
        self.dot(a, a)
    }

    fn weighted_dot_gpu(&self, weights: &[f64], a: &[f64], b: &[f64]) -> Result<f64> {
        let n = weights.len();
        let shader = self
            .device
            .compile_shader(Self::wgsl_shader(), Some("Weighted Dot f64"));

        // Create buffers
        let weights_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Weights"),
                contents: bytemuck::cast_slice(weights),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let a_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vec A"),
                contents: bytemuck::cast_slice(a),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let b_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Vec B"),
                contents: bytemuck::cast_slice(b),
                usage: wgpu::BufferUsages::STORAGE,
            });

        // For parallel reduction, we need partial sums from each workgroup
        let workgroup_size = 256;
        let n_workgroups = n.div_ceil(workgroup_size);

        let result_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Partial Sums"),
            size: (n_workgroups * 8) as u64, // f64 = 8 bytes
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = DotParams {
            n: n as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
        };

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout and pipeline
        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Weighted Dot BGL"),
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
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 4,
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

        let pl = self
            .device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Weighted Dot PL"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline = self
            .device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Weighted Dot Pipeline"),
                layout: Some(&pl),
                module: &shader,
                entry_point: "weighted_dot_parallel",
            });

        let bg = self.device.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Weighted Dot BG"),
            layout: &bgl,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: params_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: weights_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: a_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: b_buf.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: result_buf.as_entire_binding(),
                },
            ],
        });

        // Dispatch
        let mut encoder = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Weighted Dot Encoder"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Weighted Dot Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(n_workgroups as u32, 1, 1);
        }

        self.device.queue.submit(Some(encoder.finish()));

        // Read back partial sums and sum on CPU
        let staging = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: (n_workgroups * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder2 = self
            .device
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Copy Encoder"),
            });
        encoder2.copy_buffer_to_buffer(&result_buf, 0, &staging, 0, (n_workgroups * 8) as u64);
        self.device.queue.submit(Some(encoder2.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let partial_sums: &[f64] = bytemuck::cast_slice(&data);
        let result: f64 = partial_sums.iter().sum();
        drop(data);
        staging.unmap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::WgpuDevice;

    fn get_test_device() -> Arc<WgpuDevice> {
        WgpuDevice::new().expect("Failed to create test device")
    }

    #[test]
    fn test_weighted_dot_small() {
        let device = get_test_device();
        let op = WeightedDotF64::new(device).unwrap();

        let w = vec![1.0, 2.0, 3.0];
        let a = vec![1.0, 2.0, 3.0];
        let b = vec![1.0, 1.0, 1.0];

        // Expected: 1*1*1 + 2*2*1 + 3*3*1 = 1 + 4 + 9 = 14
        let result = op.weighted_dot(&w, &a, &b).unwrap();
        assert!((result - 14.0).abs() < 1e-10);
    }

    #[test]
    fn test_dot_product() {
        let device = get_test_device();
        let op = WeightedDotF64::new(device).unwrap();

        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![1.0, 1.0, 1.0, 1.0];

        let result = op.dot(&a, &b).unwrap();
        assert!((result - 10.0).abs() < 1e-10);
    }

    #[test]
    fn test_norm_squared() {
        let device = get_test_device();
        let op = WeightedDotF64::new(device).unwrap();

        let a = vec![3.0, 4.0];

        let result = op.norm_squared(&a).unwrap();
        assert!((result - 25.0).abs() < 1e-10);
    }

    #[test]
    fn test_weighted_dot_large() {
        let device = get_test_device();
        let op = WeightedDotF64::new(device).unwrap();

        let n = 10000;
        let w: Vec<f64> = (0..n).map(|i| 1.0 / (i as f64 + 1.0)).collect();
        let a: Vec<f64> = (0..n).map(|i| (i as f64).sin()).collect();
        let b: Vec<f64> = (0..n).map(|i| (i as f64).cos()).collect();

        let gpu_result = op.weighted_dot(&w, &a, &b).unwrap();
        let cpu_result = op.weighted_dot_cpu(&w, &a, &b);

        assert!(
            (gpu_result - cpu_result).abs() < 1e-8,
            "GPU: {}, CPU: {}",
            gpu_result,
            cpu_result
        );
    }
}
