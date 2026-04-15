// SPDX-License-Identifier: AGPL-3.0-or-later

use toadstool_security_sandbox::*;
use std::path::PathBuf;

#[test]
fn test_filesystem_mount_with_multiple_options() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/data"),
        target: PathBuf::from("/sandbox/data"),
        mount_type: MountType::ReadOnlyBind,
        options: vec!["ro".to_string(), "noexec".to_string(), "nosuid".to_string()],
    };

    assert_eq!(mount.options.len(), 3);
    assert!(mount.options.contains(&"noexec".to_string()));
}

#[test]
fn test_filesystem_mount_clone() {
    let mount1 = FilesystemMount {
        source: PathBuf::from("/test"),
        target: PathBuf::from("/sandbox/test"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![],
    };

    let mount2 = mount1.clone();
    assert_eq!(mount1.source, mount2.source);
    assert_eq!(mount1.target, mount2.target);
}

#[test]
fn test_filesystem_mount_readonly_bind() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/data"),
        target: PathBuf::from("/sandbox/data"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![],
    };

    assert_eq!(mount.source, PathBuf::from("/host/data"));
    assert_eq!(mount.target, PathBuf::from("/sandbox/data"));
    assert!(matches!(mount.mount_type, MountType::ReadOnlyBind));
}

#[test]
fn test_filesystem_mount_readwrite_bind() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/workspace"),
        target: PathBuf::from("/sandbox/workspace"),
        mount_type: MountType::ReadWriteBind,
        options: vec![],
    };

    assert!(matches!(mount.mount_type, MountType::ReadWriteBind));
}

#[test]
fn test_filesystem_mount_tmpfs_with_size_option() {
    let mount = FilesystemMount {
        source: PathBuf::from("none"),
        target: PathBuf::from("/sandbox/tmp"),
        mount_type: MountType::TmpFs,
        options: vec!["size=100m".to_string()],
    };

    assert!(matches!(mount.mount_type, MountType::TmpFs));
    assert_eq!(mount.options.len(), 1);
    assert_eq!(mount.options[0], "size=100m");
}

#[test]
fn test_filesystem_mount_with_options() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/lib"),
        target: PathBuf::from("/sandbox/lib"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![
            "nosuid".to_string(),
            "nodev".to_string(),
            "noexec".to_string(),
        ],
    };

    assert_eq!(mount.options.len(), 3);
    assert!(mount.options.contains(&"nosuid".to_string()));
    assert!(mount.options.contains(&"nodev".to_string()));
}

#[test]
fn test_filesystem_mount_nested_paths() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/app/config"),
        target: PathBuf::from("/sandbox/etc/app"),
        mount_type: MountType::ReadOnlyBind,
        options: vec![],
    };

    assert!(mount.source.to_str().unwrap().contains("config"));
    assert!(mount.target.to_str().unwrap().contains("etc"));
}
