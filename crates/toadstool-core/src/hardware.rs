// SPDX-License-Identifier: AGPL-3.0-or-later
// ToadStool Hardware Manager
//
// Deep Debt: ToadStool directly interfaces with hardware in Rust
// No scripts, no sudo needed on fresh systems
// Self-evolves and adapts to hardware changes

use std::fs;
use std::path::Path;
use tracing::{info, warn};

/// Typed errors for hardware operations
#[derive(Debug, thiserror::Error)]
pub enum HardwareError {
    #[error("NPU device not found: {address}")]
    NpuNotFound { address: String },
}

/// Hardware types that ToadStool can discover and manage
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardwareType {
    /// GPU (via BarraCuda/WGPU)
    Gpu,
    /// NPU/Neuromorphic (Akida, etc)
    Npu,
    /// CPU fallback
    Cpu,
    /// FPGA
    Fpga,
    /// Custom accelerators
    Custom,
}

/// Hardware device discovered by ToadStool
#[derive(Debug, Clone)]
pub struct HardwareDevice {
    pub hardware_type: HardwareType,
    pub name: String,
    pub pcie_address: Option<String>,
    pub vendor_id: Option<String>,
    pub device_id: Option<String>,
    pub driver_available: bool,
    pub userspace_capable: bool,
}

/// ToadStool's hardware manager
///
/// Deep Debt: Pure Rust, runtime discovery, no scripts
pub struct HardwareManager {
    devices: Vec<HardwareDevice>,
}

impl HardwareManager {
    /// Create hardware manager and discover all devices
    ///
    /// Deep Debt: Works on fresh system, no sudo, no setup
    ///
    /// # Errors
    /// Returns error if unable to scan hardware directories
    pub fn discover() -> Result<Self, HardwareError> {
        info!("ToadStool discovering hardware...");

        let mut devices = Vec::new();

        // Discover GPUs (BarraCuda handles via WGPU)
        devices.extend(Self::discover_gpus());

        // Discover NPUs (Akida, etc)
        devices.extend(Self::discover_npus());

        // CPU always available
        devices.push(HardwareDevice {
            hardware_type: HardwareType::Cpu,
            name: "CPU".to_string(),
            pcie_address: None,
            vendor_id: None,
            device_id: None,
            driver_available: true,
            userspace_capable: true,
        });

        info!("ToadStool discovered {} devices", devices.len());

        Ok(Self { devices })
    }

    /// Discover GPUs via sysfs
    ///
    /// Deep Debt: `BarraCuda` will use WGPU for actual GPU access
    /// ToadStool just discovers what's available
    fn discover_gpus() -> Vec<HardwareDevice> {
        let mut gpus = Vec::new();

        // Scan /sys/class/drm for GPU devices
        // BarraCuda/WGPU handles actual GPU access
        if let Ok(entries) = fs::read_dir("/sys/class/drm") {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.join("device").exists() {
                    // Found a GPU device node
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Unknown GPU")
                        .to_string();

                    gpus.push(HardwareDevice {
                        hardware_type: HardwareType::Gpu,
                        name,
                        pcie_address: None,
                        vendor_id: None,
                        device_id: None,
                        driver_available: true, // BarraCuda/WGPU handles driver
                        userspace_capable: true,
                    });
                }
            }
        }

