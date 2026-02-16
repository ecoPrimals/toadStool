//! Spin-Orbit Coupling (f64) — GPU-Accelerated HFB Term
//!
//! Computes diagonal spin-orbit corrections to the HFB Hamiltonian.
//!
//! **Evolution**: Feb 16, 2026 — TIER 2.1 from hotSpring handoff
//!
//! This was previously computed on CPU after GPU H-build readback.
//! Moving it to GPU eliminates the last CPU physics in the H-build path.
//!
//! ## Formula
//!
//! ```text
//! H_so[i,i] += w0 · ls_i · ∫ |ψ_i(r)|² · (dρ/dr) · r · dr
//! ```
//!
//! where `ls_i = (j(j+1) - l(l+1) - 3/4) / 2` is the spin-orbit factor.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let so = SpinOrbitGpu::new(device, batch_size, n_states, n_grid)?;
//!
//! // Method 1: Pre-computed gradient
//! let h_so = so.compute(&wf_squared, &drho_dr, &r_grid, &ls_factors, dr, w0)?;
//!
//! // Method 2: Compute gradient internally from density
//! let h_so = so.compute_with_density(&wf_squared, &density, &r_grid, &ls_factors, dr, w0)?;
//! ```

use crate::device::WgpuDevice;
use crate::error::{BarracudaError, Result};
use bytemuck::{Pod, Zeroable};
use std::sync::Arc;
use wgpu::util::DeviceExt;

/// Parameters for spin-orbit shader
#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct SpinOrbitParams {
    batch_size: u32,
    n_states: u32,
    n_grid: u32,
    _pad: u32,
    dr_lo: u32,
    dr_hi: u32,
    w0_lo: u32,
    w0_hi: u32,
}

impl SpinOrbitParams {
    fn new(batch_size: u32, n_states: u32, n_grid: u32, dr: f64, w0: f64) -> Self {
        let dr_bits = dr.to_bits();
        let w0_bits = w0.to_bits();
        Self {
            batch_size,
            n_states,
            n_grid,
            _pad: 0,
            dr_lo: dr_bits as u32,
            dr_hi: (dr_bits >> 32) as u32,
            w0_lo: w0_bits as u32,
            w0_hi: (w0_bits >> 32) as u32,
        }
    }
}

/// GPU-accelerated spin-orbit coupling computation
pub struct SpinOrbitGpu {
    device: Arc<WgpuDevice>,
}

impl SpinOrbitGpu {
    /// Create a new spin-orbit GPU operator
    pub fn new(device: Arc<WgpuDevice>) -> Self {
        Self { device }
    }

