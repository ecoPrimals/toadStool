//! NPU hardware setup and initialization
//!
//! This module provides pure Rust implementations for setting up Akida NPU hardware,
//! replacing shell scripts with compiled code that's portable across systems.

use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;
use tracing::{debug, info, warn};

/// Setup Akida NPU kernel driver
pub struct NpuSetup {
    driver_path: Option<PathBuf>,
    pkexec_available: bool,
}

impl NpuSetup {
    /// Create new NPU setup manager
    pub fn new() -> Self {
        Self {
            driver_path: None,
            pkexec_available: which::which("pkexec").is_ok(),
        }
    }

    /// Run complete NPU setup
    ///
    /// # Errors
    ///
    /// Returns error if any setup step fails (hardware not found, driver missing,
    /// insufficient permissions, etc.).
    pub fn run(&mut self) -> Result<()> {
        info!("Akida NPU Setup");

        // Check hardware
        check_hardware()?;

        // Find driver
        self.find_driver()?;

        // Enable PCIe devices
        self.enable_pcie_devices()?;

        // Load kernel module
        self.load_kernel_module()?;

        // Verify device nodes
        verify_device_nodes()?;

        // Setup permissions
        self.setup_permissions()?;

        info!("NPU setup complete!");
        Ok(())
    }

    /// Find Akida kernel module
    fn find_driver(&mut self) -> Result<()> {
        info!("Looking for Akida kernel module...");

        // Check if already loaded
        if is_module_loaded()? {
            info!("Kernel module already loaded");
            return Ok(());
        }

        // Search for module file
        let search_paths = vec![
            PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join("Development/ecoPrimals/akida_dw_edma/akida-pcie.ko"),
            PathBuf::from("/lib/modules")
                .join(kernel_version()?)
                .join("extra/akida-pcie.ko"),
        ];

        for path in search_paths {
            if path.exists() {
                info!("Found driver: {}", path.display());
                self.driver_path = Some(path);
                return Ok(());
            }
        }

        bail!("Akida kernel module not found. Please compile the driver.");
    }

    /// Enable PCIe devices
    fn enable_pcie_devices(&self) -> Result<()> {
        info!("Enabling PCIe devices...");

        // Find Akida PCIe addresses
        let output = Command::new("lspci").arg("-d").arg("1e7c:bca1").output()?;

        let devices = String::from_utf8_lossy(&output.stdout);

        for line in devices.lines() {
            if let Some(address) = line.split_whitespace().next() {
                let full_address = format!("0000:{address}");
                self.enable_device(&full_address)?;
            }
        }

        Ok(())
    }

    /// Enable single PCIe device
    fn enable_device(&self, address: &str) -> Result<()> {
        let enable_path = format!("/sys/bus/pci/devices/{address}/enable");

        debug!("Enabling device: {address}");

        // Try direct write first (if we have permissions)
        if fs::write(&enable_path, "1").is_ok() {
            info!("Enabled {address} (direct)");
            return Ok(());
        }

        // Need privilege escalation
        if !self.pkexec_available {
            bail!("Need root to enable device. Install pkexec or run as root.");
        }

        let status = Command::new("pkexec")
            .arg("sh")
            .arg("-c")
            .arg(format!("echo 1 > {enable_path}"))
            .status()?;

        if !status.success() {
            bail!("Failed to enable device {address} (pkexec)");
        }

        info!("Enabled {address} (pkexec)");
        Ok(())
    }

    /// Load kernel module
    fn load_kernel_module(&self) -> Result<()> {
        info!("Loading kernel module...");

        // Already loaded?
        if is_module_loaded()? {
            return Ok(());
        }

        let driver_path = self.driver_path.as_ref().context("Driver path not set")?;

        // Try direct insmod first
        if let Ok(status) = Command::new("insmod").arg(driver_path).status() {
            if status.success() {
                info!("Module loaded (direct)");
                return Ok(());
            }
        }

        // Need privilege escalation
        if !self.pkexec_available {
            bail!("Need root to load module. Install pkexec or run as root.");
        }

        let status = Command::new("pkexec")
            .arg("insmod")
            .arg(driver_path)
            .status()?;

        if !status.success() {
            bail!("Failed to load kernel module");
        }

        info!("Module loaded (pkexec)");

        // Verify it loaded
        if !is_module_loaded()? {
            bail!("Module loaded but not showing in lsmod");
        }

        Ok(())
    }

    /// Setup device permissions
    fn setup_permissions(&self) -> Result<()> {
        info!("Setting up permissions...");

        // Check if devices are already accessible
        if Path::new("/dev/akida0").exists()
            && !fs::metadata("/dev/akida0")?.permissions().readonly()
        {
            info!("Permissions already OK");
            return Ok(());
        }

        // Need to set permissions
        if !self.pkexec_available {
            warn!("Cannot set permissions without pkexec. Devices may not be accessible.");
            return Ok(());
        }

        let status = Command::new("pkexec")
            .arg("chmod")
            .arg("666")
            .arg("/dev/akida*")
            .status()?;

        if status.success() {
            info!("Permissions set");
        } else {
            warn!("Failed to set permissions. May need manual chmod.");
        }

        Ok(())
    }
}

impl Default for NpuSetup {
    fn default() -> Self {
        Self::new()
    }
}

/// Check if Akida hardware is present (associated function, no self needed)
fn check_hardware() -> Result<()> {
    info!("Checking for Akida hardware...");

    let output = Command::new("lspci").arg("-d").arg("1e7c:bca1").output()?;

    if !output.status.success() || output.stdout.is_empty() {
        bail!("No Akida NPU hardware detected. Run 'lspci -d 1e7c:bca1' to verify.");
    }

    let devices = String::from_utf8_lossy(&output.stdout);
    let count = devices.lines().count();
    info!("Found {count} Akida device(s)");

    Ok(())
}

/// Check if kernel module is loaded (associated function, no self needed)
fn is_module_loaded() -> Result<bool> {
    let output = Command::new("lsmod").output()?;
    let modules = String::from_utf8_lossy(&output.stdout);
    Ok(modules.contains("akida_pcie"))
}

/// Verify device nodes exist (associated function, no self needed)
fn verify_device_nodes() -> Result<()> {
    info!("Verifying device nodes...");

    // Wait up to 5 seconds for udev
    for _ in 0..50 {
        if Path::new("/dev/akida0").exists() {
            info!("Device nodes created");
            return Ok(());
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    bail!("Device nodes not created. Check dmesg for errors.");
}

/// Get current kernel version
fn kernel_version() -> Result<String> {
    let output = Command::new("uname").arg("-r").output()?;
    Ok(String::from_utf8(output.stdout)?.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kernel_version() {
        let version = kernel_version().unwrap();
        assert!(!version.is_empty());
        println!("Kernel version: {version}");
    }
}
