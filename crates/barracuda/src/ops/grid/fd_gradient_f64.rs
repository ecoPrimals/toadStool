//! Finite-Difference Gradient GPU Implementation (f64)
//!
//! GPU-accelerated gradient and Laplacian operations on structured grids.
//! Uses WGSL shaders for f64 precision on all GPU hardware.
//!
//! **Smart Refactoring**: Uses `fd_common` for shared infrastructure.

use super::fd_common::{
    create_staging_buffer, read_staging_f64 as read_staging, FdPipelineBuilder,
    FD_WORKGROUP_SIZE,
};
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
    ///
    /// Uses `FdPipelineBuilder` for reduced boilerplate.
    pub fn new(device: Arc<WgpuDevice>, n: usize, dx: f64) -> Result<Self> {
        let (pipeline, bind_group_layout) = FdPipelineBuilder::new(device.device(), "gradient_1d")
            .with_uniform(0) // params
            .with_input(1)   // input field
            .with_output(2)  // gradient output
            .build("gradient_1d")?;

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
            pass.dispatch_workgroups(self.n.div_ceil(FD_WORKGROUP_SIZE as usize) as u32, 1, 1);
        }

        // Read back using common utility
        let buffer_size = (self.n * std::mem::size_of::<f64>()) as u64;
        let staging = create_staging_buffer(self.device.device(), buffer_size, "grad1d_staging");
        encoder.copy_buffer_to_buffer(&output_buffer, 0, &staging, 0, buffer_size);
        self.device.queue().submit(Some(encoder.finish()));

        read_staging(self.device.device(), &staging, self.n).await
    }
}

/// 2D gradient computation (returns both components)
pub struct Gradient2D {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    nx: usize,
    ny: usize,
    dx: f64,
    dy: f64,
}

impl Gradient2D {
    /// Create a new 2D gradient operator
    ///
    /// Uses `FdPipelineBuilder` for reduced boilerplate.
    pub fn new(device: Arc<WgpuDevice>, nx: usize, ny: usize, dx: f64, dy: f64) -> Result<Self> {
        let (pipeline, bind_group_layout) = FdPipelineBuilder::new(device.device(), "gradient_2d")
            .with_uniform(0) // params
            .with_input(1)   // input field
            .with_output(2)  // grad_x output
            .with_output(3)  // grad_y output
            .build("gradient_2d")?;

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
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

    /// Compute 2D gradient (∂f/∂x, ∂f/∂y)
    ///
    /// # Arguments
    /// * `input` - 2D field as row-major array [nx × ny]
    ///
    /// # Returns
    /// Tuple (grad_x, grad_y) each as row-major [nx × ny]
    pub async fn compute(&self, input: &[f64]) -> Result<(Vec<f64>, Vec<f64>)> {
        let total = self.nx * self.ny;
        if input.len() != total {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input size mismatch: expected {} ({}×{}), got {}",
                    total,
                    self.nx,
                    self.ny,
                    input.len()
                ),
            });
        }

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            nx: u32,
            ny: u32,
            _pad0: u32,
            _pad1: u32,
            dx: f64,
            dy: f64,
        }

        let params = Params {
            nx: self.nx as u32,
            ny: self.ny as u32,
            _pad0: 0,
            _pad1: 0,
            dx: self.dx,
            dy: self.dy,
        };

        let params_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("grad2d_params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let input_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("grad2d_input"),
                    contents: bytemuck::cast_slice(input),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let buffer_size = (total * std::mem::size_of::<f64>()) as u64;

        let grad_x_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("grad2d_grad_x"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let grad_y_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("grad2d_grad_y"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("grad2d_bg"),
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
                        resource: grad_x_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: grad_y_buffer.as_entire_binding(),
                    },
                ],
            });

        let mut encoder = self
            .device
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("grad2d"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // Workgroup size is (16, 16, 1)
            pass.dispatch_workgroups(self.nx.div_ceil(16) as u32, self.ny.div_ceil(16) as u32, 1);
        }

        // Read back both gradients
        let staging_x = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_x"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_y = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_y"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&grad_x_buffer, 0, &staging_x, 0, buffer_size);
        encoder.copy_buffer_to_buffer(&grad_y_buffer, 0, &staging_y, 0, buffer_size);
        self.device.queue().submit(Some(encoder.finish()));

        // Map and read grad_x
        let grad_x = self.read_staging_buffer(&staging_x, total).await?;
        let grad_y = self.read_staging_buffer(&staging_y, total).await?;

        Ok((grad_x, grad_y))
    }

    async fn read_staging_buffer(&self, staging: &wgpu::Buffer, count: usize) -> Result<Vec<f64>> {
        let (sender, receiver) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.device.device().poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = staging.slice(..).get_mapped_range();
        let result: Vec<f64> = data
            .chunks_exact(8)
            .take(count)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().expect("8-byte chunks")))
            .collect();

        Ok(result)
    }
}

