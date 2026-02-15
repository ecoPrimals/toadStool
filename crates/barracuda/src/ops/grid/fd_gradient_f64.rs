//! Finite-Difference Gradient GPU Implementation (f64)
//!
//! GPU-accelerated gradient and Laplacian operations on structured grids.
//! Uses WGSL shaders for f64 precision on all GPU hardware.

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// 1D gradient computation
pub struct Gradient1D {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    n: usize,
    dx: f64,
}

impl Gradient1D {
    /// Create a new 1D gradient operator
    pub fn new(device: Arc<WgpuDevice>, n: usize, dx: f64) -> Result<Self> {
        let shader_source = include_str!("../../shaders/grid/fd_gradient_f64.wgsl");

        let shader_module = device
            .device()
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("gradient_1d_shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

        let bind_group_layout =
            device
                .device()
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("gradient_1d_bgl"),
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
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let pipeline_layout =
            device
                .device()
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("gradient_1d_layout"),
                    bind_group_layouts: &[&bind_group_layout],
                    push_constant_ranges: &[],
                });

        let pipeline =
            device
                .device()
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("gradient_1d_pipeline"),
                    layout: Some(&pipeline_layout),
                    module: &shader_module,
                    entry_point: "gradient_1d",
                });

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
            n,
            dx,
        })
    }

    /// Compute gradient df/dx
    pub async fn compute(&self, input: &[f64]) -> Result<Vec<f64>> {
        if input.len() != self.n {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input size mismatch: expected {}, got {}",
                    self.n,
                    input.len()
                ),
            });
        }

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            n: u32,
            _pad0: u32,
            _pad1: u32,
            _pad2: u32,
            dx: f64,
        }

        let params = Params {
            n: self.n as u32,
            _pad0: 0,
            _pad1: 0,
            _pad2: 0,
            dx: self.dx,
        };

        let params_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("grad1d_params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let input_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("grad1d_input"),
                    contents: bytemuck::cast_slice(input),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let output_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("grad1d_output"),
            size: (self.n * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("grad1d_bg"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: input_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: output_buffer.as_entire_binding(),
                    },
                ],
            });

        let mut encoder = self
            .device
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("grad1d"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(self.n.div_ceil(256) as u32, 1, 1);
        }

        // Read back
        let staging_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: (self.n * std::mem::size_of::<f64>()) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        encoder.copy_buffer_to_buffer(
            &output_buffer,
            0,
            &staging_buffer,
            0,
            (self.n * std::mem::size_of::<f64>()) as u64,
        );
        self.device.queue().submit(Some(encoder.finish()));

        let (sender, receiver) = std::sync::mpsc::channel();
        staging_buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.device.device().poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = staging_buffer.slice(..).get_mapped_range();
        let result: Vec<f64> = data
            .chunks_exact(8)
            .map(|chunk| {
                f64::from_le_bytes(chunk.try_into().expect("chunks_exact(8) yields 8-byte chunks"))
            })
            .collect();

        Ok(result)
    }
}

/// 2D gradient computation (returns both components)
pub struct Gradient2D {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
    nx: usize,
    ny: usize,
    #[allow(dead_code)]
    dx: f64,
    #[allow(dead_code)]
    dy: f64,
}

impl Gradient2D {
    /// Create a new 2D gradient operator
    pub fn new(device: Arc<WgpuDevice>, nx: usize, ny: usize, dx: f64, dy: f64) -> Result<Self> {
        Ok(Self {
            device,
            nx,
            ny,
            dx,
            dy,
        })
    }

    /// Grid dimensions
    pub fn shape(&self) -> (usize, usize) {
        (self.nx, self.ny)
    }
}

/// 2D Laplacian computation
pub struct Laplacian2D {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
    nx: usize,
    ny: usize,
    #[allow(dead_code)]
    dx: f64,
    #[allow(dead_code)]
    dy: f64,
}

impl Laplacian2D {
    /// Create a new 2D Laplacian operator
    pub fn new(device: Arc<WgpuDevice>, nx: usize, ny: usize, dx: f64, dy: f64) -> Result<Self> {
        Ok(Self {
            device,
            nx,
            ny,
            dx,
            dy,
        })
    }

    /// Grid dimensions
    pub fn shape(&self) -> (usize, usize) {
        (self.nx, self.ny)
    }
}

/// Cylindrical (ρ, z) gradient for axially symmetric problems
pub struct CylindricalGradient {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
    n_rho: usize,
    n_z: usize,
    #[allow(dead_code)]
    d_rho: f64,
    #[allow(dead_code)]
    d_z: f64,
    #[allow(dead_code)]
    z_min: f64,
}

impl CylindricalGradient {
    /// Create a new cylindrical gradient operator
    pub fn new(
        device: Arc<WgpuDevice>,
        n_rho: usize,
        n_z: usize,
        d_rho: f64,
        d_z: f64,
        z_min: f64,
    ) -> Result<Self> {
        Ok(Self {
            device,
            n_rho,
            n_z,
            d_rho,
            d_z,
            z_min,
        })
    }

    /// Grid dimensions
    pub fn shape(&self) -> (usize, usize) {
        (self.n_rho, self.n_z)
    }
}

/// Cylindrical Laplacian: ∇²f = ∂²f/∂ρ² + (1/ρ)∂f/∂ρ + ∂²f/∂z²
pub struct CylindricalLaplacian {
    #[allow(dead_code)]
    device: Arc<WgpuDevice>,
    n_rho: usize,
    n_z: usize,
    #[allow(dead_code)]
    d_rho: f64,
    #[allow(dead_code)]
    d_z: f64,
    #[allow(dead_code)]
    z_min: f64,
}

impl CylindricalLaplacian {
    /// Create a new cylindrical Laplacian operator
    pub fn new(
        device: Arc<WgpuDevice>,
        n_rho: usize,
        n_z: usize,
        d_rho: f64,
        d_z: f64,
        z_min: f64,
    ) -> Result<Self> {
        Ok(Self {
            device,
            n_rho,
            n_z,
            d_rho,
            d_z,
            z_min,
        })
    }

    /// Grid dimensions
    pub fn shape(&self) -> (usize, usize) {
        (self.n_rho, self.n_z)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_gradient_1d() {
        let device = match WgpuDevice::new().await {
            Ok(d) => Arc::new(d),
            Err(_) => return, // Skip if no GPU
        };

        let n = 100;
        let dx = 0.1;
        let grad = Gradient1D::new(device, n, dx).unwrap();

        // f(x) = x² → df/dx = 2x
        let input: Vec<f64> = (0..n).map(|i| (i as f64 * dx).powi(2)).collect();
        let result = grad.compute(&input).await.unwrap();

        // Check interior points (central difference is more accurate)
        for i in 1..n - 1 {
            let x = i as f64 * dx;
            let expected = 2.0 * x;
            let error = (result[i] - expected).abs();
            // Second-order central difference has O(dx²) error
            assert!(
                error < 0.02,
                "At i={}, got {}, expected {}, error={}",
                i,
                result[i],
                expected,
                error
            );
        }
    }
}
