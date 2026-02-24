//! Lattice shaders absorbed from hotSpring (Feb 2026).
//!
//! These WGSL shaders are self-contained and available for GPU-resident
//! lattice QCD operations.

// Re-export GPU-resident CG shaders from cg module
pub use super::cg::{
    WGSL_CG_COMPUTE_ALPHA_F64, WGSL_CG_COMPUTE_BETA_F64, WGSL_CG_UPDATE_P_F64,
    WGSL_CG_UPDATE_XR_F64, WGSL_SUM_REDUCE_F64,
};

/// SU(3) gauge force: staple sum + traceless anti-Hermitian projection.
pub const WGSL_SU3_GAUGE_FORCE_F64: &str =
    include_str!("../../shaders/lattice/su3_gauge_force_f64.wgsl");

/// Kinetic energy: T_link = -0.5 * Re Tr(P²) per link.
pub const WGSL_SU3_KINETIC_ENERGY_F64: &str =
    include_str!("../../shaders/lattice/su3_kinetic_energy_f64.wgsl");

/// Link update: U = exp(dt * P) * U via Cayley + reunitarize.
pub const WGSL_SU3_LINK_UPDATE_F64: &str =
    include_str!("../../shaders/lattice/su3_link_update_f64.wgsl");

/// Momentum update: P += dt * F.
pub const WGSL_SU3_MOMENTUM_UPDATE_F64: &str =
    include_str!("../../shaders/lattice/su3_momentum_update_f64.wgsl");

/// SU(3) algebra momentum generation via PCG hash PRNG.
pub const WGSL_SU3_RANDOM_MOMENTA_F64: &str =
    include_str!("../../shaders/lattice/su3_random_momenta_f64.wgsl");

/// Gaussian random fermion field (η ~ N(0,1)) for pseudofermion heat bath.
pub const WGSL_GAUSSIAN_FERMION_F64: &str =
    include_str!("../../shaders/lattice/gaussian_fermion_f64.wgsl");

/// Staggered fermion force: F_f = TA[ U·M ].
pub const WGSL_STAGGERED_FERMION_FORCE_F64: &str =
    include_str!("../../shaders/lattice/staggered_fermion_force_f64.wgsl");
