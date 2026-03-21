// SPDX-License-Identifier: AGPL-3.0-only
//! Sysfs filesystem helpers for GPU monitoring.
//!
//! Best-effort readers: missing or busy sysfs entries return `None`.

use std::path::{Path, PathBuf};

pub(super) fn read_sysfs_u64(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

pub(super) fn read_sysfs_hex(path: &Path) -> Option<u32> {
    let s = std::fs::read_to_string(path).ok()?;
    let trimmed = s.trim().trim_start_matches("0x");
    u32::from_str_radix(trimmed, 16).ok()
}

pub(super) fn read_sysfs_string(path: &Path) -> Option<String> {
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

pub(super) fn read_sysfs_uevent_field(uevent_path: &Path, key: &str) -> Option<String> {
    let content = std::fs::read_to_string(uevent_path).ok()?;
    let prefix = format!("{key}=");
    for line in content.lines() {
        if let Some(val) = line.strip_prefix(&prefix) {
            return Some(val.to_string());
        }
    }
    None
}

pub(super) fn find_hwmon_dir(device_path: &Path) -> Option<PathBuf> {
    let hwmon_parent = device_path.join("hwmon");
    let entries = std::fs::read_dir(&hwmon_parent).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        if name.to_string_lossy().starts_with("hwmon") {
            return Some(entry.path());
        }
    }
    None
}

pub(super) fn find_iommu_group(device_path: &Path) -> Option<u32> {
    let link = std::fs::read_link(device_path.join("iommu_group")).ok()?;
    let name = link.file_name()?.to_string_lossy();
    name.parse().ok()
}

pub(crate) fn parse_pcie_gen(speed_str: &str) -> Option<u32> {
    let s = speed_str.trim();
    if s.contains("32.0") || s.contains("32 GT") {
        Some(5)
    } else if s.contains("16.0") || s.contains("16 GT") {
        Some(4)
    } else if s.contains("8.0") || s.contains("8 GT") {
        Some(3)
    } else if s.contains("5.0") || s.contains("5 GT") {
        Some(2)
    } else if s.contains("2.5") {
        Some(1)
    } else {
        None
    }
}
