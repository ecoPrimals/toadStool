//! OS compatibility layer.
//!
//! Platform-specific compatibility implementations for Linux, Windows, macOS,
//! and legacy systems. Each layer provides isolation, resource control, and
//! execution compatibility for its target platform.

mod legacy;
mod linux;
mod macos;
mod trait_def;
mod windows;

// Re-export trait (canonical definition)
pub use trait_def::CompatibilityLayer;

// Re-export platform-specific layers and configs
pub use legacy::{LegacyCompatConfig, LegacyCompatibilityLayer};
pub use linux::{LinuxCompatConfig, LinuxCompatibilityLayer};
pub use macos::{MacOSCompatConfig, MacOSCompatibilityLayer};
pub use windows::{WindowsCompatConfig, WindowsCompatibilityLayer};

#[cfg(test)]
mod tests;
