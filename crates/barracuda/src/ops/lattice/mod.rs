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

// WGSL library preambles
pub mod complex_f64;
pub mod lcg;
pub mod su3;
pub mod su3_extended;

// GPU operators
pub mod absorbed_shaders;
pub mod cg;
pub mod dirac;
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
