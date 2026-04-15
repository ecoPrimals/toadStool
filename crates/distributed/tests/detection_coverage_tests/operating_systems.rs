// SPDX-License-Identifier: AGPL-3.0-or-later
use super::common::serde_json_roundtrip;
use toadstool_distributed::substrate::OperatingSystemSupport;

#[test]
fn operating_system_support_variants_roundtrip() {
    let samples = vec![
        OperatingSystemSupport::BSD {
            variant: "freebsd".into(),
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::Android {
            version: "14".into(),
            api_level: 34,
            security_patch: "p".into(),
        },
        OperatingSystemSupport::IOS {
            version: "17".into(),
            device_family: "iPhone".into(),
            capabilities: vec![],
        },
        OperatingSystemSupport::FreeRTOS {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::Zephyr {
            version: "1".into(),
            boards: vec![],
        },
        OperatingSystemSupport::VxWorks {
            version: "1".into(),
            bsp: "b".into(),
        },
        OperatingSystemSupport::QNX {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::RTLinux {
            version: "1".into(),
            latency_us: 1.0,
        },
        OperatingSystemSupport::Xenomai {
            version: "1".into(),
            skin: "posix".into(),
        },
        OperatingSystemSupport::Xen {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::VMware {
            product: "ESXi".into(),
            version: "1".into(),
        },
        OperatingSystemSupport::HyperV {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::KVM {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::Plan9 {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::Inferno {
            version: "1".into(),
            features: vec![],
        },
        OperatingSystemSupport::MenuetOS {
            version: "1".into(),
        },
        OperatingSystemSupport::KolibriOS {
            version: "1".into(),
        },
        OperatingSystemSupport::MSDOS {
            version: "6".into(),
        },
        OperatingSystemSupport::OS2 {
            version: "1".into(),
        },
        OperatingSystemSupport::BeOS {
            version: "1".into(),
        },
        OperatingSystemSupport::AmigaOS {
            version: "1".into(),
        },
        OperatingSystemSupport::AtariTOS {
            version: "1".into(),
        },
        OperatingSystemSupport::ZOS {
            version: "1".into(),
            subsystems: vec![],
        },
        OperatingSystemSupport::OpenVMS {
            version: "1".into(),
            clustering: false,
        },
        OperatingSystemSupport::UNICOS {
            version: "1".into(),
            features: vec![],
        },
    ];
    for p in samples {
        let q = serde_json_roundtrip(&p);
        assert_eq!(p, q);
    }
}

#[test]
fn operating_system_linux_macos_windows_roundtrip() {
    let linux = OperatingSystemSupport::Linux {
        distribution: "d".into(),
        kernel_version: "k".into(),
        init_system: "systemd".into(),
        package_manager: "apt".into(),
    };
    assert_eq!(linux, serde_json_roundtrip(&linux));
    let macos = OperatingSystemSupport::MacOS {
        version: "14".into(),
        architecture: "arm64".into(),
        frameworks: vec![],
    };
    assert_eq!(macos, serde_json_roundtrip(&macos));
    let win = OperatingSystemSupport::Windows {
        version: "11".into(),
        edition: "Pro".into(),
        features: vec![],
        subsystems: vec![],
    };
    assert_eq!(win, serde_json_roundtrip(&win));
}
