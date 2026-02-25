//! Linear Algebra Operations
//!
//! **Deep Debt Compliant Linear Algebra Module**
//!
//! Core scientific computing operations for:
//! - RBF surrogate learning (hotSpring physics integration)
//! - Molecular dynamics simulations
//! - Scientific computing workloads
//!
//! ## Precision Philosophy
//!
//! **Both CPU and GPU use f64 by default.**
//!
//! The WGSL/SPIR-V/Vulkan path bypasses CUDA's artificial fp64 throttle,
//! achieving 1:2-3 FP64:FP32 performance (not 1:32 like CUDA consumer GPUs).
//!
//! ## WGSL as Unified Math Language (Feb 16, 2026)
//!
//! The same shader runs on any hardware: NVIDIA, AMD, Intel via WebGPU/Vulkan.
//! - CUDA handicaps: vendor lock-in, artificial fp64 throttling
//! - OpenCL fragmentation: driver quality varies wildly
//! - WGSL advantage: write once, run anywhere with native fp64 builtins
//!
//! ## Operations
//!
//! ### Decompositions (CPU f64)
//!
//! - `lu_decompose` - LU decomposition with pivoting (PA = L·U)
//! - `qr_decompose` - QR decomposition (A = Q·R)
//! - `svd_decompose` - Singular value decomposition (A = U·Σ·Vᵀ)
//!
//! ### Decompositions (GPU f64 via WGSL)
//!
//! - `LuGpu::execute_f64()` - GPU LU decomposition with partial pivoting (f64)
//! - `QrGpu::execute_f64()` - GPU QR decomposition via Householder reflections (f64)
//! - `SvdGpu::execute_f64()` - GPU SVD via one-sided Jacobi (f64)
//! - `Cholesky` - GPU Cholesky decomposition (A = L·Lᵀ) (f32)
//! - `CholeskyF64` - GPU Cholesky decomposition with science-grade precision (f64)
//! - `Eigh` - GPU eigenvalue decomposition for symmetric matrices
//! - `BatchedEighGpu` - GPU batched eigenvalue decomposition for multiple matrices (f64)
//! - `GenEighGpu` - GPU generalized eigenvalue decomposition (Ax = λBx) (f64)
//!
//! ### Solvers
//!
//! - `LinSolve` - GPU linear system solve (A·x = b) (f32)
//! - `LinSolveF64` - GPU linear system solve with f64 precision
//! - `InverseF64` - GPU Gauss-Jordan matrix inverse (f64, N ≤ 32)
//! - `TriangularSolve` - GPU forward/backward substitution (f32)
//! - `TriangularSolveF64` - GPU triangular solve with f64 precision
//!   - Forward/backward substitution
//!   - Transpose solve (for Cholesky step 2: Lᵀ·x = z)
//!   - Complete `cholesky_solve()` pipeline
//! - `tridiagonal_solve` - Thomas algorithm for tridiagonal systems
//! - `cyclic_reduction_f64.wgsl` - O(log n) parallel tridiagonal for PDEs
//!
//! ## Design Principles
//!
//! - ✅ Full f64 precision via SPIR-V/Vulkan
//! - ✅ Pure WGSL (hardware-agnostic)
//! - ✅ Safe Rust (no unsafe blocks)
//! - ✅ Runtime-configured sizes
//! - ✅ Capability-based dispatch
//! - ✅ Composable operations

pub mod batched_eigh_gpu;
pub mod cholesky;
pub mod eigh;
pub mod eigh_f64;
pub mod gemm_f64;
pub mod gen_eigh_gpu;
pub mod grid_quadrature_gemm_f64; // EVOLVED: GPU Hamiltonian construction (Feb 16, 2026)
pub mod inverse_f64;
pub mod linsolve;
pub mod linsolve_f64;
pub mod lu;
pub mod lu_gpu;
pub mod qr;
pub mod qr_gpu;
pub mod svd;
pub mod svd_gpu;
pub mod triangular_solve;
pub mod tridiagonal;

pub use batched_eigh_gpu::BatchedEighGpu;
pub use cholesky::Cholesky;
pub use eigh::Eigh;
pub use eigh_f64::{eigh_householder_qr, EighDecompositionF64};
pub use gemm_f64::{GemmCachedF64, GemmF64};
pub use gen_eigh_gpu::{GenEighDecompositionGpu, GenEighGpu};
pub use grid_quadrature_gemm_f64::GridQuadratureGemm;
pub use inverse_f64::InverseF64;
pub use linsolve::LinSolve;
pub use linsolve_f64::LinSolveF64;
pub use lu::{lu_decompose, lu_det, lu_inverse, lu_solve, LuDecomposition};
pub use lu_gpu::LuGpu;
pub use qr::{qr_decompose, qr_least_squares, QrDecomposition};
pub use qr_gpu::QrGpu;
pub use svd::{svd_decompose, svd_pinv, svd_values, SvdDecomposition};
pub use svd_gpu::SvdGpu;
pub use triangular_solve::{TriangularSolve, TriangularSolveF64};
pub use tridiagonal::{tridiagonal_solve, tridiagonal_solve_batch, tridiagonal_solve_f32};

// Re-export f64 Cholesky
pub use cholesky::CholeskyF64;

/// WGSL kernel: symmetrize a square matrix (out[i,j] = (A[i,j] + A[j,i]) / 2).
pub const WGSL_SYMMETRIZE: &str = include_str!("../../shaders/linalg/symmetrize.wgsl");

/// WGSL kernel: graph Laplacian (L = D - A).
pub const WGSL_LAPLACIAN: &str = include_str!("../../shaders/linalg/laplacian.wgsl");
