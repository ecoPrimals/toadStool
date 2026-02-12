//! Interpolation methods
//!
//! This module provides various interpolation methods for approximating
//! functions from discrete data points.
//!
//! # Available Methods
//!
//! - **Cubic Spline**: C² continuous piecewise cubic interpolation
//!
//! # Example
//!
//! ```
//! use barracuda::interpolate::CubicSpline;
//!
//! // Create data points
//! let x = vec![0.0, 1.0, 2.0, 3.0, 4.0];
//! let y = vec![0.0, 1.0, 0.0, 1.0, 0.0];
//!
//! // Fit a natural cubic spline
//! let spline = CubicSpline::natural(&x, &y)?;
//!
//! // Evaluate at intermediate points
//! let y_interp = spline.eval(1.5)?;
//! let dy_interp = spline.derivative(1.5)?;
//!
//! // Integrate
//! let integral = spline.integrate(0.0, 4.0)?;
//! # Ok::<(), barracuda::error::BarracudaError>(())
//! ```
//!
//! # Applications
//!
//! - Smooth curve fitting
//! - Surrogate model construction
//! - Data visualization
//! - Numerical integration of tabulated data

pub mod cubic_spline;

pub use cubic_spline::{CubicSpline, SplineBoundary};
