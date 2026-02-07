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
pub mod yukawa;
pub mod lennard_jones;
pub mod morse;
pub mod born_mayer;

pub use coulomb::CoulombForce;
pub use yukawa::YukawaForce;
pub use lennard_jones::LennardJonesForce;
pub use morse::MorseForce;
pub use born_mayer::BornMayerForce;
