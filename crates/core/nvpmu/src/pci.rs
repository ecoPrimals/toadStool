// SPDX-License-Identifier: AGPL-3.0-or-later
//! PCI device discovery for NVIDIA GPUs via sysfs.
//!
//! Scans `/sys/bus/pci/devices/` for devices with PCI vendor ID `0x10de`
//! (NVIDIA) and class `0x0302xx` (3D controller) or `0x0300xx` (VGA).

use crate::error::{NvPmuError, Result};
use crate::firmware::FirmwareInventory;
use crate::hwmon::HwmonSensors;
use std::path::{Path, PathBuf};

const NVIDIA_VENDOR_ID: u16 = 0x10de;
const PCI_CLASS_VGA: u32 = 0x0003_0000;
const PCI_CLASS_3D: u32 = 0x0003_0200;
const PCI_CLASS_MASK: u32 = 0x00FF_FF00;

/// Discovered NVIDIA GPU with PCI identity and sysfs path.
#[derive(Debug, Clone, serde::Serialize)]
pub struct NvidiaGpu {
    pub bdf: String,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u32,
    pub sysfs_path: PathBuf,
    pub driver: Option<String>,
    pub chip: Option<String>,
}

impl NvidiaGpu {
    /// Read hwmon sensors for this GPU.
    ///
    /// # Errors
    /// Returns error if hwmon directory cannot be found or read.
    pub fn sensors(&self) -> Result<HwmonSensors> {
        HwmonSensors::from_device(&self.sysfs_path)
    }

    /// Probe firmware inventory for this GPU.
    ///
    /// # Errors
    /// Returns error if chip name cannot be determined.
    pub fn firmware(&self) -> Result<FirmwareInventory> {
        let chip = self
            .chip
            .as_deref()
            .ok_or_else(|| NvPmuError::SensorNotFound("chip name unknown".into()))?;
        Ok(FirmwareInventory::probe(chip))
    }
}

/// Discover all NVIDIA GPUs via PCI sysfs.
///
/// # Errors
/// Returns I/O error if `/sys/bus/pci/devices/` cannot be read.
pub fn discover_gpus() -> Result<Vec<NvidiaGpu>> {
    let pci_dir = Path::new("/sys/bus/pci/devices");
    if !pci_dir.exists() {
        return Ok(Vec::new());
    }

    let mut gpus = Vec::new();
    for entry in std::fs::read_dir(pci_dir)? {
        let entry = entry?;
        let path = entry.path();
        let bdf = entry.file_name().to_string_lossy().into_owned();

        let Ok(vendor) = read_hex_u16(&path.join("vendor")) else {
            continue;
        };
        if vendor != NVIDIA_VENDOR_ID {
            continue;
        }

        let Ok(class) = read_hex_u32(&path.join("class")) else {
            continue;
        };
        if (class & PCI_CLASS_MASK) != PCI_CLASS_VGA && (class & PCI_CLASS_MASK) != PCI_CLASS_3D {
            continue;
        }

        let device_id = read_hex_u16(&path.join("device")).unwrap_or(0);
        let driver = read_driver_name(&path);
        let chip = infer_chip(device_id);

        gpus.push(NvidiaGpu {
            bdf,
            vendor_id: vendor,
            device_id,
            class_code: class,
            sysfs_path: path,
            driver,
            chip,
        });
    }

    gpus.sort_by(|a, b| a.bdf.cmp(&b.bdf));
    Ok(gpus)
}

fn read_sysfs_trimmed(path: &Path) -> Result<String> {
    Ok(std::fs::read_to_string(path)?.trim().to_string())
}

fn read_hex_u16(path: &Path) -> Result<u16> {
    let s = read_sysfs_trimmed(path)?;
    let s = s.strip_prefix("0x").unwrap_or(&s);
    u16::from_str_radix(s, 16).map_err(|e| NvPmuError::Parse {
        path: path.display().to_string(),
        source: e,
    })
}

fn read_hex_u32(path: &Path) -> Result<u32> {
    let s = read_sysfs_trimmed(path)?;
    let s = s.strip_prefix("0x").unwrap_or(&s);
    u32::from_str_radix(s, 16).map_err(|e| NvPmuError::Parse {
        path: path.display().to_string(),
        source: e,
    })
}

fn read_driver_name(device_path: &Path) -> Option<String> {
    let link = device_path.join("driver");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

/// Map PCI device ID to NVIDIA chip codename for firmware probing.
///
/// Covers the GPUs in the ecoPrimals hardware matrix.
fn infer_chip(device_id: u16) -> Option<String> {
    let chip = match device_id {
        0x1D81 => "gv100",                   // Titan V
        0x1E02 | 0x1E04 | 0x1E07 => "tu102", // RTX 2080 Ti family
        0x2204 | 0x2206 => "ga102",          // RTX 3090 / RTX 3080
        0x2684 => "ad102",                   // RTX 4090
        0x2704 | 0x2782 => "ad104",          // RTX 4070 family
        _ => return None,
    };
    Some(chip.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_chip_titan_v() {
        assert_eq!(infer_chip(0x1D81).as_deref(), Some("gv100"));
    }

    #[test]
    fn infer_chip_rtx3090() {
        assert_eq!(infer_chip(0x2204).as_deref(), Some("ga102"));
    }

    #[test]
    fn infer_chip_rtx4070() {
        assert_eq!(infer_chip(0x2704).as_deref(), Some("ad104"));
    }

    #[test]
    fn infer_chip_unknown() {
        assert_eq!(infer_chip(0x0000), None);
    }

    #[test]
    fn discover_gpus_runs() {
        // Succeeds even without NVIDIA hardware — returns empty vec.
        let result = discover_gpus();
        assert!(result.is_ok());
    }
}
