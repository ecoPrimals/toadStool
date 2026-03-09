// SPDX-License-Identifier: AGPL-3.0-only
//! Device abstraction for Burn inference
//!
//! Provides a unified interface for selecting compute devices (GPU, CPU).

use burn_wgpu::WgpuDevice;
use tracing::info;

/// Information about a compute device
#[derive(Debug, Clone)]
pub struct DeviceInfo {
    pub name: String,
    pub device_type: DeviceType,
    pub memory_bytes: Option<u64>,
}

/// Type of compute device
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    /// Discrete GPU (NVIDIA, AMD)
    DiscreteGpu,
    /// Integrated GPU
    IntegratedGpu,
    /// CPU fallback
    Cpu,
}

/// Burn device wrapper for runtime device selection
pub enum BurnDevice {
    /// wgpu backend (GPU)
    Wgpu(WgpuDevice),
    /// ndarray backend (CPU)
    Cpu,
}

impl BurnDevice {
    /// Auto-select the best available device
    pub fn auto_select() -> Self {
        // Try to get a GPU device
        if let Some(device) = Self::try_wgpu() {
            device
        } else {
            info!("No GPU available, falling back to CPU");
            Self::Cpu
        }
    }

    /// Try to create a wgpu device
    fn try_wgpu() -> Option<Self> {
        // Use default wgpu device selection
        let device = WgpuDevice::default();
        info!("Using wgpu device: {:?}", device);
        Some(Self::Wgpu(device))
    }

    /// Create a specific wgpu device by index
    #[must_use]
    pub const fn wgpu(index: usize) -> Self {
        let device = WgpuDevice::DiscreteGpu(index);
        Self::Wgpu(device)
    }

    /// Create a CPU device
    #[must_use]
    pub const fn cpu() -> Self {
        Self::Cpu
    }

    /// Get device info
    #[must_use]
    pub fn info(&self) -> DeviceInfo {
        match self {
            Self::Wgpu(device) => DeviceInfo {
                name: format!("{device:?}"),
                device_type: match device {
                    WgpuDevice::DiscreteGpu(_) => DeviceType::DiscreteGpu,
                    WgpuDevice::IntegratedGpu(_) => DeviceType::IntegratedGpu,
                    _ => DeviceType::Cpu,
                },
                memory_bytes: None, // Would need wgpu adapter query
            },
            Self::Cpu => DeviceInfo {
                name: "CPU (ndarray)".to_string(),
                device_type: DeviceType::Cpu,
                memory_bytes: None,
            },
        }
    }

    /// Check if this is a GPU device
    #[must_use]
    pub const fn is_gpu(&self) -> bool {
        matches!(self, Self::Wgpu(_))
    }
}

/// Enumerate all available devices
#[must_use]
pub fn enumerate_devices() -> Vec<DeviceInfo> {
    // Always have CPU available, plus wgpu GPU target
    vec![
        DeviceInfo {
            name: "CPU (ndarray)".to_string(),
            device_type: DeviceType::Cpu,
            memory_bytes: None,
        },
        // In a full implementation, we'd query wgpu::Instance for adapters
        DeviceInfo {
            name: "wgpu (auto)".to_string(),
            device_type: DeviceType::DiscreteGpu,
            memory_bytes: None,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auto_select() {
        let device = BurnDevice::auto_select();
        let info = device.info();
        println!(
            "Auto-selected device: {} ({:?})",
            info.name, info.device_type
        );
    }

    #[test]
    fn test_enumerate_devices() {
        let devices = enumerate_devices();
        assert!(!devices.is_empty());
        println!("Available devices:");
        for d in devices {
            println!("  - {} ({:?})", d.name, d.device_type);
        }
    }
}
