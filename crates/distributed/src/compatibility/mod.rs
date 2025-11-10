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
    CompatibilityLayer,
    LinuxCompatibilityLayer, LinuxCompatConfig,
    WindowsCompatibilityLayer, WindowsCompatConfig,
    MacOSCompatibilityLayer, MacOSCompatConfig,
    LegacyCompatibilityLayer, LegacyCompatConfig,
};

// Note: The old layers.rs file is deprecated and should not be used.
// All new code should import from toadstool::os_layer::compat or this module.
