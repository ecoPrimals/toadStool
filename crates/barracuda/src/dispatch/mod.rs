//! Auto-dispatch system for CPU/GPU routing
//!
//! Provides intelligent, size-based dispatch for mathematical operations.
//! Small workloads stay on CPU (avoiding dispatch overhead), large workloads
//! use GPU acceleration.
//!
//! # Dual-Precision Architecture
//!
//! ```text
//! User calls: erf(x) or matmul(a, b)
//!       ↓
//! DispatchConfig checks:
//! - Input size vs threshold
//! - GPU availability
//! - Force flags
//!       ↓
//! Routes to:
//! - CPU f64 (small N, precision-critical)
//! - GPU f32 (large N, throughput-critical)
//! ```
//!
//! # Per-Function Thresholds
//!
//! Thresholds are empirically determined via benchmarking:
//!
//! | Operation | CPU Threshold | Reason |
//! |-----------|---------------|--------|
//! | erf | 512 | GPU dispatch overhead ~0.1ms |
//! | matmul | 64 | GPU wins at 64×64 matrices |
//! | eigh | 128 | Jacobi iteration memory-bound |
//! | cdist | 200 | Distance computation O(N²) |
//! | fft | 1024 | FFT benefits from parallelism |
//!
//! # Example
//!
//! ```
//! use barracuda::dispatch::{Dispatch, DispatchConfig};
//!
//! // Configure dispatch
//! let config = DispatchConfig::default();
//!
//! // Auto-route based on size
//! if config.should_use_gpu(1000, "matmul") {
//!     // GPU path
//! } else {
//!     // CPU path
//! }
//! ```
//!
//! # Benchmarking
//!
//! Use the benchmark module to empirically determine optimal thresholds:
//!
//! ```ignore
//! use barracuda::dispatch::benchmark::{BenchmarkSuite, BenchmarkConfig};
//!
//! let suite = BenchmarkSuite::new(BenchmarkConfig::default());
//! let results = suite.run_all()?;
//! println!("{}", results.summary());
//!
//! // Update thresholds based on results
//! let new_thresholds = results.optimal_thresholds();
//! ```

pub mod benchmark;
mod config;
pub mod domain_ops;

pub use benchmark::{
    BenchmarkConfig, BenchmarkResult, BenchmarkSuite, OperationBenchmark, ThresholdResult,
};
pub use config::{
    batch_fitness_substrate, dispatch_for, dispatch_with_config, global_config, hmm_substrate,
    ode_substrate, pairwise_substrate, spatial_substrate, Dispatch, DispatchConfig, DispatchTarget,
    DEFAULT_THRESHOLD,
};
pub use domain_ops::{
    frobenius_norm_dispatch, gelu_dispatch, hmm_forward_dispatch, l2_distance_dispatch,
    matmul_dispatch, mean_dispatch, softmax_dispatch, transpose_dispatch, variance_dispatch,
};
