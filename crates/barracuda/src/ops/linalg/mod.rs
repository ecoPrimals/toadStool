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
//! ### Decompositions (CPU f64)
//!
//! - `lu_decompose` - LU decomposition with pivoting (PA = L·U)
//! - `qr_decompose` - QR decomposition (A = Q·R)
//! - `svd_decompose` - Singular value decomposition (A = U·Σ·Vᵀ)
//!
//! ### Decompositions (GPU f32 via WGSL)
//!
//! - `Cholesky` - GPU Cholesky decomposition (A = L·Lᵀ)
//! - `Eigh` - GPU eigenvalue decomposition for symmetric matrices
//! - `LuGpu` - GPU LU decomposition with partial pivoting
//! - `QrGpu` - GPU QR decomposition via Householder reflections
//! - `SvdGpu` - GPU SVD via one-sided Jacobi
//!
//! ### Solvers
//!
//! - `LinSolve` - GPU linear system solve (A·x = b)
//! - `TriangularSolve` - GPU forward/backward substitution
//! - `tridiagonal_solve` - Thomas algorithm for tridiagonal systems
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
pub mod lu_gpu;
pub mod qr;
pub mod qr_gpu;
pub mod svd;
pub mod svd_gpu;
pub mod triangular_solve;
pub mod tridiagonal;

pub use cholesky::Cholesky;
pub use eigh::Eigh;
pub use linsolve::LinSolve;
pub use lu::{lu_decompose, lu_det, lu_inverse, lu_solve, LuDecomposition};
pub use lu_gpu::LuGpu;
pub use qr::{qr_decompose, qr_least_squares, QrDecomposition};
pub use qr_gpu::QrGpu;
pub use svd::{svd_decompose, svd_pinv, svd_values, SvdDecomposition};
pub use svd_gpu::SvdGpu;
pub use triangular_solve::TriangularSolve;
pub use tridiagonal::{tridiagonal_solve, tridiagonal_solve_batch, tridiagonal_solve_f32};
