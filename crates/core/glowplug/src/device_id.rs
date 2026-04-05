// SPDX-License-Identifier: AGPL-3.0-or-later

//! Hardware-agnostic device identity.
//!
//! [`DeviceId`] represents a stable identity for any hardware device,
//! independent of bus type. PCI devices use BDF notation, USB devices
//! use bus-port paths, sysfs devices use their canonical path, etc.

use std::fmt;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Stable identity for any hardware device, independent of bus type.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DeviceId {
    /// PCI Bus-Device-Function (e.g. `"0000:01:00.0"`).
    PciBdf(String),
    /// USB bus-port path (e.g. `"1-2.3"`).
    UsbPath(String),
    /// Canonical sysfs path (e.g. `/sys/class/net/eth0`).
    SysfsPath(PathBuf),
    /// Device node (e.g. `/dev/dri/card0`, `/dev/ttyUSB0`).
    DevNode(PathBuf),
    /// Unique serial number or hardware identifier.
    Serial(String),
    /// Platform-specific or bus-agnostic identifier.
    Platform(String),
}

impl DeviceId {
    /// Short human-readable label for the device.
    #[must_use]
    pub fn short_label(&self) -> String {
        match self {
            Self::PciBdf(bdf) => format!("pci:{bdf}"),
            Self::UsbPath(path) => format!("usb:{path}"),
            Self::SysfsPath(path) => format!(
                "sysfs:{}",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            ),
            Self::DevNode(path) => format!(
                "dev:{}",
                path.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            ),
            Self::Serial(s) => format!("serial:{}", &s[..s.len().min(12)]),
            Self::Platform(p) => format!("platform:{p}"),
        }
    }

    /// The bus/class category for grouping.
    #[must_use]
    pub const fn bus_class(&self) -> &str {
        match self {
            Self::PciBdf(_) => "pci",
            Self::UsbPath(_) => "usb",
            Self::SysfsPath(_) => "sysfs",
            Self::DevNode(_) => "devnode",
            Self::Serial(_) => "serial",
            Self::Platform(_) => "platform",
        }
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.short_label())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_formats() {
        let pci = DeviceId::PciBdf("0000:01:00.0".into());
        assert_eq!(pci.to_string(), "pci:0000:01:00.0");
        assert_eq!(pci.bus_class(), "pci");

        let usb = DeviceId::UsbPath("1-2.3".into());
        assert_eq!(usb.to_string(), "usb:1-2.3");

        let dev = DeviceId::DevNode(PathBuf::from("/dev/ttyUSB0"));
        assert_eq!(dev.to_string(), "dev:ttyUSB0");
    }

    #[test]
    fn equality_and_hash() {
        let a = DeviceId::PciBdf("0000:01:00.0".into());
        let b = DeviceId::PciBdf("0000:01:00.0".into());
        assert_eq!(a, b);

        let c = DeviceId::PciBdf("0000:02:00.0".into());
        assert_ne!(a, c);
    }

    #[test]
    fn serde_roundtrip() {
        let id = DeviceId::Serial("ABC123DEF456".into());
        let json = serde_json::to_string(&id).unwrap();
        let back: DeviceId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, back);
    }
}
