//! COVARIANCE F64 - Covariance computation - f64 precision
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//!
//! Applications:
//! - Portfolio theory
//! - PCA
//! - Kalman filters

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
    num_pairs: u32,
    stride: u32,
    ddof: u32,
}

/// f64 Covariance evaluator
pub struct CovarianceF64 {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl CovarianceF64 {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/covariance_f64.wgsl")
    }

    /// Create new Covariance f64 operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        let shader = device.compile_shader_f64(Self::wgsl_shader(), Some("CovarianceF64"));

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CovarianceF64 BGL"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
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

        let pipeline_layout =
            device
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("CovarianceF64 PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("CovarianceF64 Pipeline"),
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

    /// Compute covariance between two vectors (population covariance, ddof=0)
    pub fn covariance(&self, x: &[f64], y: &[f64]) -> Result<f64> {
        self.covariance_ddof(x, y, 0)
    }

    /// Compute sample covariance (ddof=1)
    pub fn sample_covariance(&self, x: &[f64], y: &[f64]) -> Result<f64> {
        self.covariance_ddof(x, y, 1)
    }

    /// Compute covariance with specified degrees of freedom adjustment
    pub fn covariance_ddof(&self, x: &[f64], y: &[f64], ddof: usize) -> Result<f64> {
        if x.len() != y.len() || x.is_empty() || x.len() <= ddof {
            return Ok(0.0);
        }

        let n = x.len();
        let params = Params {
            size: n as u32,
            num_pairs: 1,
            stride: n as u32,
            ddof: ddof as u32,
        };

        let x_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CovarianceF64 X"),
                contents: bytemuck::cast_slice(x),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let y_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CovarianceF64 Y"),
                contents: bytemuck::cast_slice(y),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_size = std::mem::size_of::<f64>();
        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CovarianceF64 Output"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CovarianceF64 Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("CovarianceF64 BG"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: x_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: y_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: output_buf.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: params_buf.as_entire_binding(),
                    },
                ],
            });

        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("CovarianceF64 Encoder"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("CovarianceF64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1u32.div_ceil(WORKGROUP_SIZE_1D).max(1), 1, 1);
        }

        let staging_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CovarianceF64 Staging"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, output_size as u64);

        self.device.queue.submit(Some(encoder.finish()));

        let buffer_slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result)
                .expect("map_async callback: receiver must be waiting");
        });
        self.device.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| BarracudaError::Gpu(format!("Covariance readback: {}", e)))?
            .map_err(|e| BarracudaError::Gpu(format!("Covariance map: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        Ok(result[0])
    }

    #[cfg(test)]
    fn covariance_cpu(x: &[f64], y: &[f64], ddof: usize) -> f64 {
        let n = x.len();
        if n <= ddof {
            return 0.0;
        }

        // Two-pass for numerical stability
        let mean_x: f64 = x.iter().sum::<f64>() / n as f64;
        let mean_y: f64 = y.iter().sum::<f64>() / n as f64;

        let cov_sum: f64 = x
            .iter()
            .zip(y.iter())
            .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
            .sum();

        cov_sum / (n - ddof) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_covariance_positive() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        // Positive correlation: both increase together
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // y = 2x
        let result = cov.covariance(&x, &y).unwrap();

        assert!(
            result > 0.0,
            "Covariance should be positive, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_covariance_negative() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        // Negative correlation: one increases, other decreases
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0]; // y = -2x + 12
        let result = cov.covariance(&x, &y).unwrap();

        assert!(
            result < 0.0,
            "Covariance should be negative, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_covariance_zero() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        // No correlation: independent
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![3.0, 3.0, 3.0, 3.0, 3.0]; // constant
        let result = cov.covariance(&x, &y).unwrap();

        assert!(
            result.abs() < 1e-10,
            "Covariance with constant should be 0, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_covariance_self_is_variance() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        // Cov(X, X) = Var(X)
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let cov_xx = cov.covariance(&x, &x).unwrap();

        // Population variance = 2
        assert!(
            (cov_xx - 2.0).abs() < 1e-10,
            "Cov(X,X) = {}, expected variance = 2.0",
            cov_xx
        );
    }

    #[tokio::test]
    async fn test_sample_covariance() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let cov = CovarianceF64::new(device).unwrap();

        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![1.0, 2.0, 3.0, 4.0, 5.0];

        // Sample Cov(X,X) = sample variance = 10/4 = 2.5
        let result = cov.sample_covariance(&x, &y).unwrap();

        assert!(
            (result - 2.5).abs() < 1e-10,
            "Sample Cov(X,X) = {}, expected 2.5",
            result
        );
    }
}