/// 2D Laplacian computation: ∇²f = ∂²f/∂x² + ∂²f/∂y²
pub struct Laplacian2D {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    nx: usize,
    ny: usize,
    dx: f64,
    dy: f64,
}

impl Laplacian2D {
    /// Create a new 2D Laplacian operator
    ///
    /// Uses `FdPipelineBuilder` for reduced boilerplate.
    /// Note: Shader requires dummy bindings 2-4 for shared layout compatibility.
    pub fn new(device: Arc<WgpuDevice>, nx: usize, ny: usize, dx: f64, dy: f64) -> Result<Self> {
        let (pipeline, bind_group_layout) = FdPipelineBuilder::new(device.device(), "laplacian_2d")
            .with_uniform(0) // params
            .with_input(1)   // input field
            .with_output(2)  // dummy grad_x (required by shader)
            .with_output(3)  // dummy grad_y (required by shader)
            .with_output(4)  // dummy grad_mag (required by shader)
            .with_output(5)  // laplacian output
            .build("laplacian_2d")?;

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
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

    /// Compute 2D Laplacian: ∇²f = ∂²f/∂x² + ∂²f/∂y²
    ///
    /// # Arguments
    /// * `input` - 2D field as row-major array [nx × ny]
    ///
    /// # Returns
    /// Laplacian as row-major [nx × ny]
    pub async fn compute(&self, input: &[f64]) -> Result<Vec<f64>> {
        let total = self.nx * self.ny;
        if input.len() != total {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input size mismatch: expected {} ({}×{}), got {}",
                    total,
                    self.nx,
                    self.ny,
                    input.len()
                ),
            });
        }

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            nx: u32,
            ny: u32,
            _pad0: u32,
            _pad1: u32,
            dx: f64,
            dy: f64,
        }

        let params = Params {
            nx: self.nx as u32,
            ny: self.ny as u32,
            _pad0: 0,
            _pad1: 0,
            dx: self.dx,
            dy: self.dy,
        };

        let params_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("lap2d_params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let input_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("lap2d_input"),
                    contents: bytemuck::cast_slice(input),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let buffer_size = (total * std::mem::size_of::<f64>()) as u64;

        // Create dummy buffers for unused bindings (8 bytes minimum)
        let dummy_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("lap2d_dummy"),
            size: 8,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let laplacian_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("lap2d_output"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("lap2d_bg"),
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
                        resource: dummy_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: dummy_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: dummy_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: laplacian_buffer.as_entire_binding(),
                    },
                ],
            });

        let mut encoder = self
            .device
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("lap2d"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(self.nx.div_ceil(16) as u32, self.ny.div_ceil(16) as u32, 1);
        }

        // Read back
        let staging = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("lap2d_staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&laplacian_buffer, 0, &staging, 0, buffer_size);
        self.device.queue().submit(Some(encoder.finish()));

        let (sender, receiver) = std::sync::mpsc::channel();
        staging
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        self.device.device().poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|_| BarracudaError::execution_failed("buffer mapping channel closed"))?
            .map_err(|e| BarracudaError::execution_failed(e.to_string()))?;

        let data = staging.slice(..).get_mapped_range();
        let result: Vec<f64> = data
            .chunks_exact(8)
            .take(total)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().expect("8-byte chunks")))
            .collect();

        Ok(result)
    }
}

/// Cylindrical (ρ, z) gradient for axially symmetric problems
///
/// Computes ∂f/∂ρ and ∂f/∂z on a (ρ, z) grid.
/// Used for nuclear physics (deformed nuclei), fluid dynamics, etc.
pub struct CylindricalGradient {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    n_rho: usize,
    n_z: usize,
    d_rho: f64,
    d_z: f64,
    z_min: f64,
}