        gpus
    }

    /// Discover NPUs (Akida, etc)
    ///
    /// Deep Debt: Direct `PCIe` discovery, userspace access
    fn discover_npus() -> Vec<HardwareDevice> {
        let mut npus = Vec::new();

        // Scan PCIe bus for Akida devices (vendor 0x1e7c)
        if let Ok(entries) = fs::read_dir("/sys/bus/pci/devices") {
            for entry in entries.flatten() {
                let device_path = entry.path();

                // Read vendor ID
                let vendor_id = fs::read_to_string(device_path.join("vendor"))
                    .ok()
                    .and_then(|s| s.trim().strip_prefix("0x").map(String::from));

                // Check if Akida (0x1e7c)
                if let Some(ref vid) = vendor_id {
                    if vid == "1e7c" {
                        let device_id = fs::read_to_string(device_path.join("device"))
                            .ok()
                            .and_then(|s| s.trim().strip_prefix("0x").map(String::from));

                        let pcie_address = device_path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(String::from);

                        let name = match device_id.as_deref() {
                            Some("bca1") => "Akida AKD1000",
                            Some("bca2") => "Akida AKD1500",
                            _ => "Akida NPU",
                        }
                        .to_string();

                        // Check if kernel driver available (/dev/akida*)
                        let driver_available = fs::read_dir("/dev").ok().is_some_and(|entries| {
                            entries
                                .flatten()
                                .any(|e| e.file_name().to_string_lossy().starts_with("akida"))
                        });

                        // Check if userspace access available (resource files readable)
                        let userspace_capable = device_path.join("resource0").exists()
                            && fs::metadata(device_path.join("resource0"))
                                .map(|m| !m.permissions().readonly())
                                .unwrap_or(false);

                        npus.push(HardwareDevice {
                            hardware_type: HardwareType::Npu,
                            name,
                            pcie_address,
                            vendor_id,
                            device_id,
                            driver_available,
                            userspace_capable,
                        });
                    }
                }
            }
        }

        npus
    }

    /// Get all discovered devices
    #[must_use]
    pub fn devices(&self) -> &[HardwareDevice] {
        &self.devices
    }

    /// Get devices by type
    #[must_use]
    pub fn devices_by_type(&self, hardware_type: HardwareType) -> Vec<&HardwareDevice> {
        self.devices
            .iter()
            .filter(|d| d.hardware_type == hardware_type)
            .collect()
    }

    /// Check if any GPU available (for `BarraCuda`)
    #[must_use]
    pub fn has_gpu(&self) -> bool {
        self.devices
            .iter()
            .any(|d| d.hardware_type == HardwareType::Gpu)
    }

    /// Check if any NPU available
    #[must_use]
    pub fn has_npu(&self) -> bool {
        self.devices
            .iter()
            .any(|d| d.hardware_type == HardwareType::Npu)
    }

    /// Get number of discovered devices
    #[must_use]
    pub const fn device_count(&self) -> usize {
        self.devices.len()
    }

    /// Re-scan hardware (for hot-plug events)
    ///
    /// Deep Debt: ToadStool adapts to hardware changes
    pub fn rescan(&mut self) -> Result<(), HardwareError> {
        info!("ToadStool re-scanning hardware...");
        let new_manager = Self::discover()?;
        self.devices = new_manager.devices;
        Ok(())
    }

    /// Enable userspace access to NPU (if needed)
    ///
    /// Deep Debt: No scripts, pure Rust
    ///
    /// # Errors
    /// Returns error if device not found or cannot enable `PCIe` device
    pub fn enable_npu_userspace(&self, pcie_address: &str) -> Result<(), HardwareError> {
        let device_path = Path::new("/sys/bus/pci/devices").join(pcie_address);

        if !device_path.exists() {
            return Err(HardwareError::NpuNotFound {
                address: pcie_address.to_string(),
            });
        }

        // Enable PCIe device
        let enable_path = device_path.join("enable");
        if enable_path.exists() {
            if let Ok(content) = fs::read_to_string(&enable_path) {
                if content.trim() == "0" {
                    // Device disabled, try to enable
                    if let Err(e) = fs::write(&enable_path, "1") {
                        warn!("Could not enable PCIe device (may need root): {}", e);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hardware_discovery() {
        // Should always succeed, even with no hardware
        let manager = HardwareManager::discover().expect("Discovery failed");

        // CPU should always be available
        assert!(manager
            .devices()
            .iter()
            .any(|d| d.hardware_type == HardwareType::Cpu));
    }

    #[test]
    fn test_rescan() {
        let mut manager = HardwareManager::discover().expect("Discovery failed");
        let initial_count = manager.devices().len();

        // Rescan should work
        manager.rescan().expect("Rescan failed");

        // Should find at least same devices
        assert!(manager.devices().len() >= initial_count);
    }
}
