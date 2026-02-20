//! WGSL SU(3) matrix algebra library.
//!
//! Provides `WGSL_SU3` and `su3_preamble()` for building shaders that require
//! SU(3) gauge-field algebra.  Always prepend `WGSL_COMPLEX64` first.

use super::complex_f64::WGSL_COMPLEX64;

/// Raw WGSL source for SU(3) 3×3 complex matrix algebra.
///
/// Depends on `complex_f64.wgsl` definitions — always use `su3_preamble()`
/// or prepend `WGSL_COMPLEX64` manually.
pub const WGSL_SU3: &str = include_str!("../../shaders/math/su3.wgsl");

/// Build a complete shader preamble: complex_f64 + su3.
///
/// Call once per shader; append the domain shader source after this string.
pub fn su3_preamble() -> String {
    format!("{WGSL_COMPLEX64}\n{WGSL_SU3}\n")
}
