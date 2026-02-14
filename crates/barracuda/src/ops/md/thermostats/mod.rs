//! Thermostats for Molecular Dynamics
//!
//! Temperature control for equilibration and NVT ensembles.
//!
//! **Available Thermostats**:
//! - Berendsen: Weak coupling, equilibration only (does NOT sample canonical)
//! - Nosé-Hoover: Deterministic NVT, properly samples canonical ensemble
//! - (Future) Langevin: Stochastic, friction + noise
//!
//! **Usage Pattern**:
//! 1. Equilibration: Use Berendsen with τ ≈ 5*dt for fast relaxation
//! 2. Production NVT: Switch to Nosé-Hoover with τ ≈ 100*dt
//! 3. Production NVE: Remove thermostat entirely
//!
//! **Deep Debt Compliance**:
//! - ✅ WGSL shader-first (separate .wgsl files)
//! - ✅ Full f64 precision
//! - ✅ Zero unsafe code

mod berendsen;
mod nose_hoover;

pub use berendsen::BerendsenThermostat;
pub use nose_hoover::{NoseHooverChain, NoseHooverHalfKick};
