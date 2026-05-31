// SPDX-License-Identifier: AGPL-3.0-or-later
//! Thin sysfs helpers for PCI device lifecycle operations.
//!
//! Absorbed from coralReef `coral-ember` and adapted for toadStool. Replaces
//! `coral-driver::linux_paths` with inline path construction. The [`SysfsPort`]
//! trait enables test injection without touching real hardware.

use std::path::PathBuf;

use toadstool_common::sysfs_paths::{sysfs_pci_device_file, sysfs_pci_device_path};

use crate::error::SysfsError;

/// Construct the sysfs path for a PCI device attribute.
///
/// Returns `/sys/bus/pci/devices/{bdf}/{file}`.
#[must_use]
pub fn pci_device_path(bdf: &str, file: &str) -> PathBuf {
    PathBuf::from(sysfs_pci_device_file(bdf, file))
}

/// Injectable port for sysfs reads/writes (enables test doubles).
pub trait SysfsPort: Send + Sync {
    /// Write `content` to the given sysfs path.
    fn write(&self, path: &str, content: &str) -> Result<(), SysfsError>;

    /// Read the contents of the given sysfs path (trimmed).
    fn read(&self, path: &str) -> Result<String, SysfsError>;
}

/// Production sysfs port that reads/writes real files.
#[derive(Debug, Default, Clone, Copy)]
pub struct RealSysfs;

impl SysfsPort for RealSysfs {
    fn write(&self, path: &str, content: &str) -> Result<(), SysfsError> {
        std::fs::write(path, content).map_err(|e| SysfsError::Write {
            path: path.to_string(),
            reason: e.to_string(),
        })
    }

    fn read(&self, path: &str) -> Result<String, SysfsError> {
        std::fs::read_to_string(path)
            .map(|s| s.trim().to_string())
            .map_err(|e| SysfsError::Read {
                path: path.to_string(),
                reason: e.to_string(),
            })
    }
}

/// Read a PCI config-space ID (vendor or device) from sysfs.
///
/// Returns 0 on failure (non-existent device, container env, etc.).
#[must_use]
pub fn read_pci_id(bdf: &str, field: &str) -> u16 {
    let path = pci_device_path(bdf, field);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| {
            let trimmed = s.trim().trim_start_matches("0x");
            u16::from_str_radix(trimmed, 16).ok()
        })
        .unwrap_or(0)
}

