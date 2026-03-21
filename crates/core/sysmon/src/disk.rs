// SPDX-License-Identifier: AGPL-3.0-only
//! Disk monitoring via `/proc/mounts` + `statvfs`.

use crate::error::{Result, SysmonError};

/// Information about a mounted filesystem.
#[derive(Debug, Clone)]
pub struct DiskInfo {
    /// Path where the filesystem is mounted (e.g. `/` or `/home`).
    pub mount_point: String,
    /// Filesystem type (e.g. `ext4`, `xfs`, `btrfs`).
    pub filesystem: String,
    /// Total capacity in bytes.
    pub total_space: u64,
    /// Free space available to unprivileged users in bytes.
    pub available_space: u64,
}

const VIRTUAL_FILESYSTEMS: &[&str] = &[
    "tmpfs",
    "devtmpfs",
    "sysfs",
    "proc",
    "cgroup",
    "cgroup2",
    "debugfs",
    "securityfs",
    "pstore",
    "efivarfs",
    "bpf",
    "tracefs",
    "hugetlbfs",
    "mqueue",
    "fusectl",
    "configfs",
    "devpts",
    "ramfs",
    "squashfs",
    "overlay",
    "autofs",
    "rpc_pipefs",
    "nfsd",
    "binfmt_misc",
    "fuse.portal",
    "fuse.gvfsd-fuse",
    "nsfs",
];

/// List real (non-virtual) mounted filesystems with space information.
///
/// On Linux, parses `/proc/mounts` and calls `statvfs` on each real mount.
/// Virtual filesystems (tmpfs, sysfs, proc, etc.) are excluded.
///
/// # Errors
///
/// Returns an error if `/proc/mounts` cannot be read.
#[cfg(target_os = "linux")]
pub fn disk_usage() -> Result<Vec<DiskInfo>> {
    let content =
        std::fs::read_to_string("/proc/mounts").map_err(|e| SysmonError::new("/proc/mounts", e))?;
    let mut disks = Vec::new();

    for line in content.lines() {
        let mut fields = line.split_whitespace();
        let _device = fields.next().unwrap_or("");
        let mount_point = fields.next().unwrap_or("");
        let filesystem = fields.next().unwrap_or("");

        if mount_point.is_empty() || VIRTUAL_FILESYSTEMS.contains(&filesystem) {
            continue;
        }
        // Skip /proc, /sys, /dev sub-mounts
        if mount_point.starts_with("/proc")
            || mount_point.starts_with("/sys")
            || mount_point.starts_with("/dev")
        {
            continue;
        }

        if let Ok(stat) = rustix::fs::statvfs(mount_point) {
            let block_size = stat.f_frsize;
            let total = stat.f_blocks * block_size;
            let available = stat.f_bavail * block_size;
            if total > 0 {
                disks.push(DiskInfo {
                    mount_point: mount_point.to_string(),
                    filesystem: filesystem.to_string(),
                    total_space: total,
                    available_space: available,
                });
            }
        }
    }

    Ok(disks)
}

#[cfg(not(target_os = "linux"))]
pub fn disk_usage() -> Result<Vec<DiskInfo>> {
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "linux")]
    fn test_disk_usage_returns_results() {
        let disks = disk_usage().unwrap();
        // Should have at least root filesystem on any Linux system
        assert!(!disks.is_empty(), "should find at least one disk");
        for disk in &disks {
            assert!(disk.total_space > 0);
            assert!(disk.available_space <= disk.total_space);
            assert!(!disk.mount_point.is_empty());
            assert!(!disk.filesystem.is_empty());
        }
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_virtual_fs_excluded() {
        let disks = disk_usage().unwrap();
        for disk in &disks {
            assert!(
                !VIRTUAL_FILESYSTEMS.contains(&disk.filesystem.as_str()),
                "virtual fs {} should be excluded",
                disk.filesystem
            );
        }
    }

    #[test]
    #[cfg(not(target_os = "linux"))]
    fn test_disk_usage_non_linux_returns_empty() {
        let disks = disk_usage().unwrap();
        assert!(disks.is_empty(), "non-Linux should return empty disk list");
    }

    #[test]
    fn test_disk_usage_no_error() {
        let result = disk_usage();
        assert!(result.is_ok());
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_disk_info_root_typically_present() {
        let disks = disk_usage().unwrap();
        let has_root = disks.iter().any(|d| d.mount_point == "/");
        assert!(has_root, "root mount should typically be present on Linux");
    }
}
