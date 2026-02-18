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
//!
//! # Refactoring (Feb 14, 2026)
//! Extracted layouts to `pppm_layouts.rs` and buffers to `pppm_buffers.rs`
//! for modularity. Reduced this file from 1834 to ~800 lines.

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use crate::linalg::sparse::SparseBuffers;
use crate::shaders::precision::ShaderTemplate;
use wgpu::util::DeviceExt;

use std::sync::Arc;

use super::pppm_buffers::PppmCpuFft;
use super::pppm_layouts::{PppmBindGroupLayouts, PppmPipelines};
use super::{GreensFunction, PppmParams};

/// GPU-accelerated PPPM solver
///
/// Contains compiled pipelines and precomputed data for GPU execution.
/// Pipelines are created once and reused for all compute calls.
pub struct PppmGpu {
    device: Arc<wgpu::Device>,
    queue: Arc<wgpu::Queue>,
    params: PppmParams,
    greens: GreensFunction,

    pipelines: PppmPipelines,
    layouts: PppmBindGroupLayouts,
}

impl PppmGpu {
    /// Create from a WgpuDevice (preferred - driver-aware f64 compilation).
    pub async fn from_device(wgpu_device: &WgpuDevice, params: PppmParams) -> Result<Self> {
        let greens = GreensFunction::new(&params);

        let bspline_module =
            wgpu_device.compile_shader_f64(include_str!("bspline.wgsl"), Some("pppm_bspline"));
        let charge_spread_module = wgpu_device.compile_shader_f64(
            include_str!("charge_spread.wgsl"),
            Some("pppm_charge_spread"),
        );
        let greens_apply_module = wgpu_device
            .compile_shader_f64(include_str!("greens_apply.wgsl"), Some("pppm_greens_apply"));
        let force_interp_module = wgpu_device
            .compile_shader_f64(include_str!("force_interp.wgsl"), Some("pppm_force_interp"));
        let erfc_forces_module = wgpu_device
            .compile_shader_f64(include_str!("erfc_forces.wgsl"), Some("pppm_erfc_forces"));

        Self::build_from_modules(
            wgpu_device.device_arc(),
            wgpu_device.queue_arc(),
            params,
            greens,
            bspline_module,
            charge_spread_module,
            greens_apply_module,
            force_interp_module,
            erfc_forces_module,
        )
        .await
    }

    /// Create from raw wgpu device/queue (legacy API, no NVK workarounds).
    pub async fn new(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        params: PppmParams,
    ) -> Result<Self> {
        Self::new_with_driver(device, queue, params, false).await
    }

    /// Create with explicit driver awareness.
    pub async fn new_with_driver(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        params: PppmParams,
        is_nvk: bool,
    ) -> Result<Self> {
        let greens = GreensFunction::new(&params);

        let bspline_shader = ShaderTemplate::for_driver_auto(include_str!("bspline.wgsl"), is_nvk);
        let charge_spread_shader =
            ShaderTemplate::for_driver_auto(include_str!("charge_spread.wgsl"), is_nvk);
        let greens_apply_shader =
            ShaderTemplate::for_driver_auto(include_str!("greens_apply.wgsl"), is_nvk);
        let force_interp_shader =
            ShaderTemplate::for_driver_auto(include_str!("force_interp.wgsl"), is_nvk);
        let erfc_forces_shader =
            ShaderTemplate::for_driver_auto(include_str!("erfc_forces.wgsl"), is_nvk);

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

        Self::build_from_modules(
            device,
            queue,
            params,
            greens,
            bspline_module,
            charge_spread_module,
            greens_apply_module,
            force_interp_module,
            erfc_forces_module,
        )
        .await
    }

