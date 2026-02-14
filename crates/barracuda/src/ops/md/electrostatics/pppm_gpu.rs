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

use crate::error::Result;
use crate::shaders::precision::ShaderTemplate;

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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pppm_gpu_params() {
        // Basic parameter test - GPU tests require async runtime
        let params = PppmParams::new([10.0, 10.0, 10.0], [16, 16, 16], 1.0, 3.0, 4);
        assert_eq!(params.mesh_dims, [16, 16, 16]);
        assert_eq!(params.alpha, 1.0);
    }
}
