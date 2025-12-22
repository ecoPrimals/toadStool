// ============================================================================
// Filesystem Mount Tests
// ============================================================================

#[test]
fn test_filesystem_mount_readonly() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/data"),
        target: PathBuf::from("/sandbox/data"),
        mount_type: MountType::ReadOnlyBind,
        options: vec!["ro".to_string(), "noexec".to_string()],
    };

    assert_eq!(mount.source, PathBuf::from("/host/data"));
    assert_eq!(mount.target, PathBuf::from("/sandbox/data"));
    assert!(matches!(mount.mount_type, MountType::ReadOnlyBind));
    assert_eq!(mount.options.len(), 2);
}

#[test]
fn test_filesystem_mount_readwrite() {
    let mount = FilesystemMount {
        source: PathBuf::from("/host/work"),
        target: PathBuf::from("/sandbox/work"),
        mount_type: MountType::ReadWriteBind,
        options: vec!["rw".to_string()],
    };

    assert!(matches!(mount.mount_type, MountType::ReadWriteBind));
    assert_eq!(mount.options.len(), 1);
}

#[test]
fn test_filesystem_mount_tmpfs() {
    let mount = FilesystemMount {
        source: PathBuf::from("tmpfs"),
        target: PathBuf::from("/sandbox/tmp"),
        mount_type: MountType::TmpFs,
        options: vec!["size=100M".to_string()],
    };

    assert!(matches!(mount.mount_type, MountType::TmpFs));
    assert!(mount.options.iter().any(|o| o.contains("size")));
}

#[test]
fn test_filesystem_mount_device() {
    let mount = FilesystemMount {
        source: PathBuf::from("/dev/null"),
        target: PathBuf::from("/sandbox/dev/null"),
        mount_type: MountType::Device,
        options: vec![],
    };

    assert!(matches!(mount.mount_type, MountType::Device));
}

#[test]
fn test_filesystem_mount_proc() {
    let mount = FilesystemMount {
        source: PathBuf::from("proc"),
        target: PathBuf::from("/sandbox/proc"),
        mount_type: MountType::Proc,
        options: vec![],
    };

    assert!(matches!(mount.mount_type, MountType::Proc));
}

