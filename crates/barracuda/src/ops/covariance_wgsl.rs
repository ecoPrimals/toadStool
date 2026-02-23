//! Covariance — GPU-Accelerated via WGSL
//!
//! Computes covariance: Cov(X,Y) = E[(X-μx)(Y-μy)]
//!
//! **Use cases**:
//! - Portfolio theory (wetSpring)
//! - PCA preprocessing (all springs)
//! - Kalman filters (airSpring sensor fusion)
//!
//! **Note**: f32 precision. For f64, use manual computation with weighted_dot_f64.

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Parameters for covariance shader
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct CovarianceParams {
    size: u32,
    num_pairs: u32,
    stride: u32,
    ddof: u32, // Delta degrees of freedom (0=population, 1=sample)
}

/// GPU-accelerated covariance computation
pub struct Covariance {
    device: Arc<WgpuDevice>,
}

impl Covariance {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/covariance.wgsl")
    }

    /// Create a new Covariance orchestrator
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        Ok(Self { device })
    }

    /// Compute sample covariance between two vectors (ddof=1)
    ///
    /// # Arguments
    /// * `x` - First vector (f32)
    /// * `y` - Second vector (f32)
    ///
    /// # Returns
    /// Sample covariance
    pub fn covariance(&self, x: &[f32], y: &[f32]) -> Result<f32> {
        self.covariance_with_ddof(x, y, 1)
    }

    /// Compute population covariance (ddof=0)
    pub fn population_covariance(&self, x: &[f32], y: &[f32]) -> Result<f32> {
        self.covariance_with_ddof(x, y, 0)
    }

    /// Compute covariance with specified degrees of freedom
    pub fn covariance_with_ddof(&self, x: &[f32], y: &[f32], ddof: u32) -> Result<f32> {
        let n = x.len();
        if y.len() != n {
            return Err(BarracudaError::InvalidInput {
                message: format!("Vector lengths must match: x={}, y={}", n, y.len()),
            });
        }

        if n <= ddof as usize {
            return Err(BarracudaError::InvalidInput {
                message: format!("Need more than {} elements for ddof={}", ddof, ddof),
            });
        }

        self.covariance_gpu(x, y, ddof)
    }

    /// Compute sample variance of a single vector (ddof=1)
    pub fn variance(&self, x: &[f32]) -> Result<f32> {
        self.covariance_with_ddof(x, x, 1)
    }

    /// Compute population variance (ddof=0)
    pub fn population_variance(&self, x: &[f32]) -> Result<f32> {
        self.covariance_with_ddof(x, x, 0)
    }

    /// Compute standard deviation (sqrt of variance)
    pub fn std(&self, x: &[f32]) -> Result<f32> {
        Ok(self.variance(x)?.sqrt())
    }

    /// CPU reference implementation
    #[cfg(test)]
    fn covariance_cpu(&self, x: &[f32], y: &[f32], ddof: u32) -> f32 {
        let n = x.len() as f32;
        let mean_x: f32 = x.iter().sum::<f32>() / n;
        let mean_y: f32 = y.iter().sum::<f32>() / n;

        let cov_sum: f32 = x
            .iter()
            .zip(y.iter())
            .map(|(xi, yi)| (xi - mean_x) * (yi - mean_y))
            .sum();

        cov_sum / (n - ddof as f32)
    }

    fn covariance_gpu(&self, x: &[f32], y: &[f32], ddof: u32) -> Result<f32> {
        let n = x.len();
        let shader = self
            .device
            .compile_shader(Self::wgsl_shader(), Some("Covariance"));

        // Create buffers
        let x_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("X"),
                contents: bytemuck::cast_slice(x),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let y_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Y"),
                contents: bytemuck::cast_slice(y),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Output"),
            size: 4, // single f32
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params = CovarianceParams {
            size: n as u32,
            num_pairs: 1,
            stride: n as u32,
            ddof,
        };

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        // Create bind group layout
        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Covariance BGL"),
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

        let pl = self
            .device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Covariance PL"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("Covariance Pipeline"),
                    layout: Some(&pl),
                    module: &shader,
                    entry_point: "main",
                    cache: None,
                    compilation_options: Default::default(),
                });

        let bg = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Covariance BG"),
                layout: &bgl,
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

        // Dispatch
        let mut encoder =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Covariance Encoder"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Covariance Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bg, &[]);
            pass.dispatch_workgroups(1, 1, 1);
        }

        self.device.queue.submit(Some(encoder.finish()));

        // Read back result
        let staging = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Staging"),
            size: 4,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder2 =
            self.device
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Copy Encoder"),
                });
        encoder2.copy_buffer_to_buffer(&output_buf, 0, &staging, 0, 4);
        self.device.queue.submit(Some(encoder2.finish()));

        let slice = staging.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        self.device.device.poll(wgpu::Maintain::Wait);

        let data = slice.get_mapped_range();
        let result: f32 = bytemuck::cast_slice(&data)[0];
        drop(data);
        staging.unmap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn get_test_device() -> Arc<crate::device::WgpuDevice> {
        crate::device::test_pool::get_test_device_sync()
    }

    #[test]
    fn test_variance() {
        let device = get_test_device();
        let op = Covariance::new(device).unwrap();

        let x = vec![2.0f32, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0];
        // Mean = 5, Var = Σ(x-5)² / (n-1) = (9+1+1+1+0+0+4+16) / 7 = 32/7 ≈ 4.57

        let var = op.variance(&x).unwrap();
        assert!((var - 4.571428).abs() < 0.01, "Expected ~4.57, got {}", var);
    }

    #[test]
    fn test_covariance_positive() {
        let device = get_test_device();
        let op = Covariance::new(device).unwrap();

        let x: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let y: Vec<f32> = (0..100).map(|i| i as f32 * 2.0).collect();

        let cov = op.covariance(&x, &y).unwrap();
        assert!(cov > 0.0, "Expected positive covariance, got {}", cov);
    }

    #[test]
    fn test_covariance_negative() {
        let device = get_test_device();
        let op = Covariance::new(device).unwrap();

        let x: Vec<f32> = (0..100).map(|i| i as f32).collect();
        let y: Vec<f32> = (0..100).map(|i| -(i as f32)).collect();

        let cov = op.covariance(&x, &y).unwrap();
        assert!(cov < 0.0, "Expected negative covariance, got {}", cov);
    }

    #[test]
    fn test_population_vs_sample() {
        let device = get_test_device();
        let op = Covariance::new(device).unwrap();

        let x: Vec<f32> = (0..10).map(|i| i as f32).collect();

        let pop_var = op.population_variance(&x).unwrap();
        let sample_var = op.variance(&x).unwrap();

        // Sample variance should be larger (n-1 denominator vs n)
        assert!(
            sample_var > pop_var,
            "Sample var ({}) should be > pop var ({})",
            sample_var,
            pop_var
        );
    }
}
