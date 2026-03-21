// SPDX-License-Identifier: AGPL-3.0-only
//! Operating system support
//!
//! Support for various operating systems including Unix-like, Windows, mobile,
//! embedded, real-time, hypervisors, exotic, legacy, and mainframe systems.

use serde::{Deserialize, Serialize};

/// Operating system support
///
/// Represents various operating systems and their capabilities.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[non_exhaustive]
pub enum OperatingSystemSupport {
    /// Linux distribution.
    Linux {
        /// Distribution (ubuntu, debian, etc.).
        distribution: String,
        /// Kernel version.
        kernel_version: String,
        /// Init system (systemd, openrc, etc.).
        init_system: String,
        /// Package manager (apt, dnf, etc.).
        package_manager: String,
    },
    /// BSD variant.
    BSD {
        /// Variant (freebsd, openbsd, etc.).
        variant: String,
        /// Version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// macOS.
    MacOS {
        /// macOS version.
        version: String,
        /// Architecture (arm64, x86_64).
        architecture: String,
        /// Available frameworks.
        frameworks: Vec<String>,
    },
    /// Windows.
    Windows {
        /// Windows version.
        version: String,
        /// Edition (home, pro, enterprise).
        edition: String,
        /// Enabled features.
        features: Vec<String>,
        /// Subsystems (WSL, etc.).
        subsystems: Vec<String>,
    },
    /// Android mobile OS.
    Android {
        /// Android version.
        version: String,
        /// API level.
        api_level: u32,
        /// Security patch level.
        security_patch: String,
    },
    /// iOS mobile OS.
    #[serde(rename = "iOS")]
    IOS {
        /// iOS version.
        version: String,
        /// Device family (iPhone, iPad).
        device_family: String,
        /// Capabilities.
        capabilities: Vec<String>,
    },
    /// FreeRTOS embedded.
    FreeRTOS {
        /// FreeRTOS version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Zephyr RTOS.
    Zephyr {
        /// Zephyr version.
        version: String,
        /// Supported boards.
        boards: Vec<String>,
    },
    /// VxWorks RTOS.
    VxWorks {
        /// VxWorks version.
        version: String,
        /// BSP identifier.
        bsp: String,
    },
    /// QNX RTOS.
    QNX {
        /// QNX version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// RTLinux real-time.
    RTLinux {
        /// RTLinux version.
        version: String,
        /// Latency in microseconds.
        latency_us: f64,
    },
    /// Xenomai real-time.
    Xenomai {
        /// Xenomai version.
        version: String,
        /// Skin (e.g. posix, native).
        skin: String,
    },
    /// Xen hypervisor.
    Xen {
        /// Xen version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// VMware hypervisor.
    VMware {
        /// Product (ESXi, Workstation, etc.).
        product: String,
        /// Version.
        version: String,
    },
    /// Hyper-V hypervisor.
    HyperV {
        /// Hyper-V version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// KVM hypervisor.
    KVM {
        /// KVM version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Plan 9 from Bell Labs.
    Plan9 {
        /// Plan 9 version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// Inferno OS.
    Inferno {
        /// Inferno version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
    /// TempleOS.
    TempleOS {
        /// TempleOS version.
        version: String,
    },
    /// MenuetOS.
    MenuetOS {
        /// MenuetOS version.
        version: String,
    },
    /// KolibriOS.
    KolibriOS {
        /// KolibriOS version.
        version: String,
    },
    /// MS-DOS legacy.
    MSDOS {
        /// MS-DOS version.
        version: String,
    },
    /// OS/2 legacy.
    OS2 {
        /// OS/2 version.
        version: String,
    },
    /// BeOS legacy.
    BeOS {
        /// BeOS version.
        version: String,
    },
    /// AmigaOS legacy.
    AmigaOS {
        /// AmigaOS version.
        version: String,
    },
    /// Atari TOS legacy.
    AtariTOS {
        /// Atari TOS version.
        version: String,
    },
    /// IBM z/OS mainframe.
    #[serde(rename = "z/OS")]
    ZOS {
        /// z/OS version.
        version: String,
        /// Subsystems (CICS, IMS, etc.).
        subsystems: Vec<String>,
    },
    /// OpenVMS.
    OpenVMS {
        /// OpenVMS version.
        version: String,
        /// Clustering enabled.
        clustering: bool,
    },
    /// UNICOS (Cray).
    UNICOS {
        /// UNICOS version.
        version: String,
        /// Enabled features.
        features: Vec<String>,
    },
}

impl OperatingSystemSupport {
    /// Get the OS name
    pub const fn os_name(&self) -> &'static str {
        match self {
            Self::Linux { .. } => "Linux",
            Self::BSD { .. } => "BSD",
            Self::MacOS { .. } => "macOS",
            Self::Windows { .. } => "Windows",
            Self::Android { .. } => "Android",
            Self::IOS { .. } => "iOS",
            Self::FreeRTOS { .. } => "FreeRTOS",
            Self::Zephyr { .. } => "Zephyr",
            Self::VxWorks { .. } => "VxWorks",
            Self::QNX { .. } => "QNX",
            Self::RTLinux { .. } => "RTLinux",
            Self::Xenomai { .. } => "Xenomai",
            Self::Xen { .. } => "Xen",
            Self::VMware { .. } => "VMware",
            Self::HyperV { .. } => "Hyper-V",
            Self::KVM { .. } => "KVM",
            Self::Plan9 { .. } => "Plan 9",
            Self::Inferno { .. } => "Inferno",
            Self::TempleOS { .. } => "TempleOS",
            Self::MenuetOS { .. } => "MenuetOS",
            Self::KolibriOS { .. } => "KolibriOS",
            Self::MSDOS { .. } => "MS-DOS",
            Self::OS2 { .. } => "OS/2",
            Self::BeOS { .. } => "BeOS",
            Self::AmigaOS { .. } => "AmigaOS",
            Self::AtariTOS { .. } => "Atari TOS",
            Self::ZOS { .. } => "z/OS",
            Self::OpenVMS { .. } => "OpenVMS",
            Self::UNICOS { .. } => "UNICOS",
        }
    }

    /// Check if OS is Unix-like
    pub const fn is_unix_like(&self) -> bool {
        matches!(
            self,
            Self::Linux { .. } | Self::BSD { .. } | Self::MacOS { .. }
        )
    }

    /// Check if OS is a mobile platform
    pub const fn is_mobile(&self) -> bool {
        matches!(self, Self::Android { .. } | Self::IOS { .. })
    }

    /// Check if OS is real-time
    pub const fn is_realtime(&self) -> bool {
        matches!(
            self,
            Self::FreeRTOS { .. }
                | Self::Zephyr { .. }
                | Self::VxWorks { .. }
                | Self::QNX { .. }
                | Self::RTLinux { .. }
                | Self::Xenomai { .. }
        )
    }

    /// Check if OS is a hypervisor
    pub const fn is_hypervisor(&self) -> bool {
        matches!(
            self,
            Self::Xen { .. } | Self::VMware { .. } | Self::HyperV { .. } | Self::KVM { .. }
        )
    }

    /// Check if OS is embedded
    pub const fn is_embedded(&self) -> bool {
        matches!(
            self,
            Self::FreeRTOS { .. } | Self::Zephyr { .. } | Self::VxWorks { .. } | Self::QNX { .. }
        )
    }

    /// Check if OS is exotic/experimental
    pub const fn is_exotic(&self) -> bool {
        matches!(
            self,
            Self::Plan9 { .. }
                | Self::Inferno { .. }
                | Self::TempleOS { .. }
                | Self::MenuetOS { .. }
                | Self::KolibriOS { .. }
        )
    }

    /// Check if OS is legacy
    pub const fn is_legacy(&self) -> bool {
        matches!(
            self,
            Self::MSDOS { .. }
                | Self::OS2 { .. }
                | Self::BeOS { .. }
                | Self::AmigaOS { .. }
                | Self::AtariTOS { .. }
        )
    }

    /// Check if OS is a mainframe system
    pub const fn is_mainframe(&self) -> bool {
        matches!(
            self,
            Self::ZOS { .. } | Self::OpenVMS { .. } | Self::UNICOS { .. }
        )
    }

    /// Check if OS supports POSIX
    pub const fn supports_posix(&self) -> bool {
        matches!(
            self,
            Self::Linux { .. }
                | Self::BSD { .. }
                | Self::MacOS { .. }
                | Self::QNX { .. }
                | Self::Android { .. }
        )
    }

    /// Check if OS supports containers
    pub const fn supports_containers(&self) -> bool {
        matches!(self, Self::Linux { .. })
    }

    /// Get real-time latency (if applicable)
    pub const fn realtime_latency_us(&self) -> Option<f64> {
        match self {
            Self::RTLinux { latency_us, .. } => Some(*latency_us),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unix_like() {
        let linux = OperatingSystemSupport::Linux {
            distribution: "Ubuntu".to_string(),
            kernel_version: "6.5.0".to_string(),
            init_system: "systemd".to_string(),
            package_manager: "apt".to_string(),
        };

        assert_eq!(linux.os_name(), "Linux");
        assert!(linux.is_unix_like());
        assert!(linux.supports_posix());
        assert!(linux.supports_containers());
    }

    #[test]
    fn test_mobile() {
        let android = OperatingSystemSupport::Android {
            version: "14".to_string(),
            api_level: 34,
            security_patch: "2024-01".to_string(),
        };

        assert!(android.is_mobile());
        assert!(android.supports_posix());
    }

    #[test]
    fn test_realtime() {
        let freertos = OperatingSystemSupport::FreeRTOS {
            version: "10.5.1".to_string(),
            features: vec!["Tasks".to_string(), "Queues".to_string()],
        };

        assert!(freertos.is_realtime());
        assert!(freertos.is_embedded());
    }

    #[test]
    fn test_hypervisor() {
        let kvm = OperatingSystemSupport::KVM {
            version: "7.2.0".to_string(),
            features: vec!["nested".to_string()],
        };

        assert!(kvm.is_hypervisor());
    }

    #[test]
    fn test_exotic() {
        let temple = OperatingSystemSupport::TempleOS {
            version: "5.03".to_string(),
        };

        assert!(temple.is_exotic());
        assert!(!temple.is_unix_like());
    }

    #[test]
    fn test_mainframe() {
        let zos = OperatingSystemSupport::ZOS {
            version: "2.5".to_string(),
            subsystems: vec!["JES2".to_string(), "CICS".to_string()],
        };

        assert!(zos.is_mainframe());
        assert!(!zos.is_unix_like());
    }

    #[test]
    fn test_realtime_latency() {
        let rtlinux = OperatingSystemSupport::RTLinux {
            version: "4.0".to_string(),
            latency_us: 10.0,
        };

        assert_eq!(rtlinux.realtime_latency_us(), Some(10.0));
    }

    #[test]
    fn test_serialization() {
        let os = OperatingSystemSupport::Windows {
            version: "11".to_string(),
            edition: "Pro".to_string(),
            features: vec!["WSL2".to_string()],
            subsystems: vec!["WSL".to_string()],
        };

        let json = serde_json::to_string(&os).unwrap();
        let deserialized: OperatingSystemSupport = serde_json::from_str(&json).unwrap();

        assert_eq!(os, deserialized);
    }
}