impl CylindricalGradient {
    /// Create a new cylindrical gradient operator
    ///
    /// Uses `FdPipelineBuilder` for reduced boilerplate.
    pub fn new(
        device: Arc<WgpuDevice>,
        n_rho: usize,
        n_z: usize,
        d_rho: f64,
        d_z: f64,
        z_min: f64,
    ) -> Result<Self> {
        let (pipeline, bind_group_layout) = FdPipelineBuilder::new(device.device(), "cyl_grad")
            .with_uniform(0) // params
            .with_input(1)   // input field
            .with_output(2)  // grad_rho output
            .with_output(3)  // grad_z output
            .build("gradient_cylindrical")?;

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
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

    /// Compute cylindrical gradient (∂f/∂ρ, ∂f/∂z)
    ///
    /// # Arguments
    /// * `input` - Field on (ρ,z) grid as row-major array [n_rho × n_z]
    ///
    /// # Returns
    /// Tuple (grad_rho, grad_z) each as row-major [n_rho × n_z]
    pub async fn compute(&self, input: &[f64]) -> Result<(Vec<f64>, Vec<f64>)> {
        let total = self.n_rho * self.n_z;
        if input.len() != total {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input size mismatch: expected {} ({}×{}), got {}",
                    total,
                    self.n_rho,
                    self.n_z,
                    input.len()
                ),
            });
        }

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CylParams {
            n_rho: u32,
            n_z: u32,
            _pad0: u32,
            _pad1: u32,
            d_rho: f64,
            d_z: f64,
            z_min: f64,
        }

        let params = CylParams {
            n_rho: self.n_rho as u32,
            n_z: self.n_z as u32,
            _pad0: 0,
            _pad1: 0,
            d_rho: self.d_rho,
            d_z: self.d_z,
            z_min: self.z_min,
        };

        let params_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("cyl_grad_params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let input_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("cyl_grad_input"),
                    contents: bytemuck::cast_slice(input),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let buffer_size = (total * std::mem::size_of::<f64>()) as u64;

        let grad_rho_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("cyl_grad_rho"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let grad_z_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("cyl_grad_z"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cyl_grad_bg"),
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
                        resource: grad_rho_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: grad_z_buffer.as_entire_binding(),
                    },
                ],
            });

        let mut encoder = self
            .device
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cyl_grad"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            // Workgroup size is 256, processing flat index
            pass.dispatch_workgroups(total.div_ceil(256) as u32, 1, 1);
        }

        // Read back
        let staging_rho = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_rho"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_z = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_z"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&grad_rho_buffer, 0, &staging_rho, 0, buffer_size);
        encoder.copy_buffer_to_buffer(&grad_z_buffer, 0, &staging_z, 0, buffer_size);
        self.device.queue().submit(Some(encoder.finish()));

        let grad_rho = read_staging(self.device.device(), &staging_rho, total).await?;
        let grad_z = read_staging(self.device.device(), &staging_z, total).await?;

        Ok((grad_rho, grad_z))
    }
}

/// Cylindrical Laplacian: ∇²f = ∂²f/∂ρ² + (1/ρ)∂f/∂ρ + ∂²f/∂z²
///
/// Proper cylindrical Laplacian including the 1/ρ term.
/// Used for axially symmetric problems in physics.
pub struct CylindricalLaplacian {
    device: Arc<WgpuDevice>,
    pipeline: wgpu::ComputePipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    n_rho: usize,
    n_z: usize,
    d_rho: f64,
    d_z: f64,
    z_min: f64,
}

