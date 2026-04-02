// SPDX-License-Identifier: AGPL-3.0-only

//! GPU-specific device personalities.
//!
//! Maps coralReef's `GpuPersonality` trait and personality variants into
//! toadStool's hardware-agnostic [`DevicePersonality`] trait.

use std::fmt;

use toadstool_glowplug::personality::{DevicePersonality, PersonalityRegistry};

/// GPU driver personality — what driver/mode a GPU is operating in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GpuPersonality {
    /// VFIO passthrough — direct hardware access, no kernel driver.
    Vfio,
    /// Nouveau open-source driver.
    Nouveau,
    /// NVIDIA proprietary driver.
    Nvidia,
    /// NVIDIA open kernel modules.
    NvidiaOpen,
    /// AMD GPU driver.
    Amdgpu,
    /// Intel Xe driver (modern).
    Xe,
    /// Intel i915 driver (legacy).
    I915,
    /// No driver bound.
    Unbound,
}

impl DevicePersonality for GpuPersonality {
    fn name(&self) -> &str {
        match self {
            Self::Vfio => "vfio",
            Self::Nouveau => "nouveau",
            Self::Nvidia => "nvidia",
            Self::NvidiaOpen => "nvidia-open",
            Self::Amdgpu => "amdgpu",
            Self::Xe => "xe",
            Self::I915 => "i915",
            Self::Unbound => "unbound",
        }
    }

    fn provides_direct_access(&self) -> bool {
        matches!(self, Self::Vfio)
    }

    fn driver_module(&self) -> Option<&str> {
        match self {
            Self::Vfio => Some("vfio-pci"),
            Self::Nouveau => Some("nouveau"),
            Self::Nvidia => Some("nvidia"),
            Self::NvidiaOpen => Some("nvidia"),
            Self::Amdgpu => Some("amdgpu"),
            Self::Xe => Some("xe"),
            Self::I915 => Some("i915"),
            Self::Unbound => None,
        }
    }

    fn capabilities(&self) -> &[&str] {
        match self {
            Self::Vfio => &["compute", "dma", "passthrough"],
            Self::Nouveau | Self::Nvidia | Self::NvidiaOpen => &["compute", "display", "video"],
            Self::Amdgpu => &["compute", "display", "video"],
            Self::Xe | Self::I915 => &["compute", "display"],
            Self::Unbound => &[],
        }
    }
}

impl fmt::Display for GpuPersonality {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "gpu:{}", self.name())
    }
}

/// Registry of GPU personalities available on Linux.
#[derive(Debug)]
pub struct GpuPersonalityRegistry;

impl GpuPersonalityRegistry {
    /// Create the default Linux GPU personality registry.
    #[must_use]
    pub const fn linux() -> Self {
        Self
    }
}

impl PersonalityRegistry for GpuPersonalityRegistry {
    type Personality = GpuPersonality;

    fn supported(&self) -> Vec<&str> {
        vec![
            "vfio",
            "nouveau",
            "nvidia",
            "nvidia-open",
            "amdgpu",
            "xe",
            "i915",
            "unbound",
        ]
    }

    fn create(&self, name: &str) -> Option<Self::Personality> {
        match name {
            "vfio" | "vfio-pci" => Some(GpuPersonality::Vfio),
            "nouveau" => Some(GpuPersonality::Nouveau),
            "nvidia" => Some(GpuPersonality::Nvidia),
            "nvidia-open" => Some(GpuPersonality::NvidiaOpen),
            "amdgpu" => Some(GpuPersonality::Amdgpu),
            "xe" => Some(GpuPersonality::Xe),
            "i915" => Some(GpuPersonality::I915),
            "unbound" | "" => Some(GpuPersonality::Unbound),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personality_names() {
        assert_eq!(GpuPersonality::Vfio.name(), "vfio");
        assert_eq!(GpuPersonality::Nouveau.name(), "nouveau");
        assert_eq!(GpuPersonality::Unbound.name(), "unbound");
    }

    #[test]
    fn registry_roundtrip() {
        let registry = GpuPersonalityRegistry::linux();
        for name in registry.supported() {
            let p = registry.create(name).expect(name);
            assert_eq!(p.name(), name);
        }
    }

    #[test]
    fn registry_unknown() {
        let registry = GpuPersonalityRegistry::linux();
        assert!(registry.create("nonexistent").is_none());
        assert!(!registry.supports("nonexistent"));
    }

    #[test]
    fn vfio_provides_direct_access() {
        assert!(GpuPersonality::Vfio.provides_direct_access());
        assert!(!GpuPersonality::Nouveau.provides_direct_access());
    }

    #[test]
    fn display_format() {
        assert_eq!(GpuPersonality::Amdgpu.to_string(), "gpu:amdgpu");
    }
}
