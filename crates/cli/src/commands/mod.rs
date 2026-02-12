//! CLI command implementations
//!
//! This module contains the implementations of CLI subcommands.

pub mod doctor;

// NPU commands require akida-driver (optional)
#[cfg(feature = "npu")]
pub mod npu;
