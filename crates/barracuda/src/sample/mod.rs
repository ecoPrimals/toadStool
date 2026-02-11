//! Sampling strategies for parameter space exploration
//!
//! Provides space-filling and guided sampling methods used across domains:
//! - **Surrogate learning**: Train RBF models on well-distributed points
//! - **Design of experiments**: Explore parameter spaces efficiently
//! - **Neural architecture search**: Evaluate hyperparameter configurations
//! - **Monte Carlo**: Generate quasi-random initial conditions
//!
//! # Available Samplers
//!
//! - [`latin_hypercube`] — Space-filling design with stratified intervals
//! - [`random_uniform`] — Uniform random sampling within bounds
//! - [`maximin::maximin_lhs`] — Maximin-optimized LHS (maximize min pairwise distance)
//! - [`sparsity::sparsity_sampler`] — Iterative surrogate-directed sampling (Diaw et al. 2024)
//!
//! # Examples
//!
//! ```
//! use barracuda::sample::{latin_hypercube, random_uniform};
//!
//! let bounds = vec![(-5.0, 5.0), (-5.0, 5.0)];
//!
//! // LHS: one sample per row/column in hypercube
//! let lhs_points = latin_hypercube(100, &bounds, 42)?;
//! assert_eq!(lhs_points.len(), 100);
//! assert_eq!(lhs_points[0].len(), 2);
//!
//! // Random: uniform within bounds
//! let rng_points = random_uniform(100, &bounds, 42);
//! assert_eq!(rng_points.len(), 100);
//! # Ok::<(), barracuda::error::BarracudaError>(())
//! ```

pub mod lhs;
pub mod maximin;
pub mod sparsity;

pub use lhs::{latin_hypercube, random_uniform};
