// SPDX-License-Identifier: AGPL-3.0-or-later
//! /proc and sysfs scanning helpers for pre-flight checks.

use std::os::fd::FromRawFd as _;
use std::path::Path;

use super::GuardedSysfsError;

/// Snapshot of a kernel module's state from `/proc/modules`.
#[derive(Debug, Clone)]
pub struct ModuleSnapshot {
    pub name: String,
    pub size: u64,
    pub refcount: i64,
    pub state: String,
    pub address: String,
    pub is_stuck: bool,
    pub timestamp_ms: u64,
}

impl ModuleSnapshot {
    pub fn is_live(&self) -> bool {
        self.state == "Live" && self.refcount >= 0
    }

    pub fn is_zombie(&self) -> bool {
        self.state == "Unloading" || self.refcount < 0
    }
}

impl std::fmt::Display for ModuleSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}(size={}, ref={}, state={}, stuck={})",
            self.name, self.size, self.refcount, self.state, self.is_stuck
        )
    }
}

/// Take a point-in-time snapshot of a kernel module's state.
///
/// Returns `None` if the module is not loaded (not present in `/proc/modules`).
pub fn module_snapshot(name: &str) -> Option<ModuleSnapshot> {
    let proc_modules = format!("{}/modules", crate::linux_paths::proc_root());
    let contents = std::fs::read_to_string(&proc_modules).ok()?;
    parse_module_snapshot(name, &contents)
}

/// Inner parser for `module_snapshot` — testable without /proc access.
pub(crate) fn parse_module_snapshot(name: &str, contents: &str) -> Option<ModuleSnapshot> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    for line in contents.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 5 && fields[0] == name {
            let size = fields[1].parse::<u64>().unwrap_or(0);
            let refcount = fields[2].parse::<i64>().unwrap_or(0);
            let state = fields[4].to_string();
            let address = fields.get(5).unwrap_or(&"0x0").to_string();
            let is_stuck = state == "Unloading" || state == "Loading" || refcount < 0;
            return Some(ModuleSnapshot {
                name: name.to_string(),
                size,
                refcount,
                state,
                address,
                is_stuck,
                timestamp_ms: now,
            });
        }
    }
    None
}

/// Check whether a kernel module is stuck in the "Unloading" state.
///
/// Parses `/proc/modules` for the named module and checks the state field.
/// Returns `true` if the module is in a stuck state (refcount < 0 or
/// state == "Unloading").
pub fn is_module_stuck(name: &str) -> bool {
    let proc_modules = format!("{}/modules", crate::linux_paths::proc_root());
    let Ok(contents) = std::fs::read_to_string(&proc_modules) else {
        return false;
    };
    parse_module_stuck(name, &contents)
}

/// Inner parser for `is_module_stuck` — testable without /proc access.
pub(crate) fn parse_module_stuck(name: &str, contents: &str) -> bool {
    for line in contents.lines() {
        let fields: Vec<&str> = line.split_whitespace().collect();
        if fields.len() >= 5 && fields[0] == name {
            if fields[4] == "Unloading" || fields[4] == "Loading" {
                tracing::warn!(
                    module = name,
                    state = fields[4],
                    refcount = fields[2],
                    "module in stuck state"
                );
                return true;
            }
            if let Ok(refcount) = fields[2].parse::<i64>()
                && refcount < 0
            {
                tracing::warn!(module = name, refcount, "module has negative refcount");
                return true;
            }
        }
    }
    false
}

/// Resolve the IOMMU group number for a PCI device.
fn iommu_group_number(bdf: &str) -> Option<u32> {
    let link = crate::linux_paths::sysfs_pci_device_file(bdf, "iommu_group");
    std::fs::read_link(&link).ok().and_then(|p| {
        p.file_name()
            .and_then(|n| n.to_str())
            .and_then(|s| s.parse::<u32>().ok())
    })
}

