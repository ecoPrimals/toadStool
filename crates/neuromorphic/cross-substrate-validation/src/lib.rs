// SPDX-License-Identifier: AGPL-3.0-only
#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(clippy::map_unwrap_or, clippy::must_use_candidate)]

//! Cross-substrate validation and benchmarking library

pub mod comprehensive_benchmark;

pub use comprehensive_benchmark::*;
