// SPDX-License-Identifier: AGPL-3.0-or-later

//! GPU device discovery via PCI sysfs.
//!
//! Implements [`DeviceDiscovery`] for GPU-class PCI devices. Scans
//! `/sys/bus/pci/devices/` for VGA-compatible controllers (class 0x0300)
//! and 3D controllers (class 0x0302).

use toadstool_glowplug::device_id::DeviceId;
use toadstool_glowplug::discovery::DeviceDiscovery;

/// PCI class codes for GPU devices.
const GPU_PCI_CLASSES: &[&str] = &[
    "0x030000", // VGA-compatible controller
    "0x030200", // 3D controller (e.g. Tesla/compute-only GPUs)
    "0x030100", // XGA controller (rare)
];

/// Discovers GPU devices via PCI sysfs enumeration.
#[derive(Debug, Default)]
pub struct GpuDiscovery;

impl GpuDiscovery {
    /// Create a new GPU discovery instance.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl DeviceDiscovery for GpuDiscovery {
    type Error = std::io::Error;

    fn hardware_class(&self) -> &str {
        "gpu"
    }

    async fn discover(&self) -> Result<Vec<DeviceId>, Self::Error> {
        let mut gpus = Vec::new();
        let pci_dir = std::path::Path::new("/sys/bus/pci/devices");

        if !pci_dir.exists() {
            return Ok(gpus);
        }

        let entries = std::fs::read_dir(pci_dir)?;
        for entry in entries {
            let entry = entry?;
            let bdf = entry.file_name().to_string_lossy().to_string();
            let class_path = entry.path().join("class");

            if let Ok(class) = std::fs::read_to_string(&class_path) {
                let class = class.trim();
                if GPU_PCI_CLASSES.iter().any(|&c| class.starts_with(c)) {
                    gpus.push(DeviceId::PciBdf(bdf));
                }
            }
        }

        gpus.sort_by_key(std::string::ToString::to_string);
        Ok(gpus)
    }

    async fn is_present(&self, id: &DeviceId) -> Result<bool, Self::Error> {
        match id {
            DeviceId::PciBdf(bdf) => {
                let path = format!("/sys/bus/pci/devices/{bdf}");
                Ok(std::path::Path::new(&path).exists())
            }
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn discovery_does_not_panic() {
        let disc = GpuDiscovery::new();
        let result = disc.discover().await;
        // May find 0 GPUs in CI, but should not error
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn nonexistent_device_not_present() {
        let disc = GpuDiscovery::new();
        let id = DeviceId::PciBdf("ffff:ff:ff.f".into());
        assert!(!disc.is_present(&id).await.unwrap());
    }
}
