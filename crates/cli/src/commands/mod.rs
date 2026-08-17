// SPDX-License-Identifier: AGPL-3.0-or-later
//! CLI command implementations
//!
//! This module contains command definitions and implementations of CLI subcommands.

mod definitions;
pub use definitions::{
    Commands, DeviceCommand, EcosystemCommands, ModeCommand, SovereignCommand, TransportCommands,
    UniversalCommands,
};

mod device;
pub mod dispatch;
pub mod doctor;
mod kernel_health;
mod mode;
pub mod sovereign;
pub mod transport;

/// NPU management commands (Akida hardware setup, status, listing)
#[cfg(feature = "npu")]
pub mod npu;
