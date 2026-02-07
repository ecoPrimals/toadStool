//! Force Kernels Module
//!
//! Implementations of fundamental force calculations for molecular dynamics:
//! - Coulomb (electrostatic)
//! - Yukawa (screened electrostatic)
//! - Lennard-Jones (van der Waals)
//! - Morse (bonded)
//! - Born-Mayer (hard-core repulsion)
//!
//! **Deep Debt**: All math in WGSL, zero unsafe

pub mod coulomb;

pub use coulomb::CoulombForce;
