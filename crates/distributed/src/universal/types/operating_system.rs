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
    // Unix-like systems
    Linux {
        distribution: String,
        kernel_version: String,
        init_system: String,
        package_manager: String,
    },
    BSD {
        variant: String,
        version: String,
        features: Vec<String>,
    },
    MacOS {
        version: String,
        architecture: String,
        frameworks: Vec<String>,
    },

    // Windows systems
    Windows {
        version: String,
        edition: String,
        features: Vec<String>,
        subsystems: Vec<String>,
    },

    // Mobile systems
    Android {
        version: String,
        api_level: u32,
        security_patch: String,
    },
    #[serde(rename = "iOS")]
    IOS {
        version: String,
        device_family: String,
        capabilities: Vec<String>,
    },

    // Embedded systems
    FreeRTOS {
        version: String,
        features: Vec<String>,
    },
    Zephyr {
        version: String,
        boards: Vec<String>,
    },
    VxWorks {
        version: String,
        bsp: String,
    },
    QNX {
        version: String,
        features: Vec<String>,
    },

    // Real-time systems
    RTLinux {
        version: String,
        latency_us: f64,
    },
    Xenomai {
        version: String,
        skin: String,
    },

    // Hypervisors
    Xen {
        version: String,
        features: Vec<String>,
    },
    VMware {
        product: String,
        version: String,
    },
    HyperV {
        version: String,
        features: Vec<String>,
    },
    KVM {
        version: String,
        features: Vec<String>,
    },

    // Exotic systems
    Plan9 {
        version: String,
        features: Vec<String>,
    },
    Inferno {
        version: String,
        features: Vec<String>,
    },
    TempleOS {
        version: String,
    },
    MenuetOS {
        version: String,
    },
    KolibriOS {
        version: String,
    },

    // Legacy systems
    MSDOS {
        version: String,
    },
    OS2 {
        version: String,
    },
    BeOS {
        version: String,
    },
    AmigaOS {
        version: String,
    },
    AtariTOS {
        version: String,
    },

    // Mainframe systems
    #[serde(rename = "z/OS")]
    ZOS {
        version: String,
        subsystems: Vec<String>,
    },
    OpenVMS {
        version: String,
        clustering: bool,
    },
    UNICOS {
        version: String,
        features: Vec<String>,
    },
}

impl OperatingSystemSupport {
    /// Get the OS name
    pub fn os_name(&self) -> &'static str {
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
