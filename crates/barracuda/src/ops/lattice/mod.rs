//! Lattice QCD / gauge field theory GPU primitives
//!
//! All math runs on GPU via WGSL shaders. CPU reference implementations exist
//! in test-gated modules for validation only.
//!
//! # WGSL Library Shaders
//!
//! | Module | Content |
//! |--------|---------|
//! | `complex_f64` | Complex f64 arithmetic (`c64_*` functions) |
//! | `su3` | SU(3) 3×3 matrix algebra |
//! | `lcg` | LCG PRNG for GPU lattice kernels |
//! | `su3_extended` | Reunitarize, exp_cayley, random SU(3) generation |
//!
//! # GPU Operators
//!
//! | Module | Content |
//! |--------|---------|
//! | `plaquette` | Wilson plaquette GPU op (SU(3), 4D) |
//! | `higgs_u1` | U(1) Abelian Higgs HMC force (2D) |
//! | `hmc_force_su3` | SU(3) HMC gauge force (4D, Wilson action) |
//! | `dirac` | Staggered Dirac operator (Kogut-Susskind, 4D SU(3)) |
//! | `cg` | CG vector ops (complex dot, axpy, xpay) for fermion solves |
//! | `gpu_lattice_init` | Cold/hot start lattice initialization |
//! | `gpu_wilson_action` | Per-site Wilson action (for GPU reduction) |
//! | `gpu_polyakov` | Polyakov loop (temporal Wilson line) |
//! | `gpu_hmc_leapfrog` | HMC leapfrog: momentum kick, link update, momentum gen |
//! | `gpu_kinetic_energy` | Per-link kinetic energy from momenta |
//! | `gpu_pseudofermion` | Pseudofermion heatbath noise + fermion force |
//! | `gpu_cg_solver` | GPU CG solver orchestration (D†D solve via multi-dispatch) |
//! | `gpu_hmc_trajectory` | Full dynamical fermion HMC trajectory on GPU |
//!
//! # Test-Only CPU Reference
//!
//! | Module | Content |
//! |--------|---------|
//! | `constants` | LCG constants and CPU PRNG (test reference) |
//! | `cpu_complex` | Complex64 CPU arithmetic (test reference) |
//! | `cpu_su3` | SU(3) CPU matrix ops (test reference) |
//! | `wilson` | Wilson lattice CPU (test reference) |
//! | `cpu_dirac` | Dirac/CG CPU solver (test reference) |
//! | `pseudofermion` | Pseudofermion HMC CPU (test reference) |
//!
//! # Neighbor Resolution
//!
//! Lattice shaders can resolve neighbor site indices in two ways via [`NeighborMode`]:
//! - [`NeighborMode::Compute`]: on-the-fly from lattice dimensions (default, suitable for small lattices)
//! - [`NeighborMode::PrecomputedBuffer`]: precomputed GPU buffer (faster for repeated HMC trajectories)

// WGSL library preambles
pub mod complex_f64;
pub mod lcg;
pub mod su3;
pub mod su3_extended;

// GPU operators
pub mod absorbed_shaders;
pub mod cg;
pub mod dirac;
pub mod gpu_cg_resident;
pub mod gpu_cg_solver;
pub mod gpu_hmc_leapfrog;
pub mod gpu_hmc_trajectory;
pub mod gpu_kinetic_energy;
pub mod gpu_lattice_init;
pub mod gpu_polyakov;
pub mod gpu_pseudofermion;
pub mod gpu_wilson_action;
pub mod higgs_u1;
pub mod hmc_force_su3;
pub mod plaquette;

// CPU reference implementations — test-only
#[cfg(test)]
pub mod constants;
#[cfg(test)]
pub mod cpu_complex;
#[cfg(test)]
pub mod cpu_dirac;
#[cfg(test)]
pub mod cpu_su3;
#[cfg(test)]
pub mod pseudofermion;
#[cfg(test)]
pub mod wilson;

// ── Neighbor resolution for lattice shaders ────────────────────────────────────

use std::sync::Arc;

/// How lattice shaders resolve neighbor site indices.
#[derive(Debug, Clone)]
pub enum NeighborMode {
    /// Compute neighbors on-the-fly from lattice dimensions (current default).
    /// Suitable for small lattices or one-off calculations.
    Compute,
    /// Use a precomputed GPU buffer of neighbor indices.
    /// `buffer[site * 8 + dir]` gives the neighbor in direction `dir`
    /// (±x, ±y, ±z, ±t with periodic boundary conditions).
    /// Optimal for repeated HMC trajectories on the same lattice.
    PrecomputedBuffer(Arc<wgpu::Buffer>),
}

