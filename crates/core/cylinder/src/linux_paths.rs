// SPDX-License-Identifier: AGPL-3.0-or-later
//! Linux sysfs and procfs layout roots for portable deployments and tests.
//!
//! Environment:
//! - `TOADSTOOL_SYSFS_ROOT` — sysfs mount (default `/sys`).
//! - `TOADSTOOL_PROC_ROOT` — procfs mount (default `/proc`).
//! - `TOADSTOOL_DATA_DIR` — optional data directory for dumps.
//!
//! Legacy `CORALREEF_*` equivalents are accepted as fallback with a deprecation warning.

use std::sync::OnceLock;

fn resolve_env(primary: &str, legacy: &str, default: &str) -> String {
    if let Some(v) = std::env::var(primary).ok().filter(|s| !s.is_empty()) {
        return v.trim_end_matches('/').to_string();
    }
    if let Some(v) = std::env::var(legacy).ok().filter(|s| !s.is_empty()) {
        tracing::warn!(
            legacy_var = legacy,
            modern_var = primary,
            "deprecated env var — migrate to {primary}"
        );
        return v.trim_end_matches('/').to_string();
    }
    default.to_string()
}

fn sysfs_root_storage() -> &'static str {
    static ROOT: OnceLock<String> = OnceLock::new();
    ROOT.get_or_init(|| resolve_env("TOADSTOOL_SYSFS_ROOT", "CORALREEF_SYSFS_ROOT", "/sys"))
        .as_str()
}

fn proc_root_storage() -> &'static str {
    static ROOT: OnceLock<String> = OnceLock::new();
    ROOT.get_or_init(|| resolve_env("TOADSTOOL_PROC_ROOT", "CORALREEF_PROC_ROOT", "/proc"))
        .as_str()
}

/// Resolved sysfs mount path (default `/sys`).
#[must_use]
pub fn sysfs_root() -> &'static str {
    sysfs_root_storage()
}

/// Resolved procfs mount path (default `/proc`).
#[must_use]
pub fn proc_root() -> &'static str {
    proc_root_storage()
}

/// Running kernel release string (e.g. `"6.17.9-76061709-generic"`).
///
/// Pure-Rust alternative to `uname -r`: reads `/proc/sys/kernel/osrelease`
/// once and caches the result. No process fork, no shell — the kernel
/// release goes through the Rust compiler like everything else.
///
/// Returns `None` only if procfs is unavailable (container without /proc,
/// non-Linux, or test environment with overridden proc root).
#[must_use]
pub fn kernel_release() -> Option<&'static str> {
    static KREL: OnceLock<Option<String>> = OnceLock::new();
    KREL.get_or_init(|| {
        let path = format!("{}/sys/kernel/osrelease", proc_root());
        std::fs::read_to_string(path)
            .ok()
            .map(|s| s.trim().to_string())
    })
    .as_deref()
}

/// Kernel build directory for out-of-tree module compilation.
///
/// Returns `/lib/modules/{krel}/build` where `{krel}` is from
/// [`kernel_release`]. This is the standard kbuild entry point.
#[must_use]
pub fn kbuild_dir() -> Option<String> {
    kernel_release().map(|krel| format!("/lib/modules/{krel}/build"))
}

