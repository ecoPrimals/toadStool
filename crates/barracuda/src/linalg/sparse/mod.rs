//! Sparse linear algebra for large-scale scientific computing
//!
//! Provides sparse matrix representations and solvers for problems where
//! dense matrices would be prohibitively expensive. Essential for:
//! - Large HFB basis sets (nuclear physics)
//! - Finite element methods
//! - Graph algorithms
//! - Machine learning on sparse data
//!
//! # Storage Formats
//!
//! - **COO** (Coordinate): Easy construction, inefficient operations
//! - **CSR** (Compressed Sparse Row): Efficient row access, SpMV
//! - **CSC** (Compressed Sparse Column): Efficient column access
//!
//! # Solvers
//!
//! - **CG** (Conjugate Gradient): For symmetric positive definite matrices
//! - **BiCGSTAB**: For general non-symmetric matrices
//! - **Jacobi/Gauss-Seidel**: Simple iterative methods
//!
//! # Example
//!
//! ```
//! use barracuda::linalg::sparse::{CsrMatrix, cg_solve};
//!
//! // Build a sparse SPD matrix
//! let matrix = CsrMatrix::from_triplets(3, 3, &[
//!     (0, 0, 4.0), (0, 1, -1.0),
//!     (1, 0, -1.0), (1, 1, 4.0), (1, 2, -1.0),
//!     (2, 1, -1.0), (2, 2, 4.0),
//! ]);
//!
//! let b = vec![1.0, 2.0, 3.0];
//! let x = cg_solve(&matrix, &b, 1e-10, 100)?;
//! # Ok::<(), barracuda::error::BarracudaError>(())
//! ```
//!
//! # Reference
//!
//! - hotSpring Phase 5 Handoff: Large HFB basis sets requirement
//! - Saad, Y. (2003). Iterative Methods for Sparse Linear Systems

pub mod csr;
pub mod solvers;

pub use csr::{CooMatrix, CsrMatrix};
pub use solvers::{bicgstab_solve, cg_solve, jacobi_solve, SolverConfig, SolverResult};
