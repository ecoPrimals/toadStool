//! Substrate Selection - Explicit Hardware Targeting
//!
//! **Deep Debt Principles**:
//! - ✅ Agnostic & Capability-Based (runtime discovery + selection)
//! - ✅ Modern Idiomatic Rust (enums, pattern matching)
//! - ✅ Safe Rust (zero unsafe)
//! - ✅ Self-Knowledge (substrate discovers own capabilities)
//!
//! **Purpose**: Enable explicit hardware selection for validation and testing

use crate::device::WgpuDevice;
use crate::error::Result;
use serde::{Deserialize, Serialize};

/// Compute substrate type (hardware target)
///
/// **Deep Debt**: Runtime-discoverable, no hardcoding
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubstrateType {
    /// CPU execution (any socket)
    Cpu,
    /// NVIDIA GPU (any generation)
    NvidiaGpu,
    /// AMD GPU (any generation)
    AmdGpu,
    /// Intel GPU
    IntelGpu,
    /// Apple GPU (Metal)
    AppleGpu,
    /// NPU (neuromorphic processor)
    Npu,
    /// Other/Unknown
    Other,
}

/// Specific compute substrate instance
///
/// **Deep Debt**: Capability-based (each substrate knows its capabilities)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Substrate {
    /// Substrate type
    pub substrate_type: SubstrateType,
    /// Human-readable name (e.g., "NVIDIA GeForce RTX 3090")
    pub name: String,
    /// Backend (Vulkan, DX12, Metal, OpenGL)
    pub backend: String,
    /// Device index (for multiple instances of same type)
    pub index: usize,
}

impl Substrate {
    /// Create substrate descriptor
    pub fn new(
        substrate_type: SubstrateType,
        name: String,
        backend: String,
        index: usize,
    ) -> Self {
        Self {
            substrate_type,
            name,
            backend,
            index,
        }
    }

    /// Discover all available substrates
    ///
    /// **Deep Debt**: Runtime discovery, no hardcoding
    pub fn discover_all() -> Result<Vec<Self>> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapters = instance.enumerate_adapters(wgpu::Backends::all());

        let mut substrates = Vec::new();
        let mut type_counts = std::collections::HashMap::new();

        for adapter in adapters {
            let info = adapter.get_info();

            // Skip CPU software renderers for now (we'll add explicit CPU later)
            if info.device_type == wgpu::DeviceType::Cpu {
                continue;
            }

            // Determine substrate type from vendor and name
            let substrate_type = Self::classify_substrate(&info);

            // Get index for this substrate type
            let index = type_counts.entry(substrate_type).or_insert(0);
            *index += 1;

            substrates.push(Self {
                substrate_type,
                name: info.name.clone(),
                backend: format!("{:?}", info.backend),
                index: *index - 1,
            });
        }

        Ok(substrates)
    }

    /// Classify substrate type from adapter info
    ///
    /// **Deep Debt**: Self-discovery based on vendor/name patterns
    fn classify_substrate(info: &wgpu::AdapterInfo) -> SubstrateType {
        let name_lower = info.name.to_lowercase();

        // NVIDIA detection
        if name_lower.contains("nvidia")
            || name_lower.contains("geforce")
            || name_lower.contains("quadro")
            || name_lower.contains("tesla")
        {
            return SubstrateType::NvidiaGpu;
        }

        // AMD detection
        if name_lower.contains("amd")
            || name_lower.contains("radeon")
            || name_lower.contains("rx ")
            || name_lower.contains("vega")
            || name_lower.contains("navi")
        {
            return SubstrateType::AmdGpu;
        }

        // Intel detection
        if name_lower.contains("intel")
            || name_lower.contains("hd graphics")
            || name_lower.contains("iris")
            || name_lower.contains("arc")
        {
            return SubstrateType::IntelGpu;
        }

        // Apple detection
        if name_lower.contains("apple")
            || name_lower.contains("m1")
            || name_lower.contains("m2")
            || name_lower.contains("m3")
        {
            return SubstrateType::AppleGpu;
        }

        // NPU detection (placeholder - needs custom detection)
        if name_lower.contains("npu") || name_lower.contains("akida") {
            return SubstrateType::Npu;
        }

        SubstrateType::Other
    }

    /// Create WgpuDevice on this specific substrate
    ///
    /// **Deep Debt**: Explicit selection, no implicit behavior
    pub async fn create_device(&self) -> Result<WgpuDevice> {
        let substrate_type = self.substrate_type;
        let _target_index = self.index; // Reserved for future multi-device support

        WgpuDevice::new_with_filter(wgpu::Backends::all(), move |info| {
            let detected_type = Self::classify_substrate(info);
            if detected_type != substrate_type {
                return false;
            }

            // TODO: Match specific index for multi-device setups
            // For now, we match first device of this type
            true
        })
        .await
    }
}

impl std::fmt::Display for Substrate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{:?}[{}]: {} ({})",
            self.substrate_type, self.index, self.name, self.backend
        )
    }
}

impl std::fmt::Display for SubstrateType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SubstrateType::Cpu => write!(f, "CPU"),
            SubstrateType::NvidiaGpu => write!(f, "NVIDIA GPU"),
            SubstrateType::AmdGpu => write!(f, "AMD GPU"),
            SubstrateType::IntelGpu => write!(f, "Intel GPU"),
            SubstrateType::AppleGpu => write!(f, "Apple GPU"),
            SubstrateType::Npu => write!(f, "NPU"),
            SubstrateType::Other => write!(f, "Other"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_substrate_discovery() {
        let substrates = Substrate::discover_all().unwrap();
        println!("Discovered {} substrates:", substrates.len());
        for substrate in &substrates {
            println!("  - {}", substrate);
        }
        assert!(!substrates.is_empty(), "Should discover at least one substrate");
    }

    #[tokio::test]
    async fn test_substrate_device_creation() {
        let substrates = Substrate::discover_all().unwrap();
        if let Some(substrate) = substrates.first() {
            println!("Testing device creation on: {}", substrate);
            let device = substrate.create_device().await.unwrap();
            println!("✓ Created device: {}", device.name());
        }
    }
}
