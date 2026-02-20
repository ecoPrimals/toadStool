//! Lattice QCD / gauge field theory GPU primitives
//!
//! All shaders are compiled through `ShaderTemplate::for_driver_profile()` with
//! the exp/log workaround enabled (complex_f64.wgsl defines `c64_exp` which uses
//! `exp()` / `cos()` / `sin()` builtins — safe on all drivers when patched).
//!
//! # Hierarchy
//!
//! | Module | Content |
//! |--------|---------|
//! | `complex_f64` | WGSL_COMPLEX64 constant + concat helper |
//! | `su3` | WGSL_SU3 constant (requires complex_f64 prepended) |
//! | `plaquette` | Wilson plaquette GPU op (SU(3), 4D) |
//! | `higgs_u1` | U(1) Abelian Higgs HMC force (2D) |
//! | `hmc_force_su3` | SU(3) HMC gauge force (4D, Wilson action) |
//!
//! # hotSpring absorption
//!
//! Validated CPU implementations in hotSpring v0.5.16 (`lattice/` module).
//! GPU promotion: Feb 2026 — barracuda first-class primitives.

pub mod complex_f64;
pub mod hmc_force_su3;
pub mod higgs_u1;
pub mod plaquette;
pub mod su3;
