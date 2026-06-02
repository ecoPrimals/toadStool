// SPDX-License-Identifier: AGPL-3.0-or-later
//! Linux sysfs path helpers — portable sysfs root, PCI device paths, module paths.
//!
//! All functions respect `TOADSTOOL_SYSFS_ROOT` (default `/sys`) so tests and
//! containers can point at a mock sysfs tree.

use crate::interned_strings::socket_env;
use std::sync::OnceLock;

fn resolve_env(primary: &str, default: &str) -> String {
    if let Some(v) = std::env::var(primary).ok().filter(|s| !s.is_empty()) {
        return v.trim_end_matches('/').to_string();
    }
    default.to_string()
}

/// Resolved sysfs mount path (default `/sys`).
#[must_use]
pub fn sysfs_root() -> &'static str {
    static ROOT: OnceLock<String> = OnceLock::new();
    ROOT.get_or_init(|| resolve_env(socket_env::TOADSTOOL_SYSFS_ROOT, "/sys"))
        .as_str()
}

/// Join path segments under [`sysfs_root`].
#[must_use]
pub fn sysfs_join(parts: &[&str]) -> String {
    let mut s = String::with_capacity(96);
    s.push_str(sysfs_root());
    for p in parts {
        s.push('/');
        s.push_str(p.trim_matches('/'));
    }
    s
}

/// `/…/bus/pci/devices` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_devices() -> String {
    sysfs_join(&["bus", "pci", "devices"])
}

/// `/…/bus/pci/devices/{bdf}` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_device_path(bdf: &str) -> String {
    sysfs_join(&["bus", "pci", "devices", bdf])
}

/// `/…/bus/pci/devices/{bdf}/{tail}`.
#[must_use]
pub fn sysfs_pci_device_file(bdf: &str, tail: &str) -> String {
    let base = sysfs_pci_device_path(bdf);
    if tail.is_empty() {
        base
    } else {
        format!("{base}/{}", tail.trim_start_matches('/'))
    }
}

/// `/…/bus/pci/drivers/{driver}/bind` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_driver_bind(driver: &str) -> String {
    sysfs_join(&["bus", "pci", "drivers", driver, "bind"])
}

/// `/…/bus/pci/drivers/{driver}/unbind` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_driver_unbind(driver: &str) -> String {
    sysfs_join(&["bus", "pci", "drivers", driver, "unbind"])
}

/// `/…/bus/pci/drivers/{driver}/new_id` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_driver_new_id(driver: &str) -> String {
    sysfs_join(&["bus", "pci", "drivers", driver, "new_id"])
}

/// `/…/bus/pci/drivers/{driver}/remove_id` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_driver_remove_id(driver: &str) -> String {
    sysfs_join(&["bus", "pci", "drivers", driver, "remove_id"])
}

/// `/…/bus/pci/rescan` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_bus_rescan() -> String {
    sysfs_join(&["bus", "pci", "rescan"])
}

/// `/…/bus/pci/drivers_autoprobe` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_drivers_autoprobe() -> String {
    sysfs_join(&["bus", "pci", "drivers_autoprobe"])
}

/// `/…/bus/pci/drivers_probe` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_drivers_probe() -> String {
    sysfs_join(&["bus", "pci", "drivers_probe"])
}

/// `/…/module/{name}` under [`sysfs_root`].
#[must_use]
pub fn sysfs_module_path(name: &str) -> String {
    sysfs_join(&["module", name])
}

/// `/…/module/{name}/parameters/{key}` under [`sysfs_root`].
#[must_use]
pub fn sysfs_module_parameter(name: &str, key: &str) -> String {
    sysfs_join(&["module", name, "parameters", key])
}

/// `/…/class/drm/{node}/device` under [`sysfs_root`].
#[must_use]
pub fn sysfs_class_drm_device(node_name: &str) -> String {
    sysfs_join(&["class", "drm", node_name, "device"])
}

/// `/…/kernel/iommu_groups/{group_id}/devices` under [`sysfs_root`].
#[must_use]
pub fn sysfs_kernel_iommu_group_devices(group_id: u32) -> String {
    let gid = group_id.to_string();
    sysfs_join(&["kernel", "iommu_groups", &gid, "devices"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sysfs_join_builds_path_under_root() {
        let root = sysfs_root();
        let path = sysfs_join(&["bus", "pci", "devices"]);
        assert!(path.starts_with(root));
        assert!(path.ends_with("/bus/pci/devices"));
    }

    #[test]
    fn sysfs_pci_devices_matches_join() {
        assert_eq!(sysfs_pci_devices(), sysfs_join(&["bus", "pci", "devices"]));
    }

    #[test]
    fn sysfs_pci_device_file_appends_tail() {
        let bdf = "0000:01:00.0";
        let config = sysfs_pci_device_file(bdf, "config");
        assert!(config.ends_with("/config"));
        assert!(config.contains(&format!("devices/{bdf}")));
    }

    #[test]
    fn sysfs_pci_driver_new_id_and_remove_id() {
        let new_id = sysfs_pci_driver_new_id("vfio-pci");
        assert!(new_id.ends_with("/vfio-pci/new_id"));
        let remove_id = sysfs_pci_driver_remove_id("vfio-pci");
        assert!(remove_id.ends_with("/vfio-pci/remove_id"));
    }

    #[test]
    fn sysfs_module_parameter_path() {
        let p = sysfs_module_parameter("no_bus_reset", "bdf");
        assert!(p.ends_with("/module/no_bus_reset/parameters/bdf"));
    }
}
