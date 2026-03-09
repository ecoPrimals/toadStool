// SPDX-License-Identifier: AGPL-3.0-only
pub mod biome;
pub mod compat;
pub mod manager;
pub mod platform;

// Export specific items to avoid ambiguity
pub use biome::BiomeOSIntegration;
pub use compat::{
    CompatibilityLayer as CompatLayer, LegacyCompatibilityLayer, LinuxCompatibilityLayer,
    MacOSCompatibilityLayer, WindowsCompatibilityLayer,
};
pub use manager::{CompatibilityLayer as ManagerCompatibilityLayer, OSLayerConfig, OSLayerManager};
pub use platform::PlatformInfo;
