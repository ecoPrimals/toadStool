//! CORRELATION F64 - Pearson correlation coefficient - f64 precision
//!
//! Deep Debt Principles:
//! - Self-knowledge: Operation knows its computation
//! - Zero hardcoding: Hardware-agnostic implementation
//! - Modern idiomatic Rust: Safe, zero unsafe code
//!
//! Applications:
//! - Signal correlation
//! - Feature selection
//! - Portfolio analysis

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
    _pad: u32,
}

/// f64 Pearson correlation evaluator
pub struct CorrelationF64 {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
}

impl CorrelationF64 {
    fn wgsl_shader() -> &'static str {
        include_str!("../shaders/special/correlation_f64.wgsl")
    }

    /// Create new Correlation f64 operation
    pub fn new(device: Arc<WgpuDevice>) -> Result<Self> {
        let shader = device.compile_shader_f64(Self::wgsl_shader(), Some("CorrelationF64"));

        let bind_group_layout =
            device
                .device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("CorrelationF64 BGL"),
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
                    label: Some("CorrelationF64 PL"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline = device
            .device
            .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("CorrelationF64 Pipeline"),
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

    /// Compute Pearson correlation coefficient between two vectors
    ///
    /// r = Σ(x-μx)(y-μy) / sqrt(Σ(x-μx)² * Σ(y-μy)²)
    ///
    /// # Returns
    /// Correlation coefficient in range [-1, 1]
    pub fn correlation(&self, x: &[f64], y: &[f64]) -> Result<f64> {
        if x.len() != y.len() || x.is_empty() {
            return Ok(0.0);
        }

        let n = x.len();
        let params = Params {
            size: n as u32,
            num_pairs: 1,
            stride: n as u32,
            _pad: 0,
        };

        let x_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CorrelationF64 X"),
                contents: bytemuck::cast_slice(x),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let y_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CorrelationF64 Y"),
                contents: bytemuck::cast_slice(y),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_size = std::mem::size_of::<f64>();
        let output_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CorrelationF64 Output"),
            size: output_size as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let params_buf = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("CorrelationF64 Params"),
                contents: bytemuck::bytes_of(&params),
                usage: wgpu::BufferUsages::UNIFORM,
            });

        let bind_group = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("CorrelationF64 BG"),
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
                    label: Some("CorrelationF64 Encoder"),
                });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("CorrelationF64 Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(1u32.div_ceil(WORKGROUP_SIZE_1D).max(1), 1, 1);
        }

        let staging_buf = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CorrelationF64 Staging"),
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
            .map_err(|e| BarracudaError::Gpu(format!("Correlation readback: {}", e)))?
            .map_err(|e| BarracudaError::Gpu(format!("Correlation map: {:?}", e)))?;

        let data = buffer_slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buf.unmap();

        Ok(result[0])
    }

    #[cfg(test)]
    fn correlation_cpu(x: &[f64], y: &[f64]) -> f64 {
        let n = x.len();
        if n == 0 {
            return 0.0;
        }

        // Compute means
        let mean_x: f64 = x.iter().sum::<f64>() / n as f64;
        let mean_y: f64 = y.iter().sum::<f64>() / n as f64;

        // Compute covariance and variances
        let mut cov = 0.0;
        let mut var_x = 0.0;
        let mut var_y = 0.0;

        for (xi, yi) in x.iter().zip(y.iter()) {
            let dx = xi - mean_x;
            let dy = yi - mean_y;
            cov += dx * dy;
            var_x += dx * dx;
            var_y += dy * dy;
        }

        let denom = (var_x * var_y).sqrt();
        if denom < 1e-15 {
            return 0.0; // Avoid division by zero
        }

        cov / denom
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_correlation_perfect_positive() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let corr = CorrelationF64::new(device).unwrap();

        // Perfect positive correlation (r = 1)
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![2.0, 4.0, 6.0, 8.0, 10.0]; // y = 2x
        let result = corr.correlation(&x, &y).unwrap();

        assert!(
            (result - 1.0).abs() < 1e-10,
            "Correlation should be 1.0, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_correlation_perfect_negative() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let corr = CorrelationF64::new(device).unwrap();

        // Perfect negative correlation (r = -1)
        let x = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let y = vec![10.0, 8.0, 6.0, 4.0, 2.0]; // y = -2x + 12
        let result = corr.correlation(&x, &y).unwrap();

        assert!(
            (result + 1.0).abs() < 1e-10,
            "Correlation should be -1.0, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_correlation_self() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let corr = CorrelationF64::new(device).unwrap();

        // Correlation with self is always 1
        let x = vec![1.0, 3.0, 7.0, 2.5, 9.0];
        let result = corr.correlation(&x, &x).unwrap();

        assert!(
            (result - 1.0).abs() < 1e-10,
            "Self-correlation should be 1.0, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_correlation_uncorrelated() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let corr = CorrelationF64::new(device).unwrap();

        // Nearly uncorrelated data (using orthogonal vectors)
        let x = vec![1.0, 0.0, -1.0, 0.0];
        let y = vec![0.0, 1.0, 0.0, -1.0];
        let result = corr.correlation(&x, &y).unwrap();

        assert!(
            result.abs() < 1e-10,
            "Orthogonal vectors should have correlation ~0, got {}",
            result
        );
    }

    #[tokio::test]
    async fn test_correlation_bounds() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let corr = CorrelationF64::new(device).unwrap();

        // Test with various random-ish data
        let x = vec![2.3, 5.1, 1.2, 8.7, 3.3, 6.8, 4.2];
        let y = vec![1.5, 4.2, 2.8, 7.1, 5.5, 3.9, 6.1];
        let result = corr.correlation(&x, &y).unwrap();

        assert!(
            (-1.0..=1.0).contains(&result),
            "Correlation must be in [-1, 1], got {}",
            result
        );
    }
}
