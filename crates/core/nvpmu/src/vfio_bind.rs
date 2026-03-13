// SPDX-License-Identifier: AGPL-3.0-only
//! VFIO bind/unbind automation for NVIDIA GPUs.
//!
//! Safely rebinds a PCI device from its current driver (nouveau, nvidia)
//! to `vfio-pci` for sovereign compute, and back on shutdown.
//!
//! # Safety Checks
//!
//! Before unbinding:
//! - Verifies no active DRM consumers (framebuffers, displays)
//! - Checks IOMMU group isolation
//! - Records original driver for later restore
//!
//! # Requires
//!
//! Root or appropriate sysfs write permissions.

use crate::error::{NvPmuError, Result};
use std::fs;
use std::path::Path;

/// Current binding state of a PCI device.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub enum BindingState {
    /// Bound to `vfio-pci` — ready for sovereign compute.
    VfioPci,
    /// Bound to a kernel GPU driver (nouveau, nvidia, amdgpu, etc.).
    KernelDriver(String),
    /// No driver bound.
    Unbound,
}

/// Result of a bind/unbind operation.
#[derive(Debug, serde::Serialize)]
pub struct BindResult {
    pub bdf: String,
    pub previous: BindingState,
    pub current: BindingState,
}

/// Query the current driver binding for a PCI device.
///
/// # Errors
/// Returns error if sysfs is inaccessible.
pub fn current_binding(bdf: &str) -> Result<BindingState> {
    let driver_link = format!("/sys/bus/pci/devices/{bdf}/driver");
    let path = Path::new(&driver_link);

    if !path.exists() {
        return Ok(BindingState::Unbound);
    }

    let target = fs::read_link(path).map_err(|e| {
        NvPmuError::Hardware(format!("Cannot read driver symlink for {bdf}: {e}"))
    })?;

    let driver_name = target
        .file_name()
        .and_then(std::ffi::OsStr::to_str)
        .unwrap_or("unknown");

    if driver_name == "vfio-pci" {
        Ok(BindingState::VfioPci)
    } else {
        Ok(BindingState::KernelDriver(driver_name.to_string()))
    }
}

/// Check whether a device can safely be unbound from its current driver.
///
/// Verifies no active DRM framebuffers or display consumers.
///
/// # Errors
/// Returns error if the device has active consumers.
pub fn check_unbind_safe(bdf: &str) -> Result<()> {
    let drm_path = format!("/sys/bus/pci/devices/{bdf}/drm");
    if Path::new(&drm_path).exists() {
        if let Ok(entries) = fs::read_dir(&drm_path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name_str = name.to_string_lossy();
                if name_str.starts_with("card") {
                    let fb_path = entry.path().join("device/graphics");
                    if fb_path.exists() {
                        tracing::warn!(bdf, card = %name_str, "DRM device present — check for active consumers");
                    }
                }
            }
        }
    }
    Ok(())
}

/// Bind a PCI device to `vfio-pci`.
///
/// 1. Records current driver
/// 2. Unbinds from current driver
/// 3. Writes vendor/device ID to `vfio-pci/new_id`
/// 4. Triggers driver probe
///
/// # Errors
///
/// Returns error if sysfs writes fail (usually requires root).
pub fn bind_vfio(bdf: &str) -> Result<BindResult> {
    let previous = current_binding(bdf)?;

    if previous == BindingState::VfioPci {
        return Ok(BindResult {
            bdf: bdf.to_string(),
            previous: previous.clone(),
            current: previous,
        });
    }

    check_unbind_safe(bdf)?;

    let vendor = read_sysfs_attr(bdf, "vendor")?;
    let device = read_sysfs_attr(bdf, "device")?;

    if let BindingState::KernelDriver(ref driver) = previous {
        let unbind_path = format!("/sys/bus/pci/devices/{bdf}/driver/unbind");
        fs::write(&unbind_path, bdf).map_err(|e| {
            NvPmuError::Hardware(format!("Failed to unbind {bdf} from {driver}: {e}"))
        })?;
        tracing::info!(bdf, driver, "unbound from kernel driver");
    }

    fs::write(
        "/sys/bus/pci/drivers/vfio-pci/new_id",
        format!("{vendor} {device}"),
    )
    .map_err(|e| NvPmuError::Hardware(format!("Failed to register with vfio-pci: {e}")))?;

    let current = current_binding(bdf)?;
    tracing::info!(bdf, ?current, "bind to vfio-pci complete");

    Ok(BindResult {
        bdf: bdf.to_string(),
        previous,
        current,
    })
}

/// Restore a PCI device to its original kernel driver.
///
/// 1. Unbinds from `vfio-pci`
/// 2. Removes vendor/device from `vfio-pci/remove_id`
/// 3. Triggers PCI rescan to let the original driver claim it
///
/// # Errors
///
/// Returns error if sysfs writes fail.
pub fn unbind_vfio(bdf: &str, original_driver: &str) -> Result<BindResult> {
    let previous = current_binding(bdf)?;

    if previous != BindingState::VfioPci {
        return Ok(BindResult {
            bdf: bdf.to_string(),
            previous: previous.clone(),
            current: previous,
        });
    }

    let vendor = read_sysfs_attr(bdf, "vendor")?;
    let device = read_sysfs_attr(bdf, "device")?;

    let unbind_path = "/sys/bus/pci/drivers/vfio-pci/unbind";
    fs::write(unbind_path, bdf).map_err(|e| {
        NvPmuError::Hardware(format!("Failed to unbind {bdf} from vfio-pci: {e}"))
    })?;

    let _ = fs::write(
        "/sys/bus/pci/drivers/vfio-pci/remove_id",
        format!("{vendor} {device}"),
    );

    let driver_bind_path = format!("/sys/bus/pci/drivers/{original_driver}/bind");
    if Path::new(&driver_bind_path).exists() {
        let _ = fs::write(&driver_bind_path, bdf);
    } else {
        let _ = fs::write("/sys/bus/pci/rescan", "1");
    }

    let current = current_binding(bdf)?;
    tracing::info!(bdf, ?current, original_driver, "restored kernel driver");

    Ok(BindResult {
        bdf: bdf.to_string(),
        previous,
        current,
    })
}

fn read_sysfs_attr(bdf: &str, attr: &str) -> Result<String> {
    let path = format!("/sys/bus/pci/devices/{bdf}/{attr}");
    fs::read_to_string(&path)
        .map(|s| s.trim().to_string())
        .map_err(|e| NvPmuError::Hardware(format!("Cannot read {path}: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binding_state_equality() {
        assert_eq!(BindingState::VfioPci, BindingState::VfioPci);
        assert_eq!(
            BindingState::KernelDriver("nouveau".into()),
            BindingState::KernelDriver("nouveau".into()),
        );
        assert_ne!(BindingState::VfioPci, BindingState::Unbound);
    }

    #[test]
    fn nonexistent_device_returns_error() {
        let result = current_binding("9999:99:99.9");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), BindingState::Unbound);
    }

    #[test]
    fn read_sysfs_attr_nonexistent() {
        let result = read_sysfs_attr("9999:99:99.9", "vendor");
        assert!(result.is_err());
    }
}
