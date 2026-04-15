// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_security_sandbox::*;

#[test]
fn test_mount_type_readonly_bind() {
    let mount_type = MountType::ReadOnlyBind;
    assert!(matches!(mount_type, MountType::ReadOnlyBind));
}

#[test]
fn test_mount_type_readwrite_bind() {
    let mount_type = MountType::ReadWriteBind;
    assert!(matches!(mount_type, MountType::ReadWriteBind));
}

#[test]
fn test_mount_type_tmpfs() {
    let mount_type = MountType::TmpFs;
    assert!(matches!(mount_type, MountType::TmpFs));
}

#[test]
fn test_mount_type_device() {
    let mount_type = MountType::Device;
    assert!(matches!(mount_type, MountType::Device));
}

#[test]
fn test_mount_type_proc() {
    let mount_type = MountType::Proc;
    assert!(matches!(mount_type, MountType::Proc));
}

#[test]
fn test_mount_type_equality() {
    let ro1 = MountType::ReadOnlyBind;
    let ro2 = MountType::ReadOnlyBind;

    assert!(matches!(ro1, MountType::ReadOnlyBind));
    assert!(matches!(ro2, MountType::ReadOnlyBind));
}

#[test]
fn test_mount_type_tmpfs_match() {
    let mount_type = MountType::TmpFs;
    assert!(matches!(mount_type, MountType::TmpFs));
}