    fn wgsl_shader() -> &'static str {
        include_str!("../../shaders/grid/spin_orbit_f64.wgsl")
    }

    /// Compute spin-orbit diagonal corrections with pre-computed gradient
    ///
    /// # Arguments
    /// * `wf_squared` - Squared wavefunctions [batch × n_states × n_grid]
    /// * `drho_dr` - Density gradient [batch × n_grid]
    /// * `r_grid` - Radial grid points [n_grid]
    /// * `ls_factors` - Spin-orbit factors ls_i [batch × n_states]
    /// * `dr` - Grid spacing
    /// * `w0` - Spin-orbit coupling strength (MeV·fm⁵)
    ///
    /// # Returns
    /// Diagonal corrections h_so[i,i] for each (batch, state) pair [batch × n_states]
    pub fn compute(
        &self,
        wf_squared: &[f64],
        drho_dr: &[f64],
        r_grid: &[f64],
        ls_factors: &[f64],
        dr: f64,
        w0: f64,
    ) -> Result<Vec<f64>> {
        let n_grid = r_grid.len();
        if n_grid == 0 {
            return Err(BarracudaError::InvalidInput {
                message: "r_grid cannot be empty".to_string(),
            });
        }

        // Infer batch_size and n_states from ls_factors and drho_dr
        let batch_size = drho_dr.len() / n_grid;
        if drho_dr.len() != batch_size * n_grid {
            return Err(BarracudaError::InvalidInput {
                message: "drho_dr length must be batch_size × n_grid".to_string(),
            });
        }

        let n_states = ls_factors.len() / batch_size;
        if ls_factors.len() != batch_size * n_states {
            return Err(BarracudaError::InvalidInput {
                message: "ls_factors length must be batch_size × n_states".to_string(),
            });
        }

        if wf_squared.len() != batch_size * n_states * n_grid {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "wf_squared length {} must be batch({}) × n_states({}) × n_grid({})",
                    wf_squared.len(),
                    batch_size,
                    n_states,
                    n_grid
                ),
            });
        }

        self.compute_internal(
            wf_squared,
            Some(drho_dr),
            None,
            r_grid,
            ls_factors,
            batch_size,
            n_states,
            n_grid,
            dr,
            w0,
            "spin_orbit_diagonal",
        )
    }

    /// Compute spin-orbit corrections with internal gradient computation
    ///
    /// This version takes density and computes the gradient internally,
    /// which is more efficient when density is already on GPU.
    pub fn compute_with_density(
        &self,
        wf_squared: &[f64],
        density: &[f64],
        r_grid: &[f64],
        ls_factors: &[f64],
        dr: f64,
        w0: f64,
    ) -> Result<Vec<f64>> {
        let n_grid = r_grid.len();
        if n_grid == 0 {
            return Err(BarracudaError::InvalidInput {
                message: "r_grid cannot be empty".to_string(),
            });
        }

        let batch_size = density.len() / n_grid;
        if density.len() != batch_size * n_grid {
            return Err(BarracudaError::InvalidInput {
                message: "density length must be batch_size × n_grid".to_string(),
            });
        }

        let n_states = ls_factors.len() / batch_size;
        if ls_factors.len() != batch_size * n_states {
            return Err(BarracudaError::InvalidInput {
                message: "ls_factors length must be batch_size × n_states".to_string(),
            });
        }

        if wf_squared.len() != batch_size * n_states * n_grid {
            return Err(BarracudaError::InvalidInput {
                message: format!(
                    "wf_squared length {} must be batch({}) × n_states({}) × n_grid({})",
                    wf_squared.len(),
                    batch_size,
                    n_states,
                    n_grid
                ),
            });
        }

        self.compute_internal(
            wf_squared,
            None,
            Some(density),
            r_grid,
            ls_factors,
            batch_size,
            n_states,
            n_grid,
            dr,
            w0,
            "spin_orbit_with_gradient",
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_internal(
        &self,
        wf_squared: &[f64],
        drho_dr: Option<&[f64]>,
        density: Option<&[f64]>,
        r_grid: &[f64],
        ls_factors: &[f64],
        batch_size: usize,
        n_states: usize,
        n_grid: usize,
        dr: f64,
        w0: f64,
        entry_point: &str,
    ) -> Result<Vec<f64>> {
        let shader = self
            .device
            .compile_shader(Self::wgsl_shader(), Some("SpinOrbit f64"));

        // Create bind group layout
        let mut entries = vec![
            // params
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
            // wf_squared
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
            // drho_dr
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
            // r_grid
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
            // ls_factors
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
            // h_so_diag output
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
        ];

        // Add density binding if using spin_orbit_with_gradient
        if density.is_some() {
            entries.push(wgpu::BindGroupLayoutEntry {
                binding: 6,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            });
        }

        let bgl = self
            .device
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("SpinOrbit BGL"),
                entries: &entries,
            });

        let pl = self
            .device
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("SpinOrbit PL"),
                bind_group_layouts: &[&bgl],
                push_constant_ranges: &[],
            });

        let pipeline =
            self.device
                .device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("SpinOrbit Pipeline"),
                    layout: Some(&pl),
                    module: &shader,
                    entry_point,
                });

        // Create buffers
        let params =
            SpinOrbitParams::new(batch_size as u32, n_states as u32, n_grid as u32, dr, w0);
        let params_buffer = self
            .device
            .create_uniform_buffer("SpinOrbit Params", &params);

        let wf_buffer = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SpinOrbit wf_squared"),
                contents: bytemuck::cast_slice(wf_squared),
                usage: wgpu::BufferUsages::STORAGE,
            });

        // drho_dr or dummy buffer
        let drho_buffer = if let Some(drho) = drho_dr {
            self.device
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("SpinOrbit drho_dr"),
                    contents: bytemuck::cast_slice(drho),
                    usage: wgpu::BufferUsages::STORAGE,
                })
        } else {
            // Dummy buffer for unused binding
            self.device.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("SpinOrbit drho_dr (dummy)"),
                size: 8,
                usage: wgpu::BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };

        let r_buffer = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SpinOrbit r_grid"),
                contents: bytemuck::cast_slice(r_grid),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let ls_buffer = self
            .device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("SpinOrbit ls_factors"),
                contents: bytemuck::cast_slice(ls_factors),
                usage: wgpu::BufferUsages::STORAGE,
            });

        let output_size = (batch_size * n_states * 8) as u64;
        let output_buffer = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SpinOrbit h_so_diag"),
            size: output_size,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        // Build bind group entries
        let mut bg_entries = vec![
            wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wf_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: drho_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: r_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: ls_buffer.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 5,
                resource: output_buffer.as_entire_binding(),
            },
        ];

        // Add density buffer if needed
        let density_buffer;
        if let Some(dens) = density {
            density_buffer =
                self.device
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("SpinOrbit density"),
                        contents: bytemuck::cast_slice(dens),
                        usage: wgpu::BufferUsages::STORAGE,
                    });
            bg_entries.push(wgpu::BindGroupEntry {
                binding: 6,
                resource: density_buffer.as_entire_binding(),
            });
        }

        let bg = self
            .device
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("SpinOrbit BG"),
                layout: &bgl,
                entries: &bg_entries,
            });

        // Execute
        let n_threads = batch_size * n_states;
        let n_workgroups = n_threads.div_ceil(64);
        {
            let mut encoder =
                self.device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("SpinOrbit Encoder"),
                    });
            {
                let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                    label: Some("SpinOrbit Pass"),
                    timestamp_writes: None,
                });
                pass.set_pipeline(&pipeline);
                pass.set_bind_group(0, &bg, &[]);
                pass.dispatch_workgroups(n_workgroups as u32, 1, 1);
            }
            self.device.queue.submit(Some(encoder.finish()));
        }

        // Read back results
        self.read_f64_buffer(&output_buffer, batch_size * n_states)
    }

    fn read_f64_buffer(&self, buffer: &wgpu::Buffer, count: usize) -> Result<Vec<f64>> {
        let staging_buffer = self.device.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("SpinOrbit Staging"),
            size: (count * 8) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        {
            let mut encoder =
                self.device
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("SpinOrbit Copy"),
                    });
            encoder.copy_buffer_to_buffer(buffer, 0, &staging_buffer, 0, (count * 8) as u64);
            self.device.queue.submit(Some(encoder.finish()));
        }

        let slice = staging_buffer.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            // Use .ok() instead of .unwrap() - receiver may be dropped on early return
            let _ = tx.send(result);
        });
        self.device.device.poll(wgpu::Maintain::Wait);

        rx.recv()
            .map_err(|e| BarracudaError::Device(format!("Buffer mapping channel closed: {}", e)))?
            .map_err(|e| BarracudaError::Device(format!("Buffer map error: {:?}", e)))?;

        let data = slice.get_mapped_range();
        let result: Vec<f64> = bytemuck::cast_slice(&data).to_vec();
        drop(data);
        staging_buffer.unmap();

        Ok(result)
    }
}