impl CylindricalLaplacian {
    /// Create a new cylindrical Laplacian operator
    ///
    /// Uses `FdPipelineBuilder` for reduced boilerplate.
    /// Note: Shader requires dummy bindings 2-3 for shared layout compatibility.
    pub fn new(
        device: Arc<WgpuDevice>,
        n_rho: usize,
        n_z: usize,
        d_rho: f64,
        d_z: f64,
        z_min: f64,
    ) -> Result<Self> {
        let (pipeline, bind_group_layout) = FdPipelineBuilder::new(device.device(), "cyl_lap")
            .with_uniform(0) // params
            .with_input(1)   // input field
            .with_output(2)  // dummy grad_rho (required by shader)
            .with_output(3)  // dummy grad_z (required by shader)
            .with_output(4)  // laplacian output
            .build("laplacian_cylindrical")?;

        Ok(Self {
            device,
            pipeline,
            bind_group_layout,
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

    /// Compute cylindrical Laplacian: ∇²f = ∂²f/∂ρ² + (1/ρ)∂f/∂ρ + ∂²f/∂z²
    pub async fn compute(&self, input: &[f64]) -> Result<Vec<f64>> {
        let total = self.n_rho * self.n_z;
        if input.len() != total {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "Input size mismatch: expected {} ({}×{}), got {}",
                    total,
                    self.n_rho,
                    self.n_z,
                    input.len()
                ),
            });
        }

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct CylParams {
            n_rho: u32,
            n_z: u32,
            _pad0: u32,
            _pad1: u32,
            d_rho: f64,
            d_z: f64,
            z_min: f64,
        }

        let params = CylParams {
            n_rho: self.n_rho as u32,
            n_z: self.n_z as u32,
            _pad0: 0,
            _pad1: 0,
            d_rho: self.d_rho,
            d_z: self.d_z,
            z_min: self.z_min,
        };

        let params_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("cyl_lap_params"),
                    contents: bytemuck::bytes_of(&params),
                    usage: wgpu::BufferUsages::UNIFORM,
                });

        let input_buffer =
            self.device
                .device()
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("cyl_lap_input"),
                    contents: bytemuck::cast_slice(input),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let buffer_size = (total * std::mem::size_of::<f64>()) as u64;

        // Dummy buffers for unused bindings
        let dummy_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("cyl_lap_dummy"),
            size: 8,
            usage: wgpu::BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let laplacian_buffer = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("cyl_lap_output"),
            size: buffer_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let bind_group = self
            .device
            .device()
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("cyl_lap_bg"),
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
                        resource: dummy_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: dummy_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: laplacian_buffer.as_entire_binding(),
                    },
                ],
            });

        let mut encoder = self
            .device
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("cyl_lap"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            pass.dispatch_workgroups(total.div_ceil(256) as u32, 1, 1);
        }

        // Read back
        let staging = self.device.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("cyl_lap_staging"),
            size: buffer_size,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_buffer_to_buffer(&laplacian_buffer, 0, &staging, 0, buffer_size);
        self.device.queue().submit(Some(encoder.finish()));

        read_staging(self.device.device(), &staging, total).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::test_pool::get_test_device_if_f64_gpu_available;

    #[tokio::test]
    async fn test_gradient_1d() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
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

    #[tokio::test]
    async fn test_gradient_2d() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let nx = 20;
        let ny = 20;
        let dx = 0.1;
        let dy = 0.1;
        let grad = Gradient2D::new(device, nx, ny, dx, dy).unwrap();

        // f(x,y) = x² + 2y → ∂f/∂x = 2x, ∂f/∂y = 2
        let mut input = vec![0.0; nx * ny];
        for ix in 0..nx {
            for iy in 0..ny {
                let x = ix as f64 * dx;
                let y = iy as f64 * dy;
                input[ix * ny + iy] = x * x + 2.0 * y;
            }
        }

        let (grad_x, grad_y) = grad.compute(&input).await.unwrap();

        assert_eq!(grad_x.len(), nx * ny);
        assert_eq!(grad_y.len(), nx * ny);

        // Check interior points
        for ix in 1..nx - 1 {
            for iy in 1..ny - 1 {
                let x = ix as f64 * dx;
                let idx = ix * ny + iy;

                // ∂f/∂x = 2x
                let expected_gx = 2.0 * x;
                let error_gx = (grad_x[idx] - expected_gx).abs();
                assert!(
                    error_gx < 0.05,
                    "grad_x at ({},{}) = {}, expected {}, error={}",
                    ix,
                    iy,
                    grad_x[idx],
                    expected_gx,
                    error_gx
                );

                // ∂f/∂y = 2
                let error_gy = (grad_y[idx] - 2.0).abs();
                assert!(
                    error_gy < 0.01,
                    "grad_y at ({},{}) = {}, expected 2, error={}",
                    ix,
                    iy,
                    grad_y[idx],
                    error_gy
                );
            }
        }
    }

    #[tokio::test]
    async fn test_laplacian_2d() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let nx = 20;
        let ny = 20;
        let dx = 0.1;
        let dy = 0.1;
        let lap = Laplacian2D::new(device, nx, ny, dx, dy).unwrap();

        // f(x,y) = x² + y² → ∇²f = 2 + 2 = 4
        let mut input = vec![0.0; nx * ny];
        for ix in 0..nx {
            for iy in 0..ny {
                let x = ix as f64 * dx;
                let y = iy as f64 * dy;
                input[ix * ny + iy] = x * x + y * y;
            }
        }

        let result = lap.compute(&input).await.unwrap();
        assert_eq!(result.len(), nx * ny);

        // Check interior points (boundary has Dirichlet BC applied)
        for ix in 2..nx - 2 {
            for iy in 2..ny - 2 {
                let idx = ix * ny + iy;
                let expected = 4.0;
                let error = (result[idx] - expected).abs();
                assert!(
                    error < 0.01,
                    "Laplacian at ({},{}) = {}, expected {}, error={}",
                    ix,
                    iy,
                    result[idx],
                    expected,
                    error
                );
            }
        }
    }

    #[tokio::test]
    async fn test_cylindrical_gradient() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let n_rho = 10;
        let n_z = 10;
        let d_rho = 0.2;
        let d_z = 0.2;
        let z_min = -1.0;

        let grad = CylindricalGradient::new(device, n_rho, n_z, d_rho, d_z, z_min).unwrap();

        // f(ρ, z) = ρ² + z → ∂f/∂ρ = 2ρ, ∂f/∂z = 1
        let mut input = vec![0.0; n_rho * n_z];
        for i_rho in 0..n_rho {
            for i_z in 0..n_z {
                let rho = (i_rho + 1) as f64 * d_rho; // ρ starts at d_rho
                let z = z_min + (i_z as f64 + 0.5) * d_z;
                input[i_rho * n_z + i_z] = rho * rho + z;
            }
        }

        let (grad_rho, grad_z) = grad.compute(&input).await.unwrap();

        assert_eq!(grad_rho.len(), n_rho * n_z);
        assert_eq!(grad_z.len(), n_rho * n_z);

        // Check interior points
        for i_rho in 1..n_rho - 1 {
            for i_z in 1..n_z - 1 {
                let rho = (i_rho + 1) as f64 * d_rho;
                let idx = i_rho * n_z + i_z;

                // ∂f/∂ρ = 2ρ
                let expected_rho = 2.0 * rho;
                let error_rho = (grad_rho[idx] - expected_rho).abs();
                assert!(
                    error_rho < 0.2,
                    "grad_rho at ({},{}) = {}, expected {}, error={}",
                    i_rho,
                    i_z,
                    grad_rho[idx],
                    expected_rho,
                    error_rho
                );

                // ∂f/∂z = 1
                let error_z = (grad_z[idx] - 1.0).abs();
                assert!(
                    error_z < 0.01,
                    "grad_z at ({},{}) = {}, expected 1, error={}",
                    i_rho,
                    i_z,
                    grad_z[idx],
                    error_z
                );
            }
        }
    }

    #[tokio::test]
    async fn test_cylindrical_laplacian() {
        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let n_rho = 10;
        let n_z = 10;
        let d_rho = 0.2;
        let d_z = 0.2;
        let z_min = -1.0;

        let lap = CylindricalLaplacian::new(device, n_rho, n_z, d_rho, d_z, z_min).unwrap();

        // f(ρ, z) = z²
        // ∇²f = ∂²f/∂ρ² + (1/ρ)∂f/∂ρ + ∂²f/∂z² = 0 + 0 + 2 = 2
        let mut input = vec![0.0; n_rho * n_z];
        for i_rho in 0..n_rho {
            for i_z in 0..n_z {
                let z = z_min + (i_z as f64 + 0.5) * d_z;
                input[i_rho * n_z + i_z] = z * z;
            }
        }

        let result = lap.compute(&input).await.unwrap();
        assert_eq!(result.len(), n_rho * n_z);

        // Check interior points
        for i_rho in 2..n_rho - 2 {
            for i_z in 2..n_z - 2 {
                let idx = i_rho * n_z + i_z;
                let expected = 2.0;
                let error = (result[idx] - expected).abs();
                assert!(
                    error < 0.1,
                    "Laplacian at ({},{}) = {}, expected {}, error={}",
                    i_rho,
                    i_z,
                    result[idx],
                    expected,
                    error
                );
            }
        }
    }
}
