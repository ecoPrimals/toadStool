//! CLI command implementations
//!
//! This module contains command definitions and implementations of CLI subcommands.

mod definitions;
pub use definitions::{Commands, EcosystemCommands, UniversalCommands};

pub mod doctor;

// NPU commands require akida-driver (optional)
#[cfg(feature = "npu")]
pub mod npu;
