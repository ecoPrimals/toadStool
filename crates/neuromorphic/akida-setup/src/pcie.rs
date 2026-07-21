// SPDX-License-Identifier: AGPL-3.0-or-later
//! `PCIe` device management

use crate::error::{Result, SetupError};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct AkidaDevice {
    pub pcie_address: String,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained for device identification in future multi-vendor support"
        )
    )]
    pub vendor_id: String,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "retained for device identification in future multi-vendor support"
        )
    )]
    pub device_id: String,
}

/// Discover Akida devices via lspci
pub fn discover_akida_devices() -> Result<Vec<AkidaDevice>> {
    let output = Command::new("lspci")
        .arg("-d")
        .arg("1e7c:bca1") // Akida vendor:device
        .output()
        .map_err(|e| SetupError::Setup(format!("Failed to run lspci: {e}")))?;

    if !output.status.success() {
        return Err(SetupError::Setup(format!(
            "lspci failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut devices = Vec::new();

    for line in stdout.lines() {
        // Parse: "a1:00.0 Co-processor: Brainchip..."
        if let Some(pcie_addr) = line.split_whitespace().next() {
            devices.push(AkidaDevice {
                pcie_address: format!("0000:{pcie_addr}"),
                vendor_id: "1e7c".to_string(),
                device_id: "bca1".to_string(),
            });
        }
    }

    Ok(devices)
}

/// Enable `PCIe` device via sysfs
pub fn enable_pcie_device(pcie_address: &str) -> Result<()> {
    let enable_path = format!("/sys/bus/pci/devices/{pcie_address}/enable");

    // Check if already enabled
    if let Ok(content) = fs::read_to_string(&enable_path)
        && content.trim() == "1"
    {
        tracing::debug!("Device {} already enabled", pcie_address);
        return Ok(());
    }

    // Enable device
    fs::write(&enable_path, "1")
        .map_err(|e| SetupError::Setup(format!("Failed to enable device {pcie_address}: {e}")))?;

    // Verify
    let enabled = fs::read_to_string(&enable_path)?;
    if enabled.trim() != "1" {
        return Err(SetupError::Setup(format!(
            "Failed to enable device {pcie_address}"
        )));
    }

    Ok(())
}

/// Load kernel module
pub fn load_kernel_module(module_path: &str) -> Result<()> {
    let path = Path::new(module_path);

    if !path.exists() {
        return Err(SetupError::Setup(format!(
            "Kernel module not found: {module_path}"
        )));
    }

    // Check if already loaded
    if is_module_loaded()? {
        tracing::info!("Module already loaded, unloading first...");
        unload_kernel_module()?;
    }

    // Load module with insmod
    let output = Command::new("insmod")
        .arg(module_path)
        .output()
        .map_err(|e| SetupError::Setup(format!("Failed to run insmod: {e}")))?;

    if !output.status.success() {
        return Err(SetupError::Setup(format!(
            "insmod failed: {}",
            String::from_utf8_lossy(&output.stderr)
        )));
    }

    // Verify loaded
    if !is_module_loaded()? {
        return Err(SetupError::Setup(
            "Module loaded but not found in lsmod".to_string(),
        ));
    }

    Ok(())
}

/// Check if `akida_pcie` module is loaded
pub fn is_module_loaded() -> Result<bool> {
    let output = Command::new("lsmod")
        .output()
        .map_err(|e| SetupError::Setup(format!("Failed to run lsmod: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.contains("akida_pcie"))
}

/// Unload kernel module
pub fn unload_kernel_module() -> Result<()> {
    let output = Command::new("rmmod")
        .arg("akida_pcie")
        .output()
        .map_err(|e| SetupError::Setup(format!("Failed to run rmmod: {e}")))?;

    // Ignore errors (module might not be loaded)
    if !output.status.success() {
        tracing::warn!("rmmod warning: {}", String::from_utf8_lossy(&output.stderr));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_akida_device_struct() {
        let device = AkidaDevice {
            pcie_address: "0000:01:00.0".to_string(),
            vendor_id: "1e7c".to_string(),
            device_id: "bca1".to_string(),
        };
        assert_eq!(device.pcie_address, "0000:01:00.0");
        assert_eq!(device.vendor_id, "1e7c");
        assert_eq!(device.device_id, "bca1");
    }

    #[test]
    fn test_discover_akida_devices() {
        let result = discover_akida_devices();
        assert!(result.is_ok());
        let devices = result.unwrap();
        assert!(devices.iter().all(|d| d.pcie_address.starts_with("0000:")));
    }

    #[test]
    fn test_is_module_loaded() {
        let result = is_module_loaded();
        assert!(result.is_ok());
    }

    #[test]
    fn test_unload_kernel_module() {
        let result = unload_kernel_module();
        assert!(result.is_ok());
    }
}
