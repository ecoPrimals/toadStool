//! Special mathematical functions
//!
//! This module provides special functions commonly needed in scientific
//! computing: gamma function, factorials, and orthogonal polynomials.
//!
//! # Functions
//!
//! - **gamma**: Γ(x) via Lanczos approximation
//! - **factorial**: n! with Stirling approximation for large n
//! - **laguerre**: Generalized Laguerre polynomials
//!
//! # Precision
//!
//! All functions are f64 and match scipy.special to machine precision.
//!
//! # Examples
//!
//! ```
//! use barracuda::special::{gamma, factorial};
//! use std::f64::consts::PI;
//!
//! // Γ(n) = (n-1)! for integers
//! assert!((gamma(5.0) - 24.0).abs() < 1e-12);  // Γ(5) = 4!
//!
//! // Γ(1/2) = √π
//! assert!((gamma(0.5) - PI.sqrt()).abs() < 1e-12);
//!
//! // Factorial
//! assert_eq!(factorial(5), 120.0);
//! ```

pub mod factorial;
pub mod gamma;
pub mod laguerre;

pub use factorial::factorial;
pub use gamma::gamma;
pub use laguerre::{laguerre, laguerre_all, laguerre_simple};