/// Check whether the IOMMU group for a BDF is free of external holders.
///
/// Scans `/proc/*/fd` for open file descriptors pointing to the VFIO
/// group device (`/dev/vfio/{group_id}`). Returns `Ok(())` if no process
/// holds the group, or `Err` with the holding PID.
///
/// This is the pre-flight check that would have prevented the Exp 213
/// cascade: the daemon's own VFIO FDs locked the IOMMU group, blocking
/// nouveau's probe.
pub fn iommu_group_ready(bdf: &str) -> Result<(), GuardedSysfsError> {
    let Some(group_id) = iommu_group_number(bdf) else {
        return Ok(());
    };

    let vfio_path = format!("/dev/vfio/{group_id}");
    let vfio_path_canonical = std::fs::canonicalize(&vfio_path).unwrap_or_default();

    // Quick check via fuser-like scan of /proc
    let proc_root = crate::linux_paths::proc_root();
    let Ok(entries) = std::fs::read_dir(proc_root) else {
        return Ok(());
    };

    for entry in entries.filter_map(|e| e.ok()) {
        let name = entry.file_name();
        let Some(pid_str) = name.to_str() else {
            continue;
        };
        let Ok(pid) = pid_str.parse::<u32>() else {
            continue;
        };

        let fd_dir = crate::linux_paths::proc_pid_fd_dir(pid);
        let Ok(fds) = std::fs::read_dir(&fd_dir) else {
            continue;
        };

        for fd in fds.filter_map(|f| f.ok()) {
            if let Ok(target) = std::fs::read_link(fd.path())
                && (target == Path::new(&vfio_path) || target == vfio_path_canonical)
            {
                return Err(GuardedSysfsError::PreFlightFailed {
                    reason: format!("IOMMU group {group_id} held by PID {pid} (fd → {vfio_path})"),
                });
            }
        }
    }

    Ok(())
}

/// Close all leaked sysfs `resource0` file descriptors for a PCI device
/// held by the current process.
///
/// The sovereign pipeline opens BAR0 via sysfs `resource0` for health
/// monitoring and profiling. These fds are intentionally leaked by
/// `MappedBar` (the mmap outlives the File). Before a warm handoff, the
/// kernel's `request_mem_region()` in the seeder driver will fail if any
/// process still holds the BAR region open. This function scans
/// `/proc/self/fd` and closes matching descriptors.
///
/// Returns the number of fds closed.
pub fn release_bar0_fds(bdf: &str) -> usize {
    let resource_path = crate::linux_paths::sysfs_pci_device_file(bdf, "resource0");
    let resource_canonical = std::fs::canonicalize(&resource_path).ok();

    let self_fd_dir = format!("{}/self/fd", crate::linux_paths::proc_root());
    let Ok(entries) = std::fs::read_dir(&self_fd_dir) else {
        return 0;
    };

    let mut closed = 0;
    for entry in entries.filter_map(|e| e.ok()) {
        let Ok(target) = std::fs::read_link(entry.path()) else {
            continue;
        };
        let matches = target == Path::new(&resource_path)
            || resource_canonical.as_ref().is_some_and(|c| target == **c);
        if !matches {
            continue;
        }
        let Some(fd_num) = entry
            .file_name()
            .to_str()
            .and_then(|s| s.parse::<i32>().ok())
        else {
            continue;
        };
        // SAFETY: we own this fd (it's in /proc/self/fd). Closing a leaked
        // sysfs resource0 fd is safe — the corresponding MmioRegion mmap
        // remains valid until munmap (the kernel keeps the mapping alive
        // independently of the fd). The fd was leaked intentionally by
        // MappedBar; we are reclaiming it before driver rotation.
        unsafe {
            drop(std::os::fd::OwnedFd::from_raw_fd(fd_num));
        }
        closed += 1;
        tracing::info!(bdf, fd = fd_num, "closed leaked BAR0 resource0 fd");
    }
    closed
}
