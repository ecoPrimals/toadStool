//! Time Integrators Module
//!
//! Implementations of numerical integration schemes for ODEs/PDEs:
//! - Velocity-Verlet (symplectic, MD)
//! - RK4 (high accuracy, general ODEs)
//! - Laplacian stencil (PDEs, mesh operations)
//!
//! **Deep Debt**: All math in WGSL, zero unsafe

pub mod velocity_verlet;
pub mod rk4;
pub mod laplacian;

pub use velocity_verlet::VelocityVerlet;
pub use rk4::Rk4;
pub use laplacian::Laplacian;
