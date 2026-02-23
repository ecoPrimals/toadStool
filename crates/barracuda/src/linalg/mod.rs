//! Linear algebra operations for scientific computing
//!
//! This module provides f64 linear algebra operations dispatched to GPU
//! shaders. All production math uses GPU (no CPU-only paths).
//!
//! # Available Operations
//!
//! ## Dense Decompositions
//!
//! - [`lu_decompose`] - LU decomposition with partial pivoting (PA = LU)
//! - [`qr_decompose`] - QR decomposition via Householder reflections
//! - [`svd_decompose`] - Singular value decomposition (A = UΣVᵀ)
//! - [`cholesky_f64`] - Cholesky decomposition for SPD matrices (A = LLᵀ)
//! - [`eigh_f64`] - Eigendecomposition for symmetric matrices (A = VDVᵀ)
//! - [`gen_eigh_f64`] - Generalized eigenvalue problem Ax = λBx
//!
//! ## Dense Solvers
//!
//! - [`solve_f64`] - General linear solve via Gauss-Jordan
//! - [`lu_solve`] - Linear solve via LU decomposition
//! - [`qr_least_squares`] - Least squares via QR
//! - [`tridiagonal_solve_f64`] - Thomas algorithm for tridiagonal systems
//!
//! ## Sparse Operations ([`sparse`] module)
//!
//! For large-scale problems (HFB basis sets, finite elements):
//!
//! - [`sparse::CsrMatrix`] - Compressed sparse row format
//! - [`sparse::CooMatrix`] - Coordinate format (for construction)
//! - [`sparse::cg_solve`] - Conjugate gradient (SPD matrices)
//! - [`sparse::bicgstab_solve`] - BiCGSTAB (general matrices)
//! - [`sparse::jacobi_solve`] - Jacobi iteration
//!
//! ## Utilities
//!
//! - [`lu_det`] - Determinant via LU
//! - [`lu_inverse`] - Matrix inverse via LU
//! - [`svd_pinv`] - Pseudoinverse via SVD
//!
//! # Examples
//!
//! ```no_run
//! use barracuda::linalg::{solve_f64, lu_decompose, qr_decompose};
//! use barracuda::device::WgpuDevice;
//! use std::sync::Arc;
//!
//! # async fn example() -> barracuda::error::Result<()> {
//! let device = Arc::new(WgpuDevice::new().await?);
//! let a = vec![2.0, 1.0, 1.0, 3.0];
//! let b = vec![5.0, 8.0];
//! let x = solve_f64(device, &a, &b, 2)?;
//! # Ok(())
//! # }
//!
//! // LU decomposition for multiple solves
//! let lu = lu_decompose(&a, 2)?;
//! let det = lu.det();
//! let x2 = lu.solve(&b)?;
//!
//! // QR for least squares
//! let a_overdetermined = vec![1.0, 1.0, 1.0, 2.0, 1.0, 3.0];  // 3×2
//! let qr = qr_decompose(&a_overdetermined, 3, 2)?;
//! # Ok::<(), barracuda::error::BarracudaError>(())
//! ```

pub mod cholesky;
pub mod eigh;
pub mod gen_eigh;
pub mod solve;
pub mod sparse;

// Re-export solve
pub use solve::solve_f64;

// Re-export LU decomposition from ops/linalg (already f64)
pub use crate::ops::linalg::{lu_decompose, lu_det, lu_inverse, lu_solve, LuDecomposition};

// Re-export QR decomposition from ops/linalg (already f64)
pub use crate::ops::linalg::{qr_decompose, qr_least_squares, QrDecomposition};

// Re-export SVD decomposition from ops/linalg (already f64)
pub use crate::ops::linalg::{svd_decompose, svd_pinv, svd_values, SvdDecomposition};

// Re-export tridiagonal solver from ops/linalg
pub use crate::ops::linalg::tridiagonal_solve;
/// Alias for tridiagonal_solve (consistency with other _f64 functions)
pub use crate::ops::linalg::tridiagonal_solve as tridiagonal_solve_f64;

// Export new f64 CPU implementations
pub use cholesky::cholesky_f64;
pub use eigh::eigh_f64;
pub use gen_eigh::{gen_eigh_f64, gen_eigh_identity_b, GenEighDecomposition};
