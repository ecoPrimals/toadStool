//! Time Integrators Module
//!
//! Implementations of numerical integration schemes for ODEs/PDEs:
//! - Velocity-Verlet (symplectic, MD)
//! - RK4 (high accuracy, general ODEs)
//! - Laplacian stencil (PDEs, mesh operations)
//!
//! **Deep Debt**: All math in WGSL, zero unsafe

pub mod velocity_verlet;

pub use velocity_verlet::VelocityVerlet;
