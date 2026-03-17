// SPDX-License-Identifier: AGPL-3.0-only
#![deny(unsafe_code)]
#![allow(
    clippy::nursery,
    clippy::cast_precision_loss,
    clippy::map_unwrap_or,
    clippy::must_use_candidate
)]

//! Cross-substrate validation and benchmarking library

pub mod comprehensive_benchmark;

pub use comprehensive_benchmark::*;
