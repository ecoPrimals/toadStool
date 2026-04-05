// SPDX-License-Identifier: AGPL-3.0-or-later
//! Compatibility layers for distributed execution
//!
//! This module re-exports the canonical compatibility layer implementation
//! from the core toadstool crate. All compatibility layer definitions are
//! now unified in `toadstool::os_layer::compat`.
//!
//! The previous duplicate implementation in `layers.rs` has been deprecated
//! in favor of the canonical core implementation.

// Re-export canonical compatibility layer from core
pub use toadstool::os_layer::compat::{
    CompatibilityLayer, LegacyCompatConfig, LegacyCompatibilityLayer, LinuxCompatConfig,
    LinuxCompatibilityLayer, MacOSCompatConfig, MacOSCompatibilityLayer, WindowsCompatConfig,
    WindowsCompatibilityLayer,
};

// Note: The old layers.rs file is deprecated and should not be used.
// All new code should import from toadstool::os_layer::compat or this module.
