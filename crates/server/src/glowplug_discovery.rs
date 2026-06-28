// SPDX-License-Identifier: AGPL-3.0-or-later
//! GPU PCI discovery via sysfs.
//!
//! Extracted from `glowplug_client.rs` — these are stateless sysfs probes
//! used by both `GlowPlugClient` and `kernel_sentinel`.

use crate::glowplug_types::EmberDeviceInfo;

/// Read the current driver bound to a PCI device.
pub(crate) fn read_current_driver(bdf: &str) -> Option<String> {
    let link = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "driver");
    std::fs::read_link(&link)
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().into_owned()))
}

/// Read a 32-bit value from PCI config space via sysfs.
///
/// Falls back to `0` if the device or offset is inaccessible.
#[cfg(test)]
pub(crate) fn read_pci_config_u32(bdf: &str, offset: u64) -> u32 {
    use std::io::{Read, Seek, SeekFrom};
    let path = format!("/sys/bus/pci/devices/{bdf}/config");
    let Ok(mut f) = std::fs::File::open(&path) else {
        return 0;
    };
    if f.seek(SeekFrom::Start(offset)).is_err() {
        return 0;
    }
    let mut buf = [0u8; 4];
    if f.read_exact(&mut buf).is_err() {
        return 0;
    }
    u32::from_le_bytes(buf)
}

/// Probe GPU registers from BAR0 via `nvpmu::bar0::Bar0Access`.
///
/// Returns (PMC_ENABLE at 0x200, FECS_CPUCTL at 0x409100).
/// Falls back to (0, 0) if the BAR0 resource is inaccessible.
pub(crate) fn read_bar0_registers(bdf: &str) -> (u32, u32) {
    let Ok(bar0) = nvpmu::bar0::Bar0Access::open(bdf) else {
        return (0, 0);
    };
    let pmc_enable = bar0.read_u32(0x200).unwrap_or(0);
    let fecs_cpuctl = bar0.read_u32(0x40_9100).unwrap_or(0);
    (pmc_enable, fecs_cpuctl)
}

/// Discover enriched GPU device info for all visible GPUs.
pub(crate) fn discover_gpu_devices() -> Vec<EmberDeviceInfo> {
    discover_gpu_bdfs()
        .into_iter()
        .filter_map(|bdf| discover_single_device(&bdf))
        .collect()
}

/// Probe enriched metadata for a single GPU BDF.
pub(crate) fn discover_single_device(bdf: &str) -> Option<EmberDeviceInfo> {
    let pci_path = toadstool_cylinder::linux_paths::sysfs_pci_device_path(bdf);
    if !std::path::Path::new(&pci_path).exists() || !is_gpu_bdf(bdf) {
        return None;
    }

    Some(EmberDeviceInfo {
        bdf: bdf.to_string(),
        name: read_device_name(bdf),
        vendor_id: toadstool_ember::sysfs::read_pci_id(bdf, "vendor"),
        personality: read_current_driver(bdf).unwrap_or_else(|| "unbound".into()),
        protected: is_display_connected(bdf),
        vram_alive: probe_vram_alive(bdf),
        domains_faulted: 0,
    })
}

pub(crate) fn is_gpu_bdf(bdf: &str) -> bool {
    let class_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "class");
    let Ok(class) = std::fs::read_to_string(class_path) else {
        return false;
    };
    let class_trimmed = class.trim();
    class_trimmed.starts_with("0x0302") || class_trimmed.starts_with("0x0300")
}

fn read_device_name(bdf: &str) -> Option<String> {
    let label_path = toadstool_cylinder::linux_paths::sysfs_pci_device_file(bdf, "label");
    if let Ok(label) = std::fs::read_to_string(&label_path) {
        let trimmed = label.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }
    let device_id = toadstool_ember::sysfs::read_pci_id(bdf, "device");
    if device_id == 0 {
        None
    } else {
        Some(format!("0x{device_id:04x}"))
    }
}

fn probe_vram_alive(bdf: &str) -> bool {
    let Ok(bar0) = nvpmu::bar0::Bar0Access::open(bdf) else {
        return false;
    };
    match bar0.read_u32(0) {
        Ok(val) => val != 0xFFFF_FFFF,
        Err(_) => false,
    }
}

/// True when a physical display connector is connected on this GPU's DRM card.
fn is_display_connected(bdf: &str) -> bool {
    let drm_dir = std::path::Path::new("/sys/class/drm");
    let Ok(entries) = std::fs::read_dir(drm_dir) else {
        return false;
    };

    let mut card_names = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.contains('-') || !name_str.starts_with("card") {
            continue;
        }
        let device_link = entry.path().join("device");
        if pci_bdf_matches(&device_link, bdf) {
            card_names.push(name_str.into_owned());
        }
    }

    if card_names.is_empty() {
        return false;
    }

    let Ok(entries) = std::fs::read_dir(drm_dir) else {
        return false;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.contains('-') {
            continue;
        }
        let is_connector = card_names
            .iter()
            .any(|card| name_str.starts_with(&format!("{card}-")));
        if !is_connector {
            continue;
        }
        let status_path = entry.path().join("status");
        if let Ok(status) = std::fs::read_to_string(status_path)
            && status.trim() == "connected"
        {
            return true;
        }
    }
    false
}

pub(crate) fn pci_bdf_matches(device_link: &std::path::Path, bdf: &str) -> bool {
    let Ok(canonical) = std::fs::canonicalize(device_link) else {
        return false;
    };
    canonical
        .file_name()
        .is_some_and(|name| name.to_string_lossy() == bdf)
}

/// Discover GPU BDF addresses from PCI sysfs (class 0x030000 = VGA).
pub fn discover_gpu_bdfs() -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(toadstool_cylinder::linux_paths::sysfs_pci_devices())
    else {
        return Vec::new();
    };

    let mut bdfs: Vec<String> = entries
        .flatten()
        .filter_map(|entry| {
            let class_path = entry.path().join("class");
            let class = std::fs::read_to_string(class_path).ok()?;
            let class_trimmed = class.trim();
            if class_trimmed.starts_with("0x0302") || class_trimmed.starts_with("0x0300") {
                entry.file_name().to_str().map(String::from)
            } else {
                None
            }
        })
        .collect();

    bdfs.sort();
    bdfs
}
