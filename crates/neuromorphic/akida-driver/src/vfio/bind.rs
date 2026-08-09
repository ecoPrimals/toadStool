// SPDX-License-Identifier: AGPL-3.0-or-later
//! VFIO bind/unbind/IOMMU group helpers.

/// Bind an Akida device to `vfio-pci`, unloading any existing driver.
pub fn bind_to_vfio(pcie_address: &str) -> crate::error::Result<()> {
    use crate::error::AkidaError;
    use std::path::Path;

    tracing::info!("Binding {} to vfio-pci", pcie_address);

    let driver_unbind = format!("/sys/bus/pci/devices/{pcie_address}/driver/unbind");
    if Path::new(&driver_unbind).exists() {
        std::fs::write(&driver_unbind, pcie_address).map_err(|e| {
            AkidaError::hardware_error(format!("Cannot unbind {pcie_address}: {e}"))
        })?;
        tracing::info!("Unbound from existing driver");
    }

    let new_id = "/sys/bus/pci/drivers/vfio-pci/new_id";
    if Path::new(new_id).exists() {
        std::fs::write(
            new_id,
            format!(
                "{:04x} {:04x}",
                akida_chip::pcie::BRAINCHIP_VENDOR_ID,
                0xBCA1u16
            ),
        )
        .map_err(|e| AkidaError::hardware_error(format!("Cannot write vfio-pci/new_id: {e}")))?;
    }

    let bind_path = "/sys/bus/pci/drivers/vfio-pci/bind";
    std::fs::write(bind_path, pcie_address)
        .map_err(|e| AkidaError::hardware_error(format!("Cannot bind to vfio-pci: {e}")))?;

    tracing::info!("{pcie_address} bound to vfio-pci");
    Ok(())
}

/// Unbind from `vfio-pci` and re-bind to `akida_pcie` kernel module.
///
/// # Errors
///
/// Returns an error if sysfs writes fail.
pub fn unbind_from_vfio(pcie_address: &str) -> crate::error::Result<()> {
    use crate::error::AkidaError;

    let unbind = "/sys/bus/pci/drivers/vfio-pci/unbind";
    std::fs::write(unbind, pcie_address)
        .map_err(|e| AkidaError::hardware_error(format!("Cannot unbind from vfio-pci: {e}")))?;

    let bind = "/sys/bus/pci/drivers/akida/bind";
    if std::path::Path::new(bind).exists() {
        std::fs::write(bind, pcie_address)
            .map_err(|e| AkidaError::hardware_error(format!("Cannot bind to akida driver: {e}")))?;
        tracing::info!("{pcie_address} re-bound to akida_pcie");
    } else {
        tracing::info!("{pcie_address} unbound (akida_pcie not loaded)");
    }

    Ok(())
}

/// Find the IOMMU group number for a `PCIe` device.
///
/// Reads `/sys/bus/pci/devices/{addr}/iommu_group` symlink.
///
/// # Errors
///
/// Returns `AkidaError` if the sysfs symlink cannot be read.
pub fn iommu_group(pcie_address: &str) -> crate::error::Result<u32> {
    use crate::error::AkidaError;

    let link = format!("/sys/bus/pci/devices/{pcie_address}/iommu_group");
    let target = std::fs::read_link(&link).map_err(|e| {
        AkidaError::hardware_error(format!("Cannot read iommu_group for {pcie_address}: {e}"))
    })?;

    let group = target
        .file_name()
        .and_then(|n| n.to_str())
        .and_then(|s| s.parse::<u32>().ok())
        .ok_or_else(|| {
            AkidaError::hardware_error(format!(
                "Cannot parse IOMMU group from {}",
                target.display()
            ))
        })?;

    tracing::debug!("{pcie_address} → IOMMU group {group}");
    Ok(group)
}