/// SU(3) Wilson gauge action density from average plaquette.
///
/// For SU(3) in 4 dimensions there are 6 plaquette orientations per site.
/// The per-site Wilson action density (without β factor) is:
///   `a_d = 6 × (1 − ⟨P⟩)`
/// where ⟨P⟩ = (1/3) Re Tr U_p averaged over all orientations and sites.
/// A cold (identity) configuration gives ⟨P⟩ = 1 → a_d = 0.
#[inline]
pub fn action_density(avg_plaquette: f64) -> f64 {
    6.0 * (1.0 - avg_plaquette)
}

impl NeighborMode {
    /// Build a precomputed neighbor table for a 4D periodic lattice.
    ///
    /// Uses t-major ordering (hotSpring convention): index = t*V3 + z*V2 + y*V1 + x
    /// where V1=Nx, V2=Nx*Ny, V3=Nx*Ny*Nz.
    ///
    /// Returns a buffer containing `n_sites * 8` u32 entries.
    pub fn precompute(device: &crate::device::WgpuDevice, dims: [u32; 4]) -> Self {
        let [nx, ny, nz, nt] = dims;
        let n_sites = (nx * ny * nz * nt) as usize;
        let mut table = Vec::with_capacity(n_sites * 8);

        for t in 0..nt {
            for z in 0..nz {
                for y in 0..ny {
                    for x in 0..nx {
                        // +x, -x, +y, -y, +z, -z, +t, -t (periodic BC)
                        let idx = |x: u32, y: u32, z: u32, t: u32| -> u32 {
                            t * nz * ny * nx + z * ny * nx + y * nx + x
                        };
                        table.push(idx((x + 1) % nx, y, z, t));
                        table.push(idx((x + nx - 1) % nx, y, z, t));
                        table.push(idx(x, (y + 1) % ny, z, t));
                        table.push(idx(x, (y + ny - 1) % ny, z, t));
                        table.push(idx(x, y, (z + 1) % nz, t));
                        table.push(idx(x, y, (z + nz - 1) % nz, t));
                        table.push(idx(x, y, z, (t + 1) % nt));
                        table.push(idx(x, y, z, (t + nt - 1) % nt));
                    }
                }
            }
        }

        use wgpu::util::DeviceExt;
        let buffer = device
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Neighbor Table"),
                contents: bytemuck::cast_slice(&table),
                usage: wgpu::BufferUsages::STORAGE,
            });

        Self::PrecomputedBuffer(Arc::new(buffer))
    }
}

#[cfg(test)]
mod action_density_tests {
    use super::action_density;

    #[test]
    fn cold_start_zero() {
        assert!((action_density(1.0)).abs() < 1e-15);
    }

    #[test]
    fn strong_coupling() {
        let ad = action_density(0.0);
        assert!((ad - 6.0).abs() < 1e-15);
    }

    #[test]
    fn typical_value() {
        let ad = action_density(0.55);
        assert!((ad - 2.7).abs() < 1e-14);
    }
}

#[cfg(test)]
mod neighbor_tests {
    #[test]
    fn neighbor_table_size() {
        // For a 4^4 lattice: 256 sites * 8 neighbors = 2048 entries
        let dims = [4, 4, 4, 4];
        let n_sites = 4u32.pow(4) as usize;
        let mut table = Vec::with_capacity(n_sites * 8);
        let [nx, ny, nz, nt] = dims;
        for t in 0..nt {
            for z in 0..nz {
                for y in 0..ny {
                    for x in 0..nx {
                        let idx = |x: u32, y: u32, z: u32, t: u32| -> u32 {
                            t * nz * ny * nx + z * ny * nx + y * nx + x
                        };
                        table.push(idx((x + 1) % nx, y, z, t));
                        table.push(idx((x + nx - 1) % nx, y, z, t));
                        table.push(idx(x, (y + 1) % ny, z, t));
                        table.push(idx(x, (y + ny - 1) % ny, z, t));
                        table.push(idx(x, y, (z + 1) % nz, t));
                        table.push(idx(x, y, (z + nz - 1) % nz, t));
                        table.push(idx(x, y, z, (t + 1) % nt));
                        table.push(idx(x, y, z, (t + nt - 1) % nt));
                    }
                }
            }
        }
        assert_eq!(table.len(), 2048);
    }

    #[test]
    fn neighbor_periodic_boundary() {
        let [nx, ny, nz, _nt] = [4u32, 4, 4, 4];
        let idx =
            |x: u32, y: u32, z: u32, t: u32| -> u32 { t * nz * ny * nx + z * ny * nx + y * nx + x };
        // Site (0,0,0,0): +x neighbor should be (1,0,0,0), -x should wrap to (3,0,0,0)
        let site_000 = idx(0, 0, 0, 0);
        assert_eq!(site_000, 0);
        let plus_x = idx(1, 0, 0, 0);
        let minus_x = idx(3, 0, 0, 0); // wraps
        assert_eq!(plus_x, 1);
        assert_eq!(minus_x, 3);
    }
}
