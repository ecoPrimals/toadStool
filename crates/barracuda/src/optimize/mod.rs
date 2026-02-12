//! Optimization algorithms for parameter fitting and model calibration
//!
//! This module provides both gradient-free and gradient-based optimization
//! methods suitable for scientific computing and machine learning.
//!
//! # Algorithms
//!
//! - **Nelder-Mead**: Simplex method for gradient-free local optimization
//! - **Multi-start Nelder-Mead**: Global optimization with LHS initial guesses
//! - **BFGS**: Quasi-Newton method with gradient information
//! - **Bisection**: Root-finding for 1D problems
//! - **Evaluation Cache**: Record all evaluations for surrogate training
//!
//! # Cross-Domain Applications
//!
//! - **Physics**: Nuclear EOS parameter fitting, force-field calibration
//! - **ML**: Hyperparameter tuning, architecture search
//! - **Graphics**: Camera calibration, rendering parameter optimization
//! - **Audio**: Filter design, codec parameter tuning
//!
//! # Examples
//!
//! ```
//! use barracuda::optimize::{nelder_mead, multi_start_nelder_mead};
//!
//! // Local optimization
//! let rosenbrock = |x: &[f64]| {
//!     let (a, b) = (1.0, 100.0);
//!     (a - x[0]).powi(2) + b * (x[1] - x[0].powi(2)).powi(2)
//! };
//!
//! let x0 = vec![0.0, 0.0];
//! let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];
//!
//! let (x_best, f_best, n_evals) = nelder_mead(
//!     rosenbrock,
//!     &x0,
//!     &bounds,
//!     1000,
//!     1e-8,
//! )?;
//!
//! // Global optimization with evaluation recording
//! let (best, cache, _) = multi_start_nelder_mead(
//!     rosenbrock,
//!     &bounds,
//!     16,     // n_starts (like SparsitySampler npts=16)
//!     1000,   // max_iter per start
//!     1e-8,   // tolerance
//!     42,     // seed
//! )?;
//!
//! // Use cache for surrogate training
//! let (x_data, y_data) = cache.training_data();
//! # Ok::<(), barracuda::error::BarracudaError>(())
//! ```

pub mod bfgs;
pub mod bisect;
pub mod eval_record;
pub mod multi_start;
pub mod nelder_mead;
pub mod solver_state;

pub use bfgs::{bfgs, bfgs_numerical, numerical_gradient, BfgsConfig, BfgsResult};
pub use bisect::bisect;
pub use eval_record::{EvaluationCache, EvaluationRecord};
pub use multi_start::{multi_start_nelder_mead, SolverResult};
pub use nelder_mead::nelder_mead;
pub use solver_state::{ResumableNelderMead, SolverStatus};
