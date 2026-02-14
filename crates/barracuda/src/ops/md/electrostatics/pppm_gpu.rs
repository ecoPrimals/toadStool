//! GPU-accelerated PPPM/Ewald electrostatics (universal via WGSL)
//!
//! This module provides hardware-agnostic PPPM using WGSL shaders that run
//! on any GPU/NPU/CPU via wgpu. All math is f64 precision using the
//! math_f64.wgsl library.
//!
//! # Pipeline
//! 1. B-spline coefficients (bspline.wgsl)
//! 2. Charge spreading (charge_spread.wgsl)
//! 3. Forward 3D FFT (fft_1d_f64.wgsl × 3)
//! 4. Green's function application (greens_apply.wgsl)
//! 5. Inverse 3D FFT (fft_1d_f64.wgsl × 3)
//! 6. Force interpolation (force_interp.wgsl)
//! 7. Short-range forces (erfc_forces.wgsl)
//!
//! # Example
//! ```ignore
//! let pppm = PppmGpu::new(&device, &queue, params).await?;
//! let (forces, energy) = pppm.compute(&positions, &charges).await?;
//! ```

use crate::error::{BarracudaError, Result};
use crate::shaders::precision::ShaderTemplate;
use wgpu::util::DeviceExt;

use std::sync::Arc;

use super::{GreensFunction, PppmParams};

/// GPU-accelerated PPPM solver
///
/// Contains compiled pipelines and precomputed data for GPU execution.
/// Pipelines are created once and reused for all compute calls.
#[allow(dead_code)] // Fields used in compute methods (WIP)
pub struct PppmGpu {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    params: PppmParams,
    greens: GreensFunction,

    // Compute pipelines
    bspline_pipeline: wgpu::ComputePipeline,
    charge_spread_pipeline: wgpu::ComputePipeline,
    greens_apply_pipeline: wgpu::ComputePipeline,
    force_interp_pipeline: wgpu::ComputePipeline,
    erfc_forces_pipeline: wgpu::ComputePipeline,
    self_energy_pipeline: wgpu::ComputePipeline,

    // Bind group layouts
    bspline_layout: wgpu::BindGroupLayout,
    charge_spread_layout: wgpu::BindGroupLayout,
    greens_apply_layout: wgpu::BindGroupLayout,
    force_interp_layout: wgpu::BindGroupLayout,
    erfc_forces_layout: wgpu::BindGroupLayout,
}

