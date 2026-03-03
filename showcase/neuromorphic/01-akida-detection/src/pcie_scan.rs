// SPDX-License-Identifier: AGPL-3.0-or-later
//! PCIe bus scanning for Akida devices

use anyhow::{Context, Result};
use std::process::Command;

use crate::{AKIDA_AKD1000_DEVICE_ID, BRAINCHIP_VENDOR_ID};

/// PCIe device information
#[derive(Debug, Clone)]
pub struct PcieDevice {
    /// Bus address (e.g., "0000:01:00.0")
    pub address: String,

    /// Vendor ID
    pub vendor_id: u16,

    /// Device ID
    pub device_id: u16,

    /// Device name
    pub device_name: String,
}

/// Scan PCIe bus for Akida devices
pub fn scan_for_akida() -> Result<Vec<PcieDevice>> {
    // Try lspci first (most common on Linux)
    if let Ok(devices) = scan_with_lspci() {
        return Ok(devices);
    }

    // Fallback: scan /sys/bus/pci/devices
    scan_sys_pci()
}

/// Scan using lspci command
fn scan_with_lspci() -> Result<Vec<PcieDevice>> {
    let output = Command::new("lspci")
        .args(["-n", "-D"]) // Numeric IDs, show domain
        .output()
        .context("Failed to execute lspci")?;

    if !output.status.success() {
        anyhow::bail!("lspci command failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout.lines() {
        // Parse lines like:
        // 0000:01:00.0 0108: 1e7c:0001 (rev 01)
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }

        let address = parts[0];
        let ids = parts[2];

        // Parse vendor:device IDs
        let id_parts: Vec<&str> = ids.split(':').collect();
        if id_parts.len() != 2 {
            continue;
        }

        let vendor_id = u16::from_str_radix(id_parts[0], 16).ok();
        let device_id = u16::from_str_radix(id_parts[1], 16).ok();

        if let (Some(vendor), Some(device)) = (vendor_id, device_id) {
            // Check if this is a BrainChip Akida device
            if vendor == BRAINCHIP_VENDOR_ID && device == AKIDA_AKD1000_DEVICE_ID {
                devices.push(PcieDevice {
                    address: address.to_string(),
                    vendor_id: vendor,
                    device_id: device,
                    device_name: "Akida AKD1000".to_string(),
                });
            }
        }
    }

    Ok(devices)
}

/// Scan /sys/bus/pci/devices directory
fn scan_sys_pci() -> Result<Vec<PcieDevice>> {
    use std::fs;
    use std::path::Path;

    let pci_dir = Path::new("/sys/bus/pci/devices");
    if !pci_dir.exists() {
        anyhow::bail!("PCIe sysfs not available");
    }

    let mut devices = Vec::new();

    for entry in fs::read_dir(pci_dir).context("Failed to read PCIe devices")? {
        let entry = entry?;
        let path = entry.path();

        // Read vendor ID
        let vendor_path = path.join("vendor");
        let device_path = path.join("device");

        if !vendor_path.exists() || !device_path.exists() {
            continue;
        }

        let vendor_str = fs::read_to_string(&vendor_path)
            .ok()
            .and_then(|s| u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok());

        let device_str = fs::read_to_string(&device_path)
            .ok()
            .and_then(|s| u16::from_str_radix(s.trim().trim_start_matches("0x"), 16).ok());

        if let (Some(vendor), Some(device)) = (vendor_str, device_str) {
            if vendor == BRAINCHIP_VENDOR_ID && device == AKIDA_AKD1000_DEVICE_ID {
                let address = entry.file_name().to_string_lossy().to_string();

                devices.push(PcieDevice {
                    address,
                    vendor_id: vendor,
                    device_id: device,
                    device_name: "Akida AKD1000".to_string(),
                });
            }
        }
    }

    Ok(devices)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scan_attempts() {
        // This will fail on systems without Akida boards, which is fine
        let result = scan_for_akida();

        // Test should not panic, just might find 0 devices
        match result {
            Ok(devices) => {
                println!("Found {} Akida device(s)", devices.len());
            }
            Err(e) => {
                println!("Scan failed (expected without hardware): {}", e);
            }
        }
    }
}
