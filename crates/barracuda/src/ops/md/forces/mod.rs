//! Force Kernels Module
//!
//! Implementations of fundamental force calculations for molecular dynamics:
//! - Coulomb (electrostatic)
//! - Yukawa (screened electrostatic, f32 and f64)
//! - Lennard-Jones (van der Waals)
//! - Morse (bonded)
//! - Born-Mayer (hard-core repulsion)
//!
//! **f64 Evolution**: hotSpring-validated f64 Yukawa with PBC + PE (Feb 2026)
//! - `yukawa_f64.wgsl`: All-pairs O(N²) with PBC minimum-image
//! - `yukawa_celllist_f64.wgsl`: 27-neighbor cell-list O(N)
//!
//! **Deep Debt**: All math in WGSL, zero unsafe

pub mod born_mayer;
pub mod coulomb;
pub mod lennard_jones;
pub mod morse;
pub mod yukawa;
pub mod yukawa_f64;

pub use born_mayer::BornMayerForce;
pub use coulomb::CoulombForce;
pub use lennard_jones::LennardJonesForce;
pub use morse::MorseForce;
pub use yukawa::YukawaForce;
pub use yukawa_f64::YukawaForceF64;