/// Read the PCI power state (e.g. "D0", "D3hot", "D3cold").
#[must_use]
pub fn read_power_state(bdf: &str) -> Option<String> {
    let path = pci_device_path(bdf, "power_state");
    std::fs::read_to_string(path)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Pin device power rails: disable runtime PM and block D3cold.
pub fn pin_power(bdf: &str) {
    let control = pci_device_path(bdf, "power/control");
    let d3cold = pci_device_path(bdf, "d3cold_allowed");
    let _ = std::fs::write(&control, "on");
    let _ = std::fs::write(&d3cold, "0");
}

/// Pin power via injectable [`SysfsPort`].
pub fn pin_power_with(sysfs: &dyn SysfsPort, bdf: &str) {
    let _ = sysfs.write(&pci_device_path(bdf, "power/control").display().to_string(), "on");
    let _ = sysfs.write(
        &pci_device_path(bdf, "d3cold_allowed").display().to_string(),
        "0",
    );
}

/// Pin upstream PCI bridge power (walk parents).
pub fn pin_bridge_power_with(sysfs: &dyn SysfsPort, bdf: &str) {
    let device_path = PathBuf::from(sysfs_pci_device_path(bdf));
    if let Ok(parent) = std::fs::read_link(device_path.join("..")) {
        if let Some(bridge_name) = parent.file_name().and_then(|n| n.to_str()) {
            let _ = sysfs.write(
                &pci_device_path(bridge_name, "power/control")
                    .display()
                    .to_string(),
                "on",
            );
            let _ = sysfs.write(
                &pci_device_path(bridge_name, "d3cold_allowed")
                    .display()
                    .to_string(),
                "0",
            );
        }
    }
}

/// Pin upstream PCI bridge power (direct version — single parent only).
///
/// For multi-level switch topologies (e.g. PLX PEX 8747 on Tesla K80),
/// use [`pin_bridge_hierarchy`] instead to walk the full ancestry.
pub fn pin_bridge_power(bdf: &str) {
    let device_path = PathBuf::from(sysfs_pci_device_path(bdf));
    if let Ok(parent) = std::fs::read_link(device_path.join("..")) {
        if let Some(bridge_name) = parent.file_name().and_then(|n| n.to_str()) {
            let _ = std::fs::write(pci_device_path(bridge_name, "power/control"), "on");
            let _ = std::fs::write(pci_device_path(bridge_name, "d3cold_allowed"), "0");
        }
    }
}

/// Pin power on **every** upstream PCI bridge from `bdf` to the root complex.
///
/// Walks the canonical sysfs path upward, setting `power/control=on` and
/// `d3cold_allowed=0` on each ancestor whose name contains `:` (PCI BDF
/// convention). Stops at the first non-PCI parent.
///
/// Returns the number of bridges pinned.
pub fn pin_bridge_hierarchy(bdf: &str) -> usize {
    let device_link = PathBuf::from(sysfs_pci_device_path(bdf));
    let Ok(canonical) = std::fs::canonicalize(&device_link) else {
        return 0;
    };

    let mut pinned = 0usize;
    let mut current = canonical.as_path().parent();

    while let Some(parent) = current {
        let Some(name) = parent.file_name().and_then(|n| n.to_str()) else {
            break;
        };

        if !name.contains(':') {
            break;
        }

        let control = parent.join("power/control");
        let d3cold = parent.join("d3cold_allowed");
        if control.exists() {
            let _ = std::fs::write(&control, "on");
            let _ = std::fs::write(&d3cold, "0");
            pinned += 1;
        }

        current = parent.parent();
    }

    pinned
}

/// Direct sysfs write (bypasses path construction).
pub fn sysfs_write_direct(path: &str, content: &str) -> Result<(), SysfsError> {
    std::fs::write(path, content).map_err(|e| SysfsError::Write {
        path: path.to_string(),
        reason: e.to_string(),
    })
}

/// Direct sysfs write via injectable port.
pub fn sysfs_write_direct_with(
    sysfs: &dyn SysfsPort,
    path: &str,
    content: &str,
) -> Result<(), SysfsError> {
    sysfs.write(path, content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    struct MockSysfs {
        writes: Mutex<Vec<(String, String)>>,
    }

    impl MockSysfs {
        fn new() -> Self {
            Self {
                writes: Mutex::new(Vec::new()),
            }
        }

        fn written(&self) -> Vec<(String, String)> {
            self.writes.lock().unwrap().clone()
        }
    }

    impl SysfsPort for MockSysfs {
        fn write(&self, path: &str, content: &str) -> Result<(), SysfsError> {
            self.writes
                .lock()
                .unwrap()
                .push((path.to_string(), content.to_string()));
            Ok(())
        }

        fn read(&self, path: &str) -> Result<String, SysfsError> {
            Err(SysfsError::Read {
                path: path.to_string(),
                reason: "mock: no data".to_string(),
            })
        }
    }

    #[test]
    fn pci_device_path_format() {
        let p = pci_device_path("0000:03:00.0", "vendor");
        assert_eq!(
            p.to_str().unwrap(),
            "/sys/bus/pci/devices/0000:03:00.0/vendor"
        );
    }

    #[test]
    fn pin_power_with_mock() {
        let mock = MockSysfs::new();
        pin_power_with(&mock, "0000:03:00.0");
        let writes = mock.written();
        assert_eq!(writes.len(), 2);
        assert!(writes[0].0.contains("power/control"));
        assert_eq!(writes[0].1, "on");
        assert!(writes[1].0.contains("d3cold_allowed"));
        assert_eq!(writes[1].1, "0");
    }

    #[test]
    fn read_pci_id_nonexistent_returns_zero() {
        assert_eq!(read_pci_id("9999:99:99.9", "vendor"), 0);
    }
}
