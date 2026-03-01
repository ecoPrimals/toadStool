//! Setup verification

use crate::error::{Result, SetupError};
use crate::pcie::AkidaDevice;
use crate::permissions::list_device_nodes;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::time::Duration;

/// Verify complete setup
pub fn verify_setup(devices: &[AkidaDevice]) -> Result<()> {
    // Poll for device nodes (udev creates them asynchronously). Return as soon
    // as they appear, or after 5s timeout. Replaces fixed 2s sleep with
    // condition-based wait.
    tracing::info!("Waiting for device nodes...");
    for _ in 0..50 {
        if list_device_nodes().map(|n| !n.is_empty()).unwrap_or(false) {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }

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
    let enable_path = format!("/sys/bus/pci/devices/{pcie_address}/enable");
    let enabled = fs::read_to_string(&enable_path)?;

    if enabled.trim() != "1" {
        return Err(SetupError::Setup(format!(
            "Device {pcie_address} not enabled"
        )));
    }

    // Check BARs are accessible
    let resource_path = format!("/sys/bus/pci/devices/{pcie_address}/resource0");
    if !std::path::Path::new(&resource_path).exists() {
        return Err(SetupError::Setup(format!(
            "BAR resources not accessible for {pcie_address}"
        )));
    }

    tracing::debug!("✅ {} verified", pcie_address);
    Ok(())
}

fn verify_kernel_module() -> Result<()> {
    let output = std::process::Command::new("lsmod")
        .output()
        .map_err(|e| SetupError::Setup(format!("Failed to run lsmod: {e}")))?;

    let stdout = String::from_utf8_lossy(&output.stdout);

    if !stdout.contains("akida_pcie") {
        return Err(SetupError::Setup("Kernel module not loaded".to_string()));
    }

    tracing::debug!("✅ Kernel module verified");
    Ok(())
}

fn verify_device_nodes() -> Result<()> {
    let nodes = crate::permissions::list_device_nodes()?;

    if nodes.is_empty() {
        return Err(SetupError::Setup("No /dev/akida* nodes found".to_string()));
    }

    // Check permissions
    for node in &nodes {
        let metadata = fs::metadata(node)?;
        let permissions = metadata.permissions();

        if permissions.mode() & 0o666 != 0o666 {
            return Err(SetupError::Setup(format!(
                "Incorrect permissions on {node}"
            )));
        }
    }

    tracing::info!("✅ Found {} device node(s)", nodes.len());
    Ok(())
}
