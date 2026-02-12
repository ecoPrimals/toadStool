//! Special mathematical functions
//!
//! This module provides special functions commonly needed in scientific
//! computing: gamma function, factorials, orthogonal polynomials, Bessel
//! functions, error functions, and spherical harmonics.
//!
//! # Architecture
//!
//! Each function has two paths:
//! - **CPU (f64)**: High precision, single values or small batches
//! - **GPU (f32)**: Batch processing via WGSL shaders
//!
//! # Functions
//!
//! | Function | CPU | GPU | Reference |
//! |----------|-----|-----|-----------|
//! | `gamma(x)` | ✅ | — | Lanczos approximation |
//! | `factorial(n)` | ✅ | — | Stirling for large n |
//! | `laguerre(n, α, x)` | ✅ | — | Recurrence relation |
//! | `erf(x)` | ✅ | ✅ | A&S 7.1.26 |
//! | `erfc(x)` | ✅ | ✅ | A&S 7.1.23 |
//! | `bessel_j0(x)` | ✅ | ✅ | A&S 9.4.1-9.4.3 |
//! | `bessel_j1(x)` | ✅ | ✅ | A&S 9.4.4-9.4.6 |
//! | `bessel_i0(x)` | ✅ | ✅ | A&S 9.8.1-9.8.2 |
//! | `bessel_k0(x)` | ✅ | ✅ | A&S 9.8.5-9.8.6 |
//! | `lgamma(x)` | ✅ | ✅ | Log-gamma |
//! | `digamma(x)` | ✅ | — | ψ(x) = d/dx ln Γ(x) |
//! | `beta(a, b)` | ✅ | — | B(a,b) = Γ(a)Γ(b)/Γ(a+b) |
//! | `hermite(n, x)` | ✅ | ✅ | Physicist's Hermite Hₙ(x) |
//! | `legendre(n, x)` | ✅ | ✅ | Legendre Pₙ(x) |
//! | `assoc_legendre(n, m, x)` | ✅ | — | Associated Pₙᵐ(x) |
//!
//! # Precision
//!
//! - CPU functions: f64, match scipy.special to machine precision
//! - GPU functions: f32, |ε| < 1e-5 for most inputs
//!
//! # Examples
//!
//! ```
//! use barracuda::special::{gamma, factorial, erf, bessel_j0};
//! use std::f64::consts::PI;
//!
//! // Γ(n) = (n-1)! for integers
//! assert!((gamma(5.0) - 24.0).abs() < 1e-12);
//!
//! // Γ(1/2) = √π
//! assert!((gamma(0.5) - PI.sqrt()).abs() < 1e-12);
//!
//! // Error function
//! assert!((erf(0.0) - 0.0).abs() < 1e-14);
//! assert!((erf(1.0) - 0.8427007929).abs() < 1e-7);
//!
//! // Bessel J₀
//! assert!((bessel_j0(0.0) - 1.0).abs() < 1e-14);
//! ```
//!
//! # References
//!
//! - Abramowitz & Stegun (A&S): Handbook of Mathematical Functions
//! - DLMF: Digital Library of Mathematical Functions (<https://dlmf.nist.gov>)

// Core special functions (CPU f64)
pub mod factorial;
pub mod gamma;
pub mod laguerre;
pub mod erf;
pub mod bessel;
pub mod hermite;
pub mod legendre;

// Re-export CPU functions
pub use factorial::factorial;
pub use gamma::{gamma, lgamma, digamma, beta};
pub use laguerre::{laguerre, laguerre_all, laguerre_simple};
pub use erf::{erf, erfc};
pub use bessel::{bessel_j0, bessel_j1, bessel_i0, bessel_k0};
pub use hermite::hermite;
pub use legendre::{legendre, assoc_legendre};

// Re-export GPU ops for batch processing
pub use crate::ops::erf_wgsl::Erf as ErfGpu;
pub use crate::ops::erfc_wgsl::Erfc as ErfcGpu;
pub use crate::ops::bessel_j0_wgsl::BesselJ0 as BesselJ0Gpu;
pub use crate::ops::bessel_j1_wgsl::BesselJ1 as BesselJ1Gpu;
pub use crate::ops::bessel_i0_wgsl::BesselI0 as BesselI0Gpu;
pub use crate::ops::bessel_k0_wgsl::BesselK0 as BesselK0Gpu;
pub use crate::ops::lgamma_wgsl::Lgamma as LgammaGpu;
pub use crate::ops::spherical_harmonics_wgsl::SphericalHarmonics as SphericalHarmonicsGpu;
