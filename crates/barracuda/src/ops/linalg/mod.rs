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
//! - `cholesky` - Cholesky decomposition (A = L·Lᵀ)
//! - `eigh` - Eigenvalue decomposition (A = V·D·Vᵀ for symmetric A)
//! - `linsolve` - Linear system solve (A·x = b)
//! - `triangular_solve` - Forward/backward substitution (L·x = b)
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
pub mod triangular_solve;
pub mod tridiagonal;

pub use cholesky::Cholesky;
pub use eigh::Eigh;
pub use linsolve::LinSolve;
pub use triangular_solve::TriangularSolve;
pub use tridiagonal::{tridiagonal_solve, tridiagonal_solve_batch, tridiagonal_solve_f32};
