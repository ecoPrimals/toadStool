//! Thermostats for Molecular Dynamics
//!
//! Temperature control for equilibration and NVT ensembles.
//!
//! **Available Thermostats**:
//! - Berendsen: Weak coupling, equilibration only
//! - (Future) Nosé-Hoover: Deterministic NVT
//! - (Future) Langevin: Stochastic, friction + noise
//!
//! **Deep Debt Compliance**:
//! - ✅ WGSL shader-first (separate .wgsl files)
//! - ✅ Full f64 precision
//! - ✅ Zero unsafe code

mod berendsen;

pub use berendsen::BerendsenThermostat;