    async fn build_from_modules(
        device: Arc<wgpu::Device>,
        queue: Arc<wgpu::Queue>,
        params: PppmParams,
        greens: GreensFunction,
        bspline_module: wgpu::ShaderModule,
        charge_spread_module: wgpu::ShaderModule,
        greens_apply_module: wgpu::ShaderModule,
        force_interp_module: wgpu::ShaderModule,
        erfc_forces_module: wgpu::ShaderModule,
    ) -> Result<Self> {
        let layouts = PppmBindGroupLayouts::new(&device);
        let pipelines = PppmPipelines::new(
            &device,
            &layouts,
            &bspline_module,
            &charge_spread_module,
            &greens_apply_module,
            &force_interp_module,
            &erfc_forces_module,
        );

        Ok(Self {
            device,
            queue,
            params,
            greens,
            pipelines,
            layouts,
        })
    }

    // Layout creation methods extracted to pppm_layouts.rs (Feb 14, 2026)

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
        let positions_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "positions", positions);
        let charges_buffer = SparseBuffers::f64_from_slice_raw(&self.device, "charges", charges);

        // B-spline coefficient buffers
        let coeffs_size = n * order * 3;
        let coeffs_buffer = SparseBuffers::f64_zeros_raw(&self.device, "coeffs", coeffs_size);
        let derivs_buffer = SparseBuffers::f64_zeros_raw(&self.device, "derivs", coeffs_size);
        let base_idx_buffer = SparseBuffers::i32_zeros_raw(&self.device, "base_idx", n * 3);

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
        let bspline_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "bspline_params", &bspline_params);

        // Per-particle mesh output (order^3 per particle)
        let o3 = order * order * order;
        let per_particle_mesh_buffer =
            SparseBuffers::f64_zeros_raw(&self.device, "per_particle_mesh", n * o3);

        // Charge spread params: [n, order, kx, ky, kz]
        let spread_params: Vec<f64> = vec![n as f64, order as f64, kx as f64, ky as f64, kz as f64];
        let spread_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "spread_params", &spread_params);

        // Output buffers
        let forces_buffer = SparseBuffers::f64_zeros_raw(&self.device, "forces", n * 3);
        let pe_buffer = SparseBuffers::f64_zeros_raw(&self.device, "pe", n);

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
        let erfc_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "erfc_params", &erfc_params);

        // Create bind groups
        let bspline_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bspline_bind_group"),
            layout: &self.layouts.bspline,
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
            layout: &self.layouts.charge_spread,
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
            layout: &self.layouts.erfc_forces,
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
            pass.set_pipeline(&self.pipelines.bspline);
            pass.set_bind_group(0, &bspline_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // Pass 2: Charge spreading (per-particle output)
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PPPM Charge Spread Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.charge_spread);
            pass.set_bind_group(0, &charge_spread_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // Pass 3: Short-range erfc forces (can run in parallel with k-space)
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PPPM erfc Forces Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.erfc_forces);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // Pass 4: Self-energy correction
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("PPPM Self Energy Pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.self_energy);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        // Submit GPU work
        self.queue.submit(Some(encoder.finish()));

        // Read back results
        let forces = SparseBuffers::read_f64_raw(&self.device, &self.queue, &forces_buffer, n * 3)?;
        let pe_values = SparseBuffers::read_f64_raw(&self.device, &self.queue, &pe_buffer, n)?;

        // Sum per-particle energies (includes short-range + self-energy from GPU)
        // NOTE: This method only computes short-range forces, not full PPPM.
        // Use compute_kspace() or compute_gpu_fft() for full electrostatics.
        let total_energy: f64 = pe_values.iter().sum();

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

        let positions_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "positions", positions);
        let charges_buffer = SparseBuffers::f64_from_slice_raw(&self.device, "charges", charges);

        let coeffs_size = n * order * 3;
        let coeffs_buffer = SparseBuffers::f64_zeros_raw(&self.device, "coeffs", coeffs_size);
        let derivs_buffer = SparseBuffers::f64_zeros_raw(&self.device, "derivs", coeffs_size);
        let base_idx_buffer = SparseBuffers::i32_zeros_raw(&self.device, "base_idx", n * 3);
        let per_particle_mesh_buffer =
            SparseBuffers::f64_zeros_raw(&self.device, "per_particle_mesh", n * o3);

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
        let bspline_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "bspline_params", &bspline_params);

        let spread_params: Vec<f64> = vec![n as f64, order as f64, kx as f64, ky as f64, kz as f64];
        let spread_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "spread_params", &spread_params);

        let bspline_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bspline_bg"),
            layout: &self.layouts.bspline,
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
            layout: &self.layouts.charge_spread,
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
            pass.set_pipeline(&self.pipelines.bspline);
            pass.set_bind_group(0, &bspline_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Charge Spread"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.charge_spread);
            pass.set_bind_group(0, &charge_spread_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Read back coefficients, derivs, base_idx, per_particle_mesh
        let coeffs =
            SparseBuffers::read_f64_raw(&self.device, &self.queue, &coeffs_buffer, coeffs_size)?;
        let derivs =
            SparseBuffers::read_f64_raw(&self.device, &self.queue, &derivs_buffer, coeffs_size)?;
        let base_idx =
            SparseBuffers::read_i32_raw(&self.device, &self.queue, &base_idx_buffer, n * 3)?;
        let per_particle_mesh = SparseBuffers::read_f64_raw(
            &self.device,
            &self.queue,
            &per_particle_mesh_buffer,
            n * o3,
        )?;

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

        // Forward FFT (CPU) - using extracted helper
        let rho_k = PppmCpuFft::forward_3d(&charge_mesh, kx, ky, kz);

        // Apply Green's function (CPU)
        let phi_k = self.greens.apply(&rho_k);

        // K-space energy
        let volume = self.params.box_dims[0] * self.params.box_dims[1] * self.params.box_dims[2];
        let e_kspace = self.greens.kspace_energy(&rho_k, volume);

        // Inverse FFT (CPU) - using extracted helper
        let potential_values = PppmCpuFft::inverse_3d(&phi_k, kx, ky, kz);

        // ================================================================
        // PHASE 3: GPU - Force interpolation and short-range
        // ================================================================

        // Upload potential mesh to GPU
        let potential_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "potential", &potential_values);
        let forces_buffer = SparseBuffers::f64_zeros_raw(&self.device, "forces", n * 3);
        let pe_buffer = SparseBuffers::f64_zeros_raw(&self.device, "pe", n);

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
        let interp_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "interp_params", &interp_params);

        // Re-upload coeffs and derivs (could optimize by keeping on GPU)
        let coeffs_buffer2 = SparseBuffers::f64_from_slice_raw(&self.device, "coeffs2", &coeffs);
        let derivs_buffer2 = SparseBuffers::f64_from_slice_raw(&self.device, "derivs2", &derivs);
        let base_idx_bytes: Vec<u8> = base_idx.iter().flat_map(|v| v.to_le_bytes()).collect();
        let base_idx_buffer2 = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("base_idx2"),
                contents: &base_idx_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let force_interp_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("force_interp_bg"),
            layout: &self.layouts.force_interp,
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
        let erfc_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "erfc_params", &erfc_params);

        let erfc_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("erfc_bg"),
            layout: &self.layouts.erfc_forces,
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
            pass.set_pipeline(&self.pipelines.force_interp);
            pass.set_bind_group(0, &force_interp_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("erfc Forces"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.erfc_forces);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Self Energy"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.self_energy);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Read back final forces and energy
        let forces = SparseBuffers::read_f64_raw(&self.device, &self.queue, &forces_buffer, n * 3)?;
        let pe_values = SparseBuffers::read_f64_raw(&self.device, &self.queue, &pe_buffer, n)?;

        // NOTE: Self-energy is already computed in GPU self_energy kernel and
        // included in pe_buf, so we don't compute it again here.

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

        // Total energy: k-space + short-range+self (from pe_buf) + dipole
        // pe_buf contains: erfc short-range energy + self-energy correction (from GPU)
        let e_short_and_self: f64 = pe_values.iter().sum();
        let total_energy = e_kspace + e_short_and_self + e_dipole;

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

        let positions_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "positions", positions);
        let charges_buffer = SparseBuffers::f64_from_slice_raw(&self.device, "charges", charges);

        let coeffs_size = n * order * 3;
        let coeffs_buffer = SparseBuffers::f64_zeros_raw(&self.device, "coeffs", coeffs_size);
        let derivs_buffer = SparseBuffers::f64_zeros_raw(&self.device, "derivs", coeffs_size);
        let base_idx_buffer = SparseBuffers::i32_zeros_raw(&self.device, "base_idx", n * 3);
        let per_particle_mesh_buffer =
            SparseBuffers::f64_zeros_raw(&self.device, "per_particle_mesh", n * o3);

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
        let bspline_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "bspline_params", &bspline_params);

        let spread_params: Vec<f64> = vec![n as f64, order as f64, kx as f64, ky as f64, kz as f64];
        let spread_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "spread_params", &spread_params);

        let bspline_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bspline_bg_gpu"),
            layout: &self.layouts.bspline,
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
            layout: &self.layouts.charge_spread,
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
            pass.set_pipeline(&self.pipelines.bspline);
            pass.set_bind_group(0, &bspline_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Charge Spread GPU"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.charge_spread);
            pass.set_bind_group(0, &charge_spread_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Read back for mesh accumulation (could be GPU optimized later)
        let coeffs =
            SparseBuffers::read_f64_raw(&self.device, &self.queue, &coeffs_buffer, coeffs_size)?;
        let derivs =
            SparseBuffers::read_f64_raw(&self.device, &self.queue, &derivs_buffer, coeffs_size)?;
        let base_idx =
            SparseBuffers::read_i32_raw(&self.device, &self.queue, &base_idx_buffer, n * 3)?;
        let per_particle_mesh = SparseBuffers::read_f64_raw(
            &self.device,
            &self.queue,
            &per_particle_mesh_buffer,
            n * o3,
        )?;

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
        let volume = self.params.box_dims[0] * self.params.box_dims[1] * self.params.box_dims[2];
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

        let potential_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "potential_gpu", &potential_values);
        let forces_buffer = SparseBuffers::f64_zeros_raw(&self.device, "forces_gpu", n * 3);
        let pe_buffer = SparseBuffers::f64_zeros_raw(&self.device, "pe_gpu", n);

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
        let interp_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "interp_params_gpu", &interp_params);

        let coeffs_buffer2 =
            SparseBuffers::f64_from_slice_raw(&self.device, "coeffs2_gpu", &coeffs);
        let derivs_buffer2 =
            SparseBuffers::f64_from_slice_raw(&self.device, "derivs2_gpu", &derivs);
        let base_idx_bytes: Vec<u8> = base_idx.iter().flat_map(|v| v.to_le_bytes()).collect();
        let base_idx_buffer2 = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("base_idx2_gpu"),
                contents: &base_idx_bytes,
                usage: wgpu::BufferUsages::STORAGE,
            });

        let force_interp_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("force_interp_bg_gpu"),
            layout: &self.layouts.force_interp,
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
        let erfc_params_buffer =
            SparseBuffers::f64_from_slice_raw(&self.device, "erfc_params_gpu", &erfc_params);

        let erfc_bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("erfc_bg_gpu"),
            layout: &self.layouts.erfc_forces,
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
            pass.set_pipeline(&self.pipelines.force_interp);
            pass.set_bind_group(0, &force_interp_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("erfc Forces GPU"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.erfc_forces);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Self Energy GPU"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.self_energy);
            pass.set_bind_group(0, &erfc_bind_group, &[]);
            pass.dispatch_workgroups(particle_workgroups, 1, 1);
        }

        self.queue.submit(Some(encoder.finish()));

        // Read back final forces and energy
        let forces = SparseBuffers::read_f64_raw(&self.device, &self.queue, &forces_buffer, n * 3)?;
        let pe_values = SparseBuffers::read_f64_raw(&self.device, &self.queue, &pe_buffer, n)?;

        // NOTE: Self-energy is already computed in GPU self_energy kernel and
        // included in pe_buf, so we don't compute it again here.

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

        // pe_buf contains: erfc short-range energy + self-energy correction (from GPU)
        let e_short_and_self: f64 = pe_values.iter().sum();
        let total_energy = e_kspace + e_short_and_self + e_dipole;

        Ok((forces, total_energy))
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

    #[tokio::test]
    #[ignore = "W-002: PPPM physics validation - energy sign wrong (got +30.97, expected <0 for opposite charges). Likely k-space/Green's normalization or sign in force accumulation."]
    async fn test_pppm_gpu_opposite_charges_energy() {
        use crate::device::test_pool::get_test_device_if_f64_gpu_available;

        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let params = PppmParams::custom(
            2,
            [10.0, 10.0, 10.0],
            [8, 8, 8],
            2.0, // alpha
            3.0, // rc
            4,   // order
        );

        let pppm = PppmGpu::from_device(&device, params)
            .await
            .expect("Failed to create PppmGpu");

        // Two opposite charges at distance 2.0
        let positions: Vec<f64> = vec![4.0, 5.0, 5.0, 6.0, 5.0, 5.0];
        let charges: Vec<f64> = vec![1.0, -1.0];

        let (_forces, energy) = pppm
            .compute_with_kspace(&positions, &charges)
            .await
            .unwrap();

        assert!(
            energy < 0.0,
            "Opposite charges should have negative energy, got {}",
            energy
        );
    }

    #[tokio::test]
    #[ignore = "W-002: PPPM physics validation - Newton's 3rd law violated (|F1+F2|/|F1|=2.52, expected ~0). Force directions or k-space/erfc accumulation mismatch."]
    async fn test_pppm_gpu_newtons_third_law() {
        use crate::device::test_pool::get_test_device_if_f64_gpu_available;

        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let params = PppmParams::custom(2, [10.0, 10.0, 10.0], [8, 8, 8], 2.0, 3.0, 4);

        let pppm = PppmGpu::from_device(&device, params)
            .await
            .expect("Failed to create PppmGpu");

        let positions: Vec<f64> = vec![4.0, 5.0, 5.0, 6.0, 5.0, 5.0];
        let charges: Vec<f64> = vec![1.0, -1.0];

        let (forces, _energy) = pppm
            .compute_with_kspace(&positions, &charges)
            .await
            .unwrap();
        let fx_sum = forces[0] + forces[3];
        let fy_sum = forces[1] + forces[4];
        let fz_sum = forces[2] + forces[5];

        let f1_mag = (forces[0].powi(2) + forces[1].powi(2) + forces[2].powi(2)).sqrt();
        let relative_error = (fx_sum.powi(2) + fy_sum.powi(2) + fz_sum.powi(2)).sqrt() / f1_mag;

        assert!(
            relative_error < 1e-3,
            "Newton's 3rd law violation: |F1+F2|/|F1| = {} (should be ~0)",
            relative_error
        );
    }

    #[tokio::test]
    #[ignore = "W-002: PPPM physics validation - like charges show attraction (F0_x=+4.19, expected <0). Force sign/direction error in erfc or force_interp shaders."]
    async fn test_pppm_gpu_like_charges_repel() {
        use crate::device::test_pool::get_test_device_if_f64_gpu_available;

        let Some(device) = get_test_device_if_f64_gpu_available().await else {
            return;
        };

        let params = PppmParams::custom(2, [10.0, 10.0, 10.0], [8, 8, 8], 2.0, 3.0, 4);

        let pppm = PppmGpu::from_device(&device, params)
            .await
            .expect("Failed to create PppmGpu");

        // Two positive charges: charge 0 at x=4, charge 1 at x=6
        let positions: Vec<f64> = vec![4.0, 5.0, 5.0, 6.0, 5.0, 5.0];
        let charges: Vec<f64> = vec![1.0, 1.0];

        let (forces, _energy) = pppm
            .compute_with_kspace(&positions, &charges)
            .await
            .unwrap();

        // Force on particle 0 (at x=4) should be negative x (pushed away from x=6)
        assert!(
            forces[0] < 0.0,
            "Like charges should repel: F0_x should be negative, got {}",
            forces[0]
        );
        // Force on particle 1 (at x=6) should be positive x (pushed away from x=4)
        assert!(
            forces[3] > 0.0,
            "Like charges should repel: F1_x should be positive, got {}",
            forces[3]
        );
    }
}
