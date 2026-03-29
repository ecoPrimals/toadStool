// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]
#![warn(missing_docs)]

//! Pure Rust `NeuroBench` Harness
//!
//! This crate provides a Rust implementation of `NeuroBench` benchmark suites
//! for neuromorphic hardware evaluation. It interfaces with the Akida NPU
//! through the akida-driver crate.
//!
//! # Supported Benchmarks
//!
//! - **DVS Gesture**: Dynamic Vision Sensor gesture recognition
//! - **Keyword FSCIL**: Few-shot keyword spotting
//! - **Chaotic Function**: Chaotic time series prediction (ESN workload)
//! - **NHP Motor**: Neural prosthetics motor prediction
//!
//! # Example
//!
//! ```ignore
//! use neurobench_runner::{Harness, BenchmarkConfig, Benchmark};
//!
//! let harness = Harness::new("0000:a1:00.0")?;
//! let config = BenchmarkConfig::default();
//! let result = harness.run(Benchmark::DvsGesture, &config)?;
//! println!("Accuracy: {:.2}%", result.accuracy * 100.0);
//! ```

pub mod benchmarks;
pub mod data;
pub mod harness;
pub mod metrics;

pub use benchmarks::{Benchmark, BenchmarkConfig, BenchmarkResult};
pub use harness::{Harness, HarnessConfig};
pub use metrics::{LatencyMetrics, Metrics, PowerMetrics};

/// Crate-level error type
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// NPU or hardware initialization failed.
    #[error("Hardware initialization failed: {0}")]
    HardwareInit(String),

    /// Benchmark execution failed.
    #[error("Benchmark failed: {0}")]
    BenchmarkFailed(String),

    /// Dataset or sample loading failed.
    #[error("Data loading failed: {0}")]
    DataLoad(String),

    /// Model loading onto NPU failed.
    #[error("Model loading failed: {0}")]
    ModelLoad(String),

    /// Error from the Akida NPU driver.
    #[error("NPU error: {0}")]
    Npu(#[from] akida_driver::AkidaError),

    /// I/O error during file or device access.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Result type for neurobench-runner operations.
pub type Result<T> = std::result::Result<T, Error>;
