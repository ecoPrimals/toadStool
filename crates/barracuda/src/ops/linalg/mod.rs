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
pub mod triangular_solve;

pub use cholesky::Cholesky;
pub use triangular_solve::TriangularSolve;
