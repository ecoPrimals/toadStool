//! Linear Algebra Operations
//!
//! **Deep Debt Compliant Linear Algebra Module**
//!
//! Core scientific computing operations for:
//! - RBF surrogate learning (hotSpring physics integration)
//! - Molecular dynamics simulations
//! - Scientific computing workloads
//!
//! ## Operations
//!
//! ### Decompositions
//!
//! - `cholesky` - Cholesky decomposition (A = L·Lᵀ)
//! - `lu` - LU decomposition with pivoting (PA = L·U)
//! - `qr` - QR decomposition (A = Q·R)
//! - `svd` - Singular value decomposition (A = U·Σ·Vᵀ)
//! - `eigh` - Eigenvalue decomposition (A = V·D·Vᵀ for symmetric A)
//!
//! ### Solvers
//!
//! - `linsolve` - Linear system solve (A·x = b)
//! - `triangular_solve` - Forward/backward substitution (L·x = b)
//! - `tridiagonal` - Thomas algorithm for tridiagonal systems
//!
//! ## Design Principles
//!
//! - ✅ Pure WGSL (hardware-agnostic)
//! - ✅ Safe Rust (no unsafe blocks)
//! - ✅ Runtime-configured sizes
//! - ✅ Capability-based dispatch
//! - ✅ Composable operations

pub mod cholesky;
pub mod eigh;
pub mod linsolve;
pub mod lu;
pub mod qr;
pub mod svd;
pub mod triangular_solve;
pub mod tridiagonal;

pub use cholesky::Cholesky;
pub use eigh::Eigh;
pub use linsolve::LinSolve;
pub use lu::{lu_decompose, lu_det, lu_inverse, lu_solve, LuDecomposition};
pub use qr::{qr_decompose, qr_least_squares, QrDecomposition};
pub use svd::{svd_decompose, svd_pinv, svd_values, SvdDecomposition};
pub use triangular_solve::TriangularSolve;
pub use tridiagonal::{tridiagonal_solve, tridiagonal_solve_batch, tridiagonal_solve_f32};