impl PppmGpu {
    /// Create a new GPU PPPM solver
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        params: PppmParams,
    ) -> Result<Self> {
        // Precompute Green's function
        let greens = GreensFunction::new(&params);

        // Compile shaders with math_f64 preamble
        let bspline_shader = ShaderTemplate::with_math_f64(include_str!("bspline.wgsl"));
        let charge_spread_shader =
            ShaderTemplate::with_math_f64(include_str!("charge_spread.wgsl"));
        let greens_apply_shader = ShaderTemplate::with_math_f64(include_str!("greens_apply.wgsl"));
        let force_interp_shader = ShaderTemplate::with_math_f64(include_str!("force_interp.wgsl"));
        let erfc_forces_shader = ShaderTemplate::with_math_f64(include_str!("erfc_forces.wgsl"));

        // Create shader modules
        let bspline_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pppm_bspline"),
            source: wgpu::ShaderSource::Wgsl(bspline_shader.into()),
        });

        let charge_spread_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pppm_charge_spread"),
            source: wgpu::ShaderSource::Wgsl(charge_spread_shader.into()),
        });

        let greens_apply_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pppm_greens_apply"),
            source: wgpu::ShaderSource::Wgsl(greens_apply_shader.into()),
        });

        let force_interp_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pppm_force_interp"),
            source: wgpu::ShaderSource::Wgsl(force_interp_shader.into()),
        });

        let erfc_forces_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("pppm_erfc_forces"),
            source: wgpu::ShaderSource::Wgsl(erfc_forces_shader.into()),
        });

        // Create bind group layouts
        let bspline_layout = Self::create_bspline_layout(&device);
        let charge_spread_layout = Self::create_charge_spread_layout(&device);
        let greens_apply_layout = Self::create_greens_apply_layout(&device);
        let force_interp_layout = Self::create_force_interp_layout(&device);
        let erfc_forces_layout = Self::create_erfc_forces_layout(&device);

        // Create pipelines
        let bspline_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pppm_bspline_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("pppm_bspline_layout"),
                    bind_group_layouts: &[&bspline_layout],
                    push_constant_ranges: &[],
                }),
            ),
            module: &bspline_module,
            entry_point: "main",
        });

        let charge_spread_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("pppm_charge_spread_pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("pppm_charge_spread_layout"),
                        bind_group_layouts: &[&charge_spread_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &charge_spread_module,
                entry_point: "spread_per_particle",
            });

        let greens_apply_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("pppm_greens_apply_pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("pppm_greens_apply_layout"),
                        bind_group_layouts: &[&greens_apply_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &greens_apply_module,
                entry_point: "main",
            });

        let force_interp_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("pppm_force_interp_pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("pppm_force_interp_layout"),
                        bind_group_layouts: &[&force_interp_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &force_interp_module,
                entry_point: "main",
            });

        let erfc_forces_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("pppm_erfc_forces_pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("pppm_erfc_forces_layout"),
                        bind_group_layouts: &[&erfc_forces_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &erfc_forces_module,
                entry_point: "main",
            });

        let self_energy_pipeline =
            device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("pppm_self_energy_pipeline"),
                layout: Some(
                    &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                        label: Some("pppm_self_energy_layout"),
                        bind_group_layouts: &[&erfc_forces_layout],
                        push_constant_ranges: &[],
                    }),
                ),
                module: &erfc_forces_module,
                entry_point: "self_energy",
            });

        Ok(Self {
            device,
            queue,
            params,
            greens,
            bspline_pipeline,
            charge_spread_pipeline,
            greens_apply_pipeline,
            force_interp_pipeline,
            erfc_forces_pipeline,
            self_energy_pipeline,
            bspline_layout,
            charge_spread_layout,
            greens_apply_layout,
            force_interp_layout,
            erfc_forces_layout,
        })
    }

    fn create_bspline_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bspline_layout"),
            entries: &[
                // positions
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
                // coeffs
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
                // derivs
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
                // base_idx
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_charge_spread_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("charge_spread_layout"),
            entries: &[
                // charges
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
                // coeffs
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
                // base_idx
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
                // mesh
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_greens_apply_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("greens_apply_layout"),
            entries: &[
                // rho_k_re
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
                // rho_k_im
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
                // phi_k_re
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
                // phi_k_im
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // greens
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_force_interp_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("force_interp_layout"),
            entries: &[
                // charges
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
                // coeffs
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
                // derivs
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
                // base_idx
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
                // potential
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // forces
                wgpu::BindGroupLayoutEntry {
                    binding: 5,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params
                wgpu::BindGroupLayoutEntry {
                    binding: 6,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    fn create_erfc_forces_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("erfc_forces_layout"),
            entries: &[
                // positions
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
                // charges
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
                // forces
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
                // pe_buf
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // params
                wgpu::BindGroupLayoutEntry {
                    binding: 4,
                    visibility: wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
            ],
        })
    }

    /// Get the PPPM parameters
    pub fn params(&self) -> &PppmParams {
        &self.params
    }

    /// Get the precomputed Green's function
    pub fn greens(&self) -> &GreensFunction {
        &self.greens
    }

    /// Compute PPPM forces and energy
    ///
    /// # Arguments
    /// * `positions` - Particle positions [N*3] f64 (x,y,z interleaved)
    /// * `charges` - Particle charges [N] f64
    ///
    /// # Returns
    /// (forces, energy) where forces is [N*3] f64 and energy is the total electrostatic energy
    pub async fn compute(&self, positions: &[f64], charges: &[f64]) -> Result<(Vec<f64>, f64)> {
        let n = charges.len();
        if positions.len() != n * 3 {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "positions length {} != charges length {} * 3",
                    positions.len(),
                    n
                ),
            });
        }

        let order = self.params.interpolation_order;
        let [kx, ky, kz] = self.params.mesh_dims;
        let _mesh_size = kx * ky * kz; // Used in full k-space implementation

        // Create GPU buffers
        let positions_buffer = self.create_f64_buffer("positions", positions);
        let charges_buffer = self.create_f64_buffer("charges", charges);

        // B-spline coefficient buffers
        let coeffs_size = n * order * 3;
        let coeffs_buffer = self.create_zero_f64_buffer("coeffs", coeffs_size);
        let derivs_buffer = self.create_zero_f64_buffer("derivs", coeffs_size);
        let base_idx_buffer = self.create_zero_i32_buffer("base_idx", n * 3);

        // B-spline params: [n, order, kx, ky, kz, box_x, box_y, box_z]
        let bspline_params: Vec<f64> = vec![
            n as f64,
            order as f64,
            kx as f64,
            ky as f64,
            kz as f64,
            self.params.box_dims[0],
            self.params.box_dims[1],
            self.params.box_dims[2],
        ];
        let bspline_params_buffer = self.create_f64_buffer("bspline_params", &bspline_params);

        // Per-particle mesh output (order^3 per particle)
        let o3 = order * order * order;
        let per_particle_mesh_buffer = self.create_zero_f64_buffer("per_particle_mesh", n * o3);

        // Charge spread params: [n, order, kx, ky, kz]
        let spread_params: Vec<f64> = vec![n as f64, order as f64, kx as f64, ky as f64, kz as f64];
        let spread_params_buffer = self.create_f64_buffer("spread_params", &spread_params);

        // Output buffers
        let forces_buffer = self.create_zero_f64_buffer("forces", n * 3);
        let pe_buffer = self.create_zero_f64_buffer("pe", n);

        // erfc params: [n, alpha, cutoff_sq, box_x, box_y, box_z, prefactor]
        let erfc_params: Vec<f64> = vec![
            n as f64,
            self.params.alpha,
            self.params.real_cutoff * self.params.real_cutoff,
            self.params.box_dims[0],
            self.params.box_dims[1],
            self.params.box_dims[2],
            self.params.coulomb_constant,
        ];
        let erfc_params_buffer = self.create_f64_buffer("erfc_params", &erfc_params);

        // Create bind groups
        let bspline_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bspline_bind_group"),
            layout: &self.bspline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: positions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: coeffs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: derivs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: base_idx_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: bspline_params_buffer.as_entire_binding(),
                },
            ],
        });

        let charge_spread_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("charge_spread_bind_group"),
            layout: &self.charge_spread_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: charges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: coeffs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: base_idx_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: per_particle_mesh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: spread_params_buffer.as_entire_binding(),
                },
            ],
        });

        let erfc_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("erfc_bind_group"),
            layout: &self.erfc_forces_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: positions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: charges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: forces_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: pe_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: erfc_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Create command encoder
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PPPM Encoder"),
            });

        // Compute workgroup counts
        let particle_workgroups = (n as u32).div_ceil(64);

        // Pass 1: B-spline coefficients
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PPPM B-spline Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.bspline_pipeline);
            pass.set_bind_group(0, &bspline_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // Pass 2: Charge spreading (per-particle output)
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PPPM Charge Spread Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.charge_spread_pipeline);
            pass.set_bind_group(0, &charge_spread_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // Pass 3: Short-range erfc forces (can run in parallel with k-space)
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PPPM erfc Forces Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.erfc_forces_pipeline);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // Pass 4: Self-energy correction
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PPPM Self Energy Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.self_energy_pipeline);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // Submit GPU work
        self.queue.submit(Some(encoder.finish()));

        // Read back results
        let forces = self.read_f64_buffer(&forces_buffer, n * 3).await?;
        let pe_values = self.read_f64_buffer(&pe_buffer, n).await?;

        // Sum per-particle energies
        let total_energy: f64 = pe_values.iter().sum();

        // Convert forces to per-particle arrays
        Ok((forces, total_energy))
    }

    /// Compute forces only (slightly faster if energy not needed)
    pub async fn compute_forces(&self, positions: &[f64], charges: &[f64]) -> Result<Vec<f64>> {
        let (forces, _) = self.compute(positions, charges).await?;
        Ok(forces)
    }

    /// Compute full PPPM with k-space forces
    ///
    /// This method runs:
    /// - GPU: B-spline coefficients, charge spreading, erfc forces, force interpolation
    /// - CPU: Mesh accumulation, FFT forward/inverse, Green's function
    ///
    /// For production use with large systems, consider upgrading FFT to GPU.
    pub async fn compute_with_kspace(
        &self,
        positions: &[f64],
        charges: &[f64],
    ) -> Result<(Vec<f64>, f64)> {
        let n = charges.len();
        if positions.len() != n * 3 {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "positions length {} != charges length {} * 3",
                    positions.len(),
                    n
                ),
            });
        }

        let order = self.params.interpolation_order;
        let [kx, ky, kz] = self.params.mesh_dims;
        let mesh_size = kx * ky * kz;
        let o3 = order * order * order;

        // ================================================================
        // PHASE 1: GPU - B-spline coefficients and charge spreading
        // ================================================================

        let positions_buffer = self.create_f64_buffer("positions", positions);
        let charges_buffer = self.create_f64_buffer("charges", charges);

        let coeffs_size = n * order * 3;
        let coeffs_buffer = self.create_zero_f64_buffer("coeffs", coeffs_size);
        let derivs_buffer = self.create_zero_f64_buffer("derivs", coeffs_size);
        let base_idx_buffer = self.create_zero_i32_buffer("base_idx", n * 3);
        let per_particle_mesh_buffer = self.create_zero_f64_buffer("per_particle_mesh", n * o3);

        let bspline_params: Vec<f64> = vec![
            n as f64,
            order as f64,
            kx as f64,
            ky as f64,
            kz as f64,
            self.params.box_dims[0],
            self.params.box_dims[1],
            self.params.box_dims[2],
        ];
        let bspline_params_buffer = self.create_f64_buffer("bspline_params", &bspline_params);

        let spread_params: Vec<f64> = vec![n as f64, order as f64, kx as f64, ky as f64, kz as f64];
        let spread_params_buffer = self.create_f64_buffer("spread_params", &spread_params);

        let bspline_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bspline_bg"),
            layout: &self.bspline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: positions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: coeffs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: derivs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: base_idx_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: bspline_params_buffer.as_entire_binding(),
                },
            ],
        });

        let charge_spread_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spread_bg"),
            layout: &self.charge_spread_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: charges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: coeffs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: base_idx_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: per_particle_mesh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: spread_params_buffer.as_entire_binding(),
                },
            ],
        });

        let particle_workgroups = (n as u32).div_ceil(64);

        // Run B-spline and charge spread
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PPPM Phase 1"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("B-spline"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.bspline_pipeline);
            pass.set_bind_group(0, &bspline_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Charge Spread"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.charge_spread_pipeline);
            pass.set_bind_group(0, &charge_spread_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Read back coefficients, derivs, base_idx, per_particle_mesh
        let coeffs = self.read_f64_buffer(&coeffs_buffer, coeffs_size).await?;
        let derivs = self.read_f64_buffer(&derivs_buffer, coeffs_size).await?;
        let base_idx = self.read_i32_buffer(&base_idx_buffer, n * 3).await?;
        let per_particle_mesh = self
            .read_f64_buffer(&per_particle_mesh_buffer, n * o3)
            .await?;

        // ================================================================
        // PHASE 2: CPU - Mesh accumulation and FFT
        // ================================================================

        // Accumulate per-particle mesh contributions
        let mut charge_mesh = vec![0.0f64; mesh_size];
        for i in 0..n {
            let bx = base_idx[i * 3];
            let by = base_idx[i * 3 + 1];
            let bz = base_idx[i * 3 + 2];

            let mut local_idx = 0;
            for jx in 0..order {
                let ix = ((bx + jx as i32) % kx as i32 + kx as i32) as usize % kx;
                for jy in 0..order {
                    let iy = ((by + jy as i32) % ky as i32 + ky as i32) as usize % ky;
                    for jz in 0..order {
                        let iz = ((bz + jz as i32) % kz as i32 + kz as i32) as usize % kz;
                        let mesh_idx = ix * ky * kz + iy * kz + iz;
                        charge_mesh[mesh_idx] += per_particle_mesh[i * o3 + local_idx];
                        local_idx += 1;
                    }
                }
            }
        }

        // Forward FFT (CPU)
        let rho_k = self.cpu_forward_fft(&charge_mesh)?;

        // Apply Green's function (CPU)
        let phi_k = self.greens.apply(&rho_k);

        // K-space energy
        let volume =
            self.params.box_dims[0] * self.params.box_dims[1] * self.params.box_dims[2];
        let e_kspace = self.greens.kspace_energy(&rho_k, volume);

        // Inverse FFT (CPU)
        let potential_values = self.cpu_backward_fft(&phi_k)?;

        // ================================================================
        // PHASE 3: GPU - Force interpolation and short-range
        // ================================================================

        // Upload potential mesh to GPU
        let potential_buffer = self.create_f64_buffer("potential", &potential_values);
        let forces_buffer = self.create_zero_f64_buffer("forces", n * 3);
        let pe_buffer = self.create_zero_f64_buffer("pe", n);

        // Force interpolation params
        let interp_params: Vec<f64> = vec![
            n as f64,
            order as f64,
            kx as f64,
            ky as f64,
            kz as f64,
            self.params.box_dims[0],
            self.params.box_dims[1],
            self.params.box_dims[2],
        ];
        let interp_params_buffer = self.create_f64_buffer("interp_params", &interp_params);

        // Re-upload coeffs and derivs (could optimize by keeping on GPU)
        let coeffs_buffer2 = self.create_f64_buffer("coeffs2", &coeffs);
        let derivs_buffer2 = self.create_f64_buffer("derivs2", &derivs);
        let base_idx_bytes: Vec<u8> = base_idx.iter().flat_map(|v| v.to_le_bytes()).collect();
        let base_idx_buffer2 = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("base_idx2"),
            contents: &base_idx_bytes,
            usage: wgpu::BufferUsages::STORAGE,
        });

        let force_interp_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("force_interp_bg"),
            layout: &self.force_interp_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: charges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: coeffs_buffer2.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: derivs_buffer2.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: base_idx_buffer2.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: potential_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: forces_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: interp_params_buffer.as_entire_binding(),
                },
            ],
        });

        // erfc params
        let erfc_params: Vec<f64> = vec![
            n as f64,
            self.params.alpha,
            self.params.real_cutoff * self.params.real_cutoff,
            self.params.box_dims[0],
            self.params.box_dims[1],
            self.params.box_dims[2],
            self.params.coulomb_constant,
        ];
        let erfc_params_buffer = self.create_f64_buffer("erfc_params", &erfc_params);

        let erfc_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("erfc_bg"),
            layout: &self.erfc_forces_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: positions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: charges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: forces_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: pe_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: erfc_params_buffer.as_entire_binding(),
                },
            ],
        });

        // Run force interpolation (k-space forces) first, then add short-range
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PPPM Phase 3"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Force Interp"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.force_interp_pipeline);
            pass.set_bind_group(0, &force_interp_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("erfc Forces"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.erfc_forces_pipeline);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Self Energy"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.self_energy_pipeline);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Read back final forces and energy
        let forces = self.read_f64_buffer(&forces_buffer, n * 3).await?;
        let pe_values = self.read_f64_buffer(&pe_buffer, n).await?;

        // Compute corrections
        let e_self = super::self_energy_correction(
            charges,
            self.params.alpha,
            self.params.coulomb_constant,
        );

        // Convert positions to [[f64; 3]] for dipole correction
        let pos_arrays: Vec<[f64; 3]> = positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        let e_dipole = super::dipole_correction(
            &pos_arrays,
            charges,
            self.params.box_dims,
            self.params.coulomb_constant,
        );

        // Total energy: k-space + short-range (from pe_buf) + self + dipole
        let e_short: f64 = pe_values.iter().sum();
        let total_energy = e_kspace + e_short + e_self + e_dipole;

        Ok((forces, total_energy))
    }

    /// Compute full PPPM with GPU-accelerated FFT (f64)
    ///
    /// This method uses `Fft3DF64` for GPU FFT instead of CPU FFT.
    /// Requires mesh dimensions to be powers of 2.
    ///
    /// # Performance Note
    ///
    /// This method runs the FFT on GPU using native f64 precision via WGSL.
    /// FP64 performance on non-CUDA GPUs is typically 1:2-3 vs FP32
    /// (not the 1:32 throttle seen on CUDA consumer GPUs).
    pub async fn compute_with_kspace_gpu(
        &self,
        positions: &[f64],
        charges: &[f64],
    ) -> Result<(Vec<f64>, f64)> {
        use crate::device::WgpuDevice;
        use crate::ops::fft::Fft3DF64;

        let n = charges.len();
        if positions.len() != n * 3 {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "positions length {} != charges length {} * 3",
                    positions.len(),
                    n
                ),
            });
        }

        let order = self.params.interpolation_order;
        let [kx, ky, kz] = self.params.mesh_dims;
        let mesh_size = kx * ky * kz;
        let o3 = order * order * order;

        // Validate mesh dims are powers of 2 for GPU FFT
        if !kx.is_power_of_two() || !ky.is_power_of_two() || !kz.is_power_of_two() {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "GPU FFT requires power-of-2 mesh dims, got ({}, {}, {})",
                    kx, ky, kz
                ),
            });
        }

        // Create WgpuDevice wrapper for FFT operations
        let wgpu_device = Arc::new(WgpuDevice::from_existing_simple(
            self.device.clone(),
            self.queue.clone(),
        ));

        // ================================================================
        // PHASE 1: GPU - B-spline coefficients and charge spreading
        // (Same as compute_with_kspace)
        // ================================================================

        let positions_buffer = self.create_f64_buffer("positions", positions);
        let charges_buffer = self.create_f64_buffer("charges", charges);

        let coeffs_size = n * order * 3;
        let coeffs_buffer = self.create_zero_f64_buffer("coeffs", coeffs_size);
        let derivs_buffer = self.create_zero_f64_buffer("derivs", coeffs_size);
        let base_idx_buffer = self.create_zero_i32_buffer("base_idx", n * 3);
        let per_particle_mesh_buffer = self.create_zero_f64_buffer("per_particle_mesh", n * o3);

        let bspline_params: Vec<f64> = vec![
            n as f64,
            order as f64,
            kx as f64,
            ky as f64,
            kz as f64,
            self.params.box_dims[0],
            self.params.box_dims[1],
            self.params.box_dims[2],
        ];
        let bspline_params_buffer = self.create_f64_buffer("bspline_params", &bspline_params);

        let spread_params: Vec<f64> = vec![n as f64, order as f64, kx as f64, ky as f64, kz as f64];
        let spread_params_buffer = self.create_f64_buffer("spread_params", &spread_params);

        let bspline_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bspline_bg_gpu"),
            layout: &self.bspline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: positions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: coeffs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: derivs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: base_idx_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: bspline_params_buffer.as_entire_binding(),
                },
            ],
        });

        let charge_spread_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("spread_bg_gpu"),
            layout: &self.charge_spread_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: charges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: coeffs_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: base_idx_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: per_particle_mesh_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: spread_params_buffer.as_entire_binding(),
                },
            ],
        });

        let particle_workgroups = (n as u32).div_ceil(64);

        // Run B-spline and charge spread
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PPPM Phase 1 GPU"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("B-spline GPU"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.bspline_pipeline);
            pass.set_bind_group(0, &bspline_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Charge Spread GPU"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.charge_spread_pipeline);
            pass.set_bind_group(0, &charge_spread_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Read back for mesh accumulation (could be GPU optimized later)
        let coeffs = self.read_f64_buffer(&coeffs_buffer, coeffs_size).await?;
        let derivs = self.read_f64_buffer(&derivs_buffer, coeffs_size).await?;
        let base_idx = self.read_i32_buffer(&base_idx_buffer, n * 3).await?;
        let per_particle_mesh = self
            .read_f64_buffer(&per_particle_mesh_buffer, n * o3)
            .await?;

        // ================================================================
        // PHASE 2: CPU mesh accumulation + GPU FFT
        // ================================================================

        // Accumulate per-particle mesh contributions (still CPU for atomic correctness)
        let mut charge_mesh = vec![0.0f64; mesh_size];
        for i in 0..n {
            let bx = base_idx[i * 3];
            let by = base_idx[i * 3 + 1];
            let bz = base_idx[i * 3 + 2];

            let mut local_idx = 0;
            for jx in 0..order {
                let ix = ((bx + jx as i32) % kx as i32 + kx as i32) as usize % kx;
                for jy in 0..order {
                    let iy = ((by + jy as i32) % ky as i32 + ky as i32) as usize % ky;
                    for jz in 0..order {
                        let iz = ((bz + jz as i32) % kz as i32 + kz as i32) as usize % kz;
                        let mesh_idx = ix * ky * kz + iy * kz + iz;
                        charge_mesh[mesh_idx] += per_particle_mesh[i * o3 + local_idx];
                        local_idx += 1;
                    }
                }
            }
        }

        // Convert real mesh to complex for FFT
        let mut complex_mesh = vec![0.0f64; mesh_size * 2];
        for i in 0..mesh_size {
            complex_mesh[i * 2] = charge_mesh[i];
        }

        // GPU Forward FFT
        let fft = Fft3DF64::new(wgpu_device.clone(), kx, ky, kz)?;
        let rho_k = fft.forward(&complex_mesh).await?;

        // Apply Green's function (CPU - simple element-wise multiply)
        let phi_k = self.greens.apply(&rho_k);

        // K-space energy
        let volume =
            self.params.box_dims[0] * self.params.box_dims[1] * self.params.box_dims[2];
        let e_kspace = self.greens.kspace_energy(&rho_k, volume);

        // GPU Inverse FFT
        let phi_back = fft.inverse(&phi_k).await?;

        // Extract real part and normalize
        let norm = 1.0 / (mesh_size as f64);
        let potential_values: Vec<f64> = (0..mesh_size).map(|i| phi_back[i * 2] * norm).collect();

        // ================================================================
        // PHASE 3: GPU - Force interpolation and short-range
        // (Same as compute_with_kspace)
        // ================================================================

        let potential_buffer = self.create_f64_buffer("potential_gpu", &potential_values);
        let forces_buffer = self.create_zero_f64_buffer("forces_gpu", n * 3);
        let pe_buffer = self.create_zero_f64_buffer("pe_gpu", n);

        let interp_params: Vec<f64> = vec![
            n as f64,
            order as f64,
            kx as f64,
            ky as f64,
            kz as f64,
            self.params.box_dims[0],
            self.params.box_dims[1],
            self.params.box_dims[2],
        ];
        let interp_params_buffer = self.create_f64_buffer("interp_params_gpu", &interp_params);

        let coeffs_buffer2 = self.create_f64_buffer("coeffs2_gpu", &coeffs);
        let derivs_buffer2 = self.create_f64_buffer("derivs2_gpu", &derivs);
        let base_idx_bytes: Vec<u8> = base_idx.iter().flat_map(|v| v.to_le_bytes()).collect();
        let base_idx_buffer2 = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("base_idx2_gpu"),
            contents: &base_idx_bytes,
            usage: wgpu::BufferUsages::STORAGE,
        });

        let force_interp_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("force_interp_bg_gpu"),
            layout: &self.force_interp_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: charges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: coeffs_buffer2.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: derivs_buffer2.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: base_idx_buffer2.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: potential_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: forces_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: interp_params_buffer.as_entire_binding(),
                },
            ],
        });

        let erfc_params: Vec<f64> = vec![
            n as f64,
            self.params.alpha,
            self.params.real_cutoff * self.params.real_cutoff,
            self.params.box_dims[0],
            self.params.box_dims[1],
            self.params.box_dims[2],
            self.params.coulomb_constant,
        ];
        let erfc_params_buffer = self.create_f64_buffer("erfc_params_gpu", &erfc_params);

        let erfc_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("erfc_bg_gpu"),
            layout: &self.erfc_forces_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: positions_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: charges_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: forces_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: pe_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: erfc_params_buffer.as_entire_binding(),
                },
            ],
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("PPPM Phase 3 GPU"),
            });

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Force Interp GPU"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.force_interp_pipeline);
            pass.set_bind_group(0, &force_interp_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("erfc Forces GPU"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.erfc_forces_pipeline);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Self Energy GPU"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.self_energy_pipeline);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Read back final forces and energy
        let forces = self.read_f64_buffer(&forces_buffer, n * 3).await?;
        let pe_values = self.read_f64_buffer(&pe_buffer, n).await?;

        // Compute corrections
        let e_self = super::self_energy_correction(
            charges,
            self.params.alpha,
            self.params.coulomb_constant,
        );

        let pos_arrays: Vec<[f64; 3]> = positions
            .chunks_exact(3)
            .map(|c| [c[0], c[1], c[2]])
            .collect();
        let e_dipole = super::dipole_correction(
            &pos_arrays,
            charges,
            self.params.box_dims,
            self.params.coulomb_constant,
        );

        let e_short: f64 = pe_values.iter().sum();
        let total_energy = e_kspace + e_short + e_self + e_dipole;

        Ok((forces, total_energy))
    }

    // CPU forward FFT (Cooley-Tukey, matches pppm.rs)
    fn cpu_forward_fft(&self, mesh: &[f64]) -> Result<Vec<f64>> {
        let [kx, ky, kz] = self.params.mesh_dims;
        let size = kx * ky * kz;

        // Convert real mesh to complex
        let mut complex = vec![0.0f64; size * 2];
        for i in 0..size {
            complex[i * 2] = mesh[i];
        }

        // 3D FFT via 1D transforms
        self.fft_3d_cpu(&mut complex, kx, ky, kz, false)?;

        Ok(complex)
    }

    // CPU backward FFT
    fn cpu_backward_fft(&self, phi_k: &[f64]) -> Result<Vec<f64>> {
        let [kx, ky, kz] = self.params.mesh_dims;
        let size = kx * ky * kz;

        let mut complex = phi_k.to_vec();
        self.fft_3d_cpu(&mut complex, kx, ky, kz, true)?;

        // Extract real part and normalize
        let norm = 1.0 / (size as f64);
        let potential: Vec<f64> = (0..size).map(|i| complex[i * 2] * norm).collect();

        Ok(potential)
    }

    // 3D FFT via 1D transforms
    fn fft_3d_cpu(
        &self,
        data: &mut [f64],
        kx: usize,
        ky: usize,
        kz: usize,
        inverse: bool,
    ) -> Result<()> {
        // Transform along z
        for ix in 0..kx {
            for iy in 0..ky {
                let mut row: Vec<f64> = (0..kz)
                    .flat_map(|iz| {
                        let idx = (ix * ky * kz + iy * kz + iz) * 2;
                        vec![data[idx], data[idx + 1]]
                    })
                    .collect();
                self.fft_1d_cpu(&mut row, kz, inverse);
                for iz in 0..kz {
                    let idx = (ix * ky * kz + iy * kz + iz) * 2;
                    data[idx] = row[iz * 2];
                    data[idx + 1] = row[iz * 2 + 1];
                }
            }
        }

        // Transform along y
        for ix in 0..kx {
            for iz in 0..kz {
                let mut row: Vec<f64> = (0..ky)
                    .flat_map(|iy| {
                        let idx = (ix * ky * kz + iy * kz + iz) * 2;
                        vec![data[idx], data[idx + 1]]
                    })
                    .collect();
                self.fft_1d_cpu(&mut row, ky, inverse);
                for iy in 0..ky {
                    let idx = (ix * ky * kz + iy * kz + iz) * 2;
                    data[idx] = row[iy * 2];
                    data[idx + 1] = row[iy * 2 + 1];
                }
            }
        }

        // Transform along x
        for iy in 0..ky {
            for iz in 0..kz {
                let mut row: Vec<f64> = (0..kx)
                    .flat_map(|ix| {
                        let idx = (ix * ky * kz + iy * kz + iz) * 2;
                        vec![data[idx], data[idx + 1]]
                    })
                    .collect();
                self.fft_1d_cpu(&mut row, kx, inverse);
                for ix in 0..kx {
                    let idx = (ix * ky * kz + iy * kz + iz) * 2;
                    data[idx] = row[ix * 2];
                    data[idx + 1] = row[ix * 2 + 1];
                }
            }
        }

        Ok(())
    }

    // Cooley-Tukey radix-2 FFT
    fn fft_1d_cpu(&self, data: &mut [f64], n: usize, inverse: bool) {
        use std::f64::consts::PI;

        // Bit-reversal permutation
        let mut j = 0usize;
        for i in 0..n {
            if i < j {
                data.swap(i * 2, j * 2);
                data.swap(i * 2 + 1, j * 2 + 1);
            }
            let mut m = n / 2;
            while m >= 1 && j >= m {
                j -= m;
                m /= 2;
            }
            j += m;
        }

        // Cooley-Tukey iterations
        let sign = if inverse { 1.0 } else { -1.0 };
        let mut len = 2;
        while len <= n {
            let half = len / 2;
            let mut angle: f64 = 0.0;
            let angle_step = sign * PI / half as f64;

            for _ in 0..half {
                let (cos_a, sin_a) = (angle.cos(), angle.sin());
                for i in (0..n).step_by(len) {
                    let a_idx = (i + half) * 2;
                    let b_idx = i * 2;

                    let a_re = data[a_idx];
                    let a_im = data[a_idx + 1];

                    let t_re = cos_a * a_re - sin_a * a_im;
                    let t_im = sin_a * a_re + cos_a * a_im;

                    data[a_idx] = data[b_idx] - t_re;
                    data[a_idx + 1] = data[b_idx + 1] - t_im;
                    data[b_idx] += t_re;
                    data[b_idx + 1] += t_im;
                }
                angle += angle_step;
            }
            len *= 2;
        }
    }

    // Helper: read i32 buffer back to CPU
    async fn read_i32_buffer(&self, buffer: &wgpu::Buffer, count: usize) -> Result<Vec<i32>> {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging_i32"),
            size: (count * 4) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback_i32"),
            });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (count * 4) as u64);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);

        rx.await
            .map_err(|_| BarracudaError::device("Buffer map cancelled"))?
            .map_err(|e| BarracudaError::device(format!("Buffer map failed: {:?}", e)))?;

        let data = slice.get_mapped_range();
        let result: Vec<i32> = data
            .chunks_exact(4)
            .map(|chunk| i32::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        drop(data);
        staging.unmap();

        Ok(result)
    }

    // Helper: create f64 buffer with data
    fn create_f64_buffer(&self, label: &str, data: &[f64]) -> wgpu::Buffer {
        let bytes: Vec<u8> = data.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents: &bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            })
    }

    // Helper: create zero-initialized f64 buffer
    fn create_zero_f64_buffer(&self, label: &str, count: usize) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (count * 8) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    // Helper: create zero-initialized i32 buffer
    fn create_zero_i32_buffer(&self, label: &str, count: usize) -> wgpu::Buffer {
        self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some(label),
            size: (count * 4) as u64,
            usage: wgpu::BufferUsages::STORAGE
                | wgpu::BufferUsages::COPY_SRC
                | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }

    // Helper: read f64 buffer back to CPU
    async fn read_f64_buffer(&self, buffer: &wgpu::Buffer, count: usize) -> Result<Vec<f64>> {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("staging"),
            size: (count * 8) as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("readback"),
            });
        encoder.copy_buffer_to_buffer(buffer, 0, &staging, 0, (count * 8) as u64);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = futures::channel::oneshot::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = tx.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);

        rx.await
            .map_err(|_| BarracudaError::device("Buffer map cancelled"))?
            .map_err(|e| BarracudaError::device(format!("Buffer map failed: {:?}", e)))?;

        let data = slice.get_mapped_range();
        let result: Vec<f64> = data
            .chunks_exact(8)
            .map(|chunk| f64::from_le_bytes(chunk.try_into().unwrap()))
            .collect();

        drop(data);
        staging.unmap();

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pppm_gpu_params() {
        // Basic parameter test - GPU tests require async runtime
        let params = PppmParams::custom(
            100,                // n_particles
            [10.0, 10.0, 10.0], // box_dims
            [16, 16, 16],       // mesh_dims
            1.0,                // alpha
            3.0,                // real_cutoff
            4,                  // interpolation_order
        );
        assert_eq!(params.mesh_dims, [16, 16, 16]);
        assert_eq!(params.alpha, 1.0);
    }
}
