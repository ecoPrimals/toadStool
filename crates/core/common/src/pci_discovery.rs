// SPDX-License-Identifier: AGPL-3.0-only
//! Unified PCI device discovery via sysfs.
//!
//! Shared scanner for all toadStool crates that need PCI device discovery
//! (nvpmu for GPUs, akida-driver for NPUs, etc.). Each consumer provides
//! a [`PciFilter`] describing which devices it's interested in.
//!
//! # Architecture
//!
//! This module knows nothing about GPU or NPU specifics — it only knows
//! how to scan `/sys/bus/pci/devices/` and filter by vendor/device/class.
//! Consumers enrich the results with domain-specific knowledge.
//!
//! # Usage
//!
//! ```rust,no_run
//! use toadstool_common::pci_discovery::{PciFilter, discover_pci_devices};
//!
//! let nvidia_gpus = discover_pci_devices(&PciFilter {
//!     vendor_id: Some(0x10de),
//!     class_match: Some(Box::new(|c| {
//!         let masked = c & 0x00FF_FF00;
//!         masked == 0x0003_0000 || masked == 0x0003_0200
//!     })),
//!     ..Default::default()
//! });
//! ```

use std::path::{Path, PathBuf};

/// A discovered PCI device with identity from sysfs.
#[derive(Debug, Clone)]
pub struct PciDevice {
    /// PCI Bus-Device-Function address (e.g. "0000:65:00.0").
    pub bdf: String,
    /// PCI vendor ID (e.g. 0x10de for NVIDIA, 0x1e96 for Brainchip).
    pub vendor_id: u16,
    /// PCI device ID.
    pub device_id: u16,
    /// PCI class code (24-bit).
    pub class_code: u32,
    /// Currently bound driver name, if any.
    pub driver: Option<String>,
    /// sysfs device directory path.
    pub sysfs_path: PathBuf,
}

/// Filter criteria for PCI discovery.
///
/// All `Some` fields must match. `None` fields are ignored (wildcard).
#[derive(Default)]
pub struct PciFilter {
    /// Required vendor ID (None = any vendor).
    pub vendor_id: Option<u16>,
    /// Required device IDs (empty = any device).
    pub device_ids: Vec<u16>,
    /// Class code predicate (None = any class).
    pub class_match: Option<Box<dyn Fn(u32) -> bool>>,
}

impl PciFilter {
    /// Create a filter matching a single vendor ID.
    #[must_use]
    pub fn vendor(vendor_id: u16) -> Self {
        Self {
            vendor_id: Some(vendor_id),
            ..Default::default()
        }
    }

    /// Add a class code predicate.
    #[must_use]
    pub fn with_class(mut self, predicate: impl Fn(u32) -> bool + 'static) -> Self {
        self.class_match = Some(Box::new(predicate));
        self
    }

    /// Restrict to specific device IDs.
    #[must_use]
    pub fn with_device_ids(mut self, ids: Vec<u16>) -> Self {
        self.device_ids = ids;
        self
    }

    fn matches(&self, vendor: u16, device: u16, class: u32) -> bool {
        if let Some(v) = self.vendor_id {
            if vendor != v {
                return false;
            }
        }
        if !self.device_ids.is_empty() && !self.device_ids.contains(&device) {
            return false;
        }
        if let Some(ref pred) = self.class_match {
            if !pred(class) {
                return false;
            }
        }
        true
    }
}

/// Scan `/sys/bus/pci/devices/` and return all devices matching the filter.
///
/// Returns an empty vec if sysfs is unavailable (e.g. non-Linux).
/// Devices are sorted by BDF address for deterministic ordering.
#[must_use]
pub fn discover_pci_devices(filter: &PciFilter) -> Vec<PciDevice> {
    let pci_dir = Path::new("/sys/bus/pci/devices");
    if !pci_dir.exists() {
        return Vec::new();
    }

    let Ok(entries) = std::fs::read_dir(pci_dir) else {
        return Vec::new();
    };

    let mut devices = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let bdf = entry.file_name().to_string_lossy().into_owned();

        let Some(vendor) = read_hex_u16(&path.join("vendor")) else {
            continue;
        };
        let Some(device) = read_hex_u16(&path.join("device")) else {
            continue;
        };
        let class = read_hex_u32(&path.join("class")).unwrap_or(0);

        if !filter.matches(vendor, device, class) {
            continue;
        }

        let driver = read_driver_name(&path);

        devices.push(PciDevice {
            bdf,
            vendor_id: vendor,
            device_id: device,
            class_code: class,
            driver,
            sysfs_path: path,
        });
    }

    devices.sort_by(|a, b| a.bdf.cmp(&b.bdf));
    devices
}

fn read_hex_u16(path: &Path) -> Option<u16> {
    let s = std::fs::read_to_string(path).ok()?;
    let s = s.trim().strip_prefix("0x").unwrap_or(s.trim());
    u16::from_str_radix(s, 16).ok()
}

fn read_hex_u32(path: &Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    let s = s.trim().strip_prefix("0x").unwrap_or(s.trim());
    u32::from_str_radix(s, 16).ok()
}

fn read_driver_name(device_path: &Path) -> Option<String> {
    std::fs::read_link(device_path.join("driver"))
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

/// Well-known PCI vendor IDs used in ecoPrimals.
pub mod vendors {
    pub const NVIDIA: u16 = 0x10de;
    pub const BRAINCHIP: u16 = 0x1e96;
    pub const AMD: u16 = 0x1002;
    pub const INTEL: u16 = 0x8086;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_filter_matches_anything() {
        let filter = PciFilter::default();
        assert!(filter.matches(0x10de, 0x1D81, 0x0003_0200));
        assert!(filter.matches(0x1e96, 0x0001, 0x0000_0000));
    }

    #[test]
    fn vendor_filter() {
        let filter = PciFilter::vendor(0x10de);
        assert!(filter.matches(0x10de, 0x1D81, 0x0003_0200));
        assert!(!filter.matches(0x1e96, 0x0001, 0x0000_0000));
    }

    #[test]
    fn device_id_filter() {
        let filter = PciFilter::vendor(0x10de).with_device_ids(vec![0x1D81, 0x2204]);
        assert!(filter.matches(0x10de, 0x1D81, 0x0003_0200));
        assert!(filter.matches(0x10de, 0x2204, 0x0003_0200));
        assert!(!filter.matches(0x10de, 0x9999, 0x0003_0200));
    }

    #[test]
    fn class_filter() {
        let filter = PciFilter::vendor(0x10de).with_class(|c| {
            let masked = c & 0x00FF_FF00;
            masked == 0x0003_0000 || masked == 0x0003_0200
        });
        assert!(filter.matches(0x10de, 0x1D81, 0x0003_0200));
        assert!(!filter.matches(0x10de, 0x1D81, 0x0006_0400));
    }

    #[test]
    fn discover_runs_without_hardware() {
        let filter = PciFilter::vendor(0xFFFF);
        let devices = discover_pci_devices(&filter);
        assert!(devices.is_empty());
    }

    #[test]
    fn vendor_constants() {
        assert_eq!(vendors::NVIDIA, 0x10de);
        assert_eq!(vendors::BRAINCHIP, 0x1e96);
        assert_eq!(vendors::AMD, 0x1002);
        assert_eq!(vendors::INTEL, 0x8086);
    }
}
