//! Setup verification

use crate::pcie::AkidaDevice;
use anyhow::{bail, Context, Result};
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::thread;
use std::time::Duration;

/// Verify complete setup
pub fn verify_setup(devices: &[AkidaDevice]) -> Result<()> {
    // Wait for device nodes to appear
    tracing::info!("Waiting for device nodes...");
    thread::sleep(Duration::from_secs(2));

    // Check PCIe devices enabled
    for device in devices {
        verify_pcie_enabled(&device.pcie_address)?;
    }

    // Check kernel module
    verify_kernel_module()?;

    // Check device nodes (optional - may not exist yet)
    if let Err(e) = verify_device_nodes() {
        tracing::warn!("⚠️  Device node verification: {}", e);
        tracing::warn!("   This is normal if udev hasn't created them yet");
    }

    Ok(())
}

fn verify_pcie_enabled(pcie_address: &str) -> Result<()> {
    let enable_path = format!("/sys/bus/pci/devices/{}/enable", pcie_address);
    let enabled = fs::read_to_string(&enable_path)?;

    if enabled.trim() != "1" {
        bail!("Device {} not enabled", pcie_address);
    }

    // Check BARs are accessible
    let resource_path = format!("/sys/bus/pci/devices/{}/resource0", pcie_address);
    if !std::path::Path::new(&resource_path).exists() {
        bail!("BAR resources not accessible for {}", pcie_address);
    }

    tracing::debug!("✅ {} verified", pcie_address);
    Ok(())
}

fn verify_kernel_module() -> Result<()> {
    let output = std::process::Command::new("lsmod")
        .output()
        .context("Failed to run lsmod")?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !stdout.contains("akida_pcie") {
        bail!("Kernel module not loaded");
    }

    tracing::debug!("✅ Kernel module verified");
    Ok(())
}

fn verify_device_nodes() -> Result<()> {
    let nodes = crate::permissions::list_device_nodes()?;

    if nodes.is_empty() {
        bail!("No /dev/akida* nodes found");
    }

    // Check permissions
    for node in &nodes {
        let metadata = fs::metadata(node)?;
        let permissions = metadata.permissions();

        if permissions.mode() & 0o666 != 0o666 {
            bail!("Incorrect permissions on {}", node);
        }
    }

    tracing::info!("✅ Found {} device node(s)", nodes.len());
    Ok(())
}
