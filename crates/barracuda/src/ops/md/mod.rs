//! Molecular Dynamics Operations
//!
//! Operations for molecular dynamics simulations:
//! - Periodic boundary conditions (PBC)
//! - Force kernels (Coulomb, Yukawa, LJ, etc.)
//! - Time integrators (Velocity-Verlet, RK4)
//!
//! **Deep Debt Compliance**: All math in WGSL, zero unsafe

pub mod pbc;

pub use pbc::{PbcDistance, DistanceMetric};

// Re-export for convenience
pub use pbc::PbcDistance as Pbc;
