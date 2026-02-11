//! Numerical methods for differentiation and integration
//!
//! This module provides standard numerical methods for computing
//! derivatives and integrals when analytic forms are unavailable.
//!
//! # Methods
//!
//! - **gradient_1d**: Finite-difference gradients (3-point stencil)
//! - **trapz**: Trapezoidal integration
//! - **trapz_product**: Weighted product integration
//!
//! # Examples
//!
//! ```
//! use barracuda::numerical::{gradient_1d, trapz};
//!
//! // Compute gradient of y = x²
//! let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
//! let y: Vec<f64> = x.iter().map(|xi| xi * xi).collect();
//! let dy_dx = gradient_1d(&y, 1.0);  // dx = 1.0
//!
//! // Gradient should be ≈ 2x
//! assert!((dy_dx[2] - 4.0).abs() < 0.1);  // at x=2, dy/dx ≈ 4
//!
//! // Integrate y = x from 0 to 4
//! let integral = trapz(&y, &x)?;
//! assert!((integral - 21.33).abs() < 0.1);  // ∫₀⁴ x² dx = 64/3 ≈ 21.33
//! # Ok::<(), barracuda::error::BarracudaError>(())
//! ```

pub mod gradient;
pub mod integrate;

pub use gradient::gradient_1d;
pub use integrate::{trapz, trapz_product};
