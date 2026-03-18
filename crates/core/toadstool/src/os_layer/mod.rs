// SPDX-License-Identifier: AGPL-3.0-or-later
//! OS layer abstraction for biomeOS and platform compatibility.

/// BiomeOS integration and orchestration.
pub mod biome;
/// Platform-specific compatibility layers.
pub mod compat;
/// OS layer manager and configuration.
pub mod manager;
/// Platform detection and info.
pub mod platform;

// Export specific items to avoid ambiguity
pub use biome::BiomeOSIntegration;
pub use compat::{
    CompatibilityLayer as CompatLayer, LegacyCompatibilityLayer, LinuxCompatibilityLayer,
    MacOSCompatibilityLayer, WindowsCompatibilityLayer,
};
pub use manager::{CompatibilityLayer as ManagerCompatibilityLayer, OSLayerConfig, OSLayerManager};
pub use platform::PlatformInfo;
