//! Linear algebra operations for scientific computing
//!
//! This module provides high-precision (f64) linear algebra operations
//! for scientific computing workflows. While BarraCUDA's GPU shaders
//! operate in f32 for maximum throughput, many scientific applications
//! require f64 precision for numerical stability.
//!
//! # Dual-Precision Architecture
//!
//! BarraCUDA uses a dual-precision pattern for optimal performance:
//! - **GPU (f32)**: Fast pairwise operations (cdist, matmul)
//! - **CPU (f64)**: Numerically sensitive operations (linear solves, eigendecomp)
//!
//! This gives ~90% of GPU speedup while maintaining scientific precision
//! where it matters.
//!
//! # Examples
//!
//! ```
//! use barracuda::linalg::solve_f64;
//!
//! // Solve Ax = b
//! let a = vec![2.0, 1.0, 1.0, 3.0];  // 2×2 matrix
//! let b = vec![5.0, 8.0];
//! let x = solve_f64(&a, &b, 2)?;
//!
//! assert!((x[0] - 1.0).abs() < 1e-10);
//! assert!((x[1] - 3.0).abs() < 1e-10);
//! # Ok::<(), barracuda::error::BarracudaError>(())
//! ```

pub mod solve;

pub use solve::solve_f64;
