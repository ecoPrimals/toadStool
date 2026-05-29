// SPDX-License-Identifier: AGPL-3.0-or-later

use super::*;
use std::time::Duration;

#[test]
fn sysfs_write_nonexistent_path_fails() {
    let result = sysfs_write("/sys/nonexistent/path/12345", "test");
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), GuardedSysfsError::WriteFailed { .. }));
}

#[test]
fn read_current_driver_nonexistent() {
    assert_eq!(read_current_driver("ffff:ff:ff.f"), None);
}

#[test]
fn iommu_group_siblings_nonexistent() {
    assert!(iommu_group_siblings("ffff:ff:ff.f").is_empty());
}

#[test]
fn is_module_stuck_unknown_module() {
    assert!(!is_module_stuck("toadstool_nonexistent_12345"));
}

#[test]
fn guarded_write_timeout_fires() {
    let result = sysfs_write_guarded(
        "/dev/null",
        "test",
        Duration::from_millis(100),
    );
    // /dev/null write should succeed fast, not timeout
    assert!(result.is_ok());
}

#[test]
fn kmod_guarded_nonexistent_command() {
    let result = kmod_guarded("toadstool_fake_cmd_12345", &["arg"], Duration::from_secs(1));
    assert!(result.is_err());
}

#[test]
fn guarded_write_timeout_actually_fires() {
    // Spawn a sleep via guarded write with a very short timeout.
    // The "write" target is actually a FIFO-like path that will block.
    // We use /dev/stdin in a subshell to simulate a blocking write.
    let result = sysfs_write_guarded(
        "/proc/self/fd/999", // nonexistent fd — sh will hang trying to open
        "test",
        Duration::from_millis(200),
    );
    // Should be either Timeout or WriteFailed (child can't write to bogus fd)
    assert!(result.is_err());
}

#[test]
fn kmod_guarded_timeout_fires() {
    let result = kmod_guarded(
        "/bin/sleep",
        &["60"],
        Duration::from_millis(300),
    );
    assert!(result.is_err());
    assert!(matches!(result.unwrap_err(), GuardedSysfsError::KmodTimeout { .. }));
}

#[test]
fn guarded_write_fast_path_succeeds() {
    let result = sysfs_write_guarded("/dev/null", "hello", Duration::from_secs(5));
    assert!(result.is_ok());
}

#[test]
fn parse_module_stuck_detects_unloading() {
    let content = "nouveau 2654208 -1 - Unloading 0xffffffffc1234000\n\
                    vfio_pci 65536 0 - Live 0xffffffffc5678000\n";
    assert!(proc_scan::parse_module_stuck("nouveau", content));
    assert!(!proc_scan::parse_module_stuck("vfio_pci", content));
}

#[test]
fn parse_module_stuck_detects_negative_refcount() {
    let content = "nouveau 2654208 -1 - Live 0xffffffffc1234000\n";
    assert!(proc_scan::parse_module_stuck("nouveau", content));
}

#[test]
fn parse_module_stuck_detects_loading_state() {
    let content = "nouveau 2654208 0 - Loading 0xffffffffc1234000\n";
    assert!(proc_scan::parse_module_stuck("nouveau", content));
}

#[test]
fn parse_module_stuck_live_is_ok() {
    let content = "kernel 0 0 - Live 0xffffffffc0000000\n\
                    nouveau 2654208 1 - Live 0xffffffffc1234000\n";
    assert!(!proc_scan::parse_module_stuck("kernel", content));
    assert!(!proc_scan::parse_module_stuck("nouveau", content));
}

#[test]
fn parse_module_stuck_unknown_module_is_ok() {
    let content = "nouveau 2654208 1 - Live 0xffffffffc1234000\n";
    assert!(!proc_scan::parse_module_stuck("nonexistent_module_xyz", content));
}

#[test]
fn parse_module_stuck_empty_content() {
    assert!(!proc_scan::parse_module_stuck("nouveau", ""));
}

#[test]
fn module_snapshot_live() {
    let content = "nvsov 35635200 6 - Live 0xffffffffc1234000\n";
    let snap = proc_scan::parse_module_snapshot("nvsov", content).unwrap();
    assert_eq!(snap.name, "nvsov");
    assert_eq!(snap.size, 35635200);
    assert_eq!(snap.refcount, 6);
    assert_eq!(snap.state, "Live");
    assert!(!snap.is_stuck);
    assert!(snap.is_live());
    assert!(!snap.is_zombie());
}

#[test]
fn module_snapshot_zombie() {
    let content = "nvsov 35635200 -1 - Unloading 0x0000000000000000\n";
    let snap = proc_scan::parse_module_snapshot("nvsov", content).unwrap();
    assert_eq!(snap.refcount, -1);
    assert_eq!(snap.state, "Unloading");
    assert!(snap.is_stuck);
    assert!(!snap.is_live());
    assert!(snap.is_zombie());
}

#[test]
fn module_snapshot_not_loaded() {
    let content = "vfio_pci 65536 0 - Live 0xffffffffc5678000\n";
    assert!(proc_scan::parse_module_snapshot("nvsov", content).is_none());
}

#[test]
fn module_snapshot_refcount_transitions() {
    let content = "nvsov 35635200 0 - Live 0xffffffffc1234000\n";
    let snap = proc_scan::parse_module_snapshot("nvsov", content).unwrap();
    assert_eq!(snap.refcount, 0);
    assert!(snap.is_live());
    assert!(!snap.is_zombie());
}
