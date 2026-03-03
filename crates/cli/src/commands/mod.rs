// SPDX-License-Identifier: AGPL-3.0-or-later
//! CLI command implementations
//!
//! This module contains command definitions and implementations of CLI subcommands.

mod definitions;
pub use definitions::{Commands, EcosystemCommands, UniversalCommands};

pub mod dispatch;
pub mod doctor;

// NPU commands require akida-driver (optional)
#[cfg(feature = "npu")]
pub mod npu;
