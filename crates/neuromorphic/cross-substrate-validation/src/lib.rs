// SPDX-License-Identifier: AGPL-3.0-or-later
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(
    clippy::cast_precision_loss,
    clippy::map_unwrap_or,
    clippy::must_use_candidate,
    reason = "benchmark numeric casts; map_unwrap_or reads better for scoring; ergonomic API"
)]

//! Cross-substrate validation and benchmarking library

pub mod comprehensive_benchmark;

#[cfg(test)]
mod comprehensive_benchmark_tests;

pub use comprehensive_benchmark::*;