/// Compute ls_i factor for a state with quantum numbers (l, j)
///
/// Formula: ls_i = (j(j+1) - l(l+1) - 3/4) / 2
///
/// # Arguments
/// * `l` - Orbital angular momentum quantum number
/// * `j` - Total angular momentum quantum number (l±1/2)
///
/// # Example
/// ```rust,ignore
/// let ls = compute_ls_factor(1, 1.5);  // p3/2 state
/// let ls = compute_ls_factor(1, 0.5);  // p1/2 state
/// ```
pub fn compute_ls_factor(l: u32, j: f64) -> f64 {
    let l_f = l as f64;
    (j * (j + 1.0) - l_f * (l_f + 1.0) - 0.75) / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ls_factor_computation() {
        // p1/2: l=1, j=1/2 -> ls = (0.75 - 2 - 0.75)/2 = -1
        let ls_p1_2 = compute_ls_factor(1, 0.5);
        assert!((ls_p1_2 - (-1.0)).abs() < 1e-10);

        // p3/2: l=1, j=3/2 -> ls = (3.75 - 2 - 0.75)/2 = 0.5
        let ls_p3_2 = compute_ls_factor(1, 1.5);
        assert!((ls_p3_2 - 0.5).abs() < 1e-10);

        // d3/2: l=2, j=3/2 -> ls = (3.75 - 6 - 0.75)/2 = -1.5
        let ls_d3_2 = compute_ls_factor(2, 1.5);
        assert!((ls_d3_2 - (-1.5)).abs() < 1e-10);

        // d5/2: l=2, j=5/2 -> ls = (8.75 - 6 - 0.75)/2 = 1.0
        let ls_d5_2 = compute_ls_factor(2, 2.5);
        assert!((ls_d5_2 - 1.0).abs() < 1e-10);

        // s1/2: l=0, j=1/2 -> ls = (0.75 - 0 - 0.75)/2 = 0
        let ls_s1_2 = compute_ls_factor(0, 0.5);
        assert!(ls_s1_2.abs() < 1e-10);
    }

    #[tokio::test]
    async fn test_spin_orbit_basic() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        let so = SpinOrbitGpu::new(device);

        // Simple test: 1 nucleus, 2 states, 5 grid points
        let n_grid = 5;
        let n_states = 2;
        let batch_size = 1;
        let dr = 0.5;
        let w0 = 120.0; // Typical Skyrme w0 in MeV·fm⁵

        // r_grid from dr to 5*dr
        let r_grid: Vec<f64> = (0..n_grid).map(|i| (i + 1) as f64 * dr).collect();

        // Simple wavefunction squared (normalized approximately)
        let mut wf_squared = vec![0.0; batch_size * n_states * n_grid];
        for i in 0..n_grid {
            let r = r_grid[i];
            // State 0: peaked at r=1
            wf_squared[0 * n_grid + i] = (-((r - 1.0).powi(2))).exp();
            // State 1: peaked at r=1.5
            wf_squared[1 * n_grid + i] = (-((r - 1.5).powi(2))).exp();
        }

        // Density gradient (increasing)
        let drho_dr: Vec<f64> = (0..n_grid).map(|i| 0.1 * (i as f64 + 1.0)).collect();

        // ls factors
        let ls_factors = vec![0.5, -1.0]; // p3/2, p1/2 like

        let h_so = so
            .compute(&wf_squared, &drho_dr, &r_grid, &ls_factors, dr, w0)
            .unwrap();

        assert_eq!(h_so.len(), 2);

        // State 0 has positive ls, state 1 has negative ls
        // With positive drho/dr and w0, expect opposite signs
        assert!(
            h_so[0] * h_so[1] < 0.0 || h_so[0].abs() < 1e-10 || h_so[1].abs() < 1e-10,
            "States should have opposite-sign spin-orbit corrections"
        );
    }

    #[tokio::test]
    async fn test_spin_orbit_with_density() {
        let Some(device) = crate::device::test_pool::get_test_device_if_gpu_available().await
        else {
            return;
        };

        let so = SpinOrbitGpu::new(device);

        // Test the version that computes gradient internally
        let n_grid = 10;
        let _n_states = 1;
        let _batch_size = 1;
        let dr = 0.5;
        let w0 = 120.0;

        let r_grid: Vec<f64> = (0..n_grid).map(|i| (i + 1) as f64 * dr).collect();

        // Linear density: ρ = 0.1 * r, so dρ/dr = 0.1
        let density: Vec<f64> = r_grid.iter().map(|r| 0.1 * r).collect();

        // Simple wavefunction
        let wf_squared: Vec<f64> = r_grid
            .iter()
            .map(|r| (-((r - 2.5).powi(2))).exp())
            .collect();

        let ls_factors = vec![1.0];

        let h_so = so
            .compute_with_density(&wf_squared, &density, &r_grid, &ls_factors, dr, w0)
            .unwrap();

        assert_eq!(h_so.len(), 1);
        assert!(
            h_so[0].abs() > 0.0,
            "Should have non-zero spin-orbit correction"
        );
    }
}
