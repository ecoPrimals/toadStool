// SPDX-License-Identifier: AGPL-3.0-or-later

//! GPU-specific device personalities.
//!
//! Maps the visualization service's `GpuPersonality` trait and personality
//! variants into toadStool's hardware-agnostic [`DevicePersonality`] trait.

use std::fmt;

use toadstool_glowplug::personality::{DevicePersonality, PersonalityRegistry};

/// GPU driver personality — what driver/mode a GPU is operating in.
///
/// Unified from `coral-glowplug::Personality` — covers all vendor
/// driver modes encountered across NVIDIA, AMD, Intel, and BrainChip.
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
    /// NVIDIA oracle module (custom/experimental kernel module).
    NvidiaOracle {
        /// The specific oracle module name (e.g. "nvidia_oracle_v2").
        module_name: String,
    },
    /// AMD GPU driver.
    Amdgpu,
    /// Intel Xe driver (modern).
    Xe,
    /// Intel i915 driver (legacy).
    I915,
    /// BrainChip Akida neuromorphic accelerator.
    Akida,
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
            Self::NvidiaOracle { .. } => "nvidia-oracle",
            Self::Amdgpu => "amdgpu",
            Self::Xe => "xe",
            Self::I915 => "i915",
            Self::Akida => "akida",
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
            Self::Nvidia | Self::NvidiaOpen => Some("nvidia"),
            Self::NvidiaOracle { module_name } => Some(module_name.as_str()),
            Self::Amdgpu => Some("amdgpu"),
            Self::Xe => Some("xe"),
            Self::I915 => Some("i915"),
            Self::Akida => Some("akida-pcie"),
            Self::Unbound => None,
        }
    }

    fn capabilities(&self) -> &[&str] {
        match self {
            Self::Vfio => &["compute", "dma", "passthrough"],
            Self::Nouveau | Self::Nvidia | Self::NvidiaOpen | Self::NvidiaOracle { .. } => {
                &["compute", "display", "video"]
            }
            Self::Amdgpu => &["compute", "display", "video"],
            Self::Xe | Self::I915 => &["compute", "display"],
            Self::Akida => &["neuromorphic", "inference"],
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
            "nvidia-oracle",
            "amdgpu",
            "xe",
            "i915",
            "akida",
            "unbound",
        ]
    }

    fn create(&self, name: &str) -> Option<Self::Personality> {
        match name {
            "vfio" | "vfio-pci" => Some(GpuPersonality::Vfio),
            "nouveau" => Some(GpuPersonality::Nouveau),
            "nvidia" => Some(GpuPersonality::Nvidia),
            "nvidia-open" => Some(GpuPersonality::NvidiaOpen),
            n if n.starts_with("nvidia_oracle") || n == "nvidia-oracle" => {
                Some(GpuPersonality::NvidiaOracle {
                    module_name: n.to_string(),
                })
            }
            "amdgpu" => Some(GpuPersonality::Amdgpu),
            "xe" => Some(GpuPersonality::Xe),
            "i915" => Some(GpuPersonality::I915),
            "akida" | "akida-pcie" => Some(GpuPersonality::Akida),
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
        assert_eq!(GpuPersonality::Akida.name(), "akida");
        assert_eq!(GpuPersonality::Unbound.name(), "unbound");
    }

    #[test]
    fn nvidia_oracle_name_and_module() {
        let p = GpuPersonality::NvidiaOracle {
            module_name: "nvidia_oracle_v2".into(),
        };
        assert_eq!(p.name(), "nvidia-oracle");
        assert_eq!(p.driver_module(), Some("nvidia_oracle_v2"));
    }

    #[test]
    fn akida_capabilities() {
        let caps = GpuPersonality::Akida.capabilities();
        assert!(caps.contains(&"neuromorphic"));
        assert!(caps.contains(&"inference"));
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
    fn registry_oracle_prefix() {
        let registry = GpuPersonalityRegistry::linux();
        let p = registry.create("nvidia_oracle_v3").expect("oracle");
        assert_eq!(p.name(), "nvidia-oracle");
    }

    #[test]
    fn registry_akida_aliases() {
        let registry = GpuPersonalityRegistry::linux();
        assert!(registry.create("akida").is_some());
        assert!(registry.create("akida-pcie").is_some());
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
        assert!(!GpuPersonality::Akida.provides_direct_access());
    }

    #[test]
    fn display_format() {
        assert_eq!(GpuPersonality::Amdgpu.to_string(), "gpu:amdgpu");
        assert_eq!(GpuPersonality::Akida.to_string(), "gpu:akida");
    }

    #[test]
    fn all_variants_have_driver_module_or_none() {
        let registry = GpuPersonalityRegistry::linux();
        for name in registry.supported() {
            let p = registry.create(name).unwrap();
            if name == "unbound" {
                assert!(p.driver_module().is_none());
            } else {
                assert!(
                    p.driver_module().is_some(),
                    "{name} should have a driver module"
                );
            }
        }
    }
}
