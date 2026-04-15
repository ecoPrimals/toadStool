// SPDX-License-Identifier: AGPL-3.0-or-later
use toadstool_auto_config::intelligent::PlatformInfo;

#[test]
fn test_platform_info_detect() {
    let info = PlatformInfo::detect();

    assert!(!info.os_name.is_empty(), "Should detect OS name");
    assert!(!info.architecture.is_empty(), "Should detect architecture");
    assert_eq!(info.os_name, std::env::consts::OS);
    assert_eq!(info.architecture, std::env::consts::ARCH);
}

#[test]
fn test_platform_info_os_detection() {
    let info = PlatformInfo::detect();

    assert!(
        ["linux", "macos", "windows", "freebsd", "openbsd"].contains(&info.os_name.as_str()),
        "OS should be recognized: {}",
        info.os_name
    );
}

#[test]
fn test_platform_info_architecture_detection() {
    let info = PlatformInfo::detect();

    assert!(
        ["x86_64", "aarch64", "arm", "riscv64"]
            .iter()
            .any(|&a| info.architecture.contains(a)),
        "Architecture should be recognized: {}",
        info.architecture
    );
}

#[test]
fn test_platform_info_clone() {
    let info = PlatformInfo::detect();
    let cloned = info.clone();

    assert_eq!(info.os_name, cloned.os_name);
    assert_eq!(info.architecture, cloned.architecture);
}
