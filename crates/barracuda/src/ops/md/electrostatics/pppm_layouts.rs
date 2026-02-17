//! PPPM bind group layouts and pipeline creation
//!
//! Extracted from pppm_gpu.rs for modularity (Feb 14, 2026).
//! Contains all bind group layout definitions for PPPM compute pipelines.

use std::sync::Arc;

/// PPPM bind group layouts for GPU compute pipelines
pub struct PppmLayouts;

impl PppmLayouts {
    /// Read-only storage buffer layout entry
    fn storage_ro(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    /// Read-write storage buffer layout entry
    fn storage_rw(binding: u32) -> wgpu::BindGroupLayoutEntry {
        wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::COMPUTE,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: false },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }
    }

    /// B-spline coefficient computation layout
    ///
    /// Bindings:
    /// - 0: positions (read)
    /// - 1: coeffs (write)
    /// - 2: derivs (write)
    /// - 3: base_idx (write)
    /// - 4: params (read)
    pub fn bspline(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pppm_bspline_bgl"),
            entries: &[
                Self::storage_ro(0), // positions
                Self::storage_rw(1), // coeffs
                Self::storage_rw(2), // derivs
                Self::storage_rw(3), // base_idx
                Self::storage_ro(4), // params
            ],
        })
    }

    /// Charge spreading layout
    ///
    /// Bindings:
    /// - 0: charges (read)
    /// - 1: coeffs (read)
    /// - 2: base_idx (read)
    /// - 3: mesh (write)
    /// - 4: params (read)
    pub fn charge_spread(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pppm_charge_spread_bgl"),
            entries: &[
                Self::storage_ro(0), // charges
                Self::storage_ro(1), // coeffs
                Self::storage_ro(2), // base_idx
                Self::storage_rw(3), // mesh
                Self::storage_ro(4), // params
            ],
        })
    }

    /// Green's function application layout
    ///
    /// Bindings:
    /// - 0: rho_k_re (read)
    /// - 1: rho_k_im (read)
    /// - 2: phi_k_re (write)
    /// - 3: phi_k_im (write)
    /// - 4: greens (read)
    /// - 5: params (read)
    pub fn greens_apply(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pppm_greens_apply_bgl"),
            entries: &[
                Self::storage_ro(0), // rho_k_re
                Self::storage_ro(1), // rho_k_im
                Self::storage_rw(2), // phi_k_re
                Self::storage_rw(3), // phi_k_im
                Self::storage_ro(4), // greens
                Self::storage_ro(5), // params
            ],
        })
    }

    /// Force interpolation layout
    ///
    /// Bindings:
    /// - 0: charges (read)
    /// - 1: coeffs (read)
    /// - 2: derivs (read)
    /// - 3: base_idx (read)
    /// - 4: potential (read)
    /// - 5: forces (write)
    /// - 6: params (read)
    pub fn force_interp(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pppm_force_interp_bgl"),
            entries: &[
                Self::storage_ro(0), // charges
                Self::storage_ro(1), // coeffs
                Self::storage_ro(2), // derivs
                Self::storage_ro(3), // base_idx
                Self::storage_ro(4), // potential
                Self::storage_rw(5), // forces
                Self::storage_ro(6), // params
            ],
        })
    }

    /// Short-range erfc forces layout
    ///
    /// Bindings:
    /// - 0: positions (read)
    /// - 1: charges (read)
    /// - 2: forces (write)
    /// - 3: pe_buf (write)
    /// - 4: params (read)
    pub fn erfc_forces(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("pppm_erfc_forces_bgl"),
            entries: &[
                Self::storage_ro(0), // positions
                Self::storage_ro(1), // charges
                Self::storage_rw(2), // forces
                Self::storage_rw(3), // pe_buf
                Self::storage_ro(4), // params
            ],
        })
    }
}

/// PPPM compute pipeline collection
pub struct PppmPipelines {
    pub bspline: wgpu::ComputePipeline,
    pub charge_spread: wgpu::ComputePipeline,
    pub greens_apply: wgpu::ComputePipeline,
    pub force_interp: wgpu::ComputePipeline,
    pub erfc_forces: wgpu::ComputePipeline,
    pub self_energy: wgpu::ComputePipeline,
}

/// PPPM bind group layout collection
pub struct PppmBindGroupLayouts {
    pub bspline: wgpu::BindGroupLayout,
    pub charge_spread: wgpu::BindGroupLayout,
    pub greens_apply: wgpu::BindGroupLayout,
    pub force_interp: wgpu::BindGroupLayout,
    pub erfc_forces: wgpu::BindGroupLayout,
}

impl PppmBindGroupLayouts {
    /// Create all PPPM bind group layouts
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            bspline: PppmLayouts::bspline(device),
            charge_spread: PppmLayouts::charge_spread(device),
            greens_apply: PppmLayouts::greens_apply(device),
            force_interp: PppmLayouts::force_interp(device),
            erfc_forces: PppmLayouts::erfc_forces(device),
        }
    }
}

impl PppmPipelines {
    /// Create all PPPM compute pipelines from shader modules and layouts
    pub fn new(
        device: &Arc<wgpu::Device>,
        layouts: &PppmBindGroupLayouts,
        bspline_module: &wgpu::ShaderModule,
        charge_spread_module: &wgpu::ShaderModule,
        greens_apply_module: &wgpu::ShaderModule,
        force_interp_module: &wgpu::ShaderModule,
        erfc_forces_module: &wgpu::ShaderModule,
    ) -> Self {
        let bspline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pppm_bspline_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("pppm_bspline_layout"),
                    bind_group_layouts: &[&layouts.bspline],
                    push_constant_ranges: &[],
                }),
            ),
            module: bspline_module,
            entry_point: "main",
        cache: None,
        compilation_options: Default::default(),
        });

        let charge_spread = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pppm_charge_spread_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("pppm_charge_spread_layout"),
                    bind_group_layouts: &[&layouts.charge_spread],
                    push_constant_ranges: &[],
                }),
            ),
            module: charge_spread_module,
            entry_point: "spread_per_particle",
        cache: None,
        compilation_options: Default::default(),
        });

        let greens_apply = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pppm_greens_apply_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("pppm_greens_apply_layout"),
                    bind_group_layouts: &[&layouts.greens_apply],
                    push_constant_ranges: &[],
                }),
            ),
            module: greens_apply_module,
            entry_point: "main",
        cache: None,
        compilation_options: Default::default(),
        });

        let force_interp = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pppm_force_interp_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("pppm_force_interp_layout"),
                    bind_group_layouts: &[&layouts.force_interp],
                    push_constant_ranges: &[],
                }),
            ),
            module: force_interp_module,
            entry_point: "main",
        cache: None,
        compilation_options: Default::default(),
        });

        let erfc_forces = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pppm_erfc_forces_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("pppm_erfc_forces_layout"),
                    bind_group_layouts: &[&layouts.erfc_forces],
                    push_constant_ranges: &[],
                }),
            ),
            module: erfc_forces_module,
            entry_point: "main",
        cache: None,
        compilation_options: Default::default(),
        });

        // Self-energy uses same layout as erfc_forces
        let self_energy = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("pppm_self_energy_pipeline"),
            layout: Some(
                &device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("pppm_self_energy_layout"),
                    bind_group_layouts: &[&layouts.erfc_forces],
                    push_constant_ranges: &[],
                }),
            ),
            module: erfc_forces_module,
            entry_point: "self_energy",
        cache: None,
        compilation_options: Default::default(),
        });

        Self {
            bspline,
            charge_spread,
            greens_apply,
            force_interp,
            erfc_forces,
            self_energy,
        }
    }
}