/// Optional data directory for VBIOS dumps and similar assets.
#[must_use]
pub fn optional_data_dir() -> Option<String> {
    if let Some(v) = std::env::var("TOADSTOOL_DATA_DIR").ok().filter(|s| !s.is_empty()) {
        return Some(v);
    }
    if let Some(v) = std::env::var("CORALREEF_DATA_DIR").ok().filter(|s| !s.is_empty()) {
        tracing::warn!("deprecated env var CORALREEF_DATA_DIR — migrate to TOADSTOOL_DATA_DIR");
        return Some(v);
    }
    None
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

/// `/…/bus/pci/rescan` under [`sysfs_root`].
#[must_use]
pub fn sysfs_pci_bus_rescan() -> String {
    sysfs_join(&["bus", "pci", "rescan"])
}

/// `/…/bus/pci/drivers_autoprobe` under [`sysfs_root`].
///
/// Writing `0` disables automatic driver probing on bus rescan;
/// writing `1` re-enables it.
#[must_use]
pub fn sysfs_pci_drivers_autoprobe() -> String {
    sysfs_join(&["bus", "pci", "drivers_autoprobe"])
}

/// `/…/bus/pci/drivers_probe` under [`sysfs_root`].
///
/// Writing a BDF triggers driver matching for that device, honoring
/// `driver_override` if set.
#[must_use]
pub fn sysfs_pci_drivers_probe() -> String {
    sysfs_join(&["bus", "pci", "drivers_probe"])
}

/// `/…/module/{name}` under [`sysfs_root`].
#[must_use]
pub fn sysfs_module_path(name: &str) -> String {
    sysfs_join(&["module", name])
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

/// `{proc_root()}/{pid}/fd`.
#[must_use]
pub fn proc_pid_fd_dir(pid: u32) -> String {
    format!("{}/{pid}/fd", proc_root())
}

/// `{proc_root()}/self/fd/{fd}`.
#[must_use]
pub fn proc_self_fd(fd: i32) -> String {
    format!("{}/self/fd/{fd}", proc_root())
}

/// `{proc_root()}/cmdline`.
#[must_use]
pub fn proc_cmdline() -> String {
    format!("{}/cmdline", proc_root())
}

/// Discover the VFIO cdev name for a PCI device (kernel 6.2+).
#[must_use]
pub fn sysfs_vfio_cdev_name(bdf: &str) -> Option<String> {
    let dir = sysfs_pci_device_file(bdf, "vfio-dev");
    std::fs::read_dir(dir)
        .ok()?
        .next()?
        .ok()?
        .file_name()
        .into_string()
        .ok()
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
    fn sysfs_join_trims_slashes_in_segments() {
        let path = sysfs_join(&["/class/drm/", "renderD128"]);
        assert!(path.contains("/class/drm/renderD128"));
    }

    #[test]
    fn sysfs_pci_devices_matches_join() {
        assert_eq!(sysfs_pci_devices(), sysfs_join(&["bus", "pci", "devices"]));
    }

    #[test]
    fn sysfs_pci_device_path_various_bdfs() {
        for bdf in ["0000:03:00.0", "0000:0a:00.1", "0000:41:00.0"] {
            let p = sysfs_pci_device_path(bdf);
            assert!(p.ends_with(&format!("/bus/pci/devices/{bdf}")));
        }
    }

    #[test]
    fn sysfs_pci_device_file_appends_tail() {
        let bdf = "0000:01:00.0";
        let config = sysfs_pci_device_file(bdf, "config");
        assert!(config.ends_with("/config"));
        assert!(config.contains(&format!("devices/{bdf}")));
    }

    #[test]
    fn sysfs_pci_device_file_empty_tail_is_base_only() {
        let bdf = "0000:02:00.0";
        assert_eq!(sysfs_pci_device_file(bdf, ""), sysfs_pci_device_path(bdf));
    }

    #[test]
    fn sysfs_pci_driver_bind_unbind() {
        let bind = sysfs_pci_driver_bind("vfio-pci");
        assert!(bind.ends_with("/vfio-pci/bind"));
        let unbind = sysfs_pci_driver_unbind("vfio-pci");
        assert!(unbind.ends_with("/vfio-pci/unbind"));
    }

    #[test]
    fn sysfs_pci_bus_rescan_path() {
        assert!(sysfs_pci_bus_rescan().ends_with("/bus/pci/rescan"));
    }

    #[test]
    fn sysfs_pci_drivers_autoprobe_path() {
        assert!(sysfs_pci_drivers_autoprobe().ends_with("/bus/pci/drivers_autoprobe"));
    }

    #[test]
    fn sysfs_pci_drivers_probe_path() {
        assert!(sysfs_pci_drivers_probe().ends_with("/bus/pci/drivers_probe"));
    }

    #[test]
    fn sysfs_module_path_nvidia() {
        assert!(sysfs_module_path("nvidia").ends_with("/module/nvidia"));
    }

    #[test]
    fn sysfs_class_drm_device_card0() {
        assert!(sysfs_class_drm_device("card0").ends_with("/class/drm/card0/device"));
    }

    #[test]
    fn sysfs_kernel_iommu_group_42() {
        assert!(sysfs_kernel_iommu_group_devices(42).ends_with("/kernel/iommu_groups/42/devices"));
    }

    #[test]
    fn proc_pid_fd_dir_and_self_fd() {
        let fd_dir = proc_pid_fd_dir(1234);
        assert!(fd_dir.ends_with("/1234/fd"));
        assert!(fd_dir.starts_with(proc_root()));
        let self_fd = proc_self_fd(7);
        assert!(self_fd.ends_with("/self/fd/7"));
    }

    #[test]
    fn proc_cmdline_path() {
        let c = proc_cmdline();
        assert!(c.ends_with("/cmdline"));
        assert!(c.starts_with(proc_root()));
    }

    #[test]
    fn sysfs_join_single_segment() {
        let path = sysfs_join(&["kernel"]);
        assert_eq!(path, format!("{}/kernel", sysfs_root()));
    }

    #[test]
    fn kernel_release_returns_nonempty_string() {
        if let Some(krel) = kernel_release() {
            assert!(!krel.is_empty());
            assert!(!krel.contains('\n'));
            assert!(!krel.contains(' '));
        }
    }

    #[test]
    fn kbuild_dir_matches_kernel_release() {
        if let (Some(krel), Some(dir)) = (kernel_release(), kbuild_dir()) {
            assert!(dir.contains(krel));
            assert!(dir.starts_with("/lib/modules/"));
            assert!(dir.ends_with("/build"));
        }
    }
}
