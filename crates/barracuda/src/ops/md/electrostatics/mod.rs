//! Long-Range Electrostatics (PPPM/Ewald)
//!
//! **Purpose**: Accurate Coulomb interactions for periodic systems
//!
//! **Algorithm**: Particle-Particle Particle-Mesh (PPPM)
//!
//! PPPM splits Coulomb interactions into two parts:
//! 1. **Short-range (PP)**: Direct Coulomb with erfc damping
//! 2. **Long-range (PM)**: FFT-based mesh solve for k > 0
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────┐
//! │                    PPPM Electrostatics                          │
//! ├─────────────────────────────────────────────────────────────────┤
//! │                                                                 │
//! │  1. Charge Spreading (particles → mesh)                        │
//! │     └── B-spline interpolation to mesh nodes                   │
//! │                                                                 │
//! │  2. Forward FFT (mesh → k-space)                               │
//! │     └── Uses existing Fft3D infrastructure                     │
//! │                                                                 │
//! │  3. Green's Function (k-space multiplication)                  │
//! │     └── ρ̃(k) × G(k) where G(k) = 4π/k² × influence function  │
//! │                                                                 │
//! │  4. Backward FFT (k-space → mesh)                              │
//! │     └── Get mesh potential φ(r)                                │
//! │                                                                 │
//! │  5. Force Interpolation (mesh → particles)                     │
//! │     └── B-spline gradient interpolation                        │
//! │                                                                 │
//! │  6. Short-Range Correction                                     │
//! │     └── erfc-damped direct Coulomb for nearby pairs            │
//! │                                                                 │
//! └─────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Ewald Splitting
//!
//! The Coulomb potential 1/r is split using:
//! ```text
//! 1/r = erfc(αr)/r + erf(αr)/r
//!       \_________/   \________/
//!       short-range   long-range
//!       (real space)  (k-space via FFT)
//! ```
//!
//! The splitting parameter α controls the crossover:
//! - Large α → more work in k-space (fewer mesh modes needed)
//! - Small α → more work in real space (larger cutoff needed)
//!
//! Optimal α balances real-space cutoff and mesh size.
//!
//! # PPPM Parameters
//!
//! | Parameter | Symbol | Typical Value | Notes |
//! |-----------|--------|---------------|-------|
//! | Splitting | α | box_side/6 | Balances PP/PM work |
//! | Mesh size | Kx,Ky,Kz | 32-128 | Power of 2 for FFT |
//! | Interpolation order | p | 4-7 | Higher = more accurate |
//! | Real cutoff | rc | box_side/4 | Works with cell-list |
//! | Accuracy | δ | 1e-5 | Force relative error |
//!
//! # Implementation Status
//!
//! | Component | Status | Notes |
//! |-----------|--------|-------|
//! | Fft3D (f32) | ✅ Done | Needs f64 evolution |
//! | B-spline spread | 🚧 Planned | |
//! | Green's function | 🚧 Planned | |
//! | Force interpolation | 🚧 Planned | |
//! | erfc short-range | 🚧 Planned | Uses math_f64 |
//! | Combined PPPM | 🚧 Planned | |
//!
//! # References
//!
//! - Essmann et al., JCP 103 (1995) - Smooth PME algorithm
//! - Deserno & Holm, JCP 109 (1998) - PPPM accuracy analysis
//! - LAMMPS PPPM implementation (BSD-3-Clause)

mod pppm_params;

pub use pppm_params::{PppmAccuracy, PppmParams};

// TODO: Add these modules as implementation progresses
// mod charge_spread;
// mod greens_function;
// mod force_interpolation;
// mod short_range;
// mod pppm;
