//! Molecular Dynamics Operations
//!
//! Operations for molecular dynamics simulations:
//! - Periodic boundary conditions (PBC)
//! - Force kernels (Coulomb, Yukawa, LJ, etc.)
//! - Time integrators (Velocity-Verlet, RK4, split VV)
//! - Thermostats (Berendsen, Nosé-Hoover)
//! - Observables (kinetic energy, RDF, VACF, SSF)
//!
//! **hotSpring Integration** (Feb 2026):
//! - f64 Yukawa force with PBC + PE (9/9 Sarkas cases validated)
//! - Split Velocity-Verlet (kick-drift-kick pattern)
//! - Berendsen thermostat for equilibration
//! - Nosé-Hoover thermostat for NVT production
//! - Per-particle kinetic energy for temperature
//!
//! **Deep Debt Compliance**: All math in WGSL, zero unsafe

pub mod forces;
pub mod integrators;
pub mod observables;
pub mod pbc;
pub mod thermostats;

pub use forces::*;
pub use integrators::*;
pub use observables::{compute_rdf, compute_ssf, compute_vacf, KineticEnergy, Rdf, Vacf};
pub use pbc::{DistanceMetric, PbcDistance};
pub use thermostats::{
    BerendsenThermostat, LangevinParams, LangevinStep, NoseHooverChain, NoseHooverHalfKick,
};

// Re-export for convenience
pub use pbc::PbcDistance as Pbc;
